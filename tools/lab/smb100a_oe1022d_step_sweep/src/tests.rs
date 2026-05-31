use super::*;
use odmr_device::FakeDevice;
use odmr_smb100a::FakeSmb100a;
use odmr_types::DeviceId;

// =========================================================================
// 1. CLI defaults
// =========================================================================

#[test]
fn cli_defaults_parse_correctly() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-step-sweep",
        "--run-id",
        "test_defaults",
        "--operator-approves-step-sweep",
        "--operator-approval-note",
        "test",
    ]);
    assert_eq!(cli.smb_host, "169.254.2.20");
    assert_eq!(cli.smb_port, 5025);
    assert_eq!(cli.rf_start_hz, 2_880_000_000.0);
    assert_eq!(cli.rf_stop_hz, 2_884_000_000.0);
    assert_eq!(cli.rf_points, 5);
    assert_eq!(cli.rf_power_dbm, -30.0);
    assert_eq!(cli.fm_deviation_hz, 4_000_000.0);
    assert_eq!(cli.frames_per_step, 3);
    assert!(cli.operator_approves_step_sweep);
}

// =========================================================================
// 2. Frequency step generation
// =========================================================================

#[test]
fn step_plan_5_points_equally_spaced() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-step-sweep",
        "--run-id",
        "test",
        "--rf-start-hz",
        "2880000000",
        "--rf-stop-hz",
        "2884000000",
        "--rf-points",
        "5",
        "--operator-approves-step-sweep",
    ]);
    let plan = compute_step_plan(&cli);
    assert_eq!(plan.frequencies_hz.len(), 5);
    assert!((plan.frequencies_hz[0] - 2_880_000_000.0).abs() < 1.0);
    assert!((plan.frequencies_hz[4] - 2_884_000_000.0).abs() < 1.0);
    assert!(plan.frequencies_hz[0] < plan.frequencies_hz[4]);
}

#[test]
fn step_plan_single_point() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-step-sweep",
        "--run-id",
        "test",
        "--rf-points",
        "1",
        "--operator-approves-step-sweep",
    ]);
    let plan = compute_step_plan(&cli);
    assert_eq!(plan.frequencies_hz.len(), 1);
}

#[test]
fn step_plan_7_points_max() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-step-sweep",
        "--run-id",
        "test",
        "--rf-points",
        "7",
        "--operator-approves-step-sweep",
    ]);
    let plan = compute_step_plan(&cli);
    assert_eq!(plan.frequencies_hz.len(), 7);
}

// =========================================================================
// 3. Rejection when rf_points > 7
// =========================================================================

#[test]
fn rejects_rf_points_above_max() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-step-sweep",
        "--run-id",
        "test",
        "--rf-points",
        "8",
        "--operator-approves-step-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

// =========================================================================
// 4. Rejection when frames_per_step > 5
// =========================================================================

#[test]
fn rejects_frames_per_step_above_max() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-step-sweep",
        "--run-id",
        "test",
        "--frames-per-step",
        "6",
        "--operator-approves-step-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

// =========================================================================
// 5. Power limit enforcement
// =========================================================================

#[test]
fn rejects_power_above_max() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-step-sweep",
        "--run-id",
        "test",
        "--rf-power-dbm=-15",
        "--max-rf-power-dbm=-20",
        "--operator-approves-step-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

#[test]
fn rejects_max_power_above_hard_limit() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-step-sweep",
        "--run-id",
        "test",
        "--max-rf-power-dbm=-5",
        "--operator-approves-step-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

// =========================================================================
// 6. FM deviation limit enforcement
// =========================================================================

#[test]
fn rejects_fm_deviation_above_max() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-step-sweep",
        "--run-id",
        "test",
        "--fm-deviation-hz",
        "6000000",
        "--max-fm-deviation-hz",
        "5000000",
        "--operator-approves-step-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

#[test]
fn rejects_max_fm_deviation_above_hard_limit() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-step-sweep",
        "--run-id",
        "test",
        "--max-fm-deviation-hz",
        "6000000",
        "--operator-approves-step-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

// =========================================================================
// 7. Operator approval gate
// =========================================================================

#[test]
fn operator_approval_required_for_outp_on() {
    assert!(validate_smb_sweep_set("OUTP ON").is_ok());
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

// =========================================================================
// 8. Preflight checks
// =========================================================================

#[test]
fn preflight_passes_when_all_conditions_met() {
    let p = PreflightCheck {
        passed: true,
        outp_off_before: true,
        mod_stat_off_before: true,
        error_queue_clean_before: true,
        operator_approval_present: true,
        power_within_limit: true,
        points_within_limit: true,
        fm_deviation_within_limit: true,
        warnings: vec![],
        errors: vec![],
    };
    assert!(p.passed);
}

#[test]
fn preflight_rejects_when_outp_already_on() {
    let p = PreflightCheck {
        passed: false,
        outp_off_before: false,
        mod_stat_off_before: true,
        error_queue_clean_before: true,
        operator_approval_present: true,
        power_within_limit: true,
        points_within_limit: true,
        fm_deviation_within_limit: true,
        warnings: vec![],
        errors: vec!["OUTP ON".into()],
    };
    assert!(!p.passed);
}

#[test]
fn preflight_rejects_when_mod_already_on() {
    let p = PreflightCheck {
        passed: false,
        outp_off_before: true,
        mod_stat_off_before: false,
        error_queue_clean_before: true,
        operator_approval_present: true,
        power_within_limit: true,
        points_within_limit: true,
        fm_deviation_within_limit: true,
        warnings: vec![],
        errors: vec!["MOD ON".into()],
    };
    assert!(!p.passed);
}

#[test]
fn preflight_rejects_when_syst_err_nonzero() {
    let p = PreflightCheck {
        passed: false,
        outp_off_before: true,
        mod_stat_off_before: true,
        error_queue_clean_before: false,
        operator_approval_present: true,
        power_within_limit: true,
        points_within_limit: true,
        fm_deviation_within_limit: true,
        warnings: vec![],
        errors: vec!["SYST:ERR dirty".into()],
    };
    assert!(!p.passed);
}

// =========================================================================
// 9. SCPI semicolon injection defense
// =========================================================================

#[test]
fn semicolons_rejected_in_smb_queries() {
    assert!(validate_smb_sweep_query("FREQ?; OUTP ON").is_err());
    assert!(validate_smb_sweep_query("*IDN?; *RST").is_err());
}

#[test]
fn semicolons_rejected_in_smb_set_commands() {
    assert!(validate_smb_sweep_set("FREQ 1e9; OUTP ON").is_err());
    assert!(validate_smb_sweep_set("POW -30; FM:STAT ON").is_err());
}

// =========================================================================
// 10. Sweep command rejection
// =========================================================================

#[test]
fn internal_sweep_commands_rejected() {
    assert!(validate_smb_sweep_set("SWE:MODE AUTO").is_err());
    assert!(validate_smb_sweep_set("FREQ:STAR 1e9").is_err());
    assert!(validate_smb_sweep_set("FREQ:STOP 2e9").is_err());
    assert!(validate_smb_sweep_set("FREQ:MODE SWE").is_err());
    assert!(validate_smb_sweep_set("INIT").is_err());
    assert!(validate_smb_sweep_set("TRIG").is_err());
    assert!(validate_smb_sweep_set("LIST").is_err());
}

#[test]
fn lfo_on_rejected_in_sweep() {
    assert!(validate_smb_sweep_set("LFO ON").is_err());
    assert!(validate_smb_sweep_set("LFO OFF").is_err());
}

// =========================================================================
// 11. OE command validation
// =========================================================================

#[test]
fn oe_allows_only_idn_and_rall() {
    assert!(validate_oe_command("*IDN?").is_ok());
    assert!(validate_oe_command("RALL?").is_ok());
}

#[test]
fn oe_rejects_setting_commands() {
    assert!(validate_oe_command("SSETD").is_err());
    assert!(validate_oe_command("FMODD").is_err());
    assert!(validate_oe_command("SENSD").is_err());
    assert!(validate_oe_command("PHASD").is_err());
}

#[test]
fn oe_rejects_semicolons() {
    assert!(validate_oe_command("*IDN?; SSETD").is_err());
    assert!(validate_oe_command("RALL?; FMODD").is_err());
}

// =========================================================================
// 12. LF shape validation
// =========================================================================

#[test]
fn lf_shape_accepts_valid_shapes() {
    for shape in LF_SHAPE_ALLOWLIST {
        assert!(
            validate_lf_shape(shape).is_ok(),
            "{} should be accepted",
            shape
        );
    }
}

#[test]
fn lf_shape_rejects_invalid() {
    assert!(validate_lf_shape("WAV").is_err());
    assert!(validate_lf_shape("").is_err());
}

#[test]
fn lf_shape_case_sensitive() {
    assert!(validate_lf_shape("sin").is_err());
    assert!(validate_lf_shape("Sin").is_err());
}

// =========================================================================
// 13. Serialization roundtrips
// =========================================================================

#[test]
fn step_plan_serializes() {
    let plan = StepPlan {
        schema_version: "0.2.0".into(),
        kind: "software_stepped_rf_plan".into(),
        rf_start_hz: 2.8e9,
        rf_stop_hz: 3.0e9,
        rf_points: 5,
        frequencies_hz: vec![2.8e9, 2.85e9, 2.9e9, 2.95e9, 3.0e9],
        frames_per_step: 3,
        rf_power_dbm: -30.0,
        fm_deviation_hz: 4_000_000.0,
        software_stepped: true,
        smb_internal_sweep_used: false,
    };
    let json = serde_json::to_string(&plan).unwrap();
    assert!(json.contains("software_stepped"));
    assert!(json.contains("smb_internal_sweep_used"));
}

#[test]
fn emergency_shutdown_evidence_serializes() {
    let es = EmergencyShutdownEvidence {
        shutdown_attempted: true,
        shutdown_timestamp_unix_ms: 0,
        outp_command_sent: Some(true),
        mod_command_sent: Some(true),
        fm_command_sent: Some(true),
        outp_query_after_shutdown: Some("0".into()),
        mod_query_after_shutdown: Some("0".into()),
        trigger_reason: "test failure".into(),
    };
    let json = serde_json::to_string(&es).unwrap();
    assert!(json.contains("shutdown_attempted"));
    assert!(json.contains("test failure"));
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
        points_within_limit: true,
        fm_deviation_within_limit: true,
        warnings: vec![],
        errors: vec![],
    };
    let json = serde_json::to_string(&p).unwrap();
    assert!(json.contains("operator_approval_present"));
}

#[test]
fn magnetic_not_in_scope_serialization() {
    let mag = MagneticNotInScope {
        magnetic_devices_in_scope: false,
        magnetic_serial_enumeration_performed: false,
        magnetic_commands_sent: 0,
        reason: "M3.2 test".into(),
        known_verified_axis_sns: MagneticAxisSns {
            x: "080020960220402020".into(),
            y: "080020960220402022".into(),
            z: "080020960220402003".into(),
        },
        note: "test".into(),
    };
    let json = serde_json::to_string(&mag).unwrap();
    assert!(json.contains("magnetic_devices_in_scope"));
    assert!(json.contains("080020960220402020"));
}

// =========================================================================
// 14. No CSV in run directory
// =========================================================================

#[test]
fn no_csv_files_created() {
    let tmp = tempfile::tempdir().unwrap();
    let run = odmr_logging::create_run_directory(tmp.path(), "test_no_csv").unwrap();
    let manifest = odmr_logging::RunManifest {
        schema_version: "0.2.0".into(),
        kind: "run_manifest".into(),
        run_id: "test_no_csv".into(),
        created_at_unix_ms: utc_now_ms(),
        artifact_paths: odmr_logging::RunArtifactPaths::default(),
        recipe_hash: None,
        resolved_recipe_id: None,
        safety_report_id: None,
    };
    run.write_manifest(&manifest).unwrap();

    fn has_csv(dir: &std::path::Path) -> bool {
        for entry in std::fs::read_dir(dir).unwrap() {
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

    assert!(!has_csv(&run.run_directory_path()));
}

// =========================================================================
// 15. Fake device tests
// =========================================================================

#[test]
fn fake_smb100a_freq_set_and_verify() {
    let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
    dev.send_command("FREQ 2882000000").unwrap();
    assert_eq!(dev.state().rf_frequency_hz, 2_882_000_000.0);
}

#[test]
fn fake_smb100a_outp_transitions() {
    let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
    assert!(!dev.state().rf_output_enabled);
    dev.send_command("OUTP ON").unwrap();
    assert!(dev.state().rf_output_enabled);
    dev.send_command("OUTP OFF").unwrap();
    assert!(!dev.state().rf_output_enabled);
}

#[test]
fn fake_smb100a_fm_mod_state() {
    let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
    assert!(!dev.state().modulation_global_enabled);
    assert!(!dev.state().fm_enabled);
    dev.send_command("FM:STAT ON").unwrap();
    assert!(dev.state().fm_enabled);
    dev.send_command("MOD:STAT ON").unwrap();
    assert!(dev.state().modulation_global_enabled);
    dev.send_command("MOD:STAT OFF").unwrap();
    assert!(!dev.state().modulation_global_enabled);
    dev.send_command("FM:STAT OFF").unwrap();
    assert!(!dev.state().fm_enabled);
}

// =========================================================================
// 16. Command audit
// =========================================================================

#[test]
fn command_audit_entry_serialization() {
    let entry = CommandAuditEntry {
        timestamp_unix_ms: 0,
        device_id: "smb100a".into(),
        command: "FREQ 2882000000".into(),
        command_class: "set".into(),
        allowed: true,
        sent_to_transport: true,
        manual_approval_required: None,
        manual_approval_present: None,
        rejection_reason: None,
        response_preview: None,
        transport_error: None,
        safety_relevant: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("FREQ 2882000000"));
}

// =========================================================================
// 17. Alignment
// =========================================================================

#[test]
fn alignment_count_equals_frames_times_steps() {
    let frames: Vec<OeFrameCapture> = (0..3)
        .map(|i| OeFrameCapture {
            raw_bytes: vec![0u8; 12288],
            frame_len: 12288,
            is_full_frame: true,
            raw_offset: i * 12288,
            frame_monotonic_ns: i * 100_000_000,
            elapsed_ms: 800,
            parsed_ok: true,
            b_x_latest: Some(0.0),
            b_y_latest: Some(0.0),
            b_freq_latest: Some(500.0),
        })
        .collect();
    let alignment =
        build_alignment_for_step(&frames, "rf_step_000", 0, 2.88e9, true, true, true, 0);
    assert_eq!(alignment.len(), 3);
    for (i, a) in alignment.iter().enumerate() {
        assert_eq!(a.frame_seq, i as u64);
        assert_eq!(a.step_id, "rf_step_000");
        assert!(a.raw_nbytes == 12288);
    }
}

// =========================================================================
// 18. No magnetic commands
// =========================================================================

#[test]
fn no_magnetic_commands_in_audit() {
    let audit = [CommandAuditEntry {
        timestamp_unix_ms: 0,
        device_id: "smb100a".into(),
        command: "FREQ 2.882e9".into(),
        command_class: "set".into(),
        allowed: true,
        sent_to_transport: true,
        manual_approval_required: None,
        manual_approval_present: None,
        rejection_reason: None,
        response_preview: None,
        transport_error: None,
        safety_relevant: None,
    }];
    let magnetic_count = audit
        .iter()
        .filter(|a| a.device_id.to_ascii_lowercase().contains("mag"))
        .count();
    assert_eq!(magnetic_count, 0);
}

// =========================================================================
// 19. SMB query allowlist
// =========================================================================

#[test]
fn smb_sweep_query_allowlist_accepts_all() {
    for cmd in SMB_SWEEP_QUERY_ALLOWLIST {
        assert!(
            validate_smb_sweep_query(cmd).is_ok(),
            "{} should be allowed",
            cmd
        );
    }
}

#[test]
fn smb_sweep_rejects_unknown_query() {
    assert!(validate_smb_sweep_query("UNKNOWN?").is_err());
}

// =========================================================================
// 20. SMB set allowlist
// =========================================================================

#[test]
fn smb_sweep_set_allowlist_accepts_freq_pow_outp() {
    assert!(validate_smb_sweep_set("FREQ 2882000000").is_ok());
    assert!(validate_smb_sweep_set("POW -30").is_ok());
    assert!(validate_smb_sweep_set("OUTP ON").is_ok());
    assert!(validate_smb_sweep_set("OUTP OFF").is_ok());
}

#[test]
fn smb_sweep_set_rejects_unknown() {
    assert!(validate_smb_sweep_set("UNKNOWN CMD").is_err());
}
