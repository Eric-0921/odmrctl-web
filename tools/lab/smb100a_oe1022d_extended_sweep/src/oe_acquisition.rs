use crate::oe_transport::OeSerialTransport;
use crate::types::{CommandAuditEntry, FrameToStepAlignment};
use odmr_logging::RawIndexEntry;
use odmr_oe1022d::RALL_FRAME_BYTES;
use std::time::Duration;

pub struct OeFrameCapture {
    pub raw_bytes: Vec<u8>,
    pub frame_len: usize,
    pub is_full_frame: bool,
    pub raw_offset: u64,
    pub frame_monotonic_ns: u64,
    pub elapsed_ms: u64,
    pub parsed_ok: bool,
    pub b_x_latest: Option<f64>,
    pub b_y_latest: Option<f64>,
    pub b_freq_latest: Option<f64>,
    pub b_x_all: Vec<f64>,
    pub b_y_all: Vec<f64>,
    pub parse_error: Option<String>,
}

pub struct OeAcquisitionResult {
    pub frames: Vec<OeFrameCapture>,
    pub index_entries: Vec<RawIndexEntry>,
    pub total_frames_attempted: usize,
    pub total_frames_captured: usize,
    pub total_frames_parsed: usize,
    pub total_frames_parse_failed: usize,
}

/// Acquire `count` RALL? frames using an already-open OE transport.
#[allow(clippy::too_many_arguments)]
pub fn acquire_frames(
    transport: &mut OeSerialTransport,
    audit: &mut Vec<CommandAuditEntry>,
    forbidden_attempted: &mut Vec<String>,
    count: u64,
    inter_frame_delay_ms: u64,
    frame_delay_ms: u64,
    run_id: &str,
    step_id: &str,
    _step_index: u64,
    _frequency_hz: f64,
    _rf_on: bool,
    _mod_on: bool,
    _fm_on: bool,
    _run_start_ns: u64,
) -> Result<OeAcquisitionResult, String> {
    let mut frames = Vec::new();
    let mut index_entries = Vec::new();
    let mut total_captured = 0usize;
    let mut total_parsed = 0usize;
    let mut total_parse_failed = 0usize;

    for frame_i in 0..count {
        let (raw, elapsed_ms) =
            transport.capture_rall_frame(audit, forbidden_attempted, frame_delay_ms)?;

        let frame_len = raw.len();
        let is_full = frame_len >= RALL_FRAME_BYTES;

        let (b_x, b_y, b_freq, parsed_ok, b_x_all, b_y_all, parse_error) = if is_full {
            match odmr_oe1022d::parse_rall_frame(&raw[..RALL_FRAME_BYTES]) {
                Ok(parsed) => {
                    let b_sample = odmr_oe1022d::latest_b_channel_sample(&parsed);
                    let bx_all = parsed.measurements.lockin_B_X_mv.clone();
                    let by_all = parsed.measurements.lockin_B_Y_mv.clone();
                    match b_sample {
                        Some(sample) => (
                            Some(sample.x_mv),
                            Some(sample.y_mv),
                            Some(sample.freq_hz),
                            true,
                            bx_all,
                            by_all,
                            None,
                        ),
                        None => (None, None, None, false, bx_all, by_all, None),
                    }
                }
                Err(e) => (
                    None, None, None, false,
                    Vec::new(), Vec::new(),
                    Some(format!("{:?}", e)),
                ),
            }
        } else {
            (None, None, None, false, Vec::new(), Vec::new(), None)
        };

        let global_frame_seq = index_entries.len() as u64;

        let capture = OeFrameCapture {
            raw_bytes: if is_full {
                raw[..RALL_FRAME_BYTES].to_vec()
            } else {
                raw.clone()
            },
            frame_len,
            is_full_frame: is_full,
            raw_offset: global_frame_seq * RALL_FRAME_BYTES as u64,
            frame_monotonic_ns: elapsed_ms * 1_000_000,
            elapsed_ms,
            parsed_ok,
            b_x_latest: b_x,
            b_y_latest: b_y,
            b_freq_latest: b_freq,
            b_x_all,
            b_y_all,
            parse_error,
        };

        if is_full {
            total_captured += 1;
        }
        if parsed_ok {
            total_parsed += 1;
        } else {
            total_parse_failed += 1;
        }

        index_entries.push(RawIndexEntry {
            schema_version: "0.2.0".into(),
            kind: "raw_index_entry".into(),
            run_id: run_id.into(),
            stream_id: "oe1022d".into(),
            offset_bytes: capture.raw_offset,
            length_bytes: capture.frame_len as u64,
            timestamp_unix_ms: crate::timeline::utc_now_ms(),
            step_id: Some(step_id.into()),
            sample_count: Some(50),
            frame_index: Some(global_frame_seq),
            duration_ms: Some(capture.elapsed_ms),
            parse_status: Some(if capture.parsed_ok {
                "ok".into()
            } else {
                "failed".into()
            }),
            notes: None,
        });

        frames.push(capture);

        if inter_frame_delay_ms > 0 && frame_i + 1 < count {
            std::thread::sleep(Duration::from_millis(inter_frame_delay_ms));
        }
    }

    Ok(OeAcquisitionResult {
        frames,
        index_entries,
        total_frames_attempted: count as usize,
        total_frames_captured: total_captured,
        total_frames_parsed: total_parsed,
        total_frames_parse_failed: total_parse_failed,
    })
}

/// Build alignment records mapping each captured frame to its RF step.
#[allow(clippy::too_many_arguments)]
pub fn build_alignment_for_step(
    frames: &[OeFrameCapture],
    step_id: &str,
    step_index: u64,
    repeat_index: u64,
    frequency_hz: f64,
    rf_on: bool,
    mod_on: bool,
    fm_on: bool,
    _run_start_ns: u64,
) -> Vec<FrameToStepAlignment> {
    frames
        .iter()
        .map(|f| {
            let global_seq = f.raw_offset / RALL_FRAME_BYTES as u64;
            FrameToStepAlignment {
                schema_version: "0.2.0".into(),
                frame_seq: global_seq,
                raw_offset: f.raw_offset,
                raw_nbytes: f.frame_len,
                step_id: step_id.into(),
                step_index,
                repeat_index,
                frequency_hz,
                rf_output_state: if rf_on { "on".into() } else { "off".into() },
                mod_state: if mod_on { "on".into() } else { "off".into() },
                fm_state: if fm_on { "on".into() } else { "off".into() },
                frame_monotonic_ns_since_run_start: f.frame_monotonic_ns,
                alignment_method: "software_step_active_window".into(),
                parse_status: if f.parsed_ok {
                    "ok".into()
                } else {
                    "failed".into()
                },
            }
        })
        .collect()
}
