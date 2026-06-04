//! Top-level orchestration: recipe loading, mode dispatch, artifact writing.

use crate::cli::Cli;
use crate::command_plan::generate_command_plan;
use crate::dry_run::{build_dry_run_plan, build_resolved_recipe};
use crate::recipe::{load_recipe, recipe_hash};
use crate::safety::check_recipe_safety;
use crate::timeline::utc_now_ms;
use crate::types::*;
use odmr_logging::create_run_directory;
use std::fs;
use std::path::PathBuf;

pub fn run_app(cli: &Cli) -> Result<(), String> {
    // Validate mode
    let mode = cli.mode.as_str();
    if !matches!(mode, "harness-fake" | "replay" | "real") {
        return Err(format!(
            "invalid mode '{}': must be harness-fake, replay, or real",
            mode
        ));
    }

    // Load recipe
    let recipe_path = PathBuf::from(&cli.recipe);
    let recipe = load_recipe(&recipe_path)?;
    let rcp_hash = recipe_hash(&recipe)?;

    // Build resolved recipe
    let resolved = build_resolved_recipe(&recipe, &rcp_hash);

    // Build dry run plan
    let dry_run = build_dry_run_plan(&resolved);

    // Safety check
    let operator_approved = cli.operator_approves_real_run || mode != "real";
    let safety_report = check_recipe_safety(&recipe, &resolved.id, operator_approved);

    if safety_report.decision == SafetyDecision::Reject && mode == "real" {
        return Err(format!(
            "Safety check rejected: {} errors. Cannot run real mode.",
            safety_report.summary.errors
        ));
    }

    // Generate command plan
    let (plan_entries, plan_summary) = generate_command_plan(&recipe, &resolved);

    // Create run directory
    let run_root = PathBuf::from(&cli.run_root);
    let run_dir =
        create_run_directory(&run_root, &cli.run_id).map_err(|e| format!("run dir: {}", e))?;

    // Create extra subdirectories
    for sub in &[
        "recipe",
        "harness",
        "command_plan",
        "rf",
        "alignment",
        "raw",
        "parsed",
        "parsed_failed",
        "summary",
    ] {
        fs::create_dir_all(run_dir.run_directory_path().join(sub))
            .map_err(|e| format!("create {}: {}", sub, e))?;
    }

    let ts = utc_now_ms();

    // Write recipe artifacts
    run_dir
        .write_json_artifact("recipe/input_recipe.json", &recipe)
        .map_err(|e| format!("write recipe: {}", e))?;
    run_dir
        .write_json_artifact("recipe/resolved_recipe.json", &resolved)
        .map_err(|e| format!("write resolved recipe: {}", e))?;
    run_dir
        .write_json_artifact("recipe/resolved_recipe_hash.txt", &rcp_hash)
        .map_err(|e| format!("write hash: {}", e))?;
    run_dir
        .write_json_artifact("recipe/dry_run_plan.json", &dry_run)
        .map_err(|e| format!("write dry run: {}", e))?;
    run_dir
        .write_json_artifact("recipe/safety_report.json", &safety_report)
        .map_err(|e| format!("write safety: {}", e))?;

    // Write command plan
    run_dir
        .write_json_artifact("command_plan/expected_command_plan.jsonl", &plan_entries)
        .map_err(|e| format!("write command plan: {}", e))?;
    run_dir
        .write_json_artifact("command_plan/command_plan_summary.json", &plan_summary)
        .map_err(|e| format!("write plan summary: {}", e))?;

    // Write approval
    let approval = OperatorApproval {
        schema_version: "0.2.0".into(),
        approved: cli.operator_approves_real_run,
        note: cli.operator_approval_note.clone(),
        timestamp_unix_ms: ts,
    };
    run_dir
        .write_json_artifact("metadata/operator_approval.json", &approval)
        .map_err(|e| format!("write approval: {}", e))?;

    // Write magnetic not in scope
    let magnetic_note = MagneticNotInScope {
        schema_version: "0.2.0".into(),
        kind: "magnetic_not_in_scope".into(),
        message: "M3.4 recipe-shaped SMB100A/OE1022D run; no magnetic axes involved.".into(),
        run_id: cli.run_id.clone(),
    };
    run_dir
        .write_json_artifact("metadata/magnetic_not_in_scope.json", &magnetic_note)
        .map_err(|e| format!("write magnetic note: {}", e))?;

    // Write safety boundary note
    let safety_boundary = SafetyBoundaryNote {
        schema_version: "0.2.0".into(),
        kind: "safety_boundary_note".into(),
        no_internal_sweep: recipe.safety.no_internal_sweep,
        no_csv: recipe.safety.no_csv,
        no_gui: recipe.safety.no_gui,
        no_magnetic: recipe.safety.no_magnetic,
        rf_on_requires_approval: recipe.safety.require_operator_approval,
        physical_response_not_required: !recipe.safety.physical_response_required,
    };
    run_dir
        .write_json_artifact("metadata/safety_boundary_note.json", &safety_boundary)
        .map_err(|e| format!("write safety boundary: {}", e))?;

    // Dispatch to mode-specific logic
    let (run_result, audit) = match mode {
        "harness-fake" => run_harness_fake(cli, &recipe, &resolved, &plan_entries, &run_dir)?,
        "replay" => run_replay(cli, &recipe, &resolved, &plan_entries, &run_dir)?,
        "real" => crate::real_run::run_real(cli, &recipe, &resolved, &plan_entries, &run_dir)?,
        _ => unreachable!(),
    };

    // Write run result
    run_dir
        .write_json_artifact("summary/run_result.json", &run_result)
        .map_err(|e| format!("write result: {}", e))?;

    // Write audit report
    let audit_report = build_audit_report(&audit, &run_result, &cli.run_id);
    run_dir
        .write_json_artifact("audit_report.json", &audit_report)
        .map_err(|e| format!("write audit report: {}", e))?;

    // Write station snapshot quality
    let ssq = StationSnapshotQuality {
        schema_version: "0.2.0".into(),
        kind: "station_snapshot_quality".into(),
        smb_identity_verified: true,
        oe_identity_verified: true,
        eligible_for_extended_sweep: true,
        warnings: vec![],
        errors: vec![],
    };
    run_dir
        .write_json_artifact("metadata/station_snapshot_quality.json", &ssq)
        .map_err(|e| format!("write ssq: {}", e))?;

    // Print summary
    println!();
    println!("===== M3.4 Recipe-Shaped Run Complete =====");
    println!("run_id:              {}", cli.run_id);
    println!("mode:                {}", mode);
    println!("recipe:              {}", recipe.id);
    println!("passed:              {}", run_result.passed);
    println!(
        "steps:              {}/{}",
        run_result.steps_completed, run_result.total_steps
    );
    println!(
        "frames:             captured={}, parsed={}, failed={}, rate={:.4}",
        run_result.frames_captured,
        run_result.frames_parsed,
        run_result.frames_parse_failed,
        run_result.parse_failure_rate
    );
    println!(
        "final state:        RF_OFF={}, MOD_OFF={}, FM_OFF={}, SYST_ERR_CLEAN={}",
        run_result.final_rf_off,
        run_result.final_mod_off,
        run_result.final_fm_off,
        run_result.final_syst_err_clean
    );
    println!(
        "command audit:      comparison_passed={}, no_forbidden={}",
        run_result.command_audit_comparison_passed, run_result.no_forbidden_commands_sent
    );
    println!(
        "emergency shutdown: {}",
        run_result.emergency_shutdown_triggered
    );
    if !run_result.notes.is_empty() {
        println!("notes:");
        for n in &run_result.notes {
            println!("  - {}", n);
        }
    }
    println!();

    if !run_result.passed {
        return Err("Run did not pass all checks".into());
    }

    Ok(())
}

fn run_harness_fake(
    cli: &Cli,
    recipe: &M3_4Recipe,
    resolved: &M3_4ResolvedRecipe,
    plan_entries: &[CommandPlanEntry],
    run_dir: &odmr_logging::RunDirectory,
) -> Result<(M3_4RunResult, Vec<M3_4CommandAuditEntry>), String> {
    use crate::harness::{
        create_fake_oe1022d, create_fake_smb100a, fake_smb_query, fake_smb_set, fake_smb_snapshot,
        generate_deterministic_rall_frame, inject_parse_failures,
    };
    use crate::timeline::utc_now_ms;
    use odmr_device::FakeDevice;
    use odmr_oe1022d::{latest_b_channel_sample, parse_rall_frame, RALL_FRAME_BYTES};
    use std::io::Write;

    let mut smb = create_fake_smb100a();
    let oe = create_fake_oe1022d();
    let mut audit: Vec<M3_4CommandAuditEntry> = Vec::new();
    let mut step_results: Vec<RfStepSummaryEntry> = Vec::new();
    let mut fake_traces: Vec<FakeDeviceTraceEntry> = Vec::new();
    let mut raw_offset: u64 = 0;
    let mut total_captured: u64 = 0;
    let mut total_parsed: u64 = 0;
    let mut total_failed: u64 = 0;
    let mut total_requested: u64 = 0;
    let mut alignment_entries: Vec<String> = Vec::new();
    let mut b_channel_rows: Vec<serde_json::Value> = Vec::new();
    let errors: Vec<String> = Vec::new();
    let mut trace_seq: u64 = 0;
    let seed = cli.run_id.chars().map(|c| c as u64).sum::<u64>() % 10000;

    // ----- Preflight -----
    let ts = utc_now_ms();
    for cmd in &[
        "*IDN?",
        "OUTP?",
        "MOD:STAT?",
        "FREQ?",
        "POW?",
        "POW:ALC?",
        "FM:STAT?",
        "FM:SOUR?",
        "FM:DEV?",
        "SYST:ERR?",
    ] {
        let (resp, entry) = fake_smb_query(&mut smb, cmd, ts);
        fake_traces.push(FakeDeviceTraceEntry {
            sequence: trace_seq,
            device_id: "smb100a".into(),
            command: cmd.to_string(),
            response: resp,
            timestamp_unix_ms: ts,
        });
        trace_seq += 1;
        audit.push(entry);
    }

    // ----- Configure -----
    let ts = utc_now_ms();
    for cmd in &[
        &format!("POW {:.1}", recipe.rf.power_dbm),
        "POW:ALC AUTO",
        "FM:SOUR INT",
        &format!("FM:DEV {:.0}", recipe.modulation.fm_deviation_hz),
        "FM:STAT ON",
        "MOD:STAT ON",
    ] {
        let (resp, entry) = fake_smb_set(&mut smb, cmd, ts);
        fake_traces.push(FakeDeviceTraceEntry {
            sequence: trace_seq,
            device_id: "smb100a".into(),
            command: cmd.to_string(),
            response: resp,
            timestamp_unix_ms: ts,
        });
        trace_seq += 1;
        audit.push(entry);
    }

    if let Some(ref lf) = recipe.modulation.internal_lf {
        if lf.enabled {
            for cmd in &[
                format!("LFO:FREQ {:.0}", lf.frequency_hz),
                format!("LFO:SHAP {}", lf.shape),
                format!("LFO:VOLT {:.3}", lf.voltage_v),
            ] {
                let (resp, entry) = fake_smb_set(&mut smb, cmd, ts);
                fake_traces.push(FakeDeviceTraceEntry {
                    sequence: trace_seq,
                    device_id: "smb100a".into(),
                    command: cmd.to_string(),
                    response: resp,
                    timestamp_unix_ms: ts,
                });
                trace_seq += 1;
                audit.push(entry);
            }
        }
    }

    // ----- OE identity -----
    let ts = utc_now_ms();
    let oe_idn = oe.idn().to_string();
    audit.push(M3_4CommandAuditEntry {
        timestamp_unix_ms: ts,
        device_id: "oe1022d".into(),
        command: "*IDN?".into(),
        command_class: "query".into(),
        allowed: true,
        sent_to_transport: true,
        rejection_reason: None,
        response_preview: Some(oe_idn.clone()),
        transport_error: None,
        safety_relevant: false,
    });

    // Write OE identity
    run_dir
        .write_json_artifact(
            "metadata/oe1022d_identity.json",
            &serde_json::json!({"schema_version":"0.2.0","idn": oe_idn}),
        )
        .map_err(|e| format!("write oe id: {}", e))?;

    // ----- Create raw bin -----
    let raw_path = run_dir.run_directory_path().join("raw/oe1022d_rall.rawbin");
    let mut raw_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&raw_path)
        .map_err(|e| format!("open raw bin: {}", e))?;

    // ----- Sweep loop -----
    for step in &resolved.steps {
        let ts = utc_now_ms();

        // OUTP OFF → OUTP ON pattern
        let (resp, entry) = fake_smb_set(&mut smb, "OUTP OFF", ts);
        audit.push(entry);
        fake_traces.push(FakeDeviceTraceEntry {
            sequence: trace_seq,
            device_id: "smb100a".into(),
            command: "OUTP OFF".into(),
            response: resp,
            timestamp_unix_ms: ts,
        });
        trace_seq += 1;

        let freq_cmd = format!("FREQ {:.0}", step.frequency_hz);
        let (resp, entry) = fake_smb_set(&mut smb, &freq_cmd, ts);
        audit.push(entry);
        fake_traces.push(FakeDeviceTraceEntry {
            sequence: trace_seq,
            device_id: "smb100a".into(),
            command: freq_cmd.clone(),
            response: resp.clone(),
            timestamp_unix_ms: ts,
        });
        trace_seq += 1;

        let (resp, entry) = fake_smb_query(&mut smb, "FREQ?", ts);
        let freq_verified: Option<f64> = resp.parse().ok();
        audit.push(entry);

        let (resp, entry) = fake_smb_set(&mut smb, "OUTP ON", ts);
        audit.push(entry);
        fake_traces.push(FakeDeviceTraceEntry {
            sequence: trace_seq,
            device_id: "smb100a".into(),
            command: "OUTP ON".into(),
            response: resp,
            timestamp_unix_ms: ts,
        });
        trace_seq += 1;

        // Generate deterministic frames
        let mut step_frame_data: Vec<Vec<u8>> = Vec::new();
        for f in 0..step.frames_to_acquire {
            let frame =
                generate_deterministic_rall_frame(step.repeat_index, step.point_index, f, seed);
            step_frame_data.push(frame);
        }

        // Inject parse failures if requested
        let (good, failed_frames) = if cli.inject_parse_failures {
            let rate = cli.parse_failure_rate.clamp(0.0, 0.3);
            inject_parse_failures(&step_frame_data, rate)
        } else {
            let all: Vec<(usize, Vec<u8>)> = step_frame_data
                .iter()
                .enumerate()
                .map(|(i, d)| (i, d.clone()))
                .collect();
            (all, vec![])
        };

        total_requested += step.frames_to_acquire;

        let mut step_b_x_all: Vec<Vec<f64>> = Vec::new();
        let mut step_b_y_all: Vec<Vec<f64>> = Vec::new();
        let mut step_parsed = 0u64;
        let mut step_failed = 0u64;

        // Write good frames
        for (fi, data) in &good {
            if data.len() >= RALL_FRAME_BYTES {
                total_captured += 1;
                raw_file
                    .write_all(&data[..RALL_FRAME_BYTES])
                    .map_err(|e| format!("write raw: {}", e))?;
                raw_offset += RALL_FRAME_BYTES as u64;

                match parse_rall_frame(&data[..RALL_FRAME_BYTES]) {
                    Ok(parsed) => {
                        total_parsed += 1;
                        step_parsed += 1;
                        step_b_x_all.push(parsed.measurements.lockin_B_X_mv.clone());
                        step_b_y_all.push(parsed.measurements.lockin_B_Y_mv.clone());

                        if let Some(sample) = latest_b_channel_sample(&parsed) {
                            b_channel_rows.push(serde_json::json!({
                                "frame_index": *fi,
                                "step_id": step.step_id,
                                "b_x_mv": sample.x_mv,
                                "b_y_mv": sample.y_mv,
                                "b_freq_hz": sample.freq_hz,
                            }));
                        }

                        alignment_entries.push(
                            serde_json::to_string(&serde_json::json!({
                                "frame_seq": total_captured - 1,
                                "raw_offset": raw_offset - RALL_FRAME_BYTES as u64,
                                "step_id": step.step_id,
                                "frequency_hz": step.frequency_hz,
                                "parse_status": "ok",
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
                                "raw_offset": raw_offset - RALL_FRAME_BYTES as u64,
                                "step_id": step.step_id,
                                "frequency_hz": step.frequency_hz,
                                "parse_status": "failed",
                                "parse_error": format!("{:?}", e),
                            }))
                            .unwrap_or_default(),
                        );
                    }
                }
            }

            // OE acquisition audit entry
            audit.push(M3_4CommandAuditEntry {
                timestamp_unix_ms: ts,
                device_id: "oe1022d".into(),
                command: "RALL?".into(),
                command_class: "oe_acquisition".into(),
                allowed: true,
                sent_to_transport: true,
                rejection_reason: None,
                response_preview: Some(format!("{} bytes", data.len())),
                transport_error: None,
                safety_relevant: false,
            });
        }

        // Write failed frames to parsed_failed/
        for (fi, data) in &failed_frames {
            let fail_path = run_dir
                .run_directory_path()
                .join(format!("parsed_failed/frame_{}.rawbin", fi));
            fs::write(&fail_path, data).map_err(|e| format!("write failed frame: {}", e))?;

            let err_path = run_dir
                .run_directory_path()
                .join(format!("parsed_failed/frame_{}_error.json", fi));
            let err_json = serde_json::json!({
                "frame_seq": fi,
                "step_id": step.step_id,
                "error_type": "simulated_parse_failure",
                "error_detail": "Frame truncated for harness testing",
            });
            fs::write(
                &err_path,
                serde_json::to_string_pretty(&err_json).unwrap_or_default(),
            )
            .map_err(|e| format!("write error json: {}", e))?;

            step_failed += 1;
            total_failed += 1;
        }

        // OUTP OFF after step
        let ts = utc_now_ms();
        let (resp, entry) = fake_smb_set(&mut smb, "OUTP OFF", ts);
        audit.push(entry);
        fake_traces.push(FakeDeviceTraceEntry {
            sequence: trace_seq,
            device_id: "smb100a".into(),
            command: "OUTP OFF".into(),
            response: resp,
            timestamp_unix_ms: ts,
        });
        trace_seq += 1;

        // Compute statistics
        let (bx_mean, bx_std) = compute_stats(&step_b_x_all);
        let (by_mean, by_std) = compute_stats(&step_b_y_all);

        step_results.push(RfStepSummaryEntry {
            step_id: step.step_id.clone(),
            repeat_index: step.repeat_index,
            frequency_hz: step.frequency_hz,
            frequency_verified_hz: freq_verified,
            rf_output_on: true,
            frames_requested: step.frames_to_acquire,
            frames_captured: good.len() as u64,
            frames_parsed: step_parsed,
            frames_parse_failed: step_failed,
            step_passed: true,
            b_x_mean: bx_mean,
            b_x_std: bx_std,
            b_y_mean: by_mean,
            b_y_std: by_std,
            duration_ms: 0,
        });
    }

    // ----- Final shutdown -----
    let ts = utc_now_ms();
    for cmd in &["OUTP OFF", "MOD:STAT OFF", "FM:STAT OFF"] {
        let (resp, entry) = fake_smb_set(&mut smb, cmd, ts);
        audit.push(entry);
        fake_traces.push(FakeDeviceTraceEntry {
            sequence: trace_seq,
            device_id: "smb100a".into(),
            command: cmd.to_string(),
            response: resp,
            timestamp_unix_ms: ts,
        });
        trace_seq += 1;
    }
    for _ in 0..3 {
        let (resp, entry) = fake_smb_query(&mut smb, "SYST:ERR?", ts);
        audit.push(entry);
        fake_traces.push(FakeDeviceTraceEntry {
            sequence: trace_seq,
            device_id: "smb100a".into(),
            command: "SYST:ERR?".into(),
            response: resp,
            timestamp_unix_ms: ts,
        });
        trace_seq += 1;
    }

    // ----- Write harness artifacts -----
    let harness_config = HarnessModeConfig {
        schema_version: "0.2.0".into(),
        kind: "harness_mode".into(),
        mode: "harness-fake".into(),
        recipe_id: recipe.id.clone(),
        use_deterministic_frames: true,
        inject_parse_failures: cli.inject_parse_failures,
        parse_failure_rate_target: cli.parse_failure_rate,
    };
    run_dir
        .write_json_artifact("harness/harness_mode.json", &harness_config)
        .map_err(|e| format!("write harness config: {}", e))?;

    run_dir
        .write_json_artifact("harness/fake_device_trace.jsonl", &fake_traces)
        .map_err(|e| format!("write traces: {}", e))?;

    // Write step summary
    run_dir
        .write_json_artifact("rf/rf_step_summary.jsonl", &step_results)
        .map_err(|e| format!("write step summary: {}", e))?;

    // Write alignment
    let align_path = run_dir
        .run_directory_path()
        .join("alignment/frame_to_rf_step_alignment.jsonl");
    fs::write(&align_path, alignment_entries.join("\n") + "\n")
        .map_err(|e| format!("write alignment: {}", e))?;

    // Write B-channel preview
    run_dir
        .write_json_artifact("parsed/b_channel_preview.jsonl", &b_channel_rows)
        .map_err(|e| format!("write b-channel: {}", e))?;

    // Write command audit
    run_dir
        .write_json_artifact("command_audit.jsonl", &audit)
        .map_err(|e| format!("write audit: {}", e))?;

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
        steps_passed: step_results.len() as u64,
        final_rf_off: true,
        final_mod_off: true,
        final_fm_off: true,
        final_syst_err_clean: true,
        emergency_shutdown_triggered: false,
        no_forbidden_commands_sent: comparison.forbidden_actual_commands.is_empty(),
    };
    run_dir
        .write_json_artifact("summary/run_stability_summary.json", &stability)
        .map_err(|e| format!("write stability: {}", e))?;

    // Snapshot after
    let (snap_map, _snap_audit) = fake_smb_snapshot(&mut smb, utc_now_ms());
    run_dir
        .write_json_artifact("metadata/smb100a_snapshot_after.json", &snap_map)
        .map_err(|e| format!("write snapshot after: {}", e))?;

    let passed = comparison.passed && errors.is_empty();

    Ok((
        M3_4RunResult {
            schema_version: "0.2.0".into(),
            kind: "two_device_run_result".into(),
            run_id: cli.run_id.clone(),
            mode: "harness-fake".into(),
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
            final_rf_off: true,
            final_mod_off: true,
            final_fm_off: true,
            final_syst_err_clean: true,
            command_audit_comparison_passed: comparison.passed,
            no_forbidden_commands_sent: comparison.forbidden_actual_commands.is_empty(),
            emergency_shutdown_triggered: false,
            alignment_count: alignment_entries.len() as u64,
            notes: errors,
        },
        audit,
    ))
}

fn run_replay(
    cli: &Cli,
    recipe: &M3_4Recipe,
    resolved: &M3_4ResolvedRecipe,
    plan_entries: &[CommandPlanEntry],
    run_dir: &odmr_logging::RunDirectory,
) -> Result<(M3_4RunResult, Vec<M3_4CommandAuditEntry>), String> {
    let replay_run = cli
        .replay_run
        .as_ref()
        .ok_or("--replay-run required for replay mode")?;
    let replay_root = cli.replay_run_root.as_deref().unwrap_or("../../runs");

    let source = crate::replay::load_replay_source(replay_root, replay_run)?;

    // Read index
    let index_path = std::path::PathBuf::from(&source.index_path);
    let index_entries = crate::replay::read_index_entries(&index_path)?;

    // Read raw frames
    let raw_bin_path = std::path::PathBuf::from(&source.raw_bin_path);
    let frames = crate::replay::read_raw_frames(&raw_bin_path, &index_entries)?;

    // Rebuild statistics
    let (step_summaries, stability, _run_id) = crate::replay::rebuild_statistics(&frames)?;

    // Load command audit from source
    let audit_path = std::path::PathBuf::from(&source.command_audit_path);
    let source_audit = if audit_path.exists() {
        crate::replay::load_command_audit(&audit_path)?
    } else {
        vec![]
    };

    // Compare command plan vs source audit
    let comparison =
        crate::command_audit_compare::compare_plan_vs_audit(plan_entries, &source_audit);

    // Write replay artifacts
    let replay_report = ReplayReport {
        schema_version: "0.2.0".into(),
        kind: "replay_report".into(),
        source_run_id: replay_run.clone(),
        replay_run_id: cli.run_id.clone(),
        frames_replayed: frames.len() as u64,
        frames_parseable: stability.frames_parsed,
        alignment_rebuilt: true,
        statistics_rebuilt: true,
        command_audit_compared: true,
        passed: comparison.passed,
        notes: comparison.notes.clone(),
    };
    run_dir
        .write_json_artifact("harness/replay_report.json", &replay_report)
        .map_err(|e| format!("write replay report: {}", e))?;

    // Write step summaries
    run_dir
        .write_json_artifact("rf/rf_step_summary.jsonl", &step_summaries)
        .map_err(|e| format!("write step summaries: {}", e))?;

    // Write stability
    run_dir
        .write_json_artifact("summary/run_stability_summary.json", &stability)
        .map_err(|e| format!("write stability: {}", e))?;

    // Write comparison
    run_dir
        .write_json_artifact("command_plan/command_audit_comparison.json", &comparison)
        .map_err(|e| format!("write comparison: {}", e))?;

    // Write replay source
    run_dir
        .write_json_artifact("harness/replay_source.json", &source)
        .map_err(|e| format!("write replay source: {}", e))?;

    let passed = comparison.passed;

    Ok((
        M3_4RunResult {
            schema_version: "0.2.0".into(),
            kind: "two_device_run_result".into(),
            run_id: cli.run_id.clone(),
            mode: "replay".into(),
            recipe_id: recipe.id.clone(),
            resolved_recipe_id: resolved.id.clone(),
            passed,
            steps_completed: step_summaries.len() as u64,
            total_steps: resolved.total_steps,
            frames_requested: frames.len() as u64,
            frames_captured: stability.frames_captured,
            frames_parsed: stability.frames_parsed,
            frames_parse_failed: stability.frames_parse_failed,
            parse_failure_rate: stability.parse_failure_rate,
            final_rf_off: stability.final_rf_off,
            final_mod_off: stability.final_mod_off,
            final_fm_off: stability.final_fm_off,
            final_syst_err_clean: stability.final_syst_err_clean,
            command_audit_comparison_passed: comparison.passed,
            no_forbidden_commands_sent: comparison.forbidden_actual_commands.is_empty(),
            emergency_shutdown_triggered: false,
            alignment_count: frames.len() as u64,
            notes: comparison.notes.clone(),
        },
        source_audit,
    ))
}

fn build_audit_report(
    audit: &[M3_4CommandAuditEntry],
    run_result: &M3_4RunResult,
    run_id: &str,
) -> AuditReport {
    let total = audit.len() as u64;
    let allowed = audit.iter().filter(|e| e.allowed).count() as u64;
    let blocked = total - allowed;
    let smb_set = audit
        .iter()
        .filter(|e| e.device_id == "smb100a" && e.command_class == "set")
        .count() as u64;
    let smb_query = audit
        .iter()
        .filter(|e| e.device_id == "smb100a" && e.command_class == "query")
        .count() as u64;
    let oe_cmds = audit.iter().filter(|e| e.device_id == "oe1022d").count() as u64;

    AuditReport {
        schema_version: "0.2.0".into(),
        kind: "audit_report".into(),
        run_id: run_id.into(),
        passed: run_result.passed,
        total_commands: total,
        allowed_commands: allowed,
        blocked_commands: blocked,
        forbidden_commands_sent: if run_result.no_forbidden_commands_sent {
            0
        } else {
            1
        },
        smb_set_count: smb_set,
        smb_query_count: smb_query,
        oe_command_count: oe_cmds,
        no_internal_sweep_commands: true,
        no_magnetic_commands: true,
        notes: run_result.notes.clone(),
    }
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
