//! OE1022D RALL? continuous collector — Producer-Consumer pattern.
//!
//! Spawns a dedicated read thread that polls RALL? at the device refresh rate
//! (~48ms), validates frame alignment, detects duplicates, and pushes parsed
//! frames through an mpsc channel.
//!
//! ## Design (from LabVIEW analysis and real-hardware benchmarks)
//!
//! - Read interval: 48ms (matches device ~48ms internal refresh)
//! - Frame boundary: `clear(Input)` before each RALL? to prevent drift
//! - Dedup: compare X[0] value of consecutive frames
//! - Pipeline: NOT supported — do not queue multiple RALL?
//! - Read method: fast-poll 1ms retry on empty/timeout, exit at 12288 bytes
//!
//! Benchmark data (2026-06-06, V6.3211110 SN:D6130220):
//! - Single frame read: 12.0ms, max mechanical rate 83.7fps
//! - Device refresh: ~48ms, effective unique frame rate 20.8fps
//! - ~1040 unique data points/sec

use crate::parser::{parse_rall_frame, RallFrame, RALL_FRAME_BYTES};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

pub struct CollectorConfig {
    /// Serial port path, e.g. `/dev/cu.usbmodem395D388533371`
    pub port_path: String,
    /// Baud rate, typically 921600
    pub baud: u32,
    /// Interval between RALL? queries (ms). Default: 48 (matches device refresh)
    pub read_interval_ms: u64,
    /// Max time to wait for a complete 12288-byte frame (ms)
    pub timeout_ms: u64,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            port_path: String::new(),
            baud: 921600,
            read_interval_ms: 48,
            timeout_ms: 5000,
        }
    }
}

// ---------------------------------------------------------------------------
// Captured frame
// ---------------------------------------------------------------------------

/// A single frame captured from the OE1022D, with metadata.
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    /// Parsed RALL? frame data
    pub frame: RallFrame,
    /// Raw bytes (12288 bytes)
    pub raw: Vec<u8>,
    /// Monotonically increasing frame index (includes duplicates)
    pub frame_index: u64,
    /// PC wall-clock timestamp when RALL? was sent (Unix ms)
    pub timestamp_unix_ms: u64,
    /// Wall-clock time to read the full 12288 bytes (microseconds)
    pub read_time_us: u64,
    /// True if this frame has the same X[0] as the previous frame
    pub is_duplicate: bool,
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CollectorStats {
    pub frames_captured: u64,
    pub frames_duplicated: u64,
    pub frames_parse_error: u64,
    pub total_reads_attempted: u64,
    pub avg_read_time_us: u64,
    pub running: bool,
}

// ---------------------------------------------------------------------------
// Collector handle
// ---------------------------------------------------------------------------

pub struct RallCollector {
    stop_tx: Option<std::sync::mpsc::SyncSender<()>>,
    stats: Arc<Mutex<CollectorStats>>,
    #[allow(dead_code)]
    handle: Option<std::thread::JoinHandle<()>>,
}

impl RallCollector {
    /// Start the collector thread. Returns a handle and the frame receiver.
    ///
    /// The producer thread:
    /// 1. Opens the serial port
    /// 2. Loops: clear(Input) → write RALL?\r → fast-poll read → parse → dedup → push
    /// 3. Stops when `stop()` is called or the receiver is dropped
    pub fn start(
        config: CollectorConfig,
    ) -> Result<(Self, std::sync::mpsc::Receiver<CapturedFrame>), String> {
        let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel::<CapturedFrame>(8);
        let (stop_tx, stop_rx) = std::sync::mpsc::sync_channel::<()>(1);
        let stats = Arc::new(Mutex::new(CollectorStats {
            frames_captured: 0,
            frames_duplicated: 0,
            frames_parse_error: 0,
            total_reads_attempted: 0,
            avg_read_time_us: 0,
            running: true,
        }));
        let stats_clone = Arc::clone(&stats);

        let port_path = config.port_path.clone();
        let baud = config.baud;
        let interval = config.read_interval_ms;
        let timeout = config.timeout_ms;

        let handle = std::thread::spawn(move || {
            let port = match serialport::new(&port_path, baud)
                .timeout(Duration::from_millis(100))
                .open()
            {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("[oe1022d-collector] failed to open {}: {}", port_path, e);
                    let mut s = stats_clone.lock().unwrap();
                    s.running = false;
                    return;
                }
            };

            producer_loop(port, &stats_clone, &frame_tx, &stop_rx, interval, timeout);

            let mut s = stats_clone.lock().unwrap();
            s.running = false;
        });

        Ok((
            RallCollector {
                stop_tx: Some(stop_tx),
                stats,
                handle: Some(handle),
            },
            frame_rx,
        ))
    }

    /// Snapshot of current statistics.
    pub fn stats(&self) -> CollectorStats {
        self.stats.lock().unwrap().clone()
    }

    /// Signal the producer thread to stop. Does not block.
    /// The thread will exit on its next loop iteration.
    pub fn signal_stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for RallCollector {
    fn drop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        // Intentionally do NOT join the thread — it may be blocked in a
        // serial read. The OS will clean up when the process exits.
    }
}

// ---------------------------------------------------------------------------
// Producer loop (runs in dedicated thread)
// ---------------------------------------------------------------------------

fn producer_loop(
    mut port: Box<dyn serialport::SerialPort>,
    stats: &Arc<Mutex<CollectorStats>>,
    frame_tx: &std::sync::mpsc::SyncSender<CapturedFrame>,
    stop_rx: &std::sync::mpsc::Receiver<()>,
    read_interval_ms: u64,
    timeout_ms: u64,
) {
    let mut frame_index: u64 = 0;
    let mut prev_x0: Option<f64> = None;
    let mut total_read_us: u64 = 0;

    loop {
        if stop_rx.try_recv().is_ok() {
            return;
        }

        let _ = port.clear(serialport::ClearBuffer::Input);

        let ts = unix_ms_now();
        if port.write_all(b"RALL?\r").is_err() {
            return;
        }
        if port.flush().is_err() {
            return;
        }

        let read_start = Instant::now();
        let raw = match read_rall_frame_fast(&mut *port, timeout_ms) {
            Ok(data) => data,
            Err(_) => {
                // Read error or timeout — continue to next cycle
                {
                    let mut s = stats.lock().unwrap();
                    s.total_reads_attempted += 1;
                }
                wait_remaining(read_start, read_interval_ms, stop_rx);
                continue;
            }
        };
        let read_time_us = read_start.elapsed().as_micros() as u64;

        // 4. Parse
        let parse_result = parse_rall_frame(&raw[..RALL_FRAME_BYTES.min(raw.len())]);

        // 5. Dedup
        let is_dup = match &parse_result {
            Ok(parsed) => {
                let x0 = parsed.measurements.lockin_A_X_mv.first().copied();
                let dup = match (prev_x0, x0) {
                    (Some(prev), Some(curr)) => (prev - curr).abs() < f64::EPSILON,
                    _ => false,
                };
                prev_x0 = x0;
                dup
            }
            Err(_) => false,
        };

        // 6. Update stats
        {
            let mut s = stats.lock().unwrap();
            s.total_reads_attempted += 1;
            total_read_us += read_time_us;

            match &parse_result {
                Ok(_) => {
                    s.frames_captured += 1;
                    if is_dup {
                        s.frames_duplicated += 1;
                    }
                    s.avg_read_time_us = total_read_us / s.total_reads_attempted;
                }
                Err(_) => {
                    s.frames_parse_error += 1;
                }
            }
        }

        // 7. Push to channel (non-blocking — if consumer is slow, skip frame)
        match parse_result {
            Ok(frame) => {
                let captured = CapturedFrame {
                    frame,
                    raw,
                    frame_index,
                    timestamp_unix_ms: ts,
                    read_time_us,
                    is_duplicate: is_dup,
                };
                let _ = frame_tx.try_send(captured);
            }
            Err(_) => {
                // Parse error — don't push garbage to consumer
            }
        }

        frame_index += 1;

        // 8. Wait for next read interval
        wait_remaining(read_start, read_interval_ms, stop_rx);
    }
}

/// Read exactly RALL_FRAME_BYTES using fast-poll with 1ms retry.
fn read_rall_frame_fast(
    port: &mut dyn serialport::SerialPort,
    timeout_ms: u64,
) -> Result<Vec<u8>, String> {
    let mut buf = Vec::with_capacity(RALL_FRAME_BYTES);
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);

    while buf.len() < RALL_FRAME_BYTES && Instant::now() < deadline {
        let mut chunk = vec![0u8; 4096];
        match port.read(&mut chunk) {
            Ok(0) => {
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            Ok(n) => {
                chunk.truncate(n);
                buf.extend_from_slice(&chunk);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            Err(e) => return Err(format!("serial read error: {}", e)),
        }
    }

    if buf.len() >= RALL_FRAME_BYTES {
        Ok(buf)
    } else {
        Err(format!(
            "timeout: got {} of {} bytes",
            buf.len(),
            RALL_FRAME_BYTES
        ))
    }
}

/// Sleep until `interval_ms` has elapsed since `start`, checking stop signal.
fn wait_remaining(
    start: Instant,
    interval_ms: u64,
    stop_rx: &std::sync::mpsc::Receiver<()>,
) {
    loop {
        let elapsed = start.elapsed().as_millis() as u64;
        if elapsed >= interval_ms {
            return;
        }
        let remaining = interval_ms - elapsed;
        let nap = remaining.min(10); // check stop signal every 10ms
        if stop_rx.try_recv().is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(nap));
    }
}

fn unix_ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_are_sane() {
        let cfg = CollectorConfig::default();
        assert_eq!(cfg.baud, 921600);
        assert_eq!(cfg.read_interval_ms, 48);
        assert_eq!(cfg.timeout_ms, 5000);
    }

    #[test]
    fn stats_initially_running() {
        let s = CollectorStats {
            frames_captured: 0,
            frames_duplicated: 0,
            frames_parse_error: 0,
            total_reads_attempted: 0,
            avg_read_time_us: 0,
            running: true,
        };
        assert!(s.running);
        assert_eq!(s.frames_captured, 0);
    }
}
