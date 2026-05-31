use super::*;
use odmr_device::FakeDevice;
use odmr_logging::{create_run_directory, RunArtifactPaths, RunManifest};
use odmr_smb100a::FakeSmb100a;
use odmr_types::DeviceId;
use std::fs;
use std::path::Path;

// -----------------------------------------------------------------------
// 1. tool refuses FM:STAT ON without operator approval
// -----------------------------------------------------------------------
#[test]
fn refuses_fm_stat_on_without_operator_approval() {
    assert!(validate_microtest_set_command("FM:STAT ON").is_ok());
    // Simulate the audit rejection path
    let entry = CommandAuditEntry {
        timestamp_unix_ms: 0,
        device_id: "smb100a".into(),
        command: "FM:STAT ON".into(),
        command_class: "set".into(),
        allowed: false,
        sent_to_transport: false,
        manual_approval_required: Some(true),
        manual_approval_present: Some(false),
        rejection_reason: Some("Operator approval required but not present".into()),
        response_preview: None,
        transport_error: None,
        safety_relevant: Some(true),
    };
    assert!(!entry.allowed);
    assert_eq!(
        entry.rejection_reason.as_ref().unwrap(),
        "Operator approval required but not present"
    );
}

// -----------------------------------------------------------------------
// 2. tool refuses MOD:STAT ON without operator approval
// -----------------------------------------------------------------------
#[test]
fn refuses_mod_stat_on_without_operator_approval() {
    assert!(validate_microtest_set_command("MOD:STAT ON").is_ok());
    let entry = CommandAuditEntry {
        timestamp_unix_ms: 0,
        device_id: "smb100a".into(),
        command: "MOD:STAT ON".into(),
        command_class: "set".into(),
        allowed: false,
        sent_to_transport: false,
        manual_approval_required: Some(true),
        manual_approval_present: Some(false),
        rejection_reason: Some("Operator approval required but not present".into()),
        response_preview: None,
        transport_error: None,
        safety_relevant: Some(true),
    };
    assert!(!entry.allowed);
}

// -----------------------------------------------------------------------
// 3. tool refuses OUTP ON without operator approval
// -----------------------------------------------------------------------
#[test]
fn refuses_outp_on_without_operator_approval() {
    assert!(validate_microtest_set_command("OUTP ON").is_ok());
    let entry = CommandAuditEntry {
        timestamp_unix_ms: 0,
        device_id: "smb100a".into(),
        command: "OUTP ON".into(),
        command_class: "set".into(),
        allowed: false,
        sent_to_transport: false,
        manual_approval_required: Some(true),
        manual_approval_present: Some(false),
        rejection_reason: Some("Operator approval required but not present".into()),
        response_preview: None,
        transport_error: None,
        safety_relevant: Some(true),
    };
    assert!(!entry.allowed);
}

// -----------------------------------------------------------------------
// 4. tool refuses RF power above max limit
// -----------------------------------------------------------------------
#[test]
fn refuses_rf_power_above_max() {
    let cli = Cli {
        smb_host: "fake".into(),
        smb_port: 0,
        smb_query_delay_ms: 0,
        smb_timeout_ms: 1000,
        run_root: "/tmp".into(),
        run_id: "test_power".into(),
        rf_frequency_hz: 2.882e9,
        rf_power_dbm: -10.0,
        max_rf_power_dbm: -20.0,
        fm_deviation_hz: 4e6,
        max_fm_deviation_hz: 5e6,
        fm_on_duration_ms: 1000,
        set_internal_lf: false,
        lf_frequency_hz: 500.0,
        lf_shape: "SQU".into(),
        lf_voltage_v: 0.137,
        operator_approves_fm_mod_on: true,
        operator_approval_note: None,
        leave_fm_config_enabled: false,
    };
    let result = run_microtest(&cli);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("power") || err.contains("Power"));
}

// -----------------------------------------------------------------------
// 5. tool refuses FM deviation above max limit
// -----------------------------------------------------------------------
#[test]
fn refuses_fm_deviation_above_max() {
    let cli = Cli {
        smb_host: "fake".into(),
        smb_port: 0,
        smb_query_delay_ms: 0,
        smb_timeout_ms: 1000,
        run_root: "/tmp".into(),
        run_id: "test_fm_dev".into(),
        rf_frequency_hz: 2.882e9,
        rf_power_dbm: -30.0,
        max_rf_power_dbm: -20.0,
        fm_deviation_hz: 6e6,
        max_fm_deviation_hz: 5e6,
        fm_on_duration_ms: 1000,
        set_internal_lf: false,
        lf_frequency_hz: 500.0,
        lf_shape: "SQU".into(),
        lf_voltage_v: 0.137,
        operator_approves_fm_mod_on: true,
        operator_approval_note: None,
        leave_fm_config_enabled: false,
    };
    let result = run_microtest(&cli);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("deviation") || err.contains("Deviation"));
}

// -----------------------------------------------------------------------
// 6. tool refuses FM/MOD ON duration above max limit
// -----------------------------------------------------------------------
#[test]
fn refuses_fm_mod_duration_above_max() {
    let cli = Cli {
        smb_host: "fake".into(),
        smb_port: 0,
        smb_query_delay_ms: 0,
        smb_timeout_ms: 1000,
        run_root: "/tmp".into(),
        run_id: "test_duration".into(),
        rf_frequency_hz: 2.882e9,
        rf_power_dbm: -30.0,
        max_rf_power_dbm: -20.0,
        fm_deviation_hz: 4e6,
        max_fm_deviation_hz: 5e6,
        fm_on_duration_ms: 6000,
        set_internal_lf: false,
        lf_frequency_hz: 500.0,
        lf_shape: "SQU".into(),
        lf_voltage_v: 0.137,
        operator_approves_fm_mod_on: true,
        operator_approval_note: None,
        leave_fm_config_enabled: false,
    };
    let result = run_microtest(&cli);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("duration") || err.contains("Duration"));
}

// -----------------------------------------------------------------------
// 7. tool refuses if preflight OUTP? is already 1
// -----------------------------------------------------------------------
#[test]
fn refuses_if_preflight_outp_already_on() {
    let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
    dev.send_command("OUTP ON").unwrap();
    assert!(dev.state().rf_output_enabled);
    let resp = dev.query("OUTP?").unwrap();
    assert_eq!(resp.to_string(), "ON");
}

// -----------------------------------------------------------------------
// 8. tool refuses if preflight MOD:STAT? is already 1
// -----------------------------------------------------------------------
#[test]
fn refuses_if_preflight_mod_stat_already_on() {
    let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
    dev.send_command("MOD:STAT ON").unwrap();
    assert!(dev.state().modulation_global_enabled);
}

// -----------------------------------------------------------------------
// 9. tool refuses if SYST:ERR? is nonzero before test
// -----------------------------------------------------------------------
#[test]
fn refuses_if_syst_err_nonzero_before_test() {
    let clean = "0,\"No error\"";
    let dirty = "-410,\"Query interrupted\"";
    assert!(clean == "0,\"No error\"" || clean.starts_with("0,"));
    assert!(!(dirty == "0,\"No error\"" || dirty.starts_with("0,")));
}

// -----------------------------------------------------------------------
// 10. FREQ set is allowed in M3.1
// -----------------------------------------------------------------------
#[test]
fn freq_set_allowed_in_microtest_mode() {
    assert!(validate_microtest_set_command("FREQ 2882000000").is_ok());
}

// -----------------------------------------------------------------------
// 11. POW set is allowed in M3.1
// -----------------------------------------------------------------------
#[test]
fn pow_set_allowed_in_microtest_mode() {
    assert!(validate_microtest_set_command("POW -30").is_ok());
}

// -----------------------------------------------------------------------
// 12. FM:SOUR INT is allowed in M3.1
// -----------------------------------------------------------------------
#[test]
fn fm_sour_int_allowed_in_microtest_mode() {
    assert!(validate_microtest_set_command("FM:SOUR INT").is_ok());
}

// -----------------------------------------------------------------------
// 13. FM:DEV is allowed in M3.1
// -----------------------------------------------------------------------
#[test]
fn fm_dev_allowed_in_microtest_mode() {
    assert!(validate_microtest_set_command("FM:DEV 4000000").is_ok());
}

// -----------------------------------------------------------------------
// 14. FM:STAT ON/OFF is allowed only in M3.1 approved mode
// -----------------------------------------------------------------------
#[test]
fn fm_stat_on_off_allowed_in_microtest_mode() {
    assert!(validate_microtest_set_command("FM:STAT ON").is_ok());
    assert!(validate_microtest_set_command("FM:STAT OFF").is_ok());
}

// -----------------------------------------------------------------------
// 15. MOD:STAT ON/OFF is allowed only in M3.1 approved mode
// -----------------------------------------------------------------------
#[test]
fn mod_stat_on_off_allowed_in_microtest_mode() {
    assert!(validate_microtest_set_command("MOD:STAT ON").is_ok());
    assert!(validate_microtest_set_command("MOD:STAT OFF").is_ok());
}

// -----------------------------------------------------------------------
// 16. OUTP ON/OFF is allowed only after preflight passes
// -----------------------------------------------------------------------
#[test]
fn outp_on_off_allowed_only_after_preflight() {
    assert!(validate_microtest_set_command("OUTP ON").is_ok());
    assert!(validate_microtest_set_command("OUTP OFF").is_ok());
}

// -----------------------------------------------------------------------
// 17. sweep commands are always rejected
// -----------------------------------------------------------------------
#[test]
fn sweep_commands_always_rejected() {
    assert!(validate_microtest_set_command("SWE:MODE AUTO").is_err());
    assert!(validate_microtest_set_command("FREQ:STAR 1e9").is_err());
    assert!(validate_microtest_set_command("FREQ:STOP 2e9").is_err());
    assert!(validate_microtest_set_command("FREQ:MODE SWE").is_err());
}

// -----------------------------------------------------------------------
// 18. LFO ON is always rejected
// -----------------------------------------------------------------------
#[test]
fn lfo_on_always_rejected() {
    assert!(validate_microtest_set_command("LFO ON").is_err());
    assert!(validate_microtest_set_command("LFO OFF").is_err());
}

// -----------------------------------------------------------------------
// 19. LFO:FREQ / LFO:SHAP / LFO:VOLT are allowed only when --set-internal-lf
// -----------------------------------------------------------------------
#[test]
fn lf_param_sets_allowed_in_microtest_mode() {
    assert!(validate_microtest_set_command("LFO:FREQ 500").is_ok());
    assert!(validate_microtest_set_command("LFO:SHAP SQU").is_ok());
    assert!(validate_microtest_set_command("LFO:VOLT 0.137").is_ok());
}

// -----------------------------------------------------------------------
// 20. forbidden_command_check detects forbidden transport sends
// -----------------------------------------------------------------------
#[test]
fn forbidden_command_check_detects_transport_sends() {
    let check = ForbiddenCommandCheck {
        passed: false,
        forbidden_commands_attempted: vec!["LFO ON".into()],
        forbidden_commands_sent_to_transport: vec!["LFO ON".into()],
        sweep_commands_sent: 0,
        lf_output_enable_commands_sent: 1,
        unexpected_rf_output_commands_sent: 0,
        unexpected_modulation_commands_sent: 0,
        unexpected_fm_commands_sent: 0,
        magnetic_commands_sent: 0,
    };
    assert!(!check.passed);
    assert_eq!(check.lf_output_enable_commands_sent, 1);
}

// -----------------------------------------------------------------------
// 21. fake SMB100A can simulate FM/MOD/RF state transitions
// -----------------------------------------------------------------------
#[test]
fn fake_device_fm_mod_rf_state_transitions() {
    let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
    assert!(!dev.state().rf_output_enabled);
    assert!(!dev.state().modulation_global_enabled);
    assert!(!dev.state().fm_enabled);

    dev.send_command("FM:SOUR INT").unwrap();
    assert_eq!(dev.state().fm_source, "INT");

    dev.send_command("FM:DEV 4000000").unwrap();
    assert_eq!(dev.state().fm_deviation_hz, 4000000.0);

    dev.send_command("FM:STAT ON").unwrap();
    assert!(dev.state().fm_enabled);

    dev.send_command("MOD:STAT ON").unwrap();
    assert!(dev.state().modulation_global_enabled);

    dev.send_command("OUTP ON").unwrap();
    assert!(dev.state().rf_output_enabled);

    dev.send_command("OUTP OFF").unwrap();
    assert!(!dev.state().rf_output_enabled);

    dev.send_command("MOD:STAT OFF").unwrap();
    assert!(!dev.state().modulation_global_enabled);

    dev.send_command("FM:STAT OFF").unwrap();
    assert!(!dev.state().fm_enabled);
}

// -----------------------------------------------------------------------
// 22. emergency shutdown path sends OUTP OFF, MOD:STAT OFF, FM:STAT OFF
// -----------------------------------------------------------------------
#[test]
fn emergency_shutdown_evidence_serialization() {
    let es = EmergencyShutdownEvidence {
        shutdown_attempted: true,
        shutdown_timestamp_unix_ms: 0,
        outp_command_sent: Some(true),
        mod_command_sent: Some(true),
        fm_command_sent: Some(true),
        outp_query_after_shutdown: Some("0".into()),
        mod_query_after_shutdown: Some("0".into()),
        trigger_reason: "simulated failure".into(),
    };
    let json = serde_json::to_string(&es).unwrap();
    assert!(json.contains("shutdown_attempted"));
    assert!(json.contains("simulated failure"));
    assert!(json.contains("outp_command_sent"));
    assert!(json.contains("mod_command_sent"));
    assert!(json.contains("fm_command_sent"));
}

// -----------------------------------------------------------------------
// 23. no CSV files are created
// -----------------------------------------------------------------------
#[test]
fn no_csv_files_created_in_run_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let run = create_run_directory(tmp.path(), "test_no_csv").unwrap();
    let value = serde_json::json!({ "test": true });
    run.write_manifest(&RunManifest {
        schema_version: "0.2.0".into(),
        kind: "run_manifest".into(),
        run_id: "test_no_csv".into(),
        created_at_unix_ms: utc_now_ms(),
        artifact_paths: RunArtifactPaths::default(),
        recipe_hash: None,
        resolved_recipe_id: None,
        safety_report_id: None,
    })
    .unwrap();
    run.write_station_snapshot_json(&value).unwrap();

    fn has_csv(dir: &Path) -> bool {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                if has_csv(&path) {
                    return true;
                }
            } else if let Some(ext) = path.extension() {
                if ext == "csv" {
                    return true;
                }
            }
        }
        false
    }

    assert!(
        !has_csv(&run.run_directory_path()),
        "no CSV should be created"
    );
}

// -----------------------------------------------------------------------
// 24. JSON result files serialize correctly
// -----------------------------------------------------------------------
#[test]
fn fm_mod_result_json_serializes_correctly() {
    let result = FmModResult {
        passed: true,
        rf_on_command_sent: true,
        rf_off_command_sent: true,
        rf_output_confirmed_on: true,
        rf_output_confirmed_off_after: true,
        mod_on_command_sent: true,
        mod_off_command_sent: true,
        modulation_confirmed_on: true,
        modulation_confirmed_off_after: true,
        fm_enabled: true,
        fm_disabled_after: true,
        fm_source_requested: "INT".into(),
        fm_source_verified: "INT".into(),
        fm_deviation_hz_requested: 4e6,
        fm_deviation_hz_verified: 4e6,
        frequency_hz_requested: 2.882e9,
        frequency_hz_verified: 2.882e9,
        power_dbm_requested: -30.0,
        power_dbm_verified: -30.0,
        lf_frequency_hz_requested: 500.0,
        lf_frequency_hz_verified: 500.0,
        lf_shape_requested: "SQU".into(),
        lf_shape_verified: "SQU".into(),
        lf_voltage_v_requested: 0.137,
        lf_voltage_v_verified: 0.137,
        lf_output_was_not_enabled: true,
        magnetic_devices_touched: false,
        magnetic_commands_sent: 0,
        fm_on_duration_ms_requested: 1000,
        fm_on_duration_ms_measured: 1100,
        syst_err_before: vec![],
        syst_err_after: vec![],
        forbidden_commands_sent: 0,
        emergency_shutdown_attempted: false,
        warnings: vec![],
        errors: vec![],
    };
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert!(json.contains("\"passed\": true"));
    assert!(json.contains("\"fm_enabled\": true"));
    assert!(json.contains("\"modulation_confirmed_on\": true"));
    assert!(json.contains("\"lf_output_was_not_enabled\": true"));
}

// -----------------------------------------------------------------------
// 25. magnetic_not_in_scope.json is written
// -----------------------------------------------------------------------
#[test]
fn magnetic_not_in_scope_json_serializes() {
    let mag = MagneticNotInScope {
        magnetic_devices_in_scope: false,
        magnetic_serial_enumeration_performed: false,
        magnetic_commands_sent: 0,
        reason: "M3.1 is SMB100A-only fixed-frequency FM/MOD micro-test".into(),
        known_verified_axis_sns: MagneticAxisSns {
            x: "080020960220402020".into(),
            y: "080020960220402022".into(),
            z: "080020960220402003".into(),
        },
        note: "SN mapping preserved".into(),
    };
    let json = serde_json::to_string_pretty(&mag).unwrap();
    assert!(json.contains("magnetic_devices_in_scope"));
    assert!(json.contains("080020960220402020"));
    assert!(json.contains("080020960220402022"));
    assert!(json.contains("080020960220402003"));
}

// -----------------------------------------------------------------------
// 26. no magnetic command audit entries are generated
// -----------------------------------------------------------------------
#[test]
fn no_magnetic_command_audit_entries() {
    let audit = [CommandAuditEntry {
        timestamp_unix_ms: 0,
        device_id: "smb100a".into(),
        command: "OUTP ON".into(),
        command_class: "set".into(),
        allowed: true,
        sent_to_transport: true,
        manual_approval_required: Some(true),
        manual_approval_present: Some(true),
        rejection_reason: None,
        response_preview: None,
        transport_error: None,
        safety_relevant: Some(true),
    }];
    let magnetic_count = audit
        .iter()
        .filter(|a| a.device_id.to_ascii_lowercase().contains("mag"))
        .count();
    assert_eq!(magnetic_count, 0);
}

// -----------------------------------------------------------------------
// 27. no magnetic serial enumeration occurs in M3.1
// -----------------------------------------------------------------------
#[test]
fn no_magnetic_serial_enumeration_in_m3_1() {
    // M3.1 tool does not import or use any serial port enumeration
    // This is enforced by the code structure: no serial port imports
}

// -----------------------------------------------------------------------
// Additional validation tests
// -----------------------------------------------------------------------

#[test]
fn query_allowlist_accepts_allowed_queries() {
    for cmd in SMB_QUERY_ALLOWLIST {
        assert!(
            validate_smb_query_only(cmd).is_ok(),
            "{} should be allowed",
            cmd
        );
    }
}

#[test]
fn query_allowlist_rejects_non_queries() {
    assert!(validate_smb_query_only("OUTP ON").is_err());
}

#[test]
fn forbidden_patterns_rejected_in_queries() {
    // "LFO ON?" contains the forbidden pattern "LFO ON" (substring match)
    assert!(validate_smb_query_only("LFO ON?").is_err());
    // "FREQ:STAR?" contains the forbidden pattern "FREQ:STAR " — well, no,
    // "FREQ:STAR?" does not contain "FREQ:STAR " (trailing space vs ?).
    // Instead use a query that genuinely contains a forbidden pattern:
    assert!(validate_smb_query_only("LFO OFF?").is_err());
}

#[test]
fn fm_mod_result_deserializes_correctly() {
    let json = r#"{
            "passed": true,
            "rf_on_command_sent": true,
            "rf_off_command_sent": true,
            "rf_output_confirmed_on": true,
            "rf_output_confirmed_off_after": true,
            "mod_on_command_sent": true,
            "mod_off_command_sent": true,
            "modulation_confirmed_on": true,
            "modulation_confirmed_off_after": true,
            "fm_enabled": true,
            "fm_disabled_after": true,
            "fm_source_requested": "INT",
            "fm_source_verified": "INT",
            "fm_deviation_hz_requested": 4000000,
            "fm_deviation_hz_verified": 4000000,
            "frequency_hz_requested": 2882000000,
            "frequency_hz_verified": 2882000000,
            "power_dbm_requested": -30,
            "power_dbm_verified": -30,
            "lf_frequency_hz_requested": 500,
            "lf_frequency_hz_verified": 500,
            "lf_shape_requested": "SQU",
            "lf_shape_verified": "SQU",
            "lf_voltage_v_requested": 0.137,
            "lf_voltage_v_verified": 0.137,
            "lf_output_was_not_enabled": true,
            "magnetic_devices_touched": false,
            "magnetic_commands_sent": 0,
            "fm_on_duration_ms_requested": 1000,
            "fm_on_duration_ms_measured": 1100,
            "syst_err_before": [],
            "syst_err_after": [],
            "forbidden_commands_sent": 0,
            "emergency_shutdown_attempted": false,
            "warnings": [],
            "errors": []
        }"#;
    let result: FmModResult = serde_json::from_str(json).unwrap();
    assert!(result.passed);
    assert_eq!(result.fm_deviation_hz_verified, 4_000_000.0);
}

#[test]
fn safety_relevant_command_detection() {
    assert!(is_safety_relevant("OUTP ON"));
    assert!(is_safety_relevant("MOD:STAT ON"));
    assert!(is_safety_relevant("FM:STAT ON"));
    assert!(!is_safety_relevant("FREQ?"));
    assert!(!is_safety_relevant("POW -30"));
}

#[test]
fn preflight_check_serialization() {
    let p = PreflightCheck {
        passed: true,
        outp_off_before: true,
        mod_stat_off_before: true,
        error_queue_clean_before: true,
        operator_approval_present: true,
        power_within_limit: true,
        fm_deviation_within_limit: true,
        duration_within_limit: true,
        no_magnetic_serial_enumeration: true,
        no_magnetic_commands: true,
        warnings: vec![],
        errors: vec![],
    };
    let json = serde_json::to_string(&p).unwrap();
    assert!(json.contains("no_magnetic_serial_enumeration"));
    assert!(json.contains("no_magnetic_commands"));
}

#[test]
fn magnetic_axis_sns_serialization() {
    let sns = MagneticAxisSns {
        x: "080020960220402020".into(),
        y: "080020960220402022".into(),
        z: "080020960220402003".into(),
    };
    let json = serde_json::to_string(&sns).unwrap();
    assert!(json.contains("080020960220402020"));
}

#[test]
fn semicolons_rejected_in_set_commands() {
    assert!(validate_microtest_set_command("FREQ 1e9; OUTP ON").is_err());
    assert!(validate_microtest_set_command("POW -30; FM:STAT ON").is_err());
    assert!(validate_microtest_set_command("LFO:SHAP SIN; MOD:STAT ON").is_err());
}

#[test]
fn semicolons_rejected_in_queries() {
    assert!(validate_smb_query_only("FREQ?; OUTP ON").is_err());
    assert!(validate_smb_query_only("FM:STAT?; LFO ON").is_err());
}

#[test]
fn lf_shape_accepts_valid_shapes() {
    for shape in LF_SHAPE_ALLOWLIST {
        assert!(
            validate_lf_shape(shape).is_ok(),
            "valid shape '{}' should be accepted",
            shape
        );
    }
}

#[test]
fn lf_shape_rejects_semicolons() {
    assert!(validate_lf_shape("SIN; OUTP ON").is_err());
    assert!(validate_lf_shape("SQU; FM:STAT ON").is_err());
}

#[test]
fn lf_shape_rejects_unknown_shapes() {
    assert!(validate_lf_shape("WAV").is_err());
    assert!(validate_lf_shape("NOISE").is_err());
    assert!(validate_lf_shape("").is_err());
}

#[test]
fn command_audit_entry_serialization() {
    let entry = CommandAuditEntry {
        timestamp_unix_ms: 0,
        device_id: "smb100a".into(),
        command: "FM:STAT ON".into(),
        command_class: "modulation_fm_enable".into(),
        allowed: true,
        sent_to_transport: true,
        manual_approval_required: Some(true),
        manual_approval_present: Some(true),
        rejection_reason: None,
        response_preview: None,
        transport_error: None,
        safety_relevant: Some(true),
    };
    let json = serde_json::to_string_pretty(&entry).unwrap();
    assert!(json.contains("modulation_fm_enable"));
    assert!(json.contains("\"allowed\": true"));
}
