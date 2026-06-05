//! RALL? fixed-length reader and continuous acquisition loop.
//!
//! C4 scope. Handles K3 (no terminator), K4 (macOS ~1020B per read),
//! K6 (~900ms prepare time), and produces [`RawFrameEnvelope`]s that
//! the C5+ acquisition pipeline consumes.
//!
//! ## Design
//!
//! [`RallLink`] is the trait that abstracts "send RALL?, return 12288
//! bytes" — it is implemented by:
//!
//! 1. [`MockRallLink`]: deterministic, in-memory, used in tests and
//!    on developer machines without hardware.
//! 2. [`SerialRallLink`]: backed by a real `serialport::SerialPort`.
//!    Not in this commit; lands with C5 alongside the OS-thread
//!    AcquisitionThread.
//!
//! [`RallReader`] is a stateless helper that drives a [`RallLink`]
//! through one RALL? cycle and returns a single [`RawFrameEnvelope`].
//!
//! [`ContinuousRallLoop`] owns a [`RallLink`] and pushes
//! `RawFrameEnvelope`s into a `crossbeam-channel::Sender` from a
//! dedicated OS thread. Callers (C5 AcquisitionThread) consume
//! from the receiver.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

use std::io::Read;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::constants::{
    RALL_COMMAND, RALL_FRAME_BYTES, RALL_PREPARE_DELAY, RALL_READ_BUFFER_BYTES,
    RALL_READ_DEADLINE,
};
use crate::idn::IdnResponse;

// ---------------------------------------------------------------------------
// RawFrameEnvelope
// ---------------------------------------------------------------------------

/// A complete RALL? frame plus all timing metadata.
///
/// The C5 AcquisitionThread reads these and pushes them to the
/// ParserThread. The C6 ParserThread fans the 50 samples out with
/// per-sample timestamps derived from `t_query_mono_ns` and
/// `t_recv_mono_ns`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFrameEnvelope {
    /// Stable device id (e.g. "SSI:LIA-OE1022D:D6522078").
    pub device_id: String,
    /// Monotonically increasing sequence number assigned by the
    /// acquisition loop. Never wraps within a run.
    pub sequence_no: u64,
    /// `Instant::now()` immediately before `write_all(RALL?\r)`.
    /// Used to derive per-sample timestamps (frame's 50 samples are
    /// 1ms apart, so sample `i` is at `t_query - (49 - i) * 1ms`).
    pub t_query_mono_ns: u64,
    /// `Instant::now()` immediately after the 12288th byte is read.
    /// Useful for jitter analysis and for distinguishing "frame
    /// complete" from "frame short".
    pub t_recv_mono_ns: u64,
    /// Wall-clock Unix epoch in milliseconds.
    pub t_wall_recv_ms: i64,
    /// Wall-clock Unix epoch in nanoseconds. Used by C6 to derive
    /// absolute per-sample timestamps that survive across runs.
    pub t_wall_recv_ns: i64,
    /// Wall-clock at the time of `t_query_mono_ns`. Same purpose as
    /// `t_wall_recv_ns` but on the query side; lets us compute
    /// `t_wall_query_ns = t_wall_recv_ns - (t_recv_mono_ns - t_query_mono_ns)`.
    pub t_wall_query_ns: i64,
    /// Duration of the read phase (after write + prepare delay).
    pub read_duration_ns: u64,
    /// Command that produced this frame (always `"RALL?"` for now).
    pub command: String,
    /// 12288 raw bytes, exactly.
    pub raw: Vec<u8>,
    /// Status flag — set by [`RallReader`].
    pub transport_status: TransportStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportStatus {
    /// Frame is exactly 12288 bytes and arrived within deadline.
    Ok,
    /// Frame is shorter than 12288 bytes; `raw.len()` is what we got.
    FrameShort { actual_bytes: usize },
    /// Read loop exceeded the deadline.
    ReadDeadlineExceeded { bytes_read: usize },
    /// Underlying transport reported an error other than timeout.
    IoError,
}

// ---------------------------------------------------------------------------
// RallLink trait
// ---------------------------------------------------------------------------

/// Abstraction over "send RALL?, return 12288 bytes". Implemented by
/// both [`MockRallLink`] and (in C5) the real serial link.
///
/// The `read_chunk` default method delegates to `std::io::Read` so that
/// any `RallLink` that also implements `Read` gets a working
/// `read_chunk` for free. The mock uses the default; the C5 serial
/// impl will also use the default.
pub trait RallLink: Send + Read {
    /// Send the RALL? command and prepare the device to deliver the
    /// next 12288-byte frame. After this returns, subsequent
    /// `read_chunk` calls should return the frame data.
    fn send_rall(&mut self) -> Result<Vec<u8>, RallLinkError>;

    /// Read a single chunk into `buf`. Default delegates to `Read::read`.
    fn read_chunk(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Read::read(self, buf)
    }

    /// Time the device needs to prepare a frame after `send_rall()`
    /// returns, before the first byte is available. Default uses the
    /// real-device constant [`RALL_PREPARE_DELAY`]. Mocks can override
    /// to `Duration::ZERO` for fast tests.
    fn prepare_delay(&self) -> Duration {
        RALL_PREPARE_DELAY
    }
}

#[derive(Debug, Error)]
pub enum RallLinkError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// RallReader — drives one RALL? cycle
// ---------------------------------------------------------------------------

/// Drives a single RALL? cycle on any `RallLink`. Captures all three
/// timestamps (query, recv, wall) and packages the result as a
/// `RawFrameEnvelope` with the given `device_id` and `sequence_no`.
pub struct RallReader;

impl RallReader {
    /// Run one RALL? cycle.
    ///
    /// `t_query_mono_origin` is the `Instant` corresponding to
    /// `t_query_mono_ns == 0` for the current run; we use it to compute
    /// `t_query_mono_ns` from `Instant::now()`.
    pub fn read_one<L: RallLink>(
        link: &mut L,
        device_id: &str,
        sequence_no: u64,
        t_query_mono_origin: Instant,
    ) -> RawFrameEnvelope {
        let t_query = Instant::now();
        let t_wall_query_ns = unix_epoch_ns(t_query);
        let t_query_mono_ns = t_query.duration_since(t_query_mono_origin).as_nanos() as u64;

        // Step 1: send RALL? to the device. Errors are logged but
        // do not abort the cycle — the read loop will then report
        // FrameShort because the device did not return a frame.
        let send_result = link.send_rall();
        if let Err(e) = send_result {
            eprintln!("[RallReader] send_rall failed: {e}");
        }

        let read_started = t_query + link.prepare_delay();
        let deadline = t_query + RALL_READ_DEADLINE;

        let raw_result = read_rall_frame(link, read_started, deadline);

        let t_recv = Instant::now();
        let t_wall_recv_ns = unix_epoch_ns(t_recv);
        let t_recv_mono_ns = t_recv.duration_since(t_query_mono_origin).as_nanos() as u64;
        let t_wall_recv_ms = (t_wall_recv_ns / 1_000_000) as i64;
        let read_duration_ns = t_recv.duration_since(read_started).as_nanos() as u64;

        let (raw, transport_status) = match raw_result {
            Ok(bytes) if bytes.len() == RALL_FRAME_BYTES => (bytes, TransportStatus::Ok),
            Ok(bytes) => {
                let actual = bytes.len();
                (bytes, TransportStatus::FrameShort {
                    actual_bytes: actual,
                })
            }
            Err(RallReadLoopError::DeadlineExceeded {
                bytes_read,
            }) => (
                Vec::new(),
                TransportStatus::ReadDeadlineExceeded { bytes_read },
            ),
            Err(RallReadLoopError::Io(_)) => {
                (Vec::new(), TransportStatus::IoError)
            }
        };

        RawFrameEnvelope {
            device_id: device_id.to_string(),
            sequence_no,
            t_query_mono_ns,
            t_recv_mono_ns,
            t_wall_recv_ms,
            t_wall_recv_ns,
            t_wall_query_ns,
            read_duration_ns,
            command: "RALL?".to_string(),
            raw,
            transport_status,
        }
    }
}

/// Internal error for the loop-read state machine.
#[derive(Debug, Error)]
enum RallReadLoopError {
    #[error("read deadline exceeded after reading {bytes_read} bytes")]
    DeadlineExceeded { bytes_read: usize },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Read exactly `RALL_FRAME_BYTES` from `link`, looping over partial
/// reads (K4). Bails out with `DeadlineExceeded` if the deadline
/// elapses before the frame is complete.
///
/// `read_started` is the Instant at which we should start reading
/// (after the link's `prepare_delay`); we sleep until then to model
/// K6.
fn read_rall_frame<L: RallLink>(
    link: &mut L,
    read_started: Instant,
    deadline: Instant,
) -> Result<Vec<u8>, RallReadLoopError> {
    // K6: sleep until the device is supposed to have prepared the
    // frame. Using Instant arithmetic avoids wall-clock drift.
    let now = Instant::now();
    if read_started > now {
        thread::sleep(read_started - now);
    }
    // K3 + K4: no terminator; macOS gives ~1020B per read; loop
    // until we have 12288 bytes or hit the deadline.
    let mut frame = Vec::with_capacity(RALL_FRAME_BYTES);
    let mut buf = [0u8; RALL_READ_BUFFER_BYTES];
    while frame.len() < RALL_FRAME_BYTES {
        let now = Instant::now();
        if now >= deadline {
            return Err(RallReadLoopError::DeadlineExceeded {
                bytes_read: frame.len(),
            });
        }
        let remaining_deadline = deadline - now;
        // The mock link ignores the timeout; the real link will
        // honor it via `serialport::SerialPort::set_timeout` set up
        // by the caller (C5).
        let _ = remaining_deadline;
        match link.read_chunk(&mut buf) {
            Ok(0) => {
                // EOF — for the mock this means the device "closed";
                // for the real link this should not happen mid-frame.
                return Err(RallReadLoopError::DeadlineExceeded {
                    bytes_read: frame.len(),
                });
            }
            Ok(n) => frame.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                // The underlying read timed out before we got a chunk.
                // Loop and check the deadline.
                continue;
            }
            Err(e) => return Err(RallReadLoopError::Io(e)),
        }
    }
    Ok(frame)
}

// ---------------------------------------------------------------------------
// RallLink extension removed; RallLink now has a default read_chunk.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// MockRallLink
// ---------------------------------------------------------------------------

/// Deterministic, in-memory RALL? link for tests and dev machines.
///
/// On each `send_rall()` it writes `RALL_COMMAND` to an internal
/// `Vec<u8>` (the "sent" buffer) and returns 12288 bytes from a
/// user-supplied frame source. The frame source is wrapped in an
/// `Arc<Mutex<_>>` so multiple readers can share it.
pub struct MockRallLink {
    sent_commands: Arc<Mutex<Vec<Vec<u8>>>>,
    frame_source: Arc<Mutex<MockFrameSource>>,
}

#[derive(Debug, Clone)]
pub struct MockFrameSource {
    /// Original 12288-byte template. The `frame` field is reset from
    /// this on every `send_rall()` so the mock can be re-used across
    /// multiple cycles. `pub(crate)` so tests can clear it to
    /// simulate empty frames.
    pub(crate) template: Vec<u8>,
    /// Live buffer that gets drained as `read()` consumes bytes.
    frame: Vec<u8>,
    /// K4 simulation: 0 means return the full buffer in one read.
    pub chunk_bytes: usize,
    /// K6 simulation: optional delay between send and first byte.
    /// The real device needs ~900ms; the mock can use 0 for fast
    /// tests.
    pub prepare_delay: Duration,
    /// Optional probability [0,1] of returning a short frame. Used to
    /// test the FrameShort path. Default 0.
    pub short_frame_rate: f64,
    /// Counter for deterministic short-frame injection.
    call_index: u64,
}

impl MockFrameSource {
    /// Build a new frame source. The template must be 12288 bytes;
    /// if it is shorter, the mock zero-pads; if longer, the extra
    /// bytes are silently dropped (matches K3 fixed-length semantics).
    pub fn new(template: Vec<u8>) -> Self {
        let mut template = template;
        template.resize(RALL_FRAME_BYTES, 0);
        let frame = template.clone();
        Self {
            template,
            frame,
            chunk_bytes: 0,
            prepare_delay: Duration::ZERO,
            short_frame_rate: 0.0,
            call_index: 0,
        }
    }

    /// K4 simulation: configure how many bytes the mock returns per
    /// `read_chunk` call. Use 1020 to mimic macOS behavior.
    pub fn with_chunk_bytes(mut self, chunk_bytes: usize) -> Self {
        self.chunk_bytes = chunk_bytes;
        self
    }

    /// K6 simulation: delay before the first byte becomes available.
    pub fn with_prepare_delay(mut self, delay: Duration) -> Self {
        self.prepare_delay = delay;
        self
    }

    /// Probability of returning a short frame on each call (0.0..=1.0).
    pub fn with_short_frame_rate(mut self, rate: f64) -> Self {
        self.short_frame_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Reset the live frame buffer to the template. Called by
    /// [`MockRallLink`] on every `send_rall()`.
    fn reset(&mut self) {
        self.frame = self.template.clone();
        self.call_index = 0;
    }
}

impl MockRallLink {
    pub fn new(frame_source: Arc<Mutex<MockFrameSource>>) -> Self {
        Self {
            sent_commands: Arc::new(Mutex::new(Vec::new())),
            frame_source,
        }
    }

    /// Snapshot of all commands "sent" so far. Useful in tests.
    pub fn sent_commands(&self) -> Vec<Vec<u8>> {
        self.sent_commands.lock().clone()
    }
}

impl Read for MockRallLink {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // K4 simulation: if the frame source has a chunk_bytes setting,
        // return at most that many bytes per call. Otherwise return
        // whatever fits.
        let mut src = self.frame_source.lock();
        if src.call_index == 0 && !src.prepare_delay.is_zero() {
            // Simulate K6: on the very first read after a fresh
            // send_rall, sleep to model the device preparing the frame.
            // Subsequent reads return immediately because the data
            // is already in our local buffer.
            std::thread::sleep(src.prepare_delay);
        }
        src.call_index += 1;
        let take = if src.chunk_bytes == 0 {
            buf.len()
        } else {
            buf.len().min(src.chunk_bytes)
        };
        let n = take.min(src.frame.len());
        buf[..n].copy_from_slice(&src.frame[..n]);
        // Drain the consumed bytes (we model a streaming source).
        src.frame.drain(..n);
        Ok(n)
    }
}

impl RallLink for MockRallLink {
    fn send_rall(&mut self) -> Result<Vec<u8>, RallLinkError> {
        self.sent_commands
            .lock()
            .push(RALL_COMMAND.to_vec());
        // Reset the frame source so a new frame is produced for this
        // call. The caller (RallReader) loops read_chunk until 12288
        // bytes are accumulated.
        let mut src = self.frame_source.lock();
        src.reset();
        Ok(Vec::new())
    }

    fn prepare_delay(&self) -> Duration {
        // Default to ZERO so unit tests are fast. Tests that want
        // to model the real device can override via the `MockFrameSource`
        // setting; see also the `prepare_delay_simulation` field.
        let src = self.frame_source.lock();
        src.prepare_delay
    }
}

// ---------------------------------------------------------------------------
// ContinuousRallLoop — runs RALL? cycles on a dedicated OS thread
// ---------------------------------------------------------------------------

/// Spawn a dedicated OS thread that runs RALL? cycles back-to-back and
/// pushes the resulting `RawFrameEnvelope`s into `tx`. Returns a
/// [`ContinuousRallHandle`] that can be used to stop the loop.
pub fn spawn_continuous_rall_loop<L: RallLink + 'static>(
    mut link: L,
    device_id: String,
    tx: crossbeam_channel::Sender<RawFrameEnvelope>,
) -> ContinuousRallHandle {
    let stop = Arc::new(parking_lot::Mutex::new(false));
    let stop_for_thread = Arc::clone(&stop);

    let idn = IdnResponse {
        manufacturer: "?".into(),
        model: "?".into(),
        serial_number: device_id.clone(),
        firmware_version: "?".into(),
        raw: Vec::new(),
    };
    let _ = idn; // Reserved for future use; identity is propagated via device_id.

    let handle = thread::Builder::new()
        .name(format!("rall-loop-{}", device_id))
        .spawn(move || {
            let origin = Instant::now();
            let mut sequence_no: u64 = 0;
            while !*stop_for_thread.lock() {
                let envelope =
                    RallReader::read_one(&mut link, &device_id, sequence_no, origin);
                // If the receiver is dropped, exit cleanly.
                if tx.send(envelope).is_err() {
                    break;
                }
                sequence_no += 1;
            }
        })
        .expect("failed to spawn rall-loop thread");

    ContinuousRallHandle {
        join: Some(handle),
        stop,
    }
}

/// Handle for stopping a continuous RALL? loop. Dropping the handle
/// does NOT stop the loop; call [`stop()`](Self::stop) explicitly.
pub struct ContinuousRallHandle {
    join: Option<thread::JoinHandle<()>>,
    stop: Arc<Mutex<bool>>,
}

impl ContinuousRallHandle {
    pub fn stop(&self) {
        *self.stop.lock() = true;
    }

    /// Consume the handle and join the loop thread, returning whether
    /// it exited cleanly.
    pub fn join(mut self) -> std::thread::Result<()> {
        self.stop();
        if let Some(h) = self.join.take() {
            h.join()
        } else {
            Ok(())
        }
    }

    /// Internal constructor for the `serial::spawn_continuous_rall_loop_pinned`
    /// helper, which needs to set up a thread that pins itself before
    /// running the loop body.
    #[allow(dead_code)]
    pub(crate) fn from_parts(
        join: thread::JoinHandle<()>,
        stop: Arc<Mutex<bool>>,
    ) -> Self {
        Self {
            join: Some(join),
            stop,
        }
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Wall-clock nanoseconds since the Unix epoch. Falls back to 0 if
/// the system clock is before the epoch (should never happen on a
/// sane host).
fn unix_epoch_ns(t: Instant) -> i64 {
    let wall = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    let since_t = t.elapsed();
    let total_ns = wall
        .as_nanos()
        .saturating_sub(since_t.as_nanos());
    i64::try_from(total_ns).unwrap_or(i64::MAX)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Read};
    use std::sync::Arc;
    use std::time::Duration;

    fn make_test_frame() -> Vec<u8> {
        // A 12288-byte frame: B-X = 0.001 for all 50 samples,
        // everything else zero. This is just enough to test the
        // round-trip; the C6 parser will give it real meaning.
        let mut frame = vec![0u8; RALL_FRAME_BYTES];
        // B-X lives at offset 3200..3599 = 50 f64 BE.
        for i in 0..50 {
            let value: f64 = 0.001 * (i as f64 + 1.0);
            let bytes = value.to_be_bytes();
            let off = 3200 + i * 8;
            frame[off..off + 8].copy_from_slice(&bytes);
        }
        frame
    }

    #[test]
    fn mock_returns_full_frame_in_one_read() {
        let frame = make_test_frame();
        let src = Arc::new(Mutex::new(MockFrameSource::new(frame)));
        let mut link = MockRallLink::new(src);

        // send_rall: the mock records the command.
        link.send_rall().unwrap();
        assert_eq!(link.sent_commands()[0], RALL_COMMAND.to_vec());

        // read_chunk returns the whole frame in one call.
        let mut buf = vec![0u8; RALL_FRAME_BYTES];
        let n = link.read_chunk(&mut buf).unwrap();
        assert_eq!(n, RALL_FRAME_BYTES);
    }

    #[test]
    fn k4_mock_chunks_to_1020_bytes() {
        // K4: macOS returns ~1020 bytes per read. Verify the reader
        // loop assembles the full 12288 bytes across multiple reads.
        let frame = make_test_frame();
        let src = Arc::new(Mutex::new(
            MockFrameSource::new(frame).with_chunk_bytes(1020),
        ));
        let mut link = MockRallLink::new(src);

        // One send_rall, then many reads.
        link.send_rall().unwrap();
        let mut acc = Vec::new();
        while acc.len() < RALL_FRAME_BYTES {
            let mut buf = vec![0u8; 4096];
            let n = link.read_chunk(&mut buf).unwrap();
            assert!(n <= 1020, "mock should cap reads at 1020 bytes");
            acc.extend_from_slice(&buf[..n]);
        }
        assert_eq!(acc.len(), RALL_FRAME_BYTES);
    }

    #[test]
    fn rall_reader_assembles_one_envelope() {
        let frame = make_test_frame();
        let src = Arc::new(Mutex::new(MockFrameSource::new(frame)));
        let mut link = MockRallLink::new(src);

        let origin = Instant::now();
        let env = RallReader::read_one(&mut link, "SSI:LIA-OE1022D:D6522078", 7, origin);
        assert_eq!(env.sequence_no, 7);
        assert_eq!(env.device_id, "SSI:LIA-OE1022D:D6522078");
        assert_eq!(env.command, "RALL?");
        assert_eq!(env.raw.len(), RALL_FRAME_BYTES);
        assert_eq!(env.transport_status, TransportStatus::Ok);
        // Timestamps must be sane:
        assert!(env.t_recv_mono_ns >= env.t_query_mono_ns);
    }

    #[test]
    fn continuous_loop_no_loss_for_short_burst() {
        // Run a 1-second continuous loop with a fast mock (no
        // prepare delay) and assert every envelope is OK with a
        // 12288-byte payload. No frame loss.
        let frame = make_test_frame();
        let src = Arc::new(Mutex::new(MockFrameSource::new(frame)));
        let link = MockRallLink::new(src);
        let (tx, rx) = crossbeam_channel::unbounded::<RawFrameEnvelope>();

        let handle = spawn_continuous_rall_loop(
            link,
            "SSI:LIA-OE1022D:TEST001".to_string(),
            tx,
        );

        // Let the loop run for ~1 second. With no prepare delay, the
        // loop is CPU-bound; the kernel will give us thousands of
        // iterations.
        std::thread::sleep(Duration::from_millis(1000));
        handle.stop();
        handle.join().unwrap();

        let mut count = 0u64;
        let mut last_seq: u64 = 0;
        while let Ok(env) = rx.try_recv() {
            if env.sequence_no != last_seq {
                // sequence_no must be strictly increasing without gaps.
                assert_eq!(
                    env.sequence_no,
                    last_seq,
                    "sequence_no must be contiguous from 0"
                );
            }
            last_seq += 1;
            assert_eq!(env.transport_status, TransportStatus::Ok);
            assert_eq!(env.raw.len(), RALL_FRAME_BYTES);
            count += 1;
        }
        assert!(count > 0, "loop produced no frames in 1 second");
        assert_eq!(count, last_seq, "frame count must match sequence_no");
    }

    #[test]
    fn short_frame_is_flagged_not_padded() {
        // The reader must NEVER pad a short frame to 12288 bytes. It
        // must report FrameShort and pass through whatever bytes it
        // got.
        let frame = make_test_frame();
        let src = Arc::new(Mutex::new(MockFrameSource::new(frame)));
        let mut link = MockRallLink::new(src);

        // Make the mock return 0 bytes for the next frame by clearing
        // BOTH the template (so reset() produces an empty buffer)
        // and the live frame.
        {
            let mut src = link.frame_source.lock();
            src.template.clear();
            src.frame.clear();
        }
        link.send_rall().unwrap();

        let origin = Instant::now();
        let env = RallReader::read_one(&mut link, "TEST", 0, origin);
        // Reader should report FrameShort (or DeadlineExceeded) and
        // MUST NOT have silently padded.
        assert!(matches!(
            env.transport_status,
            TransportStatus::FrameShort { .. } | TransportStatus::ReadDeadlineExceeded { .. }
        ));
        assert!(env.raw.len() < RALL_FRAME_BYTES);
    }

    // We need a `Read` impl on `MockRallLink` to satisfy the blanket
    // `Read` requirement in `RallLink::read_chunk`. The macro trick
    // above is replaced by a direct impl — this test is here to
    // confirm the trait wiring compiles.
    #[test]
    fn mock_rall_link_impls_read() {
        fn assert_read<T: Read>(_: &T) {}
        let frame = make_test_frame();
        let src = Arc::new(Mutex::new(MockFrameSource::new(frame)));
        let link = MockRallLink::new(src);
        assert_read(&link);
    }
}
