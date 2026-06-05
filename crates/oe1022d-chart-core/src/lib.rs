//! oe1022d-chart-core — Tauri command bindings and downsampling
//! service for the live chart (Plotly frontend).
//!
//! C9 scope. Two pieces:
//!
//! 1. [`ParsedRing`] — a fixed-capacity ring buffer of the most
//!    recent N [`oe1022d_transport::ParsedSample`] records, fed
//!    by the parser thread and queried by the chart's 30 Hz emit
//!    loop. The ring is what the chart "sees" — it never touches
//!    the raw RALL? frame.
//!
//! 2. [`DownsampleService`] — given a slice of samples, return
//!    a downsampled series suitable for plotting. Three methods:
//!    - [`DownsampleMethod::Latest`] — pass the most-recent N
//!    - [`DownsampleMethod::MinMaxEnvelope`] — for each bucket
//!      of size `b`, emit (min, max) so the visible range is
//!      preserved even when zoomed out
//!    - [`DownsampleMethod::MeanBucket`] — for each bucket, emit
//!      the mean
//!
//! The Tauri command bindings (for `#[tauri::command]`) are not yet
//! wired up because the Tauri shell (`apps/desktop/`) is not in
//! this commit (that's C10). What C9 ships is the Rust-side
//! service that the Tauri command will call.
//!
//! ## Threading model
//!
//! The C5-C7 acquisition pipeline has 3 OS threads
//! (acquisition/parser/writer). The chart-core does NOT own
//! any of those threads. It owns a 4th thread (the chart
//! emitter) that pulls from ParsedRing at 30 Hz. The 4 threads
//! communicate only via crossbeam channels + Mutex; no
//! shared mutable state.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use oe1022d_transport::{ParsedSample, SampleField};

pub const SCAFFOLD_VERSION: &str = "0.1.0-c9";

/// Fixed-capacity ring buffer of `ParsedSample`s.
///
/// The ring holds the most recent N samples. When full, the
/// oldest sample is dropped to make room for the next. The
/// ring is read-mostly: writes come from the parser thread,
/// reads come from the chart's 30 Hz emit loop.
#[derive(Debug)]
pub struct ParsedRing {
    inner: Arc<RwLock<ParsedRingInner>>,
}

#[derive(Debug)]
struct ParsedRingInner {
    capacity: usize,
    /// Circular buffer of `ParsedSample`s.
    buf: Vec<Option<ParsedSample>>,
    /// Index of the next write slot.
    head: usize,
    /// Total samples ever pushed.
    total_pushed: u64,
}

impl ParsedRing {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "ring capacity must be > 0");
        Self {
            inner: Arc::new(RwLock::new(ParsedRingInner {
                capacity,
                buf: vec![None; capacity],
                head: 0,
                total_pushed: 0,
            })),
        }
    }

    /// Push one sample, dropping the oldest if the ring is full.
    pub fn push(&self, s: ParsedSample) {
        let mut inner = self.inner.write();
        let head = inner.head;
        // Replace the slot; this drops the previous value if any.
        let _ = std::mem::replace(&mut inner.buf[head], Some(s));
        inner.head = (head + 1) % inner.capacity;
        inner.total_pushed += 1;
    }

    /// Snapshot the ring's contents in chronological order
    /// (oldest first). Includes slots that have never been
    /// filled as `None` (only happens before the ring is full).
    pub fn snapshot(&self) -> Vec<Option<ParsedSample>> {
        let inner = self.inner.read();
        // Walk from `head` (oldest if full) to `head` again.
        // For a not-yet-full ring, oldest is at index 0.
        let mut out = Vec::with_capacity(inner.capacity);
        if inner.total_pushed < inner.capacity as u64 {
            // Not yet full: positions 0..total_pushed hold real
            // samples; the rest are None.
            for i in 0..inner.capacity {
                out.push(inner.buf[i].clone());
            }
        } else {
            // Full: start at head (oldest) and wrap.
            for i in 0..inner.capacity {
                let idx = (inner.head + i) % inner.capacity;
                out.push(inner.buf[idx].clone());
            }
        }
        out
    }

    /// Read all currently-stored samples as a flat Vec, dropping
    /// `None` slots. The returned Vec is in chronological order.
    pub fn drain(&self) -> Vec<ParsedSample> {
        self.snapshot()
            .into_iter()
            .flatten()
            .collect()
    }

    pub fn capacity(&self) -> usize {
        self.inner.read().capacity
    }

    pub fn total_pushed(&self) -> u64 {
        self.inner.read().total_pushed
    }
}

/// How to downsample a window of samples for plotting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownsampleMethod {
    /// Pass the most-recent N samples as-is. Use this when the
    /// window is already small enough to plot.
    Latest,
    /// For each bucket of N consecutive samples, emit
    /// (min, max). The plot draws a band. Preserves the
    /// visible range when zoomed out.
    MinMaxEnvelope,
    /// For each bucket, emit the arithmetic mean. Smoother
    /// looking line, but loses peaks.
    MeanBucket,
}

#[derive(Debug, Error)]
pub enum DownsampleError {
    #[error("target_points must be > 0, got {0}")]
    ZeroTarget(usize),
    #[error("not enough samples to downsample: have {have}, need {need}")]
    NotEnoughSamples { have: usize, need: usize },
}

/// One point on the downsampled chart. The chart frontend
/// (Plotly) consumes a list of these for a single trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartPoint {
    /// Monotonic ns timestamp at the bucket center.
    pub t_mono_ns: u64,
    /// t_wall_ms for human display.
    pub t_wall_ms: i64,
    /// Plot value: min (mean) for one-bucket, or the (min,max)
    /// envelope midpoint.
    pub y: f64,
    /// For `MinMaxEnvelope`: the high edge of the band.
    /// For other methods: same as `y`.
    pub y_high: f64,
    /// For `MinMaxEnvelope`: the low edge of the band.
    /// For other methods: same as `y`.
    pub y_low: f64,
    /// Field label this point is from (e.g. "BX", "BY").
    pub field: String,
    /// True if the source ring is in K1 warmup state.
    pub partial_warmup: bool,
}

/// Downsample a window of samples to a target number of points
/// for plotting.
pub fn downsample(
    samples: &[ParsedSample],
    target_points: usize,
    method: DownsampleMethod,
) -> Result<Vec<ChartPoint>, DownsampleError> {
    if target_points == 0 {
        return Err(DownsampleError::ZeroTarget(target_points));
    }
    if samples.is_empty() {
        return Ok(Vec::new());
    }

    match method {
        DownsampleMethod::Latest => Ok(samples
            .iter()
            .rev()
            .take(target_points)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|s| ChartPoint {
                t_mono_ns: s.t_mono_ns,
                t_wall_ms: s.t_wall_ms,
                y: s.value,
                y_high: s.value,
                y_low: s.value,
                field: format!("{:?}", s.field),
                partial_warmup: s.partial_warmup,
            })
            .collect()),
        DownsampleMethod::MeanBucket | DownsampleMethod::MinMaxEnvelope => {
            // Bucket size: at least 1.
            let bucket_size = (samples.len() + target_points - 1) / target_points;
            if bucket_size == 0 {
                return Ok(Vec::new());
            }
            let mut out = Vec::new();
            let mut i = 0;
            while i < samples.len() {
                let end = (i + bucket_size).min(samples.len());
                let bucket = &samples[i..end];
                if bucket.is_empty() {
                    break;
                }
                let center_idx = bucket.len() / 2;
                let center = bucket[center_idx].clone();
                let mut ymin = f64::INFINITY;
                let mut ymax = f64::NEG_INFINITY;
                let mut ysum = 0.0;
                for s in bucket {
                    if s.value.is_finite() {
                        ymin = ymin.min(s.value);
                        ymax = ymax.max(s.value);
                        ysum += s.value;
                    }
                }
                let (y_low, y_high, y_mid) = if !ymax.is_finite() || !ymin.is_finite() {
                    // All-NaN bucket: emit zeros rather than panic.
                    (0.0, 0.0, 0.0)
                } else if method == DownsampleMethod::MinMaxEnvelope {
                    (ymin, ymax, (ymin + ymax) / 2.0)
                } else {
                    let m = ysum / bucket.len() as f64;
                    (m, m, m)
                };
                out.push(ChartPoint {
                    t_mono_ns: center.t_mono_ns,
                    t_wall_ms: center.t_wall_ms,
                    y: y_mid,
                    y_high,
                    y_low,
                    field: format!("{:?}", center.field),
                    partial_warmup: center.partial_warmup,
                });
                i = end;
            }
            Ok(out)
        }
    }
}

/// Periodic emitter: pulls from `ParsedRing` at a fixed rate and
/// runs `downsample` to produce a `ChartWindow` for the frontend.
///
/// The frontend invokes a Tauri command (not yet wired in C10)
/// that returns the latest `ChartWindow`.
pub struct ChartEmitter {
    ring: ParsedRing,
    target_points: usize,
    method: DownsampleMethod,
    field_filter: Option<SampleField>,
    last_emit: RwLock<Option<Instant>>,
    interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartWindow {
    pub points: Vec<ChartPoint>,
    pub field: String,
    pub generated_at_unix_ms: i64,
    pub total_pushed: u64,
}

impl ChartEmitter {
    pub fn new(
        ring: ParsedRing,
        target_points: usize,
        method: DownsampleMethod,
        field_filter: Option<SampleField>,
        interval: Duration,
    ) -> Self {
        Self {
            ring,
            target_points,
            method,
            field_filter,
            last_emit: RwLock::new(None),
            interval,
        }
    }

    /// Build a fresh `ChartWindow` from the current ring state.
    /// The frontend calls this on a 30 Hz timer (typically via
    /// `tauri::async_runtime::spawn` + a JavaScript `setInterval`).
    pub fn snapshot(&self) -> ChartWindow {
        let samples = self.ring.drain();
        let filtered: Vec<ParsedSample> = match self.field_filter {
            Some(f) => samples.into_iter().filter(|s| s.field == f).collect(),
            None => samples,
        };
        let points = downsample(&filtered, self.target_points, self.method)
            .unwrap_or_default();
        let field = match self.field_filter {
            Some(f) => format!("{:?}", f),
            None => "ALL".to_string(),
        };
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let total = self.ring.total_pushed();
        *self.last_emit.write() = Some(Instant::now());
        ChartWindow {
            points,
            field,
            generated_at_unix_ms: now,
            total_pushed: total,
        }
    }

    /// How long since the last snapshot(). The frontend can
    /// schedule itself to call snapshot() at `interval` to get
    /// 30 Hz updates (with interval = 33ms).
    pub fn time_since_last_emit(&self) -> Option<Duration> {
        self.last_emit.read().map(|t| t.elapsed())
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }
}

use std::time::SystemTime;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oe1022d_transport::{ParsedSample, SampleField, SampleStatus};

    fn fake_sample(t_mono_ns: u64, value: f64, field: SampleField) -> ParsedSample {
        ParsedSample {
            sample_in_frame: 0,
            t_mono_ns,
            t_wall_ns: t_mono_ns as i64,
            t_wall_ms: (t_mono_ns / 1_000_000) as i64,
            device_id: "SSI:LIA-OE1022D:TEST".into(),
            frame_sequence_no: 0,
            field,
            value,
            status: SampleStatus {
                transport_ok: true,
                frame_was_exact_size: true,
            },
            partial_warmup: false,
        }
    }

    #[test]
    fn ring_capacity_and_push() {
        let r = ParsedRing::new(3);
        assert_eq!(r.capacity(), 3);
        r.push(fake_sample(0, 0.0, SampleField::BX));
        r.push(fake_sample(1, 0.1, SampleField::BX));
        r.push(fake_sample(2, 0.2, SampleField::BX));
        r.push(fake_sample(3, 0.3, SampleField::BX)); // drops the oldest
        let v = r.drain();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].t_mono_ns, 1);
        assert_eq!(v[1].t_mono_ns, 2);
        assert_eq!(v[2].t_mono_ns, 3);
        assert_eq!(r.total_pushed(), 4);
    }

    #[test]
    fn ring_drain_partial() {
        let r = ParsedRing::new(10);
        r.push(fake_sample(0, 0.0, SampleField::BX));
        r.push(fake_sample(1, 0.1, SampleField::BX));
        let v = r.drain();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].t_mono_ns, 0);
    }

    #[test]
    fn downsample_latest_takes_most_recent_n() {
        let samples: Vec<ParsedSample> = (0..100)
            .map(|i| fake_sample(i, i as f64, SampleField::BX))
            .collect();
        let pts = downsample(&samples, 10, DownsampleMethod::Latest).unwrap();
        assert_eq!(pts.len(), 10);
        assert_eq!(pts[0].t_mono_ns, 90);
        assert_eq!(pts[9].t_mono_ns, 99);
    }

    #[test]
    fn downsample_minmax_envelope_preserves_extremes() {
        // 100 samples with one big spike at index 50.
        let mut samples: Vec<ParsedSample> = (0..100)
            .map(|i| fake_sample(i, 1.0 + (i as f64 * 0.001), SampleField::BX))
            .collect();
        samples[50] = fake_sample(50, 100.0, SampleField::BX);
        let pts = downsample(&samples, 10, DownsampleMethod::MinMaxEnvelope).unwrap();
        assert_eq!(pts.len(), 10);
        // The bucket containing sample 50 (index 50 in 10-bucket
        // split of 100 → bucket 5 covering [50,59]) should have
        // y_high = 100.
        let bucket_5 = &pts[5];
        assert!(bucket_5.y_high >= 50.0, "spike must show up in y_high: {}", bucket_5.y_high);
    }

    #[test]
    fn downsample_mean_bucket_averages() {
        let samples: Vec<ParsedSample> = (0..100)
            .map(|i| fake_sample(i, i as f64, SampleField::BX))
            .collect();
        let pts = downsample(&samples, 10, DownsampleMethod::MeanBucket).unwrap();
        assert_eq!(pts.len(), 10);
        // First bucket = samples 0..10, mean = 4.5.
        assert!((pts[0].y - 4.5).abs() < 1e-9);
        // Last bucket = samples 90..100, mean = 94.5.
        assert!((pts[9].y - 94.5).abs() < 1e-9);
    }

    #[test]
    fn downsample_rejects_zero_target() {
        let samples = vec![fake_sample(0, 1.0, SampleField::BX)];
        let err = downsample(&samples, 0, DownsampleMethod::Latest).unwrap_err();
        assert!(matches!(err, DownsampleError::ZeroTarget(_)));
    }

    #[test]
    fn downsample_empty_input_returns_empty() {
        let pts = downsample(&[], 10, DownsampleMethod::Latest).unwrap();
        assert!(pts.is_empty());
    }

    #[test]
    fn emitter_filters_by_field() {
        let ring = ParsedRing::new(100);
        for i in 0..50 {
            ring.push(fake_sample(i, i as f64, SampleField::BX));
            ring.push(fake_sample(i + 1000, i as f64 * 2.0, SampleField::BY));
        }
        let emitter = ChartEmitter::new(
            ring,
            10,
            DownsampleMethod::Latest,
            Some(SampleField::BX),
            Duration::from_millis(33),
        );
        let window = emitter.snapshot();
        assert_eq!(window.field, "BX");
        assert_eq!(window.points.len(), 10);
        for p in &window.points {
            // All values should be from the BX series (range
            // 0..49), not BY (range 0..98).
            assert!(p.y < 50.0, "BX-filtered point has BY value: {p:?}");
        }
    }

    #[test]
    fn emitter_emits_all_when_no_filter() {
        let ring = ParsedRing::new(100);
        for i in 0..30 {
            ring.push(fake_sample(i, i as f64, SampleField::BX));
        }
        let emitter = ChartEmitter::new(
            ring,
            5,
            DownsampleMethod::Latest,
            None,
            Duration::from_millis(33),
        );
        let window = emitter.snapshot();
        assert_eq!(window.field, "ALL");
        assert_eq!(window.points.len(), 5);
    }
}
