//! NDJSON + events writer for the OE1022D acquisition pipeline.
//!
//! Implements the C7 commit: each `ParsedSample` becomes one line
//! of `samples.ndjson`; each lifecycle event (start, stop, frame
//! short, warmup, writer error, etc.) becomes one line of
//! `events.jsonl`.
//!
//! ## Design
//!
//! - **8 KB write buffer**: small enough to flush on every batch,
//!   large enough to amortize `write()` syscalls (typically ~4 KB
//!   per syscall on macOS, so 8 KB means ~1 syscall per batch).
//! - **Append-only**: ndjson files are opened once in append mode
//!   and held for the duration of the run. The writer thread is
//!   the only writer, so no locking is needed.
//! - **No fsync on the hot path**: the OS flushes dirty pages
//!   asynchronously. We rely on the durability of `BufWriter`'s
//!   flush + close, which happens at run stop and on errors.
//! - **Writer errors are fatal**: per PRD 03 §17.4, a raw writer
//!   failure must emit a fatal event and stop the acquisition.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use oe1022d_transport::{parse_envelope, parse_and_expand, ParsedSample, RawFrameEnvelope, SampleField};

/// Configuration for an acquisition run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    /// Directory to write `samples.ndjson` and `events.jsonl`
    /// into. Will be created if it does not exist.
    pub run_dir: PathBuf,
    /// Human-readable run id (e.g. `"2026-06-05_demo_001"`).
    pub run_id: String,
    /// Which channel(s) to expand to samples. Empty = skip the
    /// pipeline (still writes events).
    pub fields: Vec<SampleField>,
    /// Buffer size in bytes before forcing a flush. 8 KB by
    /// default.
    #[serde(default = "default_buffer_bytes")]
    pub buffer_bytes: usize,
}

fn default_buffer_bytes() -> usize {
    8 * 1024
}

#[derive(Debug, Error)]
pub enum WriterError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("writer is shut down")]
    ShutDown,
    #[error("ndjson serialization error for sample: {0}")]
    SampleSerialize(String),
    #[error("events serialization error: {0}")]
    EventSerialize(String),
}

/// One line in `events.jsonl`. Per PRD 03 §18.2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcqEvent {
    /// `oe1022d_*` | `system` | `safety` | `parser` | `transport`
    pub kind: String,
    /// `info` | `warning` | `error` | `fatal`
    pub severity: String,
    /// Human-readable message
    pub message: String,
    /// Wall-clock Unix ns at the moment the event was emitted
    pub t_wall_ns: i64,
    /// Optional reference to the source frame sequence number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_sequence_no: Option<u64>,
    /// Optional reference to the source run id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

impl AcqEvent {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            kind: "system".into(),
            severity: "info".into(),
            message: message.into(),
            t_wall_ns: now_wall_ns(),
            frame_sequence_no: None,
            run_id: None,
        }
    }

    pub fn warn(message: impl Into<String>) -> Self {
        Self {
            kind: "system".into(),
            severity: "warning".into(),
            message: message.into(),
            t_wall_ns: now_wall_ns(),
            frame_sequence_no: None,
            run_id: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            kind: "system".into(),
            severity: "error".into(),
            message: message.into(),
            t_wall_ns: now_wall_ns(),
            frame_sequence_no: None,
            run_id: None,
        }
    }

    pub fn with_frame(mut self, seq: u64) -> Self {
        self.frame_sequence_no = Some(seq);
        self
    }

    pub fn with_run(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }
}

fn now_wall_ns() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

/// Per-run writer. Holds the open file handles and the buffered
/// writers. Cloning is cheap (inner state is `Arc<Mutex<...>>`).
#[derive(Clone)]
pub struct RunWriter {
    inner: Arc<Mutex<RunWriterInner>>,
    config: RunConfig,
}

struct RunWriterInner {
    samples: Option<BufWriter<File>>,
    events: Option<BufWriter<File>>,
    samples_written: u64,
    events_written: u64,
    /// If true, all subsequent writes return `WriterError::ShutDown`.
    shut_down: bool,
}

impl RunWriter {
    /// Create a new run writer. Creates the run directory if
    /// missing, opens `samples.ndjson` and `events.jsonl` for
    /// append, and emits the `start` event.
    pub fn create(config: RunConfig) -> Result<Self, WriterError> {
        std::fs::create_dir_all(&config.run_dir)?;
        let samples_path = config.run_dir.join("samples.ndjson");
        let events_path = config.run_dir.join("events.jsonl");

        let samples_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&samples_path)?;
        let events_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)?;

        let samples = BufWriter::with_capacity(config.buffer_bytes, samples_file);
        let events = BufWriter::with_capacity(config.buffer_bytes, events_file);

        let writer = Self {
            inner: Arc::new(Mutex::new(RunWriterInner {
                samples: Some(samples),
                events: Some(events),
                samples_written: 0,
                events_written: 0,
                shut_down: false,
            })),
            config,
        };

        // Emit the run-start event.
        let start_event = AcqEvent::info(format!("run {} started", writer.config.run_id))
            .with_run(&writer.config.run_id);
        writer.emit_event(start_event)?;

        // Write metadata.json with the run config.
        writer.write_metadata()?;

        Ok(writer)
    }

    /// Append one `ParsedSample` as one line of `samples.ndjson`.
    pub fn write_sample(&self, sample: &ParsedSample) -> Result<(), WriterError> {
        let mut inner = self.inner.lock();
        if inner.shut_down {
            return Err(WriterError::ShutDown);
        }
        let samples = inner
            .samples
            .as_mut()
            .ok_or_else(|| WriterError::ShutDown)?;
        let line = serde_json::to_string(sample)
            .map_err(|e| WriterError::SampleSerialize(e.to_string()))?;
        writeln!(samples, "{}", line)?;
        inner.samples_written += 1;
        Ok(())
    }

    /// Append one `AcqEvent` as one line of `events.jsonl`.
    pub fn emit_event(&self, event: AcqEvent) -> Result<(), WriterError> {
        let mut inner = self.inner.lock();
        if inner.shut_down {
            return Err(WriterError::ShutDown);
        }
        let events = inner.events.as_mut().ok_or_else(|| WriterError::ShutDown)?;
        let line = serde_json::to_string(&event)
            .map_err(|e| WriterError::EventSerialize(e.to_string()))?;
        writeln!(events, "{}", line)?;
        inner.events_written += 1;
        Ok(())
    }

    /// Convenience: parse a frame, expand the configured fields,
    /// write 1 ndjson line per (frame, field) sample, and return
    /// the number of samples written.
    pub fn write_frame(
        &self,
        env: &RawFrameEnvelope,
    ) -> Result<usize, WriterError> {
        let fields = self.config.fields.clone();
        let samples = if fields.is_empty() {
            Vec::new()
        } else {
            // For the writer, we only need the parsed frame once.
            let report = parse_envelope(env)
                .map_err(|e| WriterError::SampleSerialize(e.to_string()))?;
            let mut out = Vec::with_capacity(fields.len() * 50);
            for field in &fields {
                out.extend(oe1022d_transport::expand_to_samples(env, &report, *field));
            }
            out
        };
        let n = samples.len();
        for s in &samples {
            self.write_sample(s)?;
        }
        Ok(n)
    }

    /// Force a flush of both `samples.ndjson` and `events.jsonl`
    /// buffers to the OS. The OS may still have dirty pages; we do
    /// not call fsync on the hot path.
    pub fn flush(&self) -> Result<(), WriterError> {
        let mut inner = self.inner.lock();
        if let Some(s) = inner.samples.as_mut() {
            s.flush()?;
        }
        if let Some(e) = inner.events.as_mut() {
            e.flush()?;
        }
        Ok(())
    }

    /// Snapshot of how many ndjson lines / event lines have been
    /// written so far.
    pub fn stats(&self) -> WriterStats {
        let inner = self.inner.lock();
        WriterStats {
            samples_written: inner.samples_written,
            events_written: inner.events_written,
        }
    }

    /// Run directory.
    pub fn run_dir(&self) -> &Path {
        &self.config.run_dir
    }

    /// Run id.
    pub fn run_id(&self) -> &str {
        &self.config.run_id
    }

    /// Mark as shut down, flush, and close the file handles.
    /// Idempotent. The writer is unusable after this call.
    pub fn shutdown(&self) -> Result<(), WriterError> {
        let mut inner = self.inner.lock();
        if inner.shut_down {
            return Ok(());
        }
        // Emit the run-stop event before closing events.jsonl.
        // Read the counts first, then take the writer to release
        // the borrow before constructing the event string.
        let samples_written = inner.samples_written;
        let events_written = inner.events_written;
        let run_id = self.config.run_id.clone();
        if let Some(events) = inner.events.as_mut() {
            let event = AcqEvent::info(format!(
                "run {run_id} stopping ({samples_written} samples, {events_written} events)"
            ))
            .with_run(&run_id);
            let line = serde_json::to_string(&event)
                .map_err(|e| WriterError::EventSerialize(e.to_string()))?;
            let _ = writeln!(events, "{}", line);
            inner.events_written += 1;
        }
        // Flush and drop.
        if let Some(mut s) = inner.samples.take() {
            let _ = s.flush();
        }
        if let Some(mut e) = inner.events.take() {
            let _ = e.flush();
        }
        inner.shut_down = true;
        Ok(())
    }

    fn write_metadata(&self) -> Result<(), WriterError> {
        let metadata_path = self.config.run_dir.join("metadata.json");
        let metadata = serde_json::json!({
            "run_id": self.config.run_id,
            "started_at_unix_ns": now_wall_ns(),
            "fields": self.config.fields,
            "buffer_bytes": self.config.buffer_bytes,
        });
        let file = File::create(&metadata_path)?;
        serde_json::to_writer_pretty(file, &metadata)
            .map_err(|e| WriterError::SampleSerialize(e.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WriterStats {
    pub samples_written: u64,
    pub events_written: u64,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oe1022d_transport::{
        RawFrameEnvelope, SampleField, TransportStatus,
    };
    use std::time::Instant;

    fn tmp_run_dir(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "oe1022d_test_{}_{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        p
    }

    fn make_frame(field: SampleField, base: f64, step: f64) -> Vec<u8> {
        use oe1022d_transport::RALL_FRAME_BYTES;
        let mut frame = vec![0u8; RALL_FRAME_BYTES];
        let byte_start = match field {
            SampleField::AX => 0,
            SampleField::AY => 400,
            SampleField::AFreq => 800,
            SampleField::ANoise => 1200,
            SampleField::AXh1 => 1600,
            SampleField::AYh1 => 2000,
            SampleField::AXh2 => 2400,
            SampleField::AYh2 => 2800,
            SampleField::BX => 3200,
            SampleField::BY => 3600,
            SampleField::BFreq => 4000,
            SampleField::BNoise => 4400,
            SampleField::BXh1 => 4800,
            SampleField::BYh1 => 5200,
            SampleField::BXh2 => 5600,
            SampleField::BYh2 => 6000,
            SampleField::AuxAdc1 => 6400,
            SampleField::AuxAdc2 => 6800,
            SampleField::AuxAdc3 => 7200,
            SampleField::AuxAdc4 => 7600,
        };
        for i in 0..50 {
            let value = base + i as f64 * step;
            let bytes = value.to_be_bytes();
            let off = byte_start + i * 8;
            frame[off..off + 8].copy_from_slice(&bytes);
        }
        frame
    }

    fn make_envelope(raw: Vec<u8>, sequence_no: u64) -> RawFrameEnvelope {
        let query_wall_ns: i64 = 1_780_206_577_446_000_000;
        RawFrameEnvelope {
            device_id: "SSI:LIA-OE1022D:TEST".into(),
            sequence_no,
            t_query_mono_ns: 60_000_000_000,
            t_recv_mono_ns: 60_900_000_000,
            t_wall_recv_ms: query_wall_ns / 1_000_000 + 900,
            t_wall_recv_ns: query_wall_ns + 900_000_000,
            t_wall_query_ns: query_wall_ns,
            read_duration_ns: 900_000_000,
            command: "RALL?".into(),
            raw,
            transport_status: TransportStatus::Ok,
        }
    }

    #[test]
    fn create_emits_start_event_and_metadata() {
        let dir = tmp_run_dir("create");
        let config = RunConfig {
            run_dir: dir.clone(),
            run_id: "test_create".into(),
            fields: vec![SampleField::BX],
            buffer_bytes: 8 * 1024,
        };
        let writer = RunWriter::create(config).expect("create");
        // Should have emitted the start event and written metadata.
        writer.flush().expect("flush after create");
        let events = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert!(events.contains("started"), "events: {events}");
        assert!(std::fs::metadata(dir.join("metadata.json")).is_ok());
        let meta = std::fs::read_to_string(dir.join("metadata.json")).unwrap();
        assert!(meta.contains("test_create"));
        writer.shutdown().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_frame_produces_50_lines_per_field() {
        let dir = tmp_run_dir("write_frame");
        let config = RunConfig {
            run_dir: dir.clone(),
            run_id: "test_write_frame".into(),
            fields: vec![SampleField::BX, SampleField::BY],
            buffer_bytes: 8 * 1024,
        };
        let writer = RunWriter::create(config).unwrap();
        let frame = make_frame(SampleField::BX, 0.001, 0.0);
        let env = make_envelope(frame, 0);
        let n = writer.write_frame(&env).expect("write_frame");
        assert_eq!(n, 100); // 2 fields × 50 samples
        let stats = writer.stats();
        assert_eq!(stats.samples_written, 100);
        // Verify ndjson line count after flush.
        writer.flush().unwrap();
        let content = std::fs::read_to_string(dir.join("samples.ndjson")).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 100);
        writer.shutdown().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_multiple_frames_scales_linearly() {
        let dir = tmp_run_dir("multi");
        let config = RunConfig {
            run_dir: dir.clone(),
            run_id: "test_multi".into(),
            fields: vec![SampleField::BX],
            buffer_bytes: 8 * 1024,
        };
        let writer = RunWriter::create(config).unwrap();
        for seq in 0..10u64 {
            let env = make_envelope(make_frame(SampleField::BX, 0.0, 0.0), seq);
            writer.write_frame(&env).expect("write_frame");
        }
        writer.flush().unwrap();
        let stats = writer.stats();
        assert_eq!(stats.samples_written, 500); // 10 frames × 50 samples
        let content = std::fs::read_to_string(dir.join("samples.ndjson")).unwrap();
        assert_eq!(content.lines().count(), 500);
        writer.shutdown().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn shutdown_is_idempotent() {
        let dir = tmp_run_dir("shutdown");
        let config = RunConfig {
            run_dir: dir.clone(),
            run_id: "test_shutdown".into(),
            fields: vec![SampleField::BX],
            buffer_bytes: 8 * 1024,
        };
        let writer = RunWriter::create(config).unwrap();
        writer.shutdown().unwrap();
        // Second call must be a no-op, not an error.
        writer.shutdown().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn shutdown_emits_stop_event() {
        let dir = tmp_run_dir("stop_event");
        let config = RunConfig {
            run_dir: dir.clone(),
            run_id: "test_stop_event".into(),
            fields: vec![SampleField::BX],
            buffer_bytes: 8 * 1024,
        };
        let writer = RunWriter::create(config).unwrap();
        let env = make_envelope(make_frame(SampleField::BX, 0.0, 0.0), 0);
        writer.write_frame(&env).unwrap();
        writer.shutdown().unwrap();
        let content = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert!(content.contains("started"));
        assert!(content.contains("stopping"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_fields_means_no_samples() {
        let dir = tmp_run_dir("nofields");
        let config = RunConfig {
            run_dir: dir.clone(),
            run_id: "test_nofields".into(),
            fields: vec![],
            buffer_bytes: 8 * 1024,
        };
        let writer = RunWriter::create(config).unwrap();
        let env = make_envelope(make_frame(SampleField::BX, 0.0, 0.0), 0);
        let n = writer.write_frame(&env).expect("write_frame");
        assert_eq!(n, 0);
        writer.shutdown().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}

