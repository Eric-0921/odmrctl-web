//! Replay mode: read prior run artifacts, rebuild statistics, compare command audit.

use crate::types::*;
use odmr_oe1022d::{latest_b_channel_sample, parse_rall_frame, RALL_FRAME_BYTES};
use std::fs;
use std::path::{Path, PathBuf};

/// Load a replay source configuration from a prior run directory.
pub fn load_replay_source(run_root: &str, run_id: &str) -> Result<ReplaySourceConfig, String> {
    let base = PathBuf::from(run_root).join(run_id);

    let raw_bin_path = base.join("raw/oe1022d_rall.rawbin");
    let index_path = base.join("index.jsonl");
    let alignment_path = base.join("alignment/frame_to_rf_step_alignment.jsonl");
    let command_audit_path = base.join("command_audit.jsonl");
    let step_plan_path = base.join("rf/step_plan.json");

    Ok(ReplaySourceConfig {
        schema_version: "0.2.0".into(),
        kind: "replay_source".into(),
        source_run_id: run_id.to_string(),
        source_run_root: run_root.to_string(),
        raw_bin_path: raw_bin_path.to_string_lossy().to_string(),
        index_path: index_path.to_string_lossy().to_string(),
        alignment_path: alignment_path.to_string_lossy().to_string(),
        command_audit_path: command_audit_path.to_string_lossy().to_string(),
        step_plan_path: if step_plan_path.exists() {
            Some(step_plan_path.to_string_lossy().to_string())
        } else {
            None
        },
    })
}

/// Read frame boundaries from index.jsonl.
pub fn read_index_entries(path: &Path) -> Result<Vec<(u64, u64, String)>, String> {
    let contents = fs::read_to_string(path).map_err(|e| format!("read index: {}", e))?;
    let mut entries = Vec::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("parse index line: {}", e))?;
        let offset = v["offset_bytes"].as_u64().unwrap_or(0);
        let length = v["length_bytes"]
            .as_u64()
            .unwrap_or(RALL_FRAME_BYTES as u64);
        let step_id = v["step_id"].as_str().unwrap_or("unknown").to_string();
        entries.push((offset, length, step_id));
    }
    Ok(entries)
}

/// Read raw frames from raw bin using index entries.
pub fn read_raw_frames(
    raw_bin_path: &Path,
    index_entries: &[(u64, u64, String)],
) -> Result<Vec<(u64, Vec<u8>, String)>, String> {
    let data = fs::read(raw_bin_path).map_err(|e| format!("read raw bin: {}", e))?;
    let mut frames = Vec::new();
    for (offset, length, step_id) in index_entries {
        let start = *offset as usize;
        let end = start + *length as usize;
        if end <= data.len() {
            let frame_data = data[start..end].to_vec();
            frames.push((*offset, frame_data, step_id.clone()));
        }
    }
    Ok(frames)
}

/// Rebuild B-channel statistics by parsing raw frames.
pub fn rebuild_statistics(
    frames: &[(u64, Vec<u8>, String)],
) -> Result<(Vec<RfStepSummaryEntry>, RunStabilitySummary, String), String> {
    let mut step_summaries: Vec<RfStepSummaryEntry> = Vec::new();
    let mut total_captured: u64 = 0;
    let mut total_parsed: u64 = 0;
    let mut total_failed: u64 = 0;

    // Group frames by step_id
    let mut step_frames: std::collections::HashMap<String, Vec<&[u8]>> =
        std::collections::HashMap::new();
    for (_offset, data, step_id) in frames {
        if data.len() >= RALL_FRAME_BYTES {
            step_frames
                .entry(step_id.clone())
                .or_default()
                .push(&data[..RALL_FRAME_BYTES]);
            total_captured += 1;
        }
    }

    for (step_id, frame_datas) in &step_frames {
        let mut b_x_all: Vec<f64> = Vec::new();
        let mut b_y_all: Vec<f64> = Vec::new();
        let mut parsed = 0u64;
        let mut failed = 0u64;
        let mut freq = None;

        for data in frame_datas {
            match parse_rall_frame(data) {
                Ok(parsed_frame) => {
                    b_x_all.extend(parsed_frame.measurements.lockin_B_X_mv.iter().copied());
                    b_y_all.extend(parsed_frame.measurements.lockin_B_Y_mv.iter().copied());
                    if let Some(sample) = latest_b_channel_sample(&parsed_frame) {
                        freq = Some(sample.freq_hz);
                    }
                    parsed += 1;
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }

        total_parsed += parsed;
        total_failed += failed;

        let (bx_mean, bx_std) = if !b_x_all.is_empty() {
            let n = b_x_all.len() as f64;
            let sum: f64 = b_x_all.iter().sum();
            let mean = sum / n;
            let var = b_x_all.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
            (Some(mean), Some(var.sqrt()))
        } else {
            (None, None)
        };

        let (by_mean, by_std) = if !b_y_all.is_empty() {
            let n = b_y_all.len() as f64;
            let sum: f64 = b_y_all.iter().sum();
            let mean = sum / n;
            let var = b_y_all.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
            (Some(mean), Some(var.sqrt()))
        } else {
            (None, None)
        };

        step_summaries.push(RfStepSummaryEntry {
            step_id: step_id.clone(),
            repeat_index: 0,
            frequency_hz: freq.unwrap_or(0.0),
            frequency_verified_hz: freq,
            rf_output_on: true,
            frames_requested: frame_datas.len() as u64,
            frames_captured: frame_datas.len() as u64,
            frames_parsed: parsed,
            frames_parse_failed: failed,
            step_passed: true,
            b_x_mean: bx_mean,
            b_x_std: bx_std,
            b_y_mean: by_mean,
            b_y_std: by_std,
            duration_ms: 0,
        });
    }

    let parse_failure_rate = if total_captured > 0 {
        total_failed as f64 / total_captured as f64
    } else {
        0.0
    };

    let stability = RunStabilitySummary {
        schema_version: "0.2.0".into(),
        kind: "run_stability_summary".into(),
        run_id: "replay".into(),
        frames_requested: total_captured,
        frames_captured: total_captured,
        frames_parsed: total_parsed,
        frames_parse_failed: total_failed,
        parse_failure_rate,
        steps_total: step_frames.len() as u64,
        steps_passed: step_frames.len() as u64,
        final_rf_off: true,
        final_mod_off: true,
        final_fm_off: true,
        final_syst_err_clean: true,
        emergency_shutdown_triggered: false,
        no_forbidden_commands_sent: true,
    };

    Ok((step_summaries, stability, "replay".into()))
}

/// Load command audit entries from a prior run's command_audit.jsonl.
pub fn load_command_audit(path: &Path) -> Result<Vec<M3_4CommandAuditEntry>, String> {
    let contents = fs::read_to_string(path).map_err(|e| format!("read audit: {}", e))?;
    let mut entries = Vec::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let entry: M3_4CommandAuditEntry =
            serde_json::from_str(line).map_err(|e| format!("parse audit line: {}", e))?;
        entries.push(entry);
    }
    Ok(entries)
}
