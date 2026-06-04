//! Real hardware execution: SMB100A + OE1022D recipe-driven sweep.

use crate::cli::Cli;
use crate::oe_bridge::OeSerialTransport;
use crate::smb_bridge::SmbTransport;
use crate::timeline::utc_now_ms;
use crate::types::*;
use odmr_oe1022d::{parse_rall_frame, RALL_FRAME_BYTES};
use std::fs;

pub fn run_real(
    cli: &Cli,
    recipe: &M3_4Recipe,
    resolved: &M3_4ResolvedRecipe,
    plan_entries: &[CommandPlanEntry],
    run_dir: &odmr_logging::RunDirectory,
) -> Result<(M3_4RunResult, Vec<M3_4CommandAuditEntry>), String> {
    let mut audit: Vec<M3_4CommandAuditEntry> = Vec::new();
    let mut step_results: Vec<RfStepSummaryEntry> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut total_requested: u64 = 0;
    let mut total_captured: u64 = 0;
    let mut total_parsed: u64 = 0;
    let mut total_failed: u64 = 0;
    let mut alignment_entries: Vec<String> = Vec::new(); // JSONL lines
    let mut index_entries: Vec<String> = Vec::new(); // JSONL lines for replay
    let mut raw_offset: u64 = 0;
    let mut emergency = false;

    let delay_ms = cli.smb_query_delay_ms;
    let frame_delay_ms = cli.oe_frame_delay_ms;
    let inter_frame_ms = 20u64; // default inter-frame delay

    // ----- Connect SMB100A -----
    let mut smb = SmbTransport::connect(&cli.smb_host, cli.smb_port, cli.smb_timeout_ms, delay_ms)?;

    // ----- Preflight -----
    let ts = utc_now_ms();
    let idn = smb.query("*IDN?", &mut audit, ts)?;
    let _outp = smb.query("OUTP?", &mut audit, ts)?;
    let _mod = smb.query("MOD:STAT?", &mut audit, ts)?;
    let _err = smb.query("SYST:ERR?", &mut audit, ts)?;
    let _freq = smb.query("FREQ?", &mut audit, ts)?;
    let _pow = smb.query("POW?", &mut audit, ts)?;

    // Write preflight snapshot
    let snapshot_before: serde_json::Value = serde_json::json!({
        "schema_version": "0.2.0",
        "idn": idn,
        "preflight_complete": true,
    });
    run_dir
        .write_json_artifact("metadata/smb100a_snapshot_before.json", &snapshot_before)
        .map_err(|e| format!("write snapshot before: {}", e))?;

    // ----- Configure SMB -----
    let ts = utc_now_ms();
    smb.query(&format!("POW {:.1}", recipe.rf.power_dbm), &mut audit, ts)?;
    smb.query("POW:ALC AUTO", &mut audit, ts)?;
    smb.query("FM:SOUR INT", &mut audit, ts)?;
    smb.query(
        &format!("FM:DEV {:.0}", recipe.modulation.fm_deviation_hz),
        &mut audit,
        ts,
    )?;
    smb.query("FM:STAT ON", &mut audit, ts)?;
    smb.query("MOD:STAT ON", &mut audit, ts)?;

    if let Some(ref lf) = recipe.modulation.internal_lf {
        if lf.enabled {
            smb.query(&format!("LFO:FREQ {:.0}", lf.frequency_hz), &mut audit, ts)?;
            smb.query(&format!("LFO:SHAP {}", lf.shape), &mut audit, ts)?;
            smb.query(&format!("LFO:VOLT {:.3}", lf.voltage_v), &mut audit, ts)?;
        }
    }

    // ----- Connect OE1022D -----
    let mut oe = OeSerialTransport::connect(&cli.oe_port, cli.oe_baud, cli.oe_timeout_ms)?;
    let ts = utc_now_ms();
    let oe_idn = oe.query_identity(&mut audit, ts)?;
    run_dir
        .write_json_artifact(
            "metadata/oe1022d_identity.json",
            &serde_json::json!({
                "schema_version": "0.2.0",
                "idn": oe_idn,
            }),
        )
        .map_err(|e| format!("write oe identity: {}", e))?;

    // ----- Create raw bin file -----
    let raw_bin_path = run_dir.run_directory_path().join("raw/oe1022d_rall.rawbin");
    let raw_dir = raw_bin_path.parent().unwrap();
    fs::create_dir_all(raw_dir).map_err(|e| format!("create raw dir: {}", e))?;
    let mut raw_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&raw_bin_path)
        .map_err(|e| format!("open raw bin: {}", e))?;
    use std::io::Write;

    // ----- Sweep loop -----
    for step in &resolved.steps {
        if emergency {
            break;
        }

        let ts = utc_now_ms();

        // OUTP OFF before frequency change
        if let Err(e) = smb.query("OUTP OFF", &mut audit, ts) {
            errors.push(format!("OUTP OFF failed: {}", e));
            emergency = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Set frequency
        let freq_cmd = format!("FREQ {:.0}", step.frequency_hz);
        if let Err(e) = smb.query(&freq_cmd, &mut audit, ts) {
            errors.push(format!("FREQ set failed: {}", e));
            emergency = true;
            break;
        }

        // Verify frequency
        let freq_resp = smb.query("FREQ?", &mut audit, ts)?;
        let freq_verified: Option<f64> = freq_resp.parse().ok();

        // OUTP ON
        if !cli.operator_approves_real_run {
            return Err("operator_approves_real_run required for real mode".into());
        }
        if let Err(e) = smb.query("OUTP ON", &mut audit, ts) {
            errors.push(format!("OUTP ON failed: {}", e));
            emergency = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Acquire frames
        let mut step_frames: Vec<Vec<u8>> = Vec::new();
        let mut step_parsed = 0u64;
        let mut step_failed = 0u64;
        let mut b_x_vectors: Vec<Vec<f64>> = Vec::new();
        let mut b_y_vectors: Vec<Vec<f64>> = Vec::new();

        for _f in 0..step.frames_to_acquire {
            let ts = utc_now_ms();
            match oe.capture_frame(&mut audit, ts, frame_delay_ms) {
                Ok((data, _elapsed)) => {
                    total_requested += 1;
                    let frame_len = data.len();
                    let is_full = frame_len >= RALL_FRAME_BYTES;

                    if is_full {
                        total_captured += 1;
                        let frame_data = &data[..RALL_FRAME_BYTES];
                        raw_file
                            .write_all(frame_data)
                            .map_err(|e| format!("write raw: {}", e))?;

                        index_entries.push(
                            serde_json::to_string(&serde_json::json!({
                                "offset_bytes": raw_offset,
                                "length_bytes": RALL_FRAME_BYTES,
                                "step_id": step.step_id,
                            }))
                            .unwrap_or_default(),
                        );

                        match parse_rall_frame(frame_data) {
                            Ok(parsed) => {
                                total_parsed += 1;
                                step_parsed += 1;
                                let b_x = parsed.measurements.lockin_B_X_mv.clone();
                                let b_y = parsed.measurements.lockin_B_Y_mv.clone();
                                b_x_vectors.push(b_x);
                                b_y_vectors.push(b_y);

                                let parse_status = "ok";
                                alignment_entries.push(
                                    serde_json::to_string(&serde_json::json!({
                                        "frame_seq": total_captured - 1,
                                        "raw_offset": raw_offset,
                                        "step_id": step.step_id,
                                        "frequency_hz": step.frequency_hz,
                                        "parse_status": parse_status,
                                    }))
                                    .unwrap_or_default(),
                                );
                            }
                            Err(e) => {
                                total_failed += 1;
                                step_failed += 1;
                                alignment_entries.push(
                                    serde_json::to_string(&serde_json::json!({
                                        "frame_seq": total_captured - 1,
                                        "raw_offset": raw_offset,
                                        "step_id": step.step_id,
                                        "frequency_hz": step.frequency_hz,
                                        "parse_status": "failed",
                                        "parse_error": format!("{:?}", e),
                                    }))
                                    .unwrap_or_default(),
                                );
                            }
                        }
                        raw_offset += RALL_FRAME_BYTES as u64;
                    }
                    step_frames.push(data);
                }
                Err(e) => {
                    errors.push(format!("frame capture error: {}", e));
                }
            }

            if inter_frame_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(inter_frame_ms));
            }
        }

        // OUTP OFF after acquisition
        let ts = utc_now_ms();
        let _ = smb.query("OUTP OFF", &mut audit, ts);

        // Compute step statistics
        let (bx_mean, bx_std) = compute_stats(&b_x_vectors);
        let (by_mean, by_std) = compute_stats(&b_y_vectors);

        step_results.push(RfStepSummaryEntry {
            step_id: step.step_id.clone(),
            repeat_index: step.repeat_index,
            frequency_hz: step.frequency_hz,
            frequency_verified_hz: freq_verified,
            rf_output_on: true,
            frames_requested: step.frames_to_acquire,
            frames_captured: step_frames.len() as u64,
            frames_parsed: step_parsed,
            frames_parse_failed: step_failed,
            step_passed: step_failed == 0,
            b_x_mean: bx_mean,
            b_x_std: bx_std,
            b_y_mean: by_mean,
            b_y_std: by_std,
            duration_ms: 0,
        });
    }

    // ----- Final Shutdown -----
    let ts = utc_now_ms();
    let _ = smb.query("OUTP OFF", &mut audit, ts);
    let _ = smb.query("MOD:STAT OFF", &mut audit, ts);
    let final_fm_off = if !cli.leave_fm_config_enabled {
        let _ = smb.query("FM:STAT OFF", &mut audit, ts);
        true
    } else {
        false
    };

    // SYST:ERR? x3
    let mut syst_err_clean = true;
    for _ in 0..3 {
        let resp = smb.query("SYST:ERR?", &mut audit, ts).unwrap_or_default();
        if !resp.starts_with("0,") {
            syst_err_clean = false;
        }
    }

    // Final state queries
    let final_outp = smb.query("OUTP?", &mut audit, ts).unwrap_or_default();
    let final_mod = smb.query("MOD:STAT?", &mut audit, ts).unwrap_or_default();
    let final_rf_off = final_outp.contains('0') || final_outp.contains("OFF");
    let final_mod_off = final_mod.contains('0') || final_mod.contains("OFF");

    // Write final snapshot
    let snapshot_after = serde_json::json!({
        "schema_version": "0.2.0",
        "outp": final_outp,
        "mod_stat": final_mod,
        "syst_err_clean": syst_err_clean,
    });
    run_dir
        .write_json_artifact("metadata/smb100a_snapshot_after.json", &snapshot_after)
        .map_err(|e| format!("write snapshot after: {}", e))?;

    // Write step summary
    let step_path = run_dir
        .run_directory_path()
        .join("rf/rf_step_summary.jsonl");
    crate::artifacts::write_jsonl(&step_path, &step_results)?;

    // Write alignment
    let align_path = run_dir
        .run_directory_path()
        .join("alignment/frame_to_rf_step_alignment.jsonl");
    fs::create_dir_all(align_path.parent().unwrap())
        .map_err(|e| format!("create alignment dir: {}", e))?;
    fs::write(&align_path, alignment_entries.join("\n") + "\n")
        .map_err(|e| format!("write alignment: {}", e))?;

    // Write index.jsonl for replay compatibility
    let index_path = run_dir.run_directory_path().join("index.jsonl");
    fs::write(&index_path, index_entries.join("\n") + "\n")
        .map_err(|e| format!("write index: {}", e))?;

    // Write command audit
    let audit_path = run_dir.run_directory_path().join("command_audit.jsonl");
    crate::artifacts::write_jsonl(&audit_path, &audit)?;

    // Command audit comparison
    let comparison = crate::command_audit_compare::compare_plan_vs_audit(plan_entries, &audit);
    run_dir
        .write_json_artifact("command_plan/command_audit_comparison.json", &comparison)
        .map_err(|e| format!("write comparison: {}", e))?;

    // Run stability summary
    let parse_failure_rate = if total_captured > 0 {
        total_failed as f64 / total_captured as f64
    } else {
        0.0
    };

    let stability = RunStabilitySummary {
        schema_version: "0.2.0".into(),
        kind: "run_stability_summary".into(),
        run_id: cli.run_id.clone(),
        frames_requested: total_requested,
        frames_captured: total_captured,
        frames_parsed: total_parsed,
        frames_parse_failed: total_failed,
        parse_failure_rate,
        steps_total: resolved.total_steps,
        steps_passed: step_results.iter().filter(|s| s.step_passed).count() as u64,
        final_rf_off,
        final_mod_off,
        final_fm_off,
        final_syst_err_clean: syst_err_clean,
        emergency_shutdown_triggered: emergency,
        no_forbidden_commands_sent: comparison.forbidden_actual_commands.is_empty(),
    };
    run_dir
        .write_json_artifact("summary/run_stability_summary.json", &stability)
        .map_err(|e| format!("write stability: {}", e))?;

    let passed = !emergency
        && errors.is_empty()
        && comparison.passed
        && step_results.iter().all(|s| s.step_passed);

    Ok((
        M3_4RunResult {
            schema_version: "0.2.0".into(),
            kind: "two_device_run_result".into(),
            run_id: cli.run_id.clone(),
            mode: "real".into(),
            recipe_id: recipe.id.clone(),
            resolved_recipe_id: resolved.id.clone(),
            passed,
            steps_completed: step_results.len() as u64,
            total_steps: resolved.total_steps,
            frames_requested: total_requested,
            frames_captured: total_captured,
            frames_parsed: total_parsed,
            frames_parse_failed: total_failed,
            parse_failure_rate,
            final_rf_off,
            final_mod_off,
            final_fm_off,
            final_syst_err_clean: syst_err_clean,
            command_audit_comparison_passed: comparison.passed,
            no_forbidden_commands_sent: comparison.forbidden_actual_commands.is_empty(),
            emergency_shutdown_triggered: emergency,
            alignment_count: alignment_entries.len() as u64,
            notes: errors,
        },
        audit,
    ))
}

fn compute_stats(vectors: &[Vec<f64>]) -> (Option<f64>, Option<f64>) {
    let all: Vec<f64> = vectors.iter().flat_map(|v| v.iter().copied()).collect();
    if all.is_empty() || all.iter().any(|v| !v.is_finite()) {
        return (None, None);
    }
    let n = all.len() as f64;
    let mean = all.iter().sum::<f64>() / n;
    let var = all.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    (Some(mean), Some(var.sqrt()))
}
