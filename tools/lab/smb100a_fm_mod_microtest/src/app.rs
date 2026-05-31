//! Application-level run orchestration for the M3.1 FM/MOD micro-test tool.

use crate::artifacts::{sha256_file, write_jsonl};
use crate::cli::Cli;
use crate::sequence::run_microtest;
use crate::timeline::{make_event, utc_now_ms};
use crate::types::*;
use odmr_logging::{create_run_directory, EventLevel, RunArtifactPaths, RunEventType, RunManifest};
use std::fs;
use std::path::PathBuf;

pub fn run_app(cli: &Cli) -> Result<(), String> {
    let run_root = PathBuf::from(&cli.run_root);
    let run_dir = create_run_directory(&run_root, &cli.run_id)
        .map_err(|e| format!("Failed to create run directory: {}", e))?;

    let _ = fs::create_dir_all(run_dir.run_directory_path().join("microtest"));

    let created_at = utc_now_ms();

    let config = MicrotestConfig {
        schema_version: "0.2.0".into(),
        smb_host: cli.smb_host.clone(),
        smb_port: cli.smb_port,
        smb_query_delay_ms: cli.smb_query_delay_ms,
        smb_timeout_ms: cli.smb_timeout_ms,
        rf_frequency_hz: cli.rf_frequency_hz,
        rf_power_dbm: cli.rf_power_dbm,
        max_rf_power_dbm: cli.max_rf_power_dbm,
        fm_deviation_hz: cli.fm_deviation_hz,
        max_fm_deviation_hz: cli.max_fm_deviation_hz,
        fm_on_duration_ms: cli.fm_on_duration_ms,
        set_internal_lf: cli.set_internal_lf,
        lf_frequency_hz: cli.lf_frequency_hz,
        lf_shape: cli.lf_shape.clone(),
        lf_voltage_v: cli.lf_voltage_v,
        operator_approves_fm_mod_on: cli.operator_approves_fm_mod_on,
        operator_approval_note: cli.operator_approval_note.clone(),
        leave_fm_config_enabled: cli.leave_fm_config_enabled,
        created_at_unix_ms: created_at,
    };

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
            raw_bin: "raw/oe1022d.rawbin".into(),
        },
        recipe_hash: None,
        resolved_recipe_id: None,
        safety_report_id: None,
    };
    run_dir.write_manifest(&manifest).unwrap();

    let result = run_microtest(cli)?;

    // Write artifacts
    run_dir
        .write_json_artifact("metadata/smb100a_fm_mod_microtest_config.json", &config)
        .unwrap();
    run_dir
        .write_json_artifact(
            "metadata/smb100a_snapshot_before.json",
            &result.snapshot_before,
        )
        .unwrap();
    if let Some(ref during) = result.snapshot_during {
        run_dir
            .write_json_artifact("metadata/smb100a_snapshot_during_fm_mod_on.json", during)
            .unwrap();
    }
    run_dir
        .write_json_artifact(
            "metadata/smb100a_snapshot_after.json",
            &result.snapshot_after,
        )
        .unwrap();

    let station_quality = StationSnapshotQuality {
        schema_version: "0.2.0".into(),
        status: if result.fm_mod_result.passed {
            "passed".into()
        } else {
            "failed".into()
        },
        eligible_for_fm_mod_microtest: result.preflight.passed,
        warnings: result.warnings.clone(),
        errors: result.errors.clone(),
        query_interrupted_seen: result
            .fm_mod_result
            .syst_err_before
            .iter()
            .chain(result.fm_mod_result.syst_err_after.iter())
            .any(|o| o.response.contains("-410")),
        smb_query_delay_ms: cli.smb_query_delay_ms,
    };
    run_dir
        .write_json_artifact("metadata/station_snapshot_quality.json", &station_quality)
        .unwrap();

    let safety_note = SafetyBoundaryNote {
        schema_version: "0.2.0".into(),
        real_smb100a_query_only: false,
        real_smb100a_setting_commands_blocked_except_microtest: true,
        rf_on_requires_manual_approval: true,
        no_csv_policy: true,
        no_sweep: true,
        no_gui_hardware_access: true,
        no_magnetic_device_access: true,
    };
    run_dir
        .write_json_artifact("metadata/safety_boundary_note.json", &safety_note)
        .unwrap();

    if let Some(ref approval) = result.operator_approval {
        run_dir
            .write_json_artifact("metadata/operator_approval.json", approval)
            .unwrap();
    }

    run_dir
        .write_json_artifact(
            "metadata/magnetic_not_in_scope.json",
            &result.magnetic_not_in_scope,
        )
        .unwrap();

    // Events
    let mut event_writer = run_dir.open_event_writer().unwrap();
    event_writer
        .write_event(&make_event(
            &cli.run_id,
            RunEventType::RunCreated,
            EventLevel::Info,
            "M3.1 FM/MOD micro-test run created",
            "smb100a",
            None,
            None,
        ))
        .unwrap();
    event_writer
        .write_event(&make_event(
            &cli.run_id,
            RunEventType::StationSnapshotWritten,
            EventLevel::Info,
            "SMB100A snapshot completed",
            "smb100a",
            None,
            None,
        ))
        .unwrap();
    if result.fm_mod_result.rf_on_command_sent {
        event_writer
            .write_event(&make_event(
                &cli.run_id,
                RunEventType::RunStarted,
                EventLevel::Info,
                "RF ON command sent (FM/MOD enabled)",
                "smb100a",
                None,
                None,
            ))
            .unwrap();
    }
    if result.fm_mod_result.rf_off_command_sent {
        event_writer
            .write_event(&make_event(
                &cli.run_id,
                RunEventType::RunCompleted,
                EventLevel::Info,
                "RF OFF command sent",
                "smb100a",
                None,
                None,
            ))
            .unwrap();
    }
    if result.fm_mod_result.emergency_shutdown_attempted {
        event_writer
            .write_event(&make_event(
                &cli.run_id,
                RunEventType::RunFailed,
                EventLevel::Error,
                "Emergency shutdown attempted",
                "smb100a",
                None,
                None,
            ))
            .unwrap();
    }
    if result.fm_mod_result.passed {
        event_writer
            .write_event(&make_event(
                &cli.run_id,
                RunEventType::RunCompleted,
                EventLevel::Info,
                "M3.1 FM/MOD micro-test passed",
                "smb100a",
                None,
                None,
            ))
            .unwrap();
    } else {
        event_writer
            .write_event(&make_event(
                &cli.run_id,
                RunEventType::RunFailed,
                EventLevel::Error,
                &format!(
                    "M3.1 FM/MOD micro-test failed: {}",
                    result.errors.join("; ")
                ),
                "smb100a",
                None,
                None,
            ))
            .unwrap();
    }

    // JSONL files
    write_jsonl(
        &run_dir.run_directory_path().join("command_audit.jsonl"),
        &result.audit,
    )
    .unwrap();
    write_jsonl(
        &run_dir.run_directory_path().join("timeline.jsonl"),
        &result.timeline,
    )
    .unwrap();

    // Microtest artifacts
    run_dir
        .write_json_artifact("microtest/preflight_check.json", &result.preflight)
        .unwrap();

    write_jsonl(
        &run_dir
            .run_directory_path()
            .join("microtest/fm_mod_sequence.jsonl"),
        &result.audit,
    )
    .unwrap();

    run_dir
        .write_json_artifact("microtest/fm_mod_result.json", &result.fm_mod_result)
        .unwrap();
    run_dir
        .write_json_artifact(
            "microtest/forbidden_command_check.json",
            &result.forbidden_check,
        )
        .unwrap();

    if let Some(ref es) = result.emergency_shutdown {
        run_dir
            .write_json_artifact("microtest/emergency_shutdown_evidence.json", es)
            .unwrap();
    }

    // Hash manifest
    let hash_manifest = HashManifest {
        schema_version: "0.2.0".into(),
        smb100a_fm_mod_microtest_config_hash: sha256_file(
            &run_dir
                .run_directory_path()
                .join("metadata/smb100a_fm_mod_microtest_config.json"),
        )
        .unwrap_or_default(),
        smb100a_snapshot_before_hash: sha256_file(
            &run_dir
                .run_directory_path()
                .join("metadata/smb100a_snapshot_before.json"),
        )
        .unwrap_or_default(),
        smb100a_snapshot_during_hash: if result.snapshot_during.is_some() {
            sha256_file(
                &run_dir
                    .run_directory_path()
                    .join("metadata/smb100a_snapshot_during_fm_mod_on.json"),
            )
            .unwrap_or_default()
        } else {
            "n/a".into()
        },
        smb100a_snapshot_after_hash: sha256_file(
            &run_dir
                .run_directory_path()
                .join("metadata/smb100a_snapshot_after.json"),
        )
        .unwrap_or_default(),
    };
    run_dir
        .write_json_artifact("metadata/hash_manifest.json", &hash_manifest)
        .unwrap();

    // Audit report
    let audit_report = serde_json::json!({
        "schema_version": "0.2.0",
        "run_id": cli.run_id,
        "audit_completed_at_unix_ms": utc_now_ms(),
        "total_commands_audited": result.audit.len(),
        "forbidden_commands_attempted": result.forbidden_check.forbidden_commands_attempted.len(),
        "forbidden_commands_sent": result.forbidden_check.forbidden_commands_sent_to_transport.len(),
        "rf_on_command_sent": result.fm_mod_result.rf_on_command_sent,
        "rf_off_command_sent": result.fm_mod_result.rf_off_command_sent,
        "rf_output_confirmed_on": result.fm_mod_result.rf_output_confirmed_on,
        "rf_output_confirmed_off_after": result.fm_mod_result.rf_output_confirmed_off_after,
        "mod_on_command_sent": result.fm_mod_result.mod_on_command_sent,
        "mod_off_command_sent": result.fm_mod_result.mod_off_command_sent,
        "modulation_confirmed_on": result.fm_mod_result.modulation_confirmed_on,
        "modulation_confirmed_off_after": result.fm_mod_result.modulation_confirmed_off_after,
        "fm_enabled": result.fm_mod_result.fm_enabled,
        "fm_disabled_after": result.fm_mod_result.fm_disabled_after,
        "emergency_shutdown_attempted": result.fm_mod_result.emergency_shutdown_attempted,
        "passed": result.fm_mod_result.passed,
    });
    run_dir
        .write_json_artifact("audit_report.json", &audit_report)
        .unwrap();

    println!("M3.1 FM/MOD micro-test complete.");
    println!("  Passed: {}", result.fm_mod_result.passed);
    println!("  RF ON sent: {}", result.fm_mod_result.rf_on_command_sent);
    println!(
        "  RF OFF sent: {}",
        result.fm_mod_result.rf_off_command_sent
    );
    println!(
        "  RF output confirmed ON: {}",
        result.fm_mod_result.rf_output_confirmed_on
    );
    println!(
        "  RF output confirmed OFF after: {}",
        result.fm_mod_result.rf_output_confirmed_off_after
    );
    println!(
        "  MOD ON sent: {}",
        result.fm_mod_result.mod_on_command_sent
    );
    println!(
        "  MOD OFF sent: {}",
        result.fm_mod_result.mod_off_command_sent
    );
    println!(
        "  Modulation confirmed ON: {}",
        result.fm_mod_result.modulation_confirmed_on
    );
    println!(
        "  Modulation confirmed OFF after: {}",
        result.fm_mod_result.modulation_confirmed_off_after
    );
    println!("  FM enabled: {}", result.fm_mod_result.fm_enabled);
    println!(
        "  FM disabled after: {}",
        result.fm_mod_result.fm_disabled_after
    );
    println!(
        "  Frequency requested: {:.0} Hz, verified: {:.0} Hz",
        result.fm_mod_result.frequency_hz_requested, result.fm_mod_result.frequency_hz_verified
    );
    println!(
        "  Power requested: {:.2} dBm, verified: {:.2} dBm",
        result.fm_mod_result.power_dbm_requested, result.fm_mod_result.power_dbm_verified
    );
    println!(
        "  FM deviation requested: {:.0} Hz, verified: {:.0} Hz",
        result.fm_mod_result.fm_deviation_hz_requested,
        result.fm_mod_result.fm_deviation_hz_verified
    );
    println!(
        "  Duration requested: {} ms, measured: {} ms",
        result.fm_mod_result.fm_on_duration_ms_requested,
        result.fm_mod_result.fm_on_duration_ms_measured
    );
    println!(
        "  Forbidden commands attempted: {}",
        result.forbidden_check.forbidden_commands_attempted.len()
    );
    println!(
        "  Emergency shutdown: {}",
        result.fm_mod_result.emergency_shutdown_attempted
    );
    println!(
        "  Run directory: {}",
        run_dir.run_directory_path().display()
    );

    if !result.fm_mod_result.passed {
        return Err(result.errors.join("; "));
    }

    Ok(())
}
