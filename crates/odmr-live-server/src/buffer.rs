//! Thread-safe ring buffer for real-time trace points.

use crate::types::TracePoint;
use std::collections::VecDeque;

/// Bounded ring buffer of trace points, shared between consumer and HTTP handler.
pub struct TraceRingBuffer {
    buf: VecDeque<TracePoint>,
    cap: usize,
    frames_total: u64,
    frames_unique: u64,
    total_read_us: u64,
    read_samples: u64,
}

impl TraceRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: VecDeque::with_capacity(capacity),
            cap: capacity,
            frames_total: 0,
            frames_unique: 0,
            total_read_us: 0,
            read_samples: 0,
        }
    }

    /// Push all 50 samples from a non-duplicate frame.
    pub fn push_frame(&mut self, points: &[TracePoint; 50], is_dup: bool, read_time_us: u64) {
        self.frames_total += 1;
        if !is_dup {
            self.frames_unique += 1;
        }
        self.total_read_us += read_time_us;
        self.read_samples += 1;

        // Only enqueue points for unique frames
        if !is_dup {
            for pt in points {
                if self.buf.len() >= self.cap {
                    self.buf.pop_front();
                }
                self.buf.push_back(pt.clone());
            }
        }
    }

    /// Snapshot of all points currently in the ring.
    pub fn snapshot(&self) -> Vec<TracePoint> {
        self.buf.iter().cloned().collect()
    }

    /// Average read time in microseconds.
    pub fn avg_read_us(&self) -> u64 {
        self.total_read_us
            .checked_div(self.read_samples)
            .unwrap_or(0)
    }

    /// Duplicate rate as a fraction.
    pub fn dup_rate(&self) -> f64 {
        if self.frames_total == 0 {
            0.0
        } else {
            (self.frames_total - self.frames_unique) as f64 / self.frames_total as f64
        }
    }

    pub fn frames_total(&self) -> u64 {
        self.frames_total
    }

    pub fn frames_unique(&self) -> u64 {
        self.frames_unique
    }
}
