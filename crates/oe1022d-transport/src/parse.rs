//! ParserThread core: turn a [`RawFrameEnvelope`] into 50
//! per-sample records with **1 ms-spaced** timestamps.
//!
//! C6 scope. Wraps the main-repo `odmr-oe1022d::parser::parse_rall_frame`
//! with:
//! - per-sample timestamp fan-out (the 50 points in a frame are
//!   spaced 1 ms apart in the device's view; we back-compute each
//!   point's `t_mono_ns` from `t_query_mono_ns`)
//! - "warmup" tolerance: real hardware delivers the first 1-2 frames
//!   with K1 residue from the prior `*IDN?` (see
//!   `oe1022d-manual-source.md` for the lab-measured sizes 13260 /
//!   12476 / 12288). We truncate to the canonical 12288 bytes and
//!   emit a `partial_warmup` flag so downstream stages can decide
//!   what to do.
//!
//! ## Threading
//!
//! The actual ParserThread is a future C5 acquisition-thread
//! concern; this module is the **single-sample-per-call** logic
//! the thread will loop over. The 50-sample fan-out is intentionally
//! **NOT** a Rayon par_iter: it is a 50-element array fill that
//! the OS thread executes in < 1 ms on the lab device.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

use odmr_oe1022d::parser::{
    parse_rall_frame, RallParseError, RALL_FRAME_BYTES,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::rall::{RawFrameEnvelope, TransportStatus};

/// One sample = one (time, value, field) triple, where `value` is a
/// single channel's measurement at a single point in time. The
/// `field` enum picks which channel (e.g. `B_X` or `AUX_ADC1`).
///
/// Per the v0.1 PRD: 1 ndjson line = 1 sample. The 50 samples in
/// one RALL? frame share `t_wall_recv_ms` (to ms precision) but
/// have distinct `t_mono_ns` (to ns precision) so ML training
/// pipelines can do ns-level time math.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSample {
    /// 0-indexed position of this sample within the source frame
    /// (0..=49 for a 50-sample frame). Useful for re-ordering if
    /// the downstream consumer batches samples in arrival order
    /// rather than time order.
    pub sample_in_frame: u8,
    /// Monotonic timestamp for this single sample. Derived as
    /// `t_query_mono_ns - (49 - i) * 1_000_000`. Converting to wall
    /// clock requires the `t_wall_query_ns` field of the source
    /// frame, which the parser exposes via `wall_ns_at_mono_origin`.
    pub t_mono_ns: u64,
    /// Wall-clock ns, derived from `t_wall_query_ns` plus the offset
    /// from the frame's `t_query_mono_ns` to this sample.
    pub t_wall_ns: i64,
    /// Convenience: ms-resolution wall clock for human display.
    pub t_wall_ms: i64,
    /// Device id, copied from the source frame for downstream
    /// filtering.
    pub device_id: String,
    /// Frame sequence number, copied from the source frame.
    pub frame_sequence_no: u64,
    /// Which channel / field this sample is from.
    pub field: SampleField,
    /// The parsed f64 value in volts (or Hz for the freq field).
    pub value: f64,
    /// Status flags copied from the frame, so a downstream
    /// consumer can decide whether to trust the value.
    pub status: SampleStatus,
    /// True if the source frame was a `partial_warmup` (K1 residue
    /// from the prior `*IDN?` probe). The first 12288 bytes were
    /// still parsed, so values are valid, but the consumer may
    /// want to drop these samples from training data.
    pub partial_warmup: bool,
}

/// Enumerates the 20 measurement fields in a RALL? frame, in the
/// order documented in `docs/equipment_manual/oe1022d/05_oe1022d_...
/// reading.md` §2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SampleField {
    AX,
    AY,
    AFreq,
    ANoise,
    AXh1,
    AYh1,
    AXh2,
    AYh2,
    BX,
    BY,
    BFreq,
    BNoise,
    BXh1,
    BYh1,
    BXh2,
    BYh2,
    AuxAdc1,
    AuxAdc2,
    AuxAdc3,
    AuxAdc4,
}

impl SampleField {
    /// Pull one specific 50-sample slice out of a parsed frame.
    /// Units: V for X/Y/Noise/Xh1/Yh1/Xh2/Yh2/AUX, Hz for Freq.
    /// (Caller is responsible for converting mV/Hz if desired; the
    /// parser stores them in the natural units used in the field
    /// name.)
    pub fn extract<'a>(&self, frame: &'a RallFrame) -> &'a [f64] {
        use odmr_oe1022d::parser::RallMeasurements;
        let m: &RallMeasurements = &frame.measurements;
        match self {
            Self::AX => &m.lockin_A_X_mv,
            Self::AY => &m.lockin_A_Y_mv,
            Self::AFreq => &m.lockin_A_freq_hz,
            Self::ANoise => &m.lockin_A_noise_mv,
            Self::AXh1 => &m.lockin_A_Xh1_mv,
            Self::AYh1 => &m.lockin_A_Yh1_mv,
            Self::AXh2 => &m.lockin_A_Xh2_mv,
            Self::AYh2 => &m.lockin_A_Yh2_mv,
            Self::BX => &m.lockin_B_X_mv,
            Self::BY => &m.lockin_B_Y_mv,
            Self::BFreq => &m.lockin_B_freq_hz,
            Self::BNoise => &m.lockin_B_noise_mv,
            Self::BXh1 => &m.lockin_B_Xh1_mv,
            Self::BYh1 => &m.lockin_B_Yh1_mv,
            Self::BXh2 => &m.lockin_B_Xh2_mv,
            Self::BYh2 => &m.lockin_B_Yh2_mv,
            Self::AuxAdc1 => &m.aux_adc1_v,
            Self::AuxAdc2 => &m.aux_adc2_v,
            Self::AuxAdc3 => &m.aux_adc3_v,
            Self::AuxAdc4 => &m.aux_adc4_v,
        }
    }
}

/// Status carried per sample.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SampleStatus {
    /// True if the source frame's `transport_status` was `Ok`.
    pub transport_ok: bool,
    /// True if the source frame was exactly 12288 bytes.
    /// False indicates a K1 warmup residue; values are still
    /// valid (we truncated to 12288) but the consumer may want
    /// to drop the sample.
    pub frame_was_exact_size: bool,
}

/// Outcome of `parse_envelope` — a [`ParseReport`] that bundles
/// the parsed [`RallFrame`] with the per-sample timestamp fan-out
/// already pre-computed for the caller. We do NOT pre-build 20 × 50
/// = 1000 `ParsedSample` records here because the caller may only
/// care about a subset of fields; the [`RallFrame`] gives full
/// access and the per-sample timestamps are derived on demand by
/// [`expand_to_samples`].
#[derive(Debug, Clone)]
pub struct ParseReport {
    /// Parsed frame (20 params × 50 samples + config + padding flag).
    pub frame: RallFrame,
    /// True if the source envelope's `raw` was longer than
    /// `RALL_FRAME_BYTES`; we truncated to the canonical size and
    /// the tail was discarded.
    pub partial_warmup: bool,
    /// Number of bytes of K1 residue that were discarded.
    pub discarded_bytes: usize,
}

#[derive(Debug, Error)]
pub enum ParseEnvelopeError {
    #[error("frame too short: expected at least {expected} bytes, got {actual}")]
    FrameTooShort { expected: usize, actual: usize },
    #[error("transport status was not Ok: {0:?}")]
    TransportNotOk(TransportStatus),
    #[error("parser failed: {0}")]
    Parser(#[from] RallParseError),
}

/// Parse one `RawFrameEnvelope` into a `ParseReport`.
///
/// Tolerates frames that are **longer** than the canonical 12288
/// bytes (K1 residue from a prior `*IDN?`); truncates the tail and
/// records `partial_warmup = true`. Frames **shorter** than 12288
/// are rejected (`FrameTooShort`).
pub fn parse_envelope(env: &RawFrameEnvelope) -> Result<ParseReport, ParseEnvelopeError> {
    if env.raw.len() < RALL_FRAME_BYTES {
        return Err(ParseEnvelopeError::FrameTooShort {
            expected: RALL_FRAME_BYTES,
            actual: env.raw.len(),
        });
    }

    // K1 residue: if the device returned more than 12288 bytes,
    // the first 12288 are the canonical RALL? frame and the rest
    // is leftover IDN? response. We always parse the first 12288
    // and discard the tail.
    let canonical = &env.raw[..RALL_FRAME_BYTES];
    let discarded_bytes = env.raw.len() - RALL_FRAME_BYTES;
    let partial_warmup = discarded_bytes > 0;

    let frame = parse_rall_frame(canonical)?;
    Ok(ParseReport {
        frame,
        partial_warmup,
        discarded_bytes,
    })
}

/// Expand a `ParseReport` into 50 per-sample records for one
/// specific `field`. The samples carry the 1 ms-spaced timestamps
/// derived from the source envelope's `t_query_mono_ns`.
///
/// `field` picks which of the 20 measurement channels to expand.
pub fn expand_to_samples(
    env: &RawFrameEnvelope,
    report: &ParseReport,
    field: SampleField,
) -> Vec<ParsedSample> {
    let samples_data = field.extract(&report.frame);
    debug_assert_eq!(samples_data.len(), 50, "RALL? frame must have 50 samples per channel");

    // Pre-compute the offset from the source frame's t_query_mono_ns
    // to its t_wall_recv_ns, in nanoseconds. We can use this to map
    // any per-sample t_mono_ns to a per-sample t_wall_ns.
    //
    // The conversion is: t_wall_ns(mono_t) =
    //   env.t_wall_query_ns + (mono_t - env.t_query_mono_ns)
    //
    // because t_query_mono_ns and t_wall_query_ns are recorded at
    // the same wall instant (the moment we wrote the RALL? command).
    let query_wall_ns = env.t_wall_query_ns;
    let query_mono_ns = env.t_query_mono_ns;

    let partial_warmup = report.partial_warmup;
    let frame_was_exact_size = env.raw.len() == RALL_FRAME_BYTES;
    let transport_ok = matches!(env.transport_status, TransportStatus::Ok);

    let mut out = Vec::with_capacity(50);
    for i in 0..50 {
        // Frame-internal: sample i is at t_query - (49 - i) * 1ms.
        // The device sends 50 samples per frame covering the
        // 50 ms window immediately before t_query, with sample 0
        // being the oldest and sample 49 the newest.
        let offset_ns = (49 - i) as u64 * 1_000_000;
        let t_mono_ns = query_mono_ns.saturating_sub(offset_ns);
        let t_wall_ns = query_wall_ns.saturating_sub(offset_ns as i64);

        out.push(ParsedSample {
            sample_in_frame: i as u8,
            t_mono_ns,
            t_wall_ns,
            t_wall_ms: t_wall_ns / 1_000_000,
            device_id: env.device_id.clone(),
            frame_sequence_no: env.sequence_no,
            field,
            value: samples_data[i],
            status: SampleStatus {
                transport_ok,
                frame_was_exact_size,
            },
            partial_warmup,
        });
    }
    out
}

/// Convenience: parse + expand to samples for a single field.
pub fn parse_and_expand(
    env: &RawFrameEnvelope,
    field: SampleField,
) -> Result<Vec<ParsedSample>, ParseEnvelopeError> {
    let report = parse_envelope(env)?;
    Ok(expand_to_samples(env, &report, field))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use odmr_oe1022d::parser::RallMeasurements;
    use std::time::Instant;

    /// Build a synthetic 12288-byte RALL? frame where channel `field`
    /// has samples `value_at(i) = base + i as f64 * step`, and
    /// every other channel is filled with the same template.
    fn make_frame_with_field(field: SampleField, base: f64, step: f64) -> Vec<u8> {
        let mut frame = vec![0u8; RALL_FRAME_BYTES];
        let channel_byte_start = match field {
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
            let off = channel_byte_start + i * 8;
            frame[off..off + 8].copy_from_slice(&bytes);
        }
        frame
    }

    fn make_envelope(raw: Vec<u8>, sequence_no: u64) -> RawFrameEnvelope {
        let origin = Instant::now();
        let query_mono = origin;
        let query_wall_ns: i64 = 1_780_206_577_446_000_000; // 2026-05-31 ish
        RawFrameEnvelope {
            device_id: "SSI:LIA-OE1022D:D6130220".into(),
            sequence_no,
            t_query_mono_ns: query_mono.duration_since(origin).as_nanos() as u64,
            t_recv_mono_ns: query_mono.duration_since(origin).as_nanos() as u64 + 900_000_000,
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
    fn parse_exact_size_frame_yields_clean_report() {
        let frame = make_frame_with_field(SampleField::BX, 0.001, 0.0001);
        let env = make_envelope(frame, 0);
        let report = parse_envelope(&env).expect("parse exact size");
        assert!(!report.partial_warmup);
        assert_eq!(report.discarded_bytes, 0);
    }

    #[test]
    fn parse_warmup_frame_truncates_and_marks() {
        // K1 residue: real lab shows frames of 12476, 13260, etc.
        let mut frame = make_frame_with_field(SampleField::BX, 0.001, 0.0001);
        for _ in 0..(13260 - 12288) {
            frame.push(0x42); // junk IDN? residue
        }
        let env = make_envelope(frame, 7);
        let report = parse_envelope(&env).expect("parse warmup");
        assert!(report.partial_warmup);
        assert_eq!(report.discarded_bytes, 13260 - 12288);
    }

    #[test]
    fn parse_short_frame_rejected() {
        let env = make_envelope(vec![0u8; 1000], 0);
        let err = parse_envelope(&env).unwrap_err();
        assert!(matches!(err, ParseEnvelopeError::FrameTooShort { .. }));
    }

    #[test]
    fn expand_produces_50_samples_with_1ms_spaced_timestamps() {
        let frame = make_frame_with_field(SampleField::BX, 1.0, 0.01);
        // Use a real Instant and let the envelope inherit the
        // t_query/t_recv time difference.
        let origin = Instant::now();
        let query_mono_ns = origin.elapsed().as_nanos() as u64 + 60_000_000_000; // 60s in
        let recv_mono_ns = query_mono_ns + 900_000_000; // ~900ms read
        let query_wall_ns: i64 = 1_780_206_577_446_000_000;
        let env = RawFrameEnvelope {
            device_id: "SSI:LIA-OE1022D:D6130220".into(),
            sequence_no: 5,
            t_query_mono_ns: query_mono_ns,
            t_recv_mono_ns: recv_mono_ns,
            t_wall_recv_ms: query_wall_ns / 1_000_000 + 900,
            t_wall_recv_ns: query_wall_ns + 900_000_000,
            t_wall_query_ns: query_wall_ns,
            read_duration_ns: 900_000_000,
            command: "RALL?".into(),
            raw: frame,
            transport_status: TransportStatus::Ok,
        };
        let report = parse_envelope(&env).unwrap();
        let samples = expand_to_samples(&env, &report, SampleField::BX);
        assert_eq!(samples.len(), 50);
        // First sample is the oldest in the 50ms window.
        assert_eq!(samples[0].sample_in_frame, 0);
        assert_eq!(samples[49].sample_in_frame, 49);
        // 1ms = 1_000_000 ns spacing.
        for i in 1..50 {
            let dt = samples[i].t_mono_ns - samples[i - 1].t_mono_ns;
            assert_eq!(dt, 1_000_000, "sample {i} not 1ms after previous");
        }
        // Wall clock also 1ms apart.
        for i in 1..50 {
            let dt = samples[i].t_wall_ns - samples[i - 1].t_wall_ns;
            assert_eq!(dt, 1_000_000);
        }
        // Values: B-X[i] = 1.0 + i * 0.01.
        for i in 0..50 {
            let expected = 1.0 + i as f64 * 0.01;
            assert!(
                (samples[i].value - expected).abs() < 1e-12,
                "sample {i}: expected {expected}, got {}",
                samples[i].value
            );
        }
    }

    #[test]
    fn expand_carries_partial_warmup_flag() {
        // Frame with K1 residue.
        let mut frame = make_frame_with_field(SampleField::BY, 0.5, 0.001);
        frame.resize(12476, 0); // 188 bytes of junk
        let env = make_envelope(frame, 3);
        let samples = parse_and_expand(&env, SampleField::BY).expect("parse_and_expand");
        assert!(samples.iter().all(|s| s.partial_warmup));
        assert!(samples.iter().all(|s| !s.status.frame_was_exact_size));
    }

    #[test]
    fn expand_carries_exact_size_flag_on_clean_frame() {
        let frame = make_frame_with_field(SampleField::BX, 0.0, 0.0);
        let env = make_envelope(frame, 0);
        let samples = parse_and_expand(&env, SampleField::BX).expect("parse_and_expand");
        assert!(samples.iter().all(|s| !s.partial_warmup));
        assert!(samples.iter().all(|s| s.status.frame_was_exact_size));
        assert!(samples.iter().all(|s| s.status.transport_ok));
    }

    #[test]
    fn all_20_fields_have_unique_byte_offsets() {
        // Each field's 50-sample block must land at a unique byte
        // offset. This is the contract that lets us index by
        // (field, sample_in_frame) without colliding.
        let mut seen: Vec<(SampleField, usize)> = Vec::new();
        let fields = [
            SampleField::AX, SampleField::AY, SampleField::AFreq,
            SampleField::ANoise, SampleField::AXh1, SampleField::AYh1,
            SampleField::AXh2, SampleField::AYh2,
            SampleField::BX, SampleField::BY, SampleField::BFreq,
            SampleField::BNoise, SampleField::BXh1, SampleField::BYh1,
            SampleField::BXh2, SampleField::BYh2,
            SampleField::AuxAdc1, SampleField::AuxAdc2,
            SampleField::AuxAdc3, SampleField::AuxAdc4,
        ];
        for &f in &fields {
            let frame = make_frame_with_field(f, 1.0, 0.0);
            // Read sample 0 from the field's offset and confirm it
            // is the value we wrote.
            let byte_start = match f {
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
            let val_bytes: [u8; 8] = frame[byte_start..byte_start + 8].try_into().unwrap();
            let val = f64::from_be_bytes(val_bytes);
            assert!(
                (val - 1.0).abs() < 1e-12,
                "field {:?} at offset {byte_start} did not round-trip",
                f
            );
            assert!(!seen.iter().any(|(_, off)| *off == byte_start));
            seen.push((f, byte_start));
        }
        assert_eq!(seen.len(), 20);
    }

    #[test]
    fn parse_envelope_real_device_warmup_size() {
        // Exact replica of the lab observation: [13260, 12476,
        // 12288, 12288, 12288] for the first 5 frames.
        let sizes = [13260, 12476, 12288, 12288, 12288];
        for (i, &size) in sizes.iter().enumerate() {
            let mut frame = make_frame_with_field(SampleField::BX, 0.001, 0.0001);
            frame.resize(size, 0);
            let env = make_envelope(frame, i as u64);
            let report = parse_envelope(&env).unwrap_or_else(|e| {
                panic!("frame {i} (size={size}) failed: {e}");
            });
            if size > 12288 {
                assert!(
                    report.partial_warmup,
                    "frame {i} (size={size}) should be warmup"
                );
                assert_eq!(report.discarded_bytes, size - 12288);
            } else {
                assert!(!report.partial_warmup);
                assert_eq!(report.discarded_bytes, 0);
            }
        }
    }
}

// Re-export RallFrame, RallMeasurements etc. for downstream code
// that wants to inspect the parsed structure.
pub use odmr_oe1022d::parser::RallFrame;
