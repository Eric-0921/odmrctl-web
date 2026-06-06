//! Types shared between server, buffer, and frontend.

use serde::Serialize;

/// A single downsampled trace point. One per RALL? sample (50 per frame).
#[derive(Debug, Clone, Serialize)]
pub struct TracePoint {
    /// Elapsed seconds since collection start
    pub elapsed_s: f64,
    /// B-channel X (in-phase), millivolts
    pub bx_mv: f64,
    /// B-channel Y (quadrature), millivolts
    pub by_mv: f64,
    /// B-channel reference frequency, Hz
    pub freq_hz: f64,
}

/// Snapshot of the ring buffer returned by GET /api/trace.
#[derive(Debug, Clone, Serialize)]
pub struct TraceSnapshot {
    pub points: Vec<TracePoint>,
    pub frames_total: u64,
    pub frames_unique: u64,
    pub dup_rate: f64,
    pub avg_read_us: u64,
}

/// Collector health returned by GET /api/stats.
#[derive(Debug, Clone, Serialize)]
pub struct CollectorStatus {
    pub frames_captured: u64,
    pub frames_duplicated: u64,
    pub frames_parse_error: u64,
    pub total_reads_attempted: u64,
    pub avg_read_time_us: u64,
    pub running: bool,
}
