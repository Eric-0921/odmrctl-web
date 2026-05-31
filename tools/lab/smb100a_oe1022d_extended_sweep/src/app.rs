//! M3.3 orchestration: repeat sweep, per-step statistics, parse failure quarantine.

use crate::alignment::build_alignment_summary;
use crate::artifacts::{sha256_file, write_jsonl};
use crate::cli::Cli;
use crate::oe_acquisition::{acquire_frames, build_alignment_for_step};
use crate::oe_transport::OeSerialTransport;
use crate::smb_sequence::{
    compute_step_plan, configure_smb_common, execute_step_rf_off, execute_step_rf_on,
    run_final_shutdown, run_preflight, validate_safety_limits,
};
use crate::smb_transport::SmbTransport;
use crate::statistics::aggregate_b_channel_stats;
use crate::timeline::{make_event, utc_now_ms, TimelineTracker};
use crate::types::*;
use odmr_logging::{create_run_directory, EventLevel, RunArtifactPaths, RunEventType, RunManifest};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

pub fn run_app(cli: &Cli) -> Result<(), String> {
    // 0. Validate safety limits
    validate_safety_limits(cli)?;

    let step_plan = compute_step_plan(cli);

    // 1. Create run directory
    let run_root = PathBuf::from(&cli.run_root);
    let run_dir =
        create_run_directory(&run_root, &cli.run_id).map_err(|e| format!("run dir: {}", e))?;

    // Create extra subdirs
    for sub in &["rf", "alignment", "parsed", "parsed_failed"] {
        fs::create_dir_all(run_dir.run_directory_path().join(sub))
            .map_err(|e| format!("create {}: {}", sub, e))?;
    }

    let created_at = utc_now_ms();
    let mut tracker = TimelineTracker::new();
    let mut audit: Vec<CommandAuditEntry> = Vec::new();
    let mut forbidden_attempted: Vec<String> = Vec::new();
    let warnings: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let delay_ms = cli.smb_query_delay_ms;
    let approval = cli.operator_approves_extended_sweep;

    // ------ Write early metadata ------

    let config = SweepConfig {
        schema_version: "0.2.0".into(),
        smb_host: cli.smb_host.clone(),
        smb_port: cli.smb_port,
        smb_query_delay_ms: cli.smb_query_delay_ms,
        smb_timeout_ms: cli.smb_timeout_ms,
        oe_port: cli.oe_port.clone(),
        oe_baud: cli.oe_baud,
        oe_timeout_ms: cli.oe_timeout_ms,
        rf_start_hz: cli.rf_start_hz,
        rf_stop_hz: cli.rf_stop_hz,
        rf_points: cli.rf_points,
        rf_power_dbm: cli.rf_power_dbm,
        max_rf_power_dbm: cli.max_rf_power_dbm,
        fm_deviation_hz: cli.fm_deviation_hz,
        max_fm_deviation_hz: cli.max_fm_deviation_hz,
        repeat_count: cli.repeat_count,
        set_internal_lf: cli.set_internal_lf,
        lf_frequency_hz: if cli.set_internal_lf {
            Some(cli.lf_frequency_hz)
        } else {
            None
        },
        lf_shape: if cli.set_internal_lf {
            Some(cli.lf_shape.clone())
        } else {
            None
        },
        lf_voltage_v: if cli.set_internal_lf {
            Some(cli.lf_voltage_v)
        } else {
            None
        },
        frames_per_step: cli.frames_per_step,
        inter_frame_delay_ms: cli.inter_frame_delay_ms,
        oe_frame_delay_ms: cli.oe_frame_delay_ms,
        created_at_unix_ms: created_at,
    };

    run_dir
        .write_json_artifact("metadata/extended_sweep_config.json", &config)
        .map_err(|e| format!("write config: {}", e))?;

    let operator_approval = OperatorApproval {
        schema_version: "0.2.0".into(),
        approved: cli.operator_approves_extended_sweep,
        note: cli.operator_approval_note.clone(),
        timestamp_unix_ms: created_at,
    };
    run_dir
        .write_json_artifact("metadata/operator_approval.json", &operator_approval)
        .map_err(|e| format!("write approval: {}", e))?;

    let mag_not_in_scope = MagneticNotInScope {
        magnetic_devices_in_scope: false,
        magnetic_serial_enumeration_performed: false,
        magnetic_commands_sent: 0,
        reason: "M3.3 is SMB100A + OE1022D only; magnetic axes are not part of this run".into(),
        known_verified_axis_sns: MagneticAxisSns {
            x: "080020960220402020".into(),
            y: "080020960220402022".into(),
            z: "080020960220402003".into(),
        },
        note: "SN mapping preserved".into(),
    };
    run_dir
        .write_json_artifact("metadata/magnetic_not_in_scope.json", &mag_not_in_scope)
        .map_err(|e| format!("write magnetic: {}", e))?;

    // Write step plan
    run_dir
        .write_json_artifact("rf/step_plan.json", &step_plan)
        .map_err(|e| format!("write step plan: {}", e))?;

    // Manifest
    let manifest = RunManifest {
        schema_version: "0.2.0".into(),
        kind: "run_manifest".into(),
        run_id: cli.run_id.clone(),
        created_at_unix_ms: created_at,
        artifact_paths: RunArtifactPaths {
            manifest: "manifest.json".into(),
            station_snapshot: "metadata/station_snapshot.json".into(),
            recipe_lock: "metadata/recipe.lock.json".into(),
            resolved_recipe_lock: "metadata/resolved_recipe.lock.json".into(),
            dry_run_plan_lock: "metadata/dry_run_plan.lock.json".into(),
            safety_report_lock: "metadata/safety_report.lock.json".into(),
            events: "events.jsonl".into(),
            index: "index.jsonl".into(),
            raw_bin: "raw/oe1022d_rall.rawbin".into(),
        },
        recipe_hash: None,
        resolved_recipe_id: None,
        safety_report_id: None,
    };
    run_dir
        .write_manifest(&manifest)
        .map_err(|e| format!("write manifest: {}", e))?;

    // ------ Phase 1-3: SMB connect, preflight, configure ------

    let mut smb_transport = SmbTransport::connect(&cli.smb_host, cli.smb_port, cli.smb_timeout_ms)?;
    tracker.record("smb_connected", "smb100a", None);

    let preflight_result = run_preflight(
        &mut smb_transport,
        &mut audit,
        &mut forbidden_attempted,
        delay_ms,
        cli,
    )?;
    let snapshot_before = preflight_result.snapshot_before.clone();

    run_dir
        .write_json_artifact("metadata/smb100a_snapshot_before.json", &snapshot_before)
        .map_err(|e| format!("write snapshot before: {}", e))?;

    if !preflight_result.preflight.passed {
        run_dir
            .write_json_artifact(
                "microtest/preflight_check.json",
                &preflight_result.preflight,
            )
            .map_err(|e| format!("write preflight: {}", e))?;
        smb_transport.close();
        tracker.record("smb_disconnected_preflight_failed", "smb100a", None);

        write_final_artifacts_safe(
            &run_dir,
            cli,
            &audit,
            &tracker,
            &snapshot_before,
            None,
            &[],
            &warnings,
            &preflight_result.preflight.errors,
            created_at,
        )?;

        let station_quality = StationSnapshotQuality {
            schema_version: "0.2.0".into(),
            status: "failed".into(),
            eligible_for_extended_sweep: false,
            warnings: warnings.clone(),
            errors: preflight_result.preflight.errors.clone(),
            query_interrupted_seen: preflight_result
                .syst_err_before
                .iter()
                .any(|o| o.response.contains("-410")),
            smb_query_delay_ms: cli.smb_query_delay_ms,
        };
        run_dir
            .write_json_artifact("metadata/station_snapshot_quality.json", &station_quality)
            .map_err(|e| format!("write quality: {}", e))?;

        eprintln!("Preflight failed: {:?}", preflight_result.preflight.errors);
        return Err("Preflight check failed".into());
    }

    // Configure SMB common params
    configure_smb_common(
        &mut smb_transport,
        &mut audit,
        &mut forbidden_attempted,
        delay_ms,
        cli,
        &mut tracker,
    )?;

    // ------ Phase 2: OE identity ------

    let mut oe_transport =
        OeSerialTransport::connect(&cli.oe_port, cli.oe_baud, cli.oe_timeout_ms)?;
    tracker.record("oe_connected", "oe1022d", None);

    let oe_idn = oe_transport.query_identity(&mut audit, &mut forbidden_attempted)?;
    let oe_identity_json = serde_json::json!({
        "schema_version": "0.2.0",
        "device_id": "oe1022d_main",
        "idn": oe_idn,
        "queried_at_unix_ms": utc_now_ms(),
    });
    run_dir
        .write_json_artifact("metadata/oe1022d_identity.json", &oe_identity_json)
        .map_err(|e| format!("write oe identity: {}", e))?;

    // ------ Phase 4: repeat sweep ------

    let run_start_ns_for_oe = 0u64;

    let mut step_results: Vec<RfStepResult> = Vec::new();
    let mut all_alignment_entries: Vec<FrameToStepAlignment> = Vec::new();
    let mut all_index_entries: Vec<odmr_logging::RawIndexEntry> = Vec::new();
    let mut quarantine_entries: Vec<ParseQuarantineEntry> = Vec::new();
    let mut total_frames_captured = 0usize;
    let mut total_frames_parsed = 0usize;
    let mut total_frames_requested = 0usize;
    let mut total_frames_parse_failed = 0usize;
    let mut emergency_shutdown_triggered = false;
    let mut emergency_shutdown_evidence: Option<EmergencyShutdownEvidence> = None;

    let total_steps = (cli.rf_points * cli.repeat_count) as usize;
    let mut global_frame_seq: u64 = 0;

    for repeat in 0..cli.repeat_count {
        tracker.record(
            "repeat_started",
            "smb100a",
            Some(serde_json::json!({"repeat_index": repeat})),
        );

        for (step_i, step_freq) in step_plan.frequencies_hz.iter().enumerate() {
            let step_id = format!("repeat_{}_rf_step_{:03}", repeat, step_i);
            let step_start = Instant::now();

            match execute_step_rf_on(
                &mut smb_transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                *step_freq,
                approval,
                &mut tracker,
            ) {
                Ok(freq_verified) => {
                    tracker.record(
                        "rf_step_started",
                        "smb100a",
                        Some(serde_json::json!({
                            "step_id": step_id,
                            "repeat_index": repeat,
                            "frequency_hz": freq_verified
                        })),
                    );

                    let frames_requested = cli.frames_per_step;
                    total_frames_requested += frames_requested as usize;

                    let mut acq_result = match acquire_frames(
                        &mut oe_transport,
                        &mut audit,
                        &mut forbidden_attempted,
                        frames_requested,
                        cli.inter_frame_delay_ms,
                        cli.oe_frame_delay_ms,
                        &cli.run_id,
                        &step_id,
                        step_i as u64,
                        *step_freq,
                        true,
                        true,
                        true,
                        run_start_ns_for_oe,
                    ) {
                        Ok(r) => r,
                        Err(e) => {
                            errors.push(format!("Step {} OE acq error: {}", step_id, e));
                            break;
                        }
                    };

                    total_frames_captured += acq_result.total_frames_captured;
                    total_frames_parsed += acq_result.total_frames_parsed;
                    total_frames_parse_failed += acq_result.total_frames_parse_failed;

                    // Write raw frames
                    {
                        let mut raw_writer = run_dir
                            .open_raw_bin_writer_at("raw/oe1022d_rall.rawbin")
                            .map_err(|e| format!("open raw writer: {}", e))?;
                        for frame in &acq_result.frames {
                            raw_writer
                                .append_frame(&frame.raw_bytes)
                                .map_err(|e| format!("append raw: {}", e))?;
                        }
                    }

                    // Handle parse failures — quarantine raw bytes + error JSON
                    for frame in &acq_result.frames {
                        if let Some(ref parse_error) = frame.parse_error {
                            let rawbin_path = run_dir.run_directory_path().join(format!(
                                "parsed_failed/frame_{}.rawbin",
                                global_frame_seq
                            ));
                            fs::write(&rawbin_path, &frame.raw_bytes)
                                .map_err(|e| format!("write quarantine rawbin: {}", e))?;

                            let q_entry = ParseQuarantineEntry {
                                frame_seq: global_frame_seq,
                                step_id: step_id.clone(),
                                raw_nbytes: frame.frame_len,
                                error_type: "RallParseError".into(),
                                error_detail: parse_error.clone(),
                            };
                            let q_json = serde_json::to_string_pretty(&q_entry)
                                .map_err(|e| format!("serialize quarantine: {}", e))?;
                            let q_path = rawbin_path.with_extension("json");
                            fs::write(&q_path, &q_json)
                                .map_err(|e| format!("write quarantine json: {}", e))?;

                            quarantine_entries.push(q_entry);
                        }
                        global_frame_seq += 1;
                    }

                    // Update index entries with correct global frame seq
                    for (idx_offset, idx_entry) in acq_result.index_entries.iter_mut().enumerate() {
                        idx_entry.offset_bytes =
                            idx_offset as u64 * odmr_oe1022d::RALL_FRAME_BYTES as u64;
                    }

                    all_index_entries.extend(acq_result.index_entries);

                    // Compute B-channel statistics for this step
                    let b_x_vectors: Vec<Vec<f64>> = acq_result
                        .frames
                        .iter()
                        .filter(|f| f.parsed_ok && !f.b_x_all.is_empty())
                        .map(|f| f.b_x_all.clone())
                        .collect();
                    let b_y_vectors: Vec<Vec<f64>> = acq_result
                        .frames
                        .iter()
                        .filter(|f| f.parsed_ok && !f.b_y_all.is_empty())
                        .map(|f| f.b_y_all.clone())
                        .collect();

                    let statistics = if !b_x_vectors.is_empty() && !b_y_vectors.is_empty() {
                        match aggregate_b_channel_stats(&b_x_vectors, &b_y_vectors) {
                            Some((x_stats, y_stats)) => Some(StepStatistics {
                                repeat_index: repeat,
                                step_id: step_id.clone(),
                                frequency_hz: *step_freq,
                                b_x_mean_mv: x_stats.mean,
                                b_x_std_mv: x_stats.std,
                                b_x_min_mv: x_stats.min,
                                b_x_max_mv: x_stats.max,
                                b_y_mean_mv: y_stats.mean,
                                b_y_std_mv: y_stats.std,
                                b_y_min_mv: y_stats.min,
                                b_y_max_mv: y_stats.max,
                                frames_used: b_x_vectors.len(),
                                frames_parse_failed: acq_result.total_frames_parse_failed,
                            }),
                            None => None,
                        }
                    } else {
                        None
                    };

                    // Build alignment
                    let alignment = build_alignment_for_step(
                        &acq_result.frames,
                        &step_id,
                        step_i as u64,
                        repeat,
                        *step_freq,
                        true,
                        true,
                        true,
                        run_start_ns_for_oe,
                    );
                    all_alignment_entries.extend(alignment);

                    // Write parsed B-channel preview
                    {
                        let preview_rows: Vec<serde_json::Value> = acq_result
                            .frames
                            .iter()
                            .map(|f| {
                                serde_json::json!({
                                    "schema_version": "0.2.0",
                                    "step_id": step_id,
                                    "repeat_index": repeat,
                                    "frame_len": f.frame_len,
                                    "is_full_frame": f.is_full_frame,
                                    "parsed_ok": f.parsed_ok,
                                    "b_x_latest": f.b_x_latest,
                                    "b_y_latest": f.b_y_latest,
                                    "b_freq_latest": f.b_freq_latest,
                                    "b_x_sample_count": f.b_x_all.len(),
                                    "b_y_sample_count": f.b_y_all.len(),
                                    "parse_error": f.parse_error,
                                    "elapsed_ms": f.elapsed_ms,
                                })
                            })
                            .collect();
                        write_jsonl(
                            &run_dir
                                .run_directory_path()
                                .join("parsed/b_channel_preview.jsonl"),
                            &preview_rows,
                        )
                        .map_err(|e| format!("write preview: {}", e))?;

                        let summary_rows: Vec<serde_json::Value> = acq_result
                            .frames
                            .iter()
                            .enumerate()
                            .map(|(fi, f)| {
                                serde_json::json!({
                                    "schema_version": "0.2.0",
                                    "step_id": step_id,
                                    "repeat_index": repeat,
                                    "frame_index_in_step": fi,
                                    "frame_len": f.frame_len,
                                    "parsed_ok": f.parsed_ok,
                                })
                            })
                            .collect();
                        write_jsonl(
                            &run_dir
                                .run_directory_path()
                                .join("parsed/frame_summary.jsonl"),
                            &summary_rows,
                        )
                        .map_err(|e| format!("write frame summary: {}", e))?;
                    }

                    // OUTP OFF after acquisition
                    let rf_off_ok = match execute_step_rf_off(
                        &mut smb_transport,
                        &mut audit,
                        &mut forbidden_attempted,
                        delay_ms,
                        &mut tracker,
                    ) {
                        Ok(ok) => ok,
                        Err(e) => {
                            errors.push(format!("{} RF OFF error: {}", step_id, e));
                            emergency_shutdown_triggered = true;
                            false
                        }
                    };

                    let duration_ms = step_start.elapsed().as_millis() as u64;
                    tracker.record(
                        "rf_step_completed",
                        "smb100a",
                        Some(serde_json::json!({
                            "step_id": step_id,
                            "repeat_index": repeat,
                        })),
                    );

                    let step_result = RfStepResult {
                        schema_version: "0.2.0".into(),
                        step_id,
                        step_index: step_i as u64,
                        repeat_index: repeat,
                        frequency_hz_requested: *step_freq,
                        frequency_hz_verified: freq_verified,
                        frequency_set_ok: true,
                        rf_on_sent: true,
                        rf_off_sent: true,
                        rf_on_confirmed: true,
                        rf_off_confirmed_after_step: rf_off_ok,
                        frames_requested: cli.frames_per_step,
                        frames_captured: acq_result.total_frames_captured,
                        frames_parsed: acq_result.total_frames_parsed,
                        frames_failed: acq_result.total_frames_attempted
                            - acq_result.total_frames_captured,
                        frames_parse_failed: acq_result.total_frames_parse_failed,
                        step_passed: rf_off_ok && acq_result.total_frames_captured > 0,
                        duration_ms,
                        statistics: statistics.clone(),
                        warnings: if statistics.is_none() && acq_result.total_frames_parsed > 0 {
                            vec!["B-channel statistics unavailable".into()]
                        } else {
                            Vec::new()
                        },
                        errors: if rf_off_ok {
                            Vec::new()
                        } else {
                            vec!["RF OFF not confirmed after step".into()]
                        },
                    };
                    step_results.push(step_result);
                }
                Err(e) => {
                    errors.push(format!("{} RF ON error: {}", step_id, e));
                    emergency_shutdown_triggered = true;
                    break;
                }
            }

            if emergency_shutdown_triggered {
                break;
            }
        }

        tracker.record(
            "repeat_completed",
            "smb100a",
            Some(serde_json::json!({"repeat_index": repeat})),
        );

        if emergency_shutdown_triggered {
            break;
        }
    }

    // ------ Phase 5: final shutdown ------

    let snapshot_after;
    let final_rf_off;
    let final_mod_off;
    let final_fm_off;
    let syst_err_clean_after;

    if !emergency_shutdown_triggered {
        snapshot_after = run_final_shutdown(
            &mut smb_transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            cli.leave_fm_config_enabled,
            &mut tracker,
        )?;
        final_rf_off = snapshot_after
            .queries
            .iter()
            .any(|q| q.command == "OUTP?" && (q.response.trim() == "0" || q.response.trim().eq_ignore_ascii_case("OFF")));
        final_mod_off = snapshot_after
            .queries
            .iter()
            .any(|q| q.command == "MOD:STAT?" && (q.response.trim() == "0" || q.response.trim().eq_ignore_ascii_case("OFF")));
        final_fm_off = if cli.leave_fm_config_enabled {
            true
        } else {
            snapshot_after
                .queries
                .iter()
                .any(|q| q.command == "FM:STAT?" && (q.response.trim() == "0" || q.response.trim().eq_ignore_ascii_case("OFF")))
        };
        syst_err_clean_after = true;
    } else {
        emergency_shutdown_evidence = Some(crate::shutdown::attempt_emergency_shutdown(
            &mut smb_transport,
            delay_ms,
            &errors.join("; "),
        ));
        snapshot_after = Smb100aSnapshot {
            schema_version: "0.2.0".into(),
            device_id: "smb100a_main".into(),
            idn: "emergency_shutdown".into(),
            queried_at_unix_ms: utc_now_ms(),
            queries: vec![],
            connection_closed: false,
        };
        final_rf_off = emergency_shutdown_evidence
            .as_ref()
            .is_some_and(|e| e.outp_command_sent.unwrap_or(false));
        final_mod_off = emergency_shutdown_evidence
            .as_ref()
            .is_some_and(|e| e.mod_command_sent.unwrap_or(false));
        final_fm_off = emergency_shutdown_evidence
            .as_ref()
            .is_some_and(|e| e.fm_command_sent.unwrap_or(false));
        syst_err_clean_after = false;
    }

    smb_transport.close();
    oe_transport.close();
    tracker.record("smb_disconnected", "smb100a", None);
    tracker.record("oe_disconnected", "oe1022d", None);

    // ------ Write remaining artifacts ------

    write_final_artifacts_safe(
        &run_dir,
        cli,
        &audit,
        &tracker,
        &snapshot_before,
        Some(&snapshot_after),
        &step_results,
        &warnings,
        &errors,
        created_at,
    )?;

    // Write step results summary
    write_jsonl(
        &run_dir
            .run_directory_path()
            .join("rf/rf_step_summary.jsonl"),
        &step_results,
    )
    .map_err(|e| format!("write step summary: {}", e))?;

    // Write alignment
    write_jsonl(
        &run_dir
            .run_directory_path()
            .join("alignment/frame_to_rf_step_alignment.jsonl"),
        &all_alignment_entries,
    )
    .map_err(|e| format!("write alignment: {}", e))?;

    let alignment_summary = build_alignment_summary(
        &step_results
            .iter()
            .map(|r| (r.repeat_index, r.step_index, r.frames_captured))
            .collect::<Vec<_>>(),
    );
    run_dir
        .write_json_artifact("alignment/alignment_summary.json", &alignment_summary)
        .map_err(|e| format!("write alignment summary: {}", e))?;

    // Write indexes
    write_jsonl(
        &run_dir.run_directory_path().join("index.jsonl"),
        &all_index_entries,
    )
    .map_err(|e| format!("write index: {}", e))?;

    // Write quarantine manifest
    if !quarantine_entries.is_empty() {
        write_jsonl(
            &run_dir
                .run_directory_path()
                .join("parsed_failed/quarantine.jsonl"),
            &quarantine_entries,
        )
        .map_err(|e| format!("write quarantine: {}", e))?;
    }

    // Run stability summary
    let parse_failure_rate = if total_frames_requested > 0 {
        total_frames_parse_failed as f64 / total_frames_requested as f64
    } else {
        0.0
    };
    let stability = RunStabilitySummary {
        rf_points: cli.rf_points as usize,
        repeat_count: cli.repeat_count as usize,
        frames_requested: total_frames_requested,
        frames_captured: total_frames_captured,
        frames_parsed: total_frames_parsed,
        frames_parse_failed: total_frames_parse_failed,
        parse_failure_rate,
        steps_requested: total_steps,
        steps_completed: step_results.len(),
        final_rf_off,
        final_mod_off,
        final_fm_off,
        syst_err_clean_after,
    };
    run_dir
        .write_json_artifact("metadata/run_stability_summary.json", &stability)
        .map_err(|e| format!("write stability: {}", e))?;

    // Hash manifest
    let hash_manifest = HashManifest {
        schema_version: "0.2.0".into(),
        extended_sweep_config_hash: sha256_file(
            &run_dir
                .run_directory_path()
                .join("metadata/extended_sweep_config.json"),
        )
        .unwrap_or_default(),
        smb100a_snapshot_before_hash: sha256_file(
            &run_dir
                .run_directory_path()
                .join("metadata/smb100a_snapshot_before.json"),
        )
        .unwrap_or_default(),
        smb100a_snapshot_after_hash: sha256_file(
            &run_dir
                .run_directory_path()
                .join("metadata/smb100a_snapshot_after.json"),
        )
        .unwrap_or_default(),
    };
    run_dir
        .write_json_artifact("metadata/hash_manifest.json", &hash_manifest)
        .map_err(|e| format!("write hash: {}", e))?;

    // Audit report
    let audit_report = serde_json::json!({
        "schema_version": "0.2.0",
        "run_id": cli.run_id,
        "audit_completed_at_unix_ms": utc_now_ms(),
        "total_commands_audited": audit.len(),
        "forbidden_commands_attempted": forbidden_attempted.len(),
        "total_steps": total_steps,
        "steps_completed": step_results.len(),
        "repeats_completed": if step_results.is_empty() { 0 } else {
            step_results.iter().map(|r| r.repeat_index).max().map_or(0, |m| m + 1)
        },
        "total_frames_requested": total_frames_requested,
        "total_frames_captured": total_frames_captured,
        "total_frames_parsed": total_frames_parsed,
        "total_frames_parse_failed": total_frames_parse_failed,
        "parse_failure_rate": parse_failure_rate,
        "quarantine_entries": quarantine_entries.len(),
        "emergency_shutdown_attempted": emergency_shutdown_evidence.is_some(),
        "passed": errors.is_empty() && step_results.len() == total_steps,
    });
    run_dir
        .write_json_artifact("audit_report.json", &audit_report)
        .map_err(|e| format!("write audit: {}", e))?;

    // ------ Print summary ------

    let passed =
        errors.is_empty() && step_results.len() == total_steps && !emergency_shutdown_triggered;

    println!("M3.3 SMB100A + OE1022D extended sweep complete.");
    println!("  Passed: {}", passed);
    println!(
        "  Steps: {}/{} completed ({} repeats × {} points)",
        step_results.len(),
        total_steps,
        cli.repeat_count,
        cli.rf_points
    );
    println!(
        "  Frames: {} requested, {} captured, {} parsed, {} parse-failed (rate: {:.4})",
        total_frames_requested, total_frames_captured, total_frames_parsed,
        total_frames_parse_failed, parse_failure_rate
    );
    println!("  Quarantine entries: {}", quarantine_entries.len());
    println!("  OE1022D identity: {}", oe_idn);
    println!("  SMB100A identity: {}", preflight_result.idn);
    println!("  RF power: {:.2} dBm requested", cli.rf_power_dbm);
    println!("  FM deviation: {:.0} Hz", cli.fm_deviation_hz);
    println!(
        "  Frequency plan: {:.0} → {:.0} Hz, {} points × {} repeats",
        cli.rf_start_hz, cli.rf_stop_hz, cli.rf_points, cli.repeat_count
    );
    println!(
        "  Forbidden commands attempted: {}",
        forbidden_attempted.len()
    );
    println!(
        "  Emergency shutdown: {}",
        emergency_shutdown_evidence.is_some()
    );
    println!("  Final RF OFF: {final_rf_off}, MOD OFF: {final_mod_off}, FM OFF: {final_fm_off}");
    println!(
        "  Run directory: {}",
        run_dir.run_directory_path().display()
    );

    if !passed {
        return Err("Extended sweep not fully passed".into());
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_final_artifacts_safe(
    run_dir: &odmr_logging::RunDirectory,
    cli: &Cli,
    audit: &[CommandAuditEntry],
    tracker: &TimelineTracker,
    _snapshot_before: &Smb100aSnapshot,
    snapshot_after: Option<&Smb100aSnapshot>,
    _step_results: &[RfStepResult],
    warnings: &[String],
    errors: &[String],
    _created_at: u64,
) -> Result<(), String> {
    // Station snapshot quality
    let station_quality = StationSnapshotQuality {
        schema_version: "0.2.0".into(),
        status: if errors.is_empty() {
            "passed".into()
        } else {
            "failed".into()
        },
        eligible_for_extended_sweep: errors.is_empty(),
        warnings: warnings.to_vec(),
        errors: errors.to_vec(),
        query_interrupted_seen: false,
        smb_query_delay_ms: cli.smb_query_delay_ms,
    };
    run_dir
        .write_json_artifact("metadata/station_snapshot_quality.json", &station_quality)
        .map_err(|e| format!("write quality: {}", e))?;

    // Safety boundary note
    let safety_note = SafetyBoundaryNote {
        schema_version: "0.2.0".into(),
        real_smb100a_query_only: false,
        real_smb100a_setting_commands_blocked_except_sweep: true,
        rf_on_requires_manual_approval: true,
        no_csv_policy: true,
        no_internal_sweep: true,
        no_gui_hardware_access: true,
        no_magnetic_device_access: true,
    };
    run_dir
        .write_json_artifact("metadata/safety_boundary_note.json", &safety_note)
        .map_err(|e| format!("write safety note: {}", e))?;

    if let Some(after) = snapshot_after {
        run_dir
            .write_json_artifact("metadata/smb100a_snapshot_after.json", after)
            .map_err(|e| format!("write snapshot after: {}", e))?;
    }

    // Events
    let mut event_writer = run_dir.open_event_writer().map_err(|e| e.to_string())?;
    event_writer
        .write_event(&make_event(
            &cli.run_id,
            RunEventType::RunCreated,
            EventLevel::Info,
            "M3.3 extended sweep run created",
            "smb100a",
            None,
            None,
        ))
        .map_err(|e| e.to_string())?;
    event_writer
        .write_event(&make_event(
            &cli.run_id,
            RunEventType::RunCompleted,
            EventLevel::Info,
            "M3.3 extended sweep complete",
            "smb100a",
            None,
            None,
        ))
        .map_err(|e| e.to_string())?;

    // Command audit
    write_jsonl(
        &run_dir.run_directory_path().join("command_audit.jsonl"),
        audit,
    )
    .map_err(|e| format!("write audit: {}", e))?;

    // Timeline
    write_jsonl(
        &run_dir.run_directory_path().join("timeline.jsonl"),
        &tracker.events,
    )
    .map_err(|e| format!("write timeline: {}", e))?;

    Ok(())
}
