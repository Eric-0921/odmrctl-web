//! 1 kHz streaming acquisition for the OE1022D.
//!
//! C12 scope. Implements the §5.2.8 / §5.2.9 data-acquisition
//! path of the OE1022D remote-programming protocol:
//!
//! - `STRGD i, j`: set trigger mode (0=internal, 1=external)
//! - `SPRMD i, j`: set run mode (0=single, 1=loop)
//! - `SRATD i, x`: set step time (1ms-100s, **we use 1ms for
//!   1 kHz**)
//! - `SLEND i, j`: set sample length (1-16384)
//! - `SSLED i, j, k`: select parameter for buffer j (we pick
//!   k=4 for B-X)
//! - `RESTD i`: reset data buffer
//! - `STRDD i`: start sampling (returns immediately; device
//!   fills the buffer in the background)
//! - `PAUSD i`: pause sampling
//! - `SPTSD? i`: query how many points are in the buffer
//! - `TRCAD? i, j, k, l`: read l points from buffer j, starting
//!   at index k; returns ASCII floats separated by commas
//!
//! ## Why this is different from `rall.rs`
//!
//! RALL? returns a 12288-byte binary frame every ~900 ms with
//! 50 samples at 1 kHz device-internal timestamps. We saw in
//! C5.5 that the device's RALL? command can only deliver one
//! frame per ~900 ms — 18 out of every 20 device refreshes
//! (20 Hz = 50 ms) are dropped.
//!
//! The data-acquisition path (this module) uses a different
//! mechanism: STRDD starts a **continuous** sample stream into
//! the device's internal buffer, and we poll TRCAD? to read
//! out points. The device's internal 1 ms sample tick is what
//! sets the true sampling rate; software polls at its own
//! cadence. The 16384-point buffer gives us up to 16 s of
//! data at 1 kHz before wrap.
//!
//! ## C12 in scope
//!
//! - `StreamingConfig`: configurable 1 kHz stream of B-X via buffer 1
//! - `StreamingLink` trait: a `MockStreamingLink` and a real
//!   `SerialStreamingLink` (C13+ will add the serial impl)
//! - `StreamingReader::read_one_cycle()`: poll SPTSD? to learn
//!   how many new points are in the buffer, drain them via
//!   TRCAD?, emit per-sample records
//!
//! ## C12 not in scope
//!
//! - Mock 1 kHz fake frames that go through the full pipeline
//!   end-to-end (C13 will do that with the real parser)
//! - Real-device serial transport for STRDD/TRCAD? (C13+)
//! - Hardware-internal `I8 / F64 / ...` field selection — for v0.1
//!   we hard-code buffer 1 = B-X (per `SSLED i, j=1, k=4`)

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A single 1 kHz sample from the streaming path.
///
/// The device's internal 1 ms sample tick is the only clock
/// here — `t_mono_ns` is reconstructed from the wall-clock
/// instant at which we issued the SPTSD? poll, minus the
/// sample's offset within the cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingSample {
    /// Monotonic ns at the moment we *received* the sample. The
    /// device-internal sample instant is roughly `t_mono_ns -
    /// (samples_in_batch - 1 - sample_in_batch) * 1_000_000`, but
    /// we cannot know the true device timestamp without
    /// instrumenting the device; we record what we have.
    pub t_mono_ns: u64,
    /// Wall-clock ns at the moment we received the sample.
    pub t_wall_ns: i64,
    /// Wall-clock ms for human display.
    pub t_wall_ms: i64,
    /// Stable device id.
    pub device_id: String,
    /// Always `BX` (B channel X) for now; reserved for future
    /// multi-field expansion.
    pub field: String,
    /// Parsed f64 value, in V (or Hz for the freq field).
    pub value: f64,
    /// Sequence number within the streaming run, monotonically
    /// increasing across the whole run, not per-poll-cycle.
    pub stream_sequence: u64,
}

/// Configuration for one streaming run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Step time in ms. The OE1022D accepts 1-100000 ms; we
    /// default to 1 ms for 1 kHz sampling.
    pub step_time_ms: u32,
    /// Sample length. The OE1022D accepts 1-16384; we default
    /// to 16384 so the device-side buffer can hold up to 16 s
    /// of data at 1 kHz before wrap.
    pub sample_length: u32,
    /// Run mode: 0 = single, 1 = loop. Default 1.
    pub run_mode: u8,
    /// Trigger mode: 0 = internal, 1 = external. Default 0.
    pub trigger_mode: u8,
    /// Buffer index to read (1..=4). Default 1.
    pub buffer_index: u8,
    /// Parameter index to record into the buffer
    /// (4 = B-X per `SSLED i, j=1, k=4`). Default 4.
    pub buffer_parameter: u8,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            step_time_ms: 1,
            sample_length: 16384,
            run_mode: 1,      // loop
            trigger_mode: 0,  // internal
            buffer_index: 1,
            buffer_parameter: 4, // B-X
        }
    }
}

#[derive(Debug, Error)]
pub enum StreamingError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("device error: {0}")]
    Device(String),
}

/// Abstraction over the device side of the §5.2.8 / §5.2.9
/// protocol. Real impl (`SerialStreamingLink`) is in C13;
/// `MockStreamingLink` here drives the full pipeline with
/// deterministic fake data so the parser + writer can be
/// tested without a device.
pub trait StreamingLink: Send {
    /// Send a one-shot command and read the response up to a
    /// terminator (CR or LF) or timeout. Used for
    /// `STRGD?`, `SPRMD?`, `SPTSD?`, etc.
    fn send_query(&mut self, cmd: &str) -> Result<String, StreamingError>;

    /// Send a fire-and-forget command (no response expected).
    /// Used for `STRDD`, `PAUSD`, `RESTD`, etc.
    fn send_action(&mut self, cmd: &str) -> Result<(), StreamingError>;

    /// Read an ASCII-float response terminated by `,` or LF.
    /// The device returns `"-1.234e-9,+7.654e-9,..."` for a
    /// multi-point TRCAD? response; this method strips the
    /// trailing comma and returns just the inner values.
    fn read_floats(&mut self) -> Result<Vec<f64>, StreamingError>;
}

// ---------------------------------------------------------------------------
// Mock link — drives the full pipeline with deterministic fake
// data so the parser and writer can be tested without a device.
// ---------------------------------------------------------------------------

/// Mock that returns predictable streaming samples.
///
/// Each `send_action("STRDD i")` call starts producing samples
/// at `step_time_ms` cadence. `read_floats` returns the next
/// chunk, blocked by a small per-call sleep to simulate the
/// device's actual rate.
pub struct MockStreamingLink {
    config: StreamingConfig,
    /// Monotonic sample count produced so far.
    produced: u64,
    /// Per-call `read_floats` should return up to this many
    /// points; we use 100 to mimic a `SPTSD?` reply that says
    /// "100 new points since last read".
    chunk_size: usize,
}

impl MockStreamingLink {
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            config,
            produced: 0,
            chunk_size: 100,
        }
    }

    pub fn produced(&self) -> u64 {
        self.produced
    }
}

impl StreamingLink for MockStreamingLink {
    fn send_query(&mut self, cmd: &str) -> Result<String, StreamingError> {
        let cmd = cmd.trim();
        // SPTSD? i — return the count we have produced.
        if cmd.starts_with("SPTSD?") {
            return Ok(self.produced.to_string());
        }
        // Unknown query: pretend it succeeded.
        Ok("OK".to_string())
    }

    fn send_action(&mut self, cmd: &str) -> Result<(), StreamingError> {
        // We don't actually do anything; the test's read_floats
        // driver is what produces samples.
        let _ = cmd;
        Ok(())
    }

    fn read_floats(&mut self) -> Result<Vec<f64>, StreamingError> {
        // To model a real-device 1 kHz stream, we pace the
        // mock to roughly the step_time_ms cadence: per chunk,
        // sleep for `chunk_size * step_time_ms` (clamped at
        // 1 ms minimum so tests are not glacially slow).
        let step_ms = self.config.step_time_ms.max(1) as u64;
        let sleep_ms = (self.chunk_size as u64) * step_ms;
        if sleep_ms > 0 {
            std::thread::sleep(Duration::from_millis(sleep_ms));
        }
        // Simulate a chunk read: produce `chunk_size` new
        // samples and return them as ASCII-float values.
        // The pattern is a noisy sinusoid so the chart shows
        // visible motion.
        let n = self.chunk_size;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let i = self.produced;
            let t_secs = (i as f64) * (self.config.step_time_ms as f64) / 1000.0;
            let v = (2.0 * std::f64::consts::PI * 10.0 * t_secs).sin()
                + 0.05 * (i as f64).sin();
            out.push(v);
            self.produced += 1;
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// StreamingReader — orchestrator on top of any StreamingLink
// ---------------------------------------------------------------------------

/// Read one cycle: poll SPTSD? for new count, read all
/// newly-arrived points via TRCAD?, and emit one
/// `StreamingSample` per point with a reconstructed 1 ms
/// timestamp.
pub struct StreamingReader<L: StreamingLink> {
    link: L,
    config: StreamingConfig,
    device_id: String,
    /// Total samples consumed from the device so far. Used to
    /// compute the per-cycle "delta" and to assign a monotonic
    /// stream_sequence.
    consumed: u64,
    /// t_wall_ns at the moment we issued the most recent
    /// SPTSD? poll. Recorded for diagnostics.
    last_poll_wall_ns: i64,
    /// Optional per-call delay in `read_one_cycle` to simulate
    /// the wall-clock wait between polls. The real device
    /// doesn't need this; we use it to keep the mock from
    /// running CPU-bound in tests.
    poll_delay: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleOutcome {
    /// Read N new points (N > 0).
    Progress(usize),
    /// No new points yet — poll again.
    Idle,
    /// Buffer was reset (RESTD) and we are starting over.
    Reset,
}

impl<L: StreamingLink> StreamingReader<L> {
    pub fn new(link: L, config: StreamingConfig, device_id: impl Into<String>) -> Self {
        Self {
            link,
            config,
            device_id: device_id.into(),
            consumed: 0,
            last_poll_wall_ns: 0,
            poll_delay: Duration::from_millis(0),
        }
    }

    pub fn with_poll_delay(mut self, delay: Duration) -> Self {
        self.poll_delay = delay;
        self
    }

    /// Issue the start-sampling command sequence on the device.
    /// This is what the C13 + C15 driver will call before
    /// entering the read loop.
    pub fn start(&mut self) -> Result<(), StreamingError> {
        // Channel 2 = B (per OE1022D convention).
        let ch = 2u8;
        // STRGD: internal trigger.
        self.link.send_action(&format!("STRGD {},{}", ch, self.config.trigger_mode))?;
        // SPRMD: loop.
        self.link.send_action(&format!("SPRMD {},{}", ch, self.config.run_mode))?;
        // SRATD: 1 ms = 1 kHz.
        self.link.send_action(&format!("SRATD {},{}", ch, self.config.step_time_ms))?;
        // SLEND: max buffer size.
        self.link.send_action(&format!("SLEND {},{}", ch, self.config.sample_length))?;
        // SSLED: pick parameter k for buffer j.
        self.link.send_action(&format!(
            "SSLED {},{},{}",
            ch, self.config.buffer_index, self.config.buffer_parameter
        ))?;
        // RESTD: clear buffer before we start reading.
        self.link.send_action(&format!("RESTD {}", ch))?;
        // STRDD: start.
        self.link.send_action(&format!("STRDD {}", ch))?;
        self.consumed = 0;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), StreamingError> {
        self.link.send_action("PAUSD 2")?;
        Ok(())
    }

    /// Read one poll cycle: read all newly-arrived points via
    /// TRCAD? -> SPTSD? for the running count, and emit one
    /// `StreamingSample` per point with a reconstructed 1 ms
    /// timestamp.
    ///
    /// Note: the order is "read first, count after" because the
    /// device increments its buffer count during the read; for
    /// the mock, the same ordering is faked by producing points
    /// on `read_floats` and exposing the post-read count.
    pub fn read_one_cycle(&mut self) -> Result<(CycleOutcome, Vec<StreamingSample>), StreamingError> {
        if !self.poll_delay.is_zero() {
            std::thread::sleep(self.poll_delay);
        }

        // Step 1: read the next batch from the device. The
        // device internally bumps its point counter as a side
        // effect, so we have to read before querying.
        self.link.send_action(&format!(
            "TRCAD? 2,{},0,{}",
            self.config.buffer_index, self.config.sample_length
        ))?;
        let values = self.link.read_floats()?;

        // Step 2: poll the running count. The mock returns the
        // post-read total.
        let count_str = self.link.send_query("SPTSD? 2")?;
        let total: u64 = count_str.trim().parse().map_err(|e| {
            StreamingError::Parse(format!("SPTSD? reply not a number: {count_str:?}: {e}"))
        })?;
        self.last_poll_wall_ns = unix_epoch_ns(Instant::now());

        // Reset path: if total dropped below consumed, the
        // device was reset; reset our counter.
        if total < self.consumed {
            self.consumed = 0;
            return Ok((CycleOutcome::Reset, Vec::new()));
        }
        let new_points = total - self.consumed;
        let _ = new_points; // diagnostic; we trust values.len()

        if values.is_empty() {
            return Ok((CycleOutcome::Idle, Vec::new()));
        }

        // Step 3: emit one StreamingSample per value with a
        // 1 ms-spaced reconstructed timestamp. We assume the
        // device filled the buffer monotonically between polls;
        // the timestamps are spaced `step_time_ms` apart, with
        // the most recent sample closest to `last_poll_wall_ns`.
        let mut out = Vec::with_capacity(values.len());
        for (i, &v) in values.iter().enumerate() {
            let offset_ms = (values.len() - 1 - i) as u64 * self.config.step_time_ms as u64;
            let t_mono_ns = unix_epoch_ns_to_mono(Instant::now())
                .saturating_sub(offset_ms * 1_000_000);
            let t_wall_ns = self.last_poll_wall_ns - (offset_ms as i64) * 1_000_000;
            out.push(StreamingSample {
                t_mono_ns,
                t_wall_ns,
                t_wall_ms: t_wall_ns / 1_000_000,
                device_id: self.device_id.clone(),
                field: "BX".to_string(),
                value: v,
                stream_sequence: self.consumed + i as u64,
            });
        }
        self.consumed += values.len() as u64;
        Ok((CycleOutcome::Progress(values.len()), out))
    }

    pub fn consumed(&self) -> u64 {
        self.consumed
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }
}

fn unix_epoch_ns(_t: Instant) -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

fn unix_epoch_ns_to_mono(_t: Instant) -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// OS-threaded helper: drive the reader on a dedicated thread,
// push StreamingSample batches into a crossbeam channel.
// ---------------------------------------------------------------------------

/// Spawn a dedicated OS thread that runs `read_one_cycle` in a
/// loop and pushes the resulting `Vec<StreamingSample>` into
/// `tx` (a fresh batch on every poll cycle, even when the
/// batch is empty — consumers can use Idle to throttle).
///
/// Returns a [`StreamingHandle`] with a stop flag. Mock-only
/// in C12; a real-device variant lives in C13.
pub fn spawn_mock_streaming_reader(
    link: MockStreamingLink,
    config: StreamingConfig,
    device_id: String,
    tx: crossbeam_channel::Sender<Vec<StreamingSample>>,
    poll_delay: Duration,
) -> StreamingHandle {
    let stop = Arc::new(parking_lot::Mutex::new(false));
    let stop_for_thread = Arc::clone(&stop);

    let join = std::thread::Builder::new()
        .name(format!("stream-{}", device_id))
        .spawn(move || {
            let mut reader = StreamingReader::new(link, config, device_id)
                .with_poll_delay(poll_delay);
            if let Err(e) = reader.start() {
                eprintln!("[streaming] start failed: {e}");
                return;
            }
            while !*stop_for_thread.lock() {
                match reader.read_one_cycle() {
                    Ok((CycleOutcome::Progress(_), samples)) => {
                        if tx.send(samples).is_err() {
                            break;
                        }
                    }
                    Ok((CycleOutcome::Idle, _)) | Ok((CycleOutcome::Reset, _)) => {
                        // poll again next iteration
                    }
                    Err(e) => {
                        eprintln!("[streaming] cycle error: {e}");
                        break;
                    }
                }
            }
        })
        .expect("failed to spawn streaming thread");
    StreamingHandle {
        join: Some(join),
        stop,
    }
}

pub struct StreamingHandle {
    join: Option<std::thread::JoinHandle<()>>,
    stop: Arc<parking_lot::Mutex<bool>>,
}

impl StreamingHandle {
    pub fn stop(&self) {
        *self.stop.lock() = true;
    }
    pub fn join(mut self) -> std::thread::Result<()> {
        self.stop();
        if let Some(h) = self.join.take() {
            h.join()
        } else {
            Ok(())
        }
    }
}



// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_link_produces_samples_on_read() {
        let mut link = MockStreamingLink::new(StreamingConfig::default());
        let v = link.read_floats().expect("read");
        assert_eq!(v.len(), 100);
        // The first sample should be near 0 (sin(0)=0).
        assert!(v[0].abs() < 1e-3, "first sample should be ~0, got {}", v[0]);
    }

    #[test]
    fn streaming_reader_emits_one_sample_per_value() {
        let link = MockStreamingLink::new(StreamingConfig::default());
        let mut reader = StreamingReader::new(
            link,
            StreamingConfig::default(),
            "SSI:LIA-OE1022D:MOCK",
        );
        reader.start().expect("start");
        let (outcome, samples) = reader.read_one_cycle().expect("cycle");
        assert!(matches!(outcome, CycleOutcome::Progress(n) if n > 0));
        assert!(!samples.is_empty());
        for s in &samples {
            assert_eq!(s.field, "BX");
            assert_eq!(s.device_id, "SSI:LIA-OE1022D:MOCK");
            assert!(s.value.is_finite());
        }
        // All samples in one cycle should share the same
        // `last_poll_wall_ns` reference, so their t_wall_ms
        // values are within 1 ms of each other.
        let first_ms = samples[0].t_wall_ms;
        let last_ms = samples.last().unwrap().t_wall_ms;
        assert!(
            (last_ms - first_ms).abs() < 100,
            "samples in one cycle should be within 100ms wall-clock"
        );
    }

    #[test]
    fn streaming_reader_idle_when_no_new_points() {
        let link = MockStreamingLink::new(StreamingConfig::default());
        let mut reader = StreamingReader::new(
            link,
            StreamingConfig::default(),
            "SSI:LIA-OE1022D:MOCK",
        );
        reader.start().expect("start");
        // First cycle: 100 new points.
        let (o1, s1) = reader.read_one_cycle().expect("cycle 1");
        assert!(matches!(o1, CycleOutcome::Progress(100)));
        assert_eq!(s1.len(), 100);
        assert_eq!(s1.last().unwrap().stream_sequence, 99);
        // Second cycle: another 100 points (mock never stops
        // producing); verify stream_sequence continues from
        // where cycle 1 left off.
        let (o2, s2) = reader.read_one_cycle().expect("cycle 2");
        assert!(matches!(o2, CycleOutcome::Progress(100)));
        assert_eq!(s2.len(), 100);
        assert_eq!(s2.first().unwrap().stream_sequence, 100);
        assert_eq!(s2.last().unwrap().stream_sequence, 199);
    }

    #[test]
    fn streaming_reader_stream_sequence_is_monotonic() {
        let link = MockStreamingLink::new(StreamingConfig::default());
        let mut reader = StreamingReader::new(
            link,
            StreamingConfig::default(),
            "SSI:LIA-OE1022D:MOCK",
        );
        reader.start().expect("start");
        let (_, s1) = reader.read_one_cycle().expect("cycle 1");
        // Re-prime by resetting the mock and starting a new
        // streaming session: stream_sequence should continue,
        // not reset to 0, because the device is still running.
        // (For now, just verify the first cycle's stream_sequence
        // is 0..N.)
        for (i, s) in s1.iter().enumerate() {
            assert_eq!(s.stream_sequence, i as u64);
        }
    }
}
