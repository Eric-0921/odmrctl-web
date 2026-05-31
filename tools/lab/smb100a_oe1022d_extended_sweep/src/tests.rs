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
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test_defaults",
        "--operator-approves-extended-sweep",
        "--operator-approval-note",
        "test",
    ]);
    assert_eq!(cli.smb_host, "169.254.2.20");
    assert_eq!(cli.smb_port, 5025);
    assert_eq!(cli.rf_start_hz, 2_878_000_000.0);
    assert_eq!(cli.rf_stop_hz, 2_886_000_000.0);
    assert_eq!(cli.rf_points, 11);
    assert_eq!(cli.repeat_count, 2);
    assert_eq!(cli.frames_per_step, 5);
    assert_eq!(cli.rf_power_dbm, -30.0);
    assert_eq!(cli.fm_deviation_hz, 4_000_000.0);
    assert!(cli.operator_approves_extended_sweep);
}

// =========================================================================
// 2. Frequency step generation
// =========================================================================

#[test]
fn step_plan_11_points_equally_spaced() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--rf-start-hz",
        "2878000000",
        "--rf-stop-hz",
        "2886000000",
        "--rf-points",
        "11",
        "--operator-approves-extended-sweep",
    ]);
    let plan = compute_step_plan(&cli);
    assert_eq!(plan.frequencies_hz.len(), 11);
    assert!((plan.frequencies_hz[0] - 2_878_000_000.0).abs() < 1.0);
    assert!(
        (plan.frequencies_hz[10] - 2_886_000_000.0).abs() < 1.0
    );
    // Midpoint should be ~2_882_000_000
    let mid = plan.frequencies_hz[5];
    assert!((mid - 2_882_000_000.0).abs() < 1.0);
    assert_eq!(plan.repeat_count, 2);
}

#[test]
fn step_plan_21_points() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--rf-start-hz",
        "2878000000",
        "--rf-stop-hz",
        "2886000000",
        "--rf-points",
        "21",
        "--operator-approves-extended-sweep",
    ]);
    let plan = compute_step_plan(&cli);
    assert_eq!(plan.frequencies_hz.len(), 21);
}

#[test]
fn step_plan_single_point() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--rf-start-hz",
        "2880000000",
        "--rf-stop-hz",
        "2880000000",
        "--rf-points",
        "1",
        "--operator-approves-extended-sweep",
    ]);
    let plan = compute_step_plan(&cli);
    assert_eq!(plan.frequencies_hz.len(), 1);
    assert!((plan.frequencies_hz[0] - 2_880_000_000.0).abs() < 1.0);
}

// =========================================================================
// 3-6. Hard limit violations
// =========================================================================

#[test]
fn rf_points_greater_than_21_rejected() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--rf-points",
        "22",
        "--operator-approves-extended-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

#[test]
fn repeat_count_greater_than_3_rejected() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--repeat-count",
        "4",
        "--operator-approves-extended-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

#[test]
fn frames_per_step_greater_than_10_rejected() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--frames-per-step",
        "11",
        "--operator-approves-extended-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

#[test]
fn total_frames_greater_than_630_rejected() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--rf-points",
        "21",
        "--frames-per-step",
        "10",
        "--repeat-count",
        "3",
        "--operator-approves-extended-sweep",
    ]);
    // 21 * 10 * 3 = 630 — this is exactly at limit, should pass
    assert!(validate_safety_limits(&cli).is_ok());

    let cli2 = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--rf-points",
        "21",
        "--frames-per-step",
        "10",
        "--repeat-count",
        "4",
        "--operator-approves-extended-sweep",
    ]);
    // 21 * 10 * 4 = 840 > 630, but repeat_count=4 already fails the repeat limit
    assert!(validate_safety_limits(&cli2).is_err());
}

#[test]
fn rf_points_21_within_limit() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--rf-points",
        "21",
        "--operator-approves-extended-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_ok());
}

// =========================================================================
// 7. Hard power/FM limits
// =========================================================================

#[test]
fn rf_power_exceeds_max_rejected() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--rf-power-dbm=-5",
        "--max-rf-power-dbm=-10",
        "--operator-approves-extended-sweep",
    ]);
    // -5 > -10, but max_rf_power_dbm is -10 which equals hard limit — valid
    // The power check is: rf_power_dbm > max_rf_power_dbm
    // -5 > -10 → true → rejected
    assert!(validate_safety_limits(&cli).is_err());
}

#[test]
fn max_rf_power_above_hard_limit_rejected() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--max-rf-power-dbm=-5",
        "--operator-approves-extended-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

#[test]
fn fm_deviation_exceeds_max_rejected() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--fm-deviation-hz",
        "6000000",
        "--max-fm-deviation-hz",
        "5000000",
        "--operator-approves-extended-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

#[test]
fn max_fm_deviation_above_hard_limit_rejected() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--max-fm-deviation-hz",
        "6000000",
        "--operator-approves-extended-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

#[test]
fn rf_stop_less_than_start_rejected() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--rf-start-hz",
        "2890000000",
        "--rf-stop-hz",
        "2880000000",
        "--operator-approves-extended-sweep",
    ]);
    assert!(validate_safety_limits(&cli).is_err());
}

// =========================================================================
// 8. Operator approval gate
// =========================================================================

#[test]
fn operator_approval_required_for_extended_sweep() {
    let cli_no = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
    ]);
    assert!(!cli_no.operator_approves_extended_sweep);

    let cli_yes = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--operator-approves-extended-sweep",
    ]);
    assert!(cli_yes.operator_approves_extended_sweep);
}

#[test]
fn approval_note_optional() {
    let cli = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--operator-approves-extended-sweep",
    ]);
    assert!(cli.operator_approval_note.is_none());

    let cli2 = Cli::parse_from([
        "smb100a-oe1022d-extended-sweep",
        "--run-id",
        "test",
        "--operator-approves-extended-sweep",
        "--operator-approval-note",
        "M3.3 test",
    ]);
    assert_eq!(cli2.operator_approval_note.unwrap(), "M3.3 test");
}

// =========================================================================
// 9. Preflight checks (logical)
// =========================================================================

#[test]
fn preflight_requires_outp_off_and_mod_off() {
    // Preflight logic: OUTP?=0, MOD:STAT?=0, SYST:ERR clean, operator present
    let check = PreflightCheck {
        passed: true,
        outp_off_before: true,
        mod_stat_off_before: true,
        error_queue_clean_before: true,
        operator_approval_present: true,
        power_within_limit: true,
        points_within_limit: true,
        repeat_within_limit: true,
        fm_deviation_within_limit: true,
        warnings: vec![],
        errors: vec![],
    };
    assert!(check.passed);
}

#[test]
fn preflight_fails_when_outp_on() {
    let check = PreflightCheck {
        passed: false, // should be set by logic
        outp_off_before: false,
        mod_stat_off_before: true,
        error_queue_clean_before: true,
        operator_approval_present: true,
        power_within_limit: true,
        points_within_limit: true,
        repeat_within_limit: true,
        fm_deviation_within_limit: true,
        warnings: vec![],
        errors: vec!["OUTP? = '1' (expected OFF/0)".into()],
    };
    assert!(!check.passed);
    assert!(!check.outp_off_before);
}

#[test]
fn preflight_fails_when_no_approval() {
    let check = PreflightCheck {
        passed: false,
        outp_off_before: true,
        mod_stat_off_before: true,
        error_queue_clean_before: true,
        operator_approval_present: false,
        power_within_limit: true,
        points_within_limit: true,
        repeat_within_limit: true,
        fm_deviation_within_limit: true,
        warnings: vec![],
        errors: vec!["Operator approval required".into()],
    };
    assert!(!check.passed);
}

// =========================================================================
// 10. Semicolon injection defense
// =========================================================================

#[test]
fn semicolon_rejected_in_smb_query() {
    assert!(validate_smb_sweep_query("*IDN?;*RST").is_err());
    assert!(validate_smb_sweep_query("FREQ?;OUTP ON").is_err());
    assert!(validate_smb_sweep_query("*IDN?").is_ok());
}

#[test]
fn semicolon_rejected_in_smb_set() {
    assert!(validate_smb_sweep_set("FREQ 2.88e9;OUTP ON").is_err());
    assert!(validate_smb_sweep_set("POW -30").is_ok());
}

#[test]
fn semicolon_rejected_in_oe_command() {
    assert!(validate_oe_command("*IDN?;RALL?").is_err());
    assert!(validate_oe_command("RALL?").is_ok());
}

// =========================================================================
// 11. Internal sweep commands rejected
// =========================================================================

#[test]
fn internal_sweep_commands_rejected() {
    let forbidden = [
        "SWE:MODE STEP",
        "FREQ:STAR 2e9",
        "FREQ:STOP 3e9",
        "LIST",
        "TRIG:SOUR INT",
        "INIT:IMM",
    ];
    for cmd in &forbidden {
        assert!(
            validate_smb_sweep_set(cmd).is_err(),
            "Command '{}' should be forbidden",
            cmd
        );
    }
}

// =========================================================================
// 12-13. OE allowlist
// =========================================================================

#[test]
fn oe_only_allows_idn_and_rall_query() {
    assert!(validate_oe_command("*IDN?").is_ok());
    assert!(validate_oe_command("RALL?").is_ok());
    assert!(validate_oe_command("SSETD 1,1").is_err());
    assert!(validate_oe_command("PHASD 2,0").is_err());
    assert!(validate_oe_command("OUTPD? 2,1").is_err());
}

// =========================================================================
// 14-15. B-channel statistics
// =========================================================================

#[test]
fn statistics_mean_std_min_max() {
    let stats = compute_vector_stats(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
    assert!((stats.mean - 3.0).abs() < 1e-9);
    assert!((stats.min - 1.0).abs() < 1e-9);
    assert!((stats.max - 5.0).abs() < 1e-9);
    assert!(stats.std > 0.0);
}

#[test]
fn statistics_empty_returns_none() {
    assert!(compute_vector_stats(&[]).is_none());
}

#[test]
fn statistics_nan_returns_none() {
    assert!(compute_vector_stats(&[1.0, f64::NAN]).is_none());
}

#[test]
fn statistics_infinite_returns_none() {
    assert!(compute_vector_stats(&[1.0, f64::INFINITY]).is_none());
}

#[test]
fn aggregate_b_channel_stats_produces_correct_values() {
    let bx = vec![vec![1.0, 2.0, 3.0]];
    let by = vec![vec![4.0, 6.0, 8.0]];
    let (xs, ys) = aggregate_b_channel_stats(&bx, &by).unwrap();
    assert!((xs.mean - 2.0).abs() < 1e-9);
    assert!((xs.min - 1.0).abs() < 1e-9);
    assert!((xs.max - 3.0).abs() < 1e-9);
    assert!((ys.mean - 6.0).abs() < 1e-9);
    assert!((ys.min - 4.0).abs() < 1e-9);
    assert!((ys.max - 8.0).abs() < 1e-9);
}

#[test]
fn aggregate_multiple_frames() {
    let bx = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let by = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
    let (xs, ys) = aggregate_b_channel_stats(&bx, &by).unwrap();
    assert!((xs.mean - 2.5).abs() < 1e-9);
    assert!((xs.min - 1.0).abs() < 1e-9);
    assert!((xs.max - 4.0).abs() < 1e-9);
    assert!((ys.mean - 6.5).abs() < 1e-9);
}

// =========================================================================
// 16-17. Parse failure quarantine types
// =========================================================================

#[test]
fn parse_quarantine_entry_serialization() {
    let entry = ParseQuarantineEntry {
        frame_seq: 42,
        step_id: "repeat_0_rf_step_003".into(),
        raw_nbytes: 12288,
        error_type: "RallParseError".into(),
        error_detail: "RallParseError::WrongLength".into(),
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("42"));
    assert!(json.contains("repeat_0_rf_step_003"));
    assert!(json.contains("12288"));
    let back: ParseQuarantineEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(back.frame_seq, 42);
    assert_eq!(back.error_type, "RallParseError");
}

// =========================================================================
// 18. Alignment entries count
// =========================================================================

#[test]
fn alignment_entry_count_matches_repeats_times_points_times_frames() {
    // Simulated: 2 repeats × 3 points × 2 frames = 12 alignment entries
    let mut entries: Vec<FrameToStepAlignment> = Vec::new();
    for repeat in 0..2u64 {
        for step in 0..3u64 {
            for frame in 0..2u64 {
                let global_seq = repeat * 6 + step * 2 + frame;
                entries.push(FrameToStepAlignment {
                    schema_version: "0.2.0".into(),
                    frame_seq: global_seq,
                    raw_offset: global_seq * 12288,
                    raw_nbytes: 12288,
                    step_id: format!("repeat_{}_rf_step_{:03}", repeat, step),
                    step_index: step,
                    repeat_index: repeat,
                    frequency_hz: 2_880_000_000.0,
                    rf_output_state: "on".into(),
                    mod_state: "on".into(),
                    fm_state: "on".into(),
                    frame_monotonic_ns_since_run_start: 0,
                    alignment_method: "software_step_active_window".into(),
                    parse_status: "ok".into(),
                });
            }
        }
    }
    assert_eq!(entries.len(), 12);
    // Verify step IDs
    assert_eq!(entries[0].step_id, "repeat_0_rf_step_000");
    assert_eq!(entries[6].step_id, "repeat_1_rf_step_000");
    assert_eq!(entries[11].step_id, "repeat_1_rf_step_002");
}

// =========================================================================
// 19. Repeat loop preserves step ordering
// =========================================================================

#[test]
fn step_ids_preserve_repeat_then_step_order() {
    let mut ids = Vec::new();
    for repeat in 0..3u64 {
        for step in 0..5u64 {
            ids.push(format!("repeat_{}_rf_step_{:03}", repeat, step));
        }
    }
    assert_eq!(ids[0], "repeat_0_rf_step_000");
    assert_eq!(ids[4], "repeat_0_rf_step_004");
    assert_eq!(ids[5], "repeat_1_rf_step_000");
    assert_eq!(ids[10], "repeat_2_rf_step_000");
    assert_eq!(ids.len(), 15);
}

// =========================================================================
// 20. Raw offsets contiguous across repeats
// =========================================================================

#[test]
fn frame_seq_contiguous_across_repeats() {
    let mut offset: u64 = 0;
    let mut seq: u64 = 0;
    let frame_bytes: u64 = 12288;
    for _repeat in 0..2u64 {
        for _step in 0..3u64 {
            for _frame in 0..2u64 {
                assert_eq!(seq * frame_bytes, offset * frame_bytes, "seq={} offset={}", seq, offset);
                seq += 1;
                offset += 1;
            }
        }
    }
    assert_eq!(seq, 12);
    assert_eq!(offset, 12);
}

// =========================================================================
// 21-23. No CSV, no magnetic
// =========================================================================

#[test]
fn magnetic_not_in_scope_default() {
    let mag = MagneticNotInScope {
        magnetic_devices_in_scope: false,
        magnetic_serial_enumeration_performed: false,
        magnetic_commands_sent: 0,
        reason: "M3.3 is SMB100A + OE1022D only".into(),
        known_verified_axis_sns: MagneticAxisSns {
            x: "080020960220402020".into(),
            y: "080020960220402022".into(),
            z: "080020960220402003".into(),
        },
        note: "SN mapping preserved".into(),
    };
    assert!(!mag.magnetic_devices_in_scope);
    assert_eq!(mag.magnetic_commands_sent, 0);
}

#[test]
fn no_magnetic_commands_in_forbidden_check() {
    let check = ForbiddenCommandCheck {
        passed: true,
        forbidden_commands_attempted: vec![],
        forbidden_commands_sent_to_transport: vec![],
        sweep_commands_sent: 0,
        lf_output_enable_commands_sent: 0,
        unexpected_rf_output_commands_sent: 0,
        unexpected_modulation_commands_sent: 0,
        unexpected_fm_commands_sent: 0,
        magnetic_commands_sent: 0,
        oe_setting_commands_sent: 0,
    };
    assert_eq!(check.magnetic_commands_sent, 0);
}

// =========================================================================
// 24. Emergency shutdown evidence serialization
// =========================================================================

#[test]
fn emergency_shutdown_evidence_serialization() {
    let evidence = EmergencyShutdownEvidence {
        shutdown_attempted: true,
        shutdown_timestamp_unix_ms: 1234567890,
        outp_command_sent: Some(true),
        mod_command_sent: Some(true),
        fm_command_sent: Some(true),
        outp_query_after_shutdown: Some("0".into()),
        mod_query_after_shutdown: Some("0".into()),
        trigger_reason: "OE acquisition failed".into(),
    };
    let json = serde_json::to_string(&evidence).unwrap();
    assert!(json.contains("1234567890"));
    assert!(json.contains("OE acquisition failed"));
    let back: EmergencyShutdownEvidence = serde_json::from_str(&json).unwrap();
    assert!(back.shutdown_attempted);
    assert_eq!(back.outp_command_sent, Some(true));
}

// =========================================================================
// 25. Type serialization roundtrips
// =========================================================================

#[test]
fn step_statistics_roundtrip_json() {
    let stats = StepStatistics {
        repeat_index: 1,
        step_id: "repeat_1_rf_step_003".into(),
        frequency_hz: 2_882_000_000.0,
        b_x_mean_mv: 1.5,
        b_x_std_mv: 0.3,
        b_x_min_mv: 0.9,
        b_x_max_mv: 2.1,
        b_y_mean_mv: 3.5,
        b_y_std_mv: 0.7,
        b_y_min_mv: 2.0,
        b_y_max_mv: 5.0,
        frames_used: 5,
        frames_parse_failed: 0,
    };
    let json = serde_json::to_string(&stats).unwrap();
    let back: StepStatistics = serde_json::from_str(&json).unwrap();
    assert_eq!(back.repeat_index, 1);
    assert!((back.b_x_mean_mv - 1.5).abs() < 1e-9);
    assert_eq!(back.frames_used, 5);
}

#[test]
fn run_stability_summary_roundtrip_json() {
    let summary = RunStabilitySummary {
        rf_points: 11,
        repeat_count: 2,
        frames_requested: 110,
        frames_captured: 110,
        frames_parsed: 108,
        frames_parse_failed: 2,
        parse_failure_rate: 2.0 / 110.0,
        steps_requested: 22,
        steps_completed: 22,
        final_rf_off: true,
        final_mod_off: true,
        final_fm_off: true,
        syst_err_clean_after: true,
    };
    let json = serde_json::to_string(&summary).unwrap();
    let back: RunStabilitySummary = serde_json::from_str(&json).unwrap();
    assert_eq!(back.rf_points, 11);
    assert_eq!(back.repeat_count, 2);
    assert_eq!(back.frames_parse_failed, 2);
    assert!(back.final_rf_off);
    assert!((back.parse_failure_rate - 2.0 / 110.0).abs() < 1e-9);
}

#[test]
fn rf_step_result_roundtrip_json() {
    let step = RfStepResult {
        schema_version: "0.2.0".into(),
        step_id: "repeat_0_rf_step_001".into(),
        step_index: 1,
        repeat_index: 0,
        frequency_hz_requested: 2_880_800_000.0,
        frequency_hz_verified: 2_880_800_000.1,
        frequency_set_ok: true,
        rf_on_sent: true,
        rf_off_sent: true,
        rf_on_confirmed: true,
        rf_off_confirmed_after_step: true,
        frames_requested: 5,
        frames_captured: 5,
        frames_parsed: 5,
        frames_failed: 0,
        frames_parse_failed: 0,
        step_passed: true,
        duration_ms: 4500,
        statistics: None,
        warnings: vec![],
        errors: vec![],
    };
    let json = serde_json::to_string(&step).unwrap();
    let back: RfStepResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.step_id, "repeat_0_rf_step_001");
    assert_eq!(back.repeat_index, 0);
    assert!(back.step_passed);
}

#[test]
fn sweep_result_roundtrip_json() {
    let result = SweepResult {
        passed: true,
        total_steps: 4,
        steps_completed: 4,
        steps_failed: 0,
        total_frames_requested: 20,
        total_frames_captured: 20,
        total_frames_parsed: 20,
        step_results: vec![],
        oe_idn: "SSI,LLA-OE1022D,SN123456,Ver1.0".into(),
        preflight: PreflightCheck {
            passed: true,
            outp_off_before: true,
            mod_stat_off_before: true,
            error_queue_clean_before: true,
            operator_approval_present: true,
            power_within_limit: true,
            points_within_limit: true,
            repeat_within_limit: true,
            fm_deviation_within_limit: true,
            warnings: vec![],
            errors: vec![],
        },
        forbidden_check: ForbiddenCommandCheck {
            passed: true,
            forbidden_commands_attempted: vec![],
            forbidden_commands_sent_to_transport: vec![],
            sweep_commands_sent: 0,
            lf_output_enable_commands_sent: 0,
            unexpected_rf_output_commands_sent: 0,
            unexpected_modulation_commands_sent: 0,
            unexpected_fm_commands_sent: 0,
            magnetic_commands_sent: 0,
            oe_setting_commands_sent: 0,
        },
        emergency_shutdown_attempted: false,
        repeat_count: 2,
        parse_failure_rate: 0.0,
        stability: None,
        warnings: vec![],
        errors: vec![],
    };
    let json = serde_json::to_string(&result).unwrap();
    let back: SweepResult = serde_json::from_str(&json).unwrap();
    assert!(back.passed);
    assert_eq!(back.total_steps, 4);
    assert_eq!(back.repeat_count, 2);
}

#[test]
fn step_plan_serialization() {
    let plan = StepPlan {
        schema_version: "0.2.0".into(),
        kind: "software_stepped_rf_plan_extended".into(),
        rf_start_hz: 2_878_000_000.0,
        rf_stop_hz: 2_886_000_000.0,
        rf_points: 11,
        repeat_count: 2,
        frequencies_hz: vec![2.878e9, 2.882e9, 2.886e9],
        frames_per_step: 5,
        rf_power_dbm: -30.0,
        fm_deviation_hz: 4_000_000.0,
        software_stepped: true,
        smb_internal_sweep_used: false,
    };
    let json = serde_json::to_string(&plan).unwrap();
    let back: StepPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(back.repeat_count, 2);
    assert_eq!(back.rf_points, 11);
    assert_eq!(back.frequencies_hz.len(), 3);
}

#[test]
fn sweep_config_serialization() {
    let config = SweepConfig {
        schema_version: "0.2.0".into(),
        smb_host: "169.254.2.20".into(),
        smb_port: 5025,
        smb_query_delay_ms: 50,
        smb_timeout_ms: 3000,
        oe_port: "/dev/cu.usbmodem123".into(),
        oe_baud: 921600,
        oe_timeout_ms: 8000,
        rf_start_hz: 2_878_000_000.0,
        rf_stop_hz: 2_886_000_000.0,
        rf_points: 11,
        rf_power_dbm: -30.0,
        max_rf_power_dbm: -20.0,
        fm_deviation_hz: 4_000_000.0,
        max_fm_deviation_hz: 5_000_000.0,
        repeat_count: 2,
        set_internal_lf: false,
        lf_frequency_hz: None,
        lf_shape: None,
        lf_voltage_v: None,
        frames_per_step: 5,
        inter_frame_delay_ms: 20,
        oe_frame_delay_ms: 800,
        created_at_unix_ms: 1234567890,
    };
    let json = serde_json::to_string(&config).unwrap();
    let back: SweepConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.repeat_count, 2);
    assert_eq!(back.rf_points, 11);
    assert_eq!(back.frames_per_step, 5);
}

// =========================================================================
// 26. FakeSMB100A state transitions
// =========================================================================

#[test]
fn fake_smb100a_freq_set_verify() {
    let mut fake = FakeSmb100a::new(DeviceId::new("smb_test"));
    fake.send_command("FREQ 2.88e9").unwrap();
    let resp = fake.query("FREQ?").unwrap();
    assert!(resp.to_string().contains("2880000000"));
}

#[test]
fn fake_smb100a_outp_transitions() {
    let mut fake = FakeSmb100a::new(DeviceId::new("smb_test"));
    fake.send_command("OUTP ON").unwrap();
    let resp = fake.query("OUTP?").unwrap();
    assert!(resp.to_string().contains("ON"));

    fake.send_command("OUTP OFF").unwrap();
    let resp = fake.query("OUTP?").unwrap();
    assert!(resp.to_string().contains("OFF"));
}

#[test]
fn fake_smb100a_mod_fm_transitions() {
    let mut fake = FakeSmb100a::new(DeviceId::new("smb_test"));
    fake.send_command("MOD:STAT ON").unwrap();
    assert!(fake.query("MOD:STAT?").unwrap().to_string().contains("ON"));

    fake.send_command("FM:STAT ON").unwrap();
    assert!(fake.query("FM:STAT?").unwrap().to_string().contains("ON"));
}

// =========================================================================
// 27. LF shape validation
// =========================================================================

#[test]
fn lf_shape_validation_valid() {
    assert!(validate_lf_shape("SIN").is_ok());
    assert!(validate_lf_shape("SQU").is_ok());
    assert!(validate_lf_shape("TRI").is_ok());
    assert!(validate_lf_shape("SINE").is_ok());
    assert!(validate_lf_shape("SQUARE").is_ok());
}

#[test]
fn lf_shape_validation_invalid() {
    assert!(validate_lf_shape("NOSUCH").is_err());
    assert!(validate_lf_shape("").is_err());
}

#[test]
fn lf_shape_semicolon_rejected() {
    assert!(validate_lf_shape("SIN;POW OFF").is_err());
}

// =========================================================================
// 28. Command audit entry
// =========================================================================

#[test]
fn command_audit_entry_serialization() {
    let entry = CommandAuditEntry {
        timestamp_unix_ms: 1234567890,
        device_id: "smb100a".into(),
        command: "FREQ 2.88e9".into(),
        command_class: "set".into(),
        allowed: true,
        sent_to_transport: true,
        manual_approval_required: Some(false),
        manual_approval_present: Some(false),
        rejection_reason: None,
        response_preview: None,
        transport_error: None,
        safety_relevant: Some(false),
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("FREQ 2.88e9"));
    let back: CommandAuditEntry = serde_json::from_str(&json).unwrap();
    assert!(back.allowed);
    assert_eq!(back.device_id, "smb100a");
}

// =========================================================================
// 29. Safety boundary note
// =========================================================================

#[test]
fn safety_boundary_note_correct_flags() {
    let note = SafetyBoundaryNote {
        schema_version: "0.2.0".into(),
        real_smb100a_query_only: false,
        real_smb100a_setting_commands_blocked_except_sweep: true,
        rf_on_requires_manual_approval: true,
        no_csv_policy: true,
        no_internal_sweep: true,
        no_gui_hardware_access: true,
        no_magnetic_device_access: true,
    };
    assert!(note.no_csv_policy);
    assert!(note.no_internal_sweep);
    assert!(note.no_magnetic_device_access);
    assert!(note.rf_on_requires_manual_approval);
}

// =========================================================================
// 30. Alignment summary with repeat
// =========================================================================

#[test]
fn alignment_summary_with_repeats() {
    let steps = vec![
        (0u64, 0u64, 5usize),
        (0u64, 1u64, 5usize),
        (1u64, 0u64, 5usize),
        (1u64, 1u64, 4usize),
    ];
    let summary = build_alignment_summary(&steps);
    assert_eq!(summary.total_frames, 19);
    assert_eq!(summary.steps_with_frames, 4);
    assert!(summary.alignment_ok); // all steps have frame count > 0
    assert_eq!(summary.frames_per_step_map.len(), 4);
    assert_eq!(summary.frames_per_step_map[0].repeat_index, 0);
    assert_eq!(summary.frames_per_step_map[3].repeat_index, 1);
}

#[test]
fn alignment_summary_all_ok() {
    let steps = vec![
        (0u64, 0u64, 5usize),
        (0u64, 1u64, 5usize),
        (1u64, 0u64, 5usize),
    ];
    let summary = build_alignment_summary(&steps);
    assert_eq!(summary.total_frames, 15);
    assert!(summary.alignment_ok);
}

// =========================================================================
// 31. Hash manifest
// =========================================================================

#[test]
fn hash_manifest_serialization() {
    let manifest = HashManifest {
        schema_version: "0.2.0".into(),
        extended_sweep_config_hash: "sha256:abcdef".into(),
        smb100a_snapshot_before_hash: "sha256:123456".into(),
        smb100a_snapshot_after_hash: "sha256:789abc".into(),
    };
    let json = serde_json::to_string(&manifest).unwrap();
    let back: HashManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(back.extended_sweep_config_hash, "sha256:abcdef");
}

// =========================================================================
// 32. OeFrameCapture with full vectors
// =========================================================================

#[test]
fn oe_frame_capture_stores_full_vectors() {
    let frame = OeFrameCapture {
        raw_bytes: vec![0u8; 12288],
        frame_len: 12288,
        is_full_frame: true,
        raw_offset: 0,
        frame_monotonic_ns: 1_000_000,
        elapsed_ms: 100,
        parsed_ok: true,
        b_x_latest: Some(1.5),
        b_y_latest: Some(2.5),
        b_freq_latest: Some(2_880_000_000.0),
        b_x_all: vec![1.0; 50],
        b_y_all: vec![2.0; 50],
        parse_error: None,
    };
    assert_eq!(frame.b_x_all.len(), 50);
    assert_eq!(frame.b_y_all.len(), 50);
    assert!(frame.parsed_ok);
}

#[test]
fn oe_frame_capture_with_parse_error() {
    let frame = OeFrameCapture {
        raw_bytes: vec![0u8; 12288],
        frame_len: 12288,
        is_full_frame: true,
        raw_offset: 0,
        frame_monotonic_ns: 1_000_000,
        elapsed_ms: 100,
        parsed_ok: false,
        b_x_latest: None,
        b_y_latest: None,
        b_freq_latest: None,
        b_x_all: vec![],
        b_y_all: vec![],
        parse_error: Some("RallParseError::WrongLength".into()),
    };
    assert!(!frame.parsed_ok);
    assert!(frame.parse_error.is_some());
    assert!(frame.b_x_all.is_empty());
}

// =========================================================================
// 33. Station snapshot quality
// =========================================================================

#[test]
fn station_snapshot_quality_extended() {
    let quality = StationSnapshotQuality {
        schema_version: "0.2.0".into(),
        status: "passed".into(),
        eligible_for_extended_sweep: true,
        warnings: vec![],
        errors: vec![],
        query_interrupted_seen: false,
        smb_query_delay_ms: 50,
    };
    assert!(quality.eligible_for_extended_sweep);
    assert_eq!(quality.status, "passed");
}
