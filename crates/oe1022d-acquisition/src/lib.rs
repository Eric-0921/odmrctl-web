//! oe1022d-acquisition — 4-thread acquisition core
//! (acquisition / parser / writer / downsample) with ndjson
//! persistence.
//!
//! C7 scope covers the writer half:
//! - [`RunWriter`]: one per run, holds `samples.ndjson` and
//!   `events.jsonl` in append mode, with 8 KB write buffers.
//! - [`AcqEvent`]: structured event line for `events.jsonl`.
//! - [`RunConfig`]: per-run configuration (run dir, fields, buffer).
//!
//! The full 4-thread AcquisitionCore (C5 thread + parser + writer
//! + downsample tied together) is a later commit; C7 only delivers
//! the writer so the parser and the data-shape can be integration
//! tested in isolation.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod ndjson;

pub use ndjson::{AcqEvent, RunConfig, RunWriter, WriterError, WriterStats};
