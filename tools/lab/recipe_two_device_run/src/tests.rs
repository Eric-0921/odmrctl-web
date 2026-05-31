//! Tests for M3.4 recipe-shaped two-device run.

use super::*;

// ---------------------------------------------------------------------------
// Recipe loading
// ---------------------------------------------------------------------------

#[test]
fn recipe_loads_valid_json() {
    let json = r#"{
        "schema_version": "0.2.0",
        "kind": "two_device_odmr_like_sweep_recipe",
        "id": "test_recipe",
        "devices": {
            "smb100a": { "device_id": "smb1", "mode": "real_or_fake_by_runtime" },
            "oe1022d": { "device_id": "oe1", "mode": "real_or_fake_by_runtime" },
            "magnetic": { "in_scope": false }
        },
        "rf": { "start_hz": 2.8e9, "stop_hz": 2.9e9, "points": 5, "power_dbm": -30, "max_power_dbm": -20 },
        "modulation": { "fm_source": "INT", "fm_deviation_hz": 4e6, "max_fm_deviation_hz": 5e6 },
        "acquisition": { "frames_per_step": 5, "repeat_count": 2, "inter_frame_delay_ms": 20 },
        "safety": { "require_operator_approval": true, "no_internal_sweep": true, "no_csv": true, "no_gui": true, "no_magnetic": true, "physical_response_required": false }
    }"#;
    let recipe: types::M3_4Recipe = serde_json::from_str(json).unwrap();
    assert_eq!(recipe.id, "test_recipe");
    assert_eq!(recipe.rf.points, 5);
    assert!(!recipe.devices.magnetic.in_scope);
}

#[test]
fn recipe_rejects_wrong_kind() {
    let json = r#"{
        "schema_version": "0.2.0",
        "kind": "wrong_kind",
        "id": "test",
        "devices": {
            "smb100a": { "device_id": "s", "mode": "real" },
            "oe1022d": { "device_id": "o", "mode": "real" },
            "magnetic": { "in_scope": false }
        },
        "rf": { "start_hz": 2.8e9, "stop_hz": 2.9e9, "points": 5, "power_dbm": -30, "max_power_dbm": -20 },
        "modulation": { "fm_deviation_hz": 4e6, "max_fm_deviation_hz": 5e6 },
        "acquisition": { "frames_per_step": 5, "repeat_count": 2 },
        "safety": { "require_operator_approval": true }
    }"#;
    let err = recipe::validate_recipe(&serde_json::from_str::<types::M3_4Recipe>(json).unwrap())
        .unwrap_err();
    assert!(err.contains("two_device_odmr_like_sweep_recipe"), "{}", err);
}

#[test]
fn recipe_rejects_power_above_limit() {
    let err = recipe::validate_recipe(&types::M3_4Recipe {
        schema_version: "0.2.0".into(),
        kind: "two_device_odmr_like_sweep_recipe".into(),
        id: "t".into(),
        description: None,
        devices: types::RecipeDevices {
            smb100a: types::RecipeDeviceRef {
                device_id: "s".into(),
                mode: "real".into(),
            },
            oe1022d: types::RecipeDeviceRef {
                device_id: "o".into(),
                mode: "real".into(),
            },
            magnetic: types::RecipeMagneticRef { in_scope: false },
        },
        rf: types::RecipeRfConfig {
            start_hz: 2.8e9,
            stop_hz: 2.9e9,
            points: 5,
            power_dbm: -15.0,
            max_power_dbm: -20.0,
        },
        modulation: types::RecipeModulationConfig {
            fm_source: "INT".into(),
            fm_deviation_hz: 4e6,
            max_fm_deviation_hz: 5e6,
            internal_lf: None,
        },
        acquisition: types::RecipeAcquisitionConfig {
            frames_per_step: 5,
            repeat_count: 2,
            inter_frame_delay_ms: 20,
        },
        safety: types::RecipeSafetyConfig {
            require_operator_approval: true,
            no_internal_sweep: true,
            no_csv: true,
            no_gui: true,
            no_magnetic: true,
            physical_response_required: false,
        },
    })
    .unwrap_err();
    assert!(err.contains("power_dbm"), "{}", err);
}

#[test]
fn recipe_rejects_too_many_frames_per_step() {
    let err = recipe::validate_recipe(&types::M3_4Recipe {
        schema_version: "0.2.0".into(),
        kind: "two_device_odmr_like_sweep_recipe".into(),
        id: "t".into(),
        description: None,
        devices: types::RecipeDevices {
            smb100a: types::RecipeDeviceRef {
                device_id: "s".into(),
                mode: "real".into(),
            },
            oe1022d: types::RecipeDeviceRef {
                device_id: "o".into(),
                mode: "real".into(),
            },
            magnetic: types::RecipeMagneticRef { in_scope: false },
        },
        rf: types::RecipeRfConfig {
            start_hz: 2.8e9,
            stop_hz: 2.9e9,
            points: 5,
            power_dbm: -30.0,
            max_power_dbm: -20.0,
        },
        modulation: types::RecipeModulationConfig {
            fm_source: "INT".into(),
            fm_deviation_hz: 4e6,
            max_fm_deviation_hz: 5e6,
            internal_lf: None,
        },
        acquisition: types::RecipeAcquisitionConfig {
            frames_per_step: 20,
            repeat_count: 2,
            inter_frame_delay_ms: 20,
        },
        safety: types::RecipeSafetyConfig {
            require_operator_approval: true,
            no_internal_sweep: true,
            no_csv: true,
            no_gui: true,
            no_magnetic: true,
            physical_response_required: false,
        },
    })
    .unwrap_err();
    assert!(err.contains("frames_per_step"), "{}", err);
}

#[test]
fn recipe_rejects_too_many_repeats() {
    let err = recipe::validate_recipe(&types::M3_4Recipe {
        schema_version: "0.2.0".into(),
        kind: "two_device_odmr_like_sweep_recipe".into(),
        id: "t".into(),
        description: None,
        devices: types::RecipeDevices {
            smb100a: types::RecipeDeviceRef {
                device_id: "s".into(),
                mode: "real".into(),
            },
            oe1022d: types::RecipeDeviceRef {
                device_id: "o".into(),
                mode: "real".into(),
            },
            magnetic: types::RecipeMagneticRef { in_scope: false },
        },
        rf: types::RecipeRfConfig {
            start_hz: 2.8e9,
            stop_hz: 2.9e9,
            points: 5,
            power_dbm: -30.0,
            max_power_dbm: -20.0,
        },
        modulation: types::RecipeModulationConfig {
            fm_source: "INT".into(),
            fm_deviation_hz: 4e6,
            max_fm_deviation_hz: 5e6,
            internal_lf: None,
        },
        acquisition: types::RecipeAcquisitionConfig {
            frames_per_step: 5,
            repeat_count: 10,
            inter_frame_delay_ms: 20,
        },
        safety: types::RecipeSafetyConfig {
            require_operator_approval: true,
            no_internal_sweep: true,
            no_csv: true,
            no_gui: true,
            no_magnetic: true,
            physical_response_required: false,
        },
    })
    .unwrap_err();
    assert!(err.contains("repeat_count"), "{}", err);
}

#[test]
fn recipe_rejects_magnetic_in_scope() {
    let err = recipe::validate_recipe(&types::M3_4Recipe {
        schema_version: "0.2.0".into(),
        kind: "two_device_odmr_like_sweep_recipe".into(),
        id: "t".into(),
        description: None,
        devices: types::RecipeDevices {
            smb100a: types::RecipeDeviceRef {
                device_id: "s".into(),
                mode: "real".into(),
            },
            oe1022d: types::RecipeDeviceRef {
                device_id: "o".into(),
                mode: "real".into(),
            },
            magnetic: types::RecipeMagneticRef { in_scope: true },
        },
        rf: types::RecipeRfConfig {
            start_hz: 2.8e9,
            stop_hz: 2.9e9,
            points: 5,
            power_dbm: -30.0,
            max_power_dbm: -20.0,
        },
        modulation: types::RecipeModulationConfig {
            fm_source: "INT".into(),
            fm_deviation_hz: 4e6,
            max_fm_deviation_hz: 5e6,
            internal_lf: None,
        },
        acquisition: types::RecipeAcquisitionConfig {
            frames_per_step: 5,
            repeat_count: 2,
            inter_frame_delay_ms: 20,
        },
        safety: types::RecipeSafetyConfig {
            require_operator_approval: true,
            no_internal_sweep: true,
            no_csv: true,
            no_gui: true,
            no_magnetic: true,
            physical_response_required: false,
        },
    })
    .unwrap_err();
    assert!(err.contains("magnetic"), "{}", err);
}

#[test]
fn recipe_rejects_lf_output_enabled() {
    let err = recipe::validate_recipe(&types::M3_4Recipe {
        schema_version: "0.2.0".into(),
        kind: "two_device_odmr_like_sweep_recipe".into(),
        id: "t".into(),
        description: None,
        devices: types::RecipeDevices {
            smb100a: types::RecipeDeviceRef {
                device_id: "s".into(),
                mode: "real".into(),
            },
            oe1022d: types::RecipeDeviceRef {
                device_id: "o".into(),
                mode: "real".into(),
            },
            magnetic: types::RecipeMagneticRef { in_scope: false },
        },
        rf: types::RecipeRfConfig {
            start_hz: 2.8e9,
            stop_hz: 2.9e9,
            points: 5,
            power_dbm: -30.0,
            max_power_dbm: -20.0,
        },
        modulation: types::RecipeModulationConfig {
            fm_source: "INT".into(),
            fm_deviation_hz: 4e6,
            max_fm_deviation_hz: 5e6,
            internal_lf: Some(types::RecipeInternalLf {
                enabled: true,
                frequency_hz: 500.0,
                shape: "SQU".into(),
                voltage_v: 0.137,
                lf_output_enabled: true,
            }),
        },
        acquisition: types::RecipeAcquisitionConfig {
            frames_per_step: 5,
            repeat_count: 2,
            inter_frame_delay_ms: 20,
        },
        safety: types::RecipeSafetyConfig {
            require_operator_approval: true,
            no_internal_sweep: true,
            no_csv: true,
            no_gui: true,
            no_magnetic: true,
            physical_response_required: false,
        },
    })
    .unwrap_err();
    assert!(err.contains("LF output"), "{}", err);
}

// ---------------------------------------------------------------------------
// Resolved recipe determinism
// ---------------------------------------------------------------------------

#[test]
fn resolved_recipe_is_deterministic() {
    let recipe = make_valid_recipe();
    let hash1 = recipe::recipe_hash(&recipe).unwrap();
    let hash2 = recipe::recipe_hash(&recipe).unwrap();
    assert_eq!(hash1, hash2);

    let r1 = dry_run::build_resolved_recipe(&recipe, &hash1);
    let r2 = dry_run::build_resolved_recipe(&recipe, &hash1);
    assert_eq!(r1.total_steps, r2.total_steps);
    assert_eq!(r1.steps.len(), r2.steps.len());
    for (a, b) in r1.steps.iter().zip(r2.steps.iter()) {
        assert_eq!(a.frequency_hz, b.frequency_hz);
        assert_eq!(a.step_id, b.step_id);
        assert_eq!(a.repeat_index, b.repeat_index);
    }
}

#[test]
fn dry_run_plan_is_deterministic() {
    let recipe = make_valid_recipe();
    let h = recipe::recipe_hash(&recipe).unwrap();
    let r = dry_run::build_resolved_recipe(&recipe, &h);
    let d1 = dry_run::build_dry_run_plan(&r);
    let d2 = dry_run::build_dry_run_plan(&r);
    assert_eq!(d1.summary.step_count, d2.summary.step_count);
    assert_eq!(d1.summary.total_frames, d2.summary.total_frames);
}

#[test]
fn resolved_recipe_step_count_correct() {
    let recipe = make_valid_recipe(); // 5 points, 2 repeats
    let h = recipe::recipe_hash(&recipe).unwrap();
    let r = dry_run::build_resolved_recipe(&recipe, &h);
    assert_eq!(r.total_steps, 10); // 5 × 2
    assert_eq!(r.steps.len(), 10);
    assert_eq!(r.steps[0].repeat_index, 0);
    assert_eq!(r.steps[5].repeat_index, 1);
}

// ---------------------------------------------------------------------------
// Safety report
// ---------------------------------------------------------------------------

#[test]
fn safety_rejects_no_operator_approval() {
    let recipe = make_valid_recipe();
    let report = safety::check_recipe_safety(&recipe, "resolved_test", false);
    let approval_finding = report
        .findings
        .iter()
        .find(|f| f.check == "operator_approval")
        .unwrap();
    assert!(!approval_finding.passed);
    assert_eq!(report.decision, types::SafetyDecision::Reject);
}

#[test]
fn safety_allows_with_operator_approval() {
    let recipe = make_valid_recipe();
    let report = safety::check_recipe_safety(&recipe, "resolved_test", true);
    let approval_finding = report
        .findings
        .iter()
        .find(|f| f.check == "operator_approval")
        .unwrap();
    assert!(approval_finding.passed);
}

#[test]
fn safety_rejects_too_many_points() {
    let mut recipe = make_valid_recipe();
    recipe.rf.points = 22;
    let report = safety::check_recipe_safety(&recipe, "r", true);
    let finding = report
        .findings
        .iter()
        .find(|f| f.check == "rf_points_limit")
        .unwrap();
    assert!(!finding.passed);
}

#[test]
fn safety_rejects_power_above_hard_limit() {
    let mut recipe = make_valid_recipe();
    recipe.rf.max_power_dbm = -5.0;
    let report = safety::check_recipe_safety(&recipe, "r", true);
    let finding = report
        .findings
        .iter()
        .find(|f| f.check == "rf_power_limit")
        .unwrap();
    assert!(!finding.passed);
}

#[test]
fn safety_rejects_magnetic_in_scope() {
    let mut recipe = make_valid_recipe();
    recipe.devices.magnetic.in_scope = true;
    let report = safety::check_recipe_safety(&recipe, "r", true);
    let finding = report
        .findings
        .iter()
        .find(|f| f.check == "no_magnetic")
        .unwrap();
    assert!(!finding.passed);
    assert_eq!(report.decision, types::SafetyDecision::Reject);
}

#[test]
fn safety_check_count_matches() {
    let recipe = make_valid_recipe();
    let report = safety::check_recipe_safety(&recipe, "r", true);
    assert!(report.findings.len() >= 10);
    assert!(report.summary.total_checks >= 10);
}

// ---------------------------------------------------------------------------
// Frequency generation
// ---------------------------------------------------------------------------

#[test]
fn frequency_generation_evenly_spaced() {
    let recipe = make_valid_recipe(); // 5 points
    let freqs = recipe::generate_frequencies(&recipe);
    assert_eq!(freqs.len(), 5);
    assert!((freqs[0] - 2.8e9).abs() < 1.0);
    assert!((freqs[4] - 2.9e9).abs() < 1.0);
}

#[test]
fn frequency_generation_two_points() {
    let mut recipe = make_valid_recipe();
    recipe.rf.points = 2;
    recipe.rf.start_hz = 1e9;
    recipe.rf.stop_hz = 2e9;
    let freqs = recipe::generate_frequencies(&recipe);
    assert_eq!(freqs.len(), 2);
    assert!((freqs[0] - 1e9).abs() < 1.0);
    assert!((freqs[1] - 2e9).abs() < 1.0);
}

// ---------------------------------------------------------------------------
// Fake SMB100A
// ---------------------------------------------------------------------------

#[test]
fn fake_smb_handles_freq_set_query() {
    let mut smb = harness::create_fake_smb100a();
    let (resp, _) = harness::fake_smb_set(&mut smb, "FREQ 2.88e9", 1);
    assert!(resp.contains("ACK") || !resp.is_empty());
    let (resp2, _) = harness::fake_smb_query(&mut smb, "FREQ?", 2);
    assert!(!resp2.is_empty());
}

#[test]
fn fake_smb_handles_outp_on_off() {
    let mut smb = harness::create_fake_smb100a();
    let (_, _) = harness::fake_smb_set(&mut smb, "OUTP ON", 1);
    let (resp, _) = harness::fake_smb_query(&mut smb, "OUTP?", 2);
    assert!(resp.to_ascii_uppercase().contains("ON") || resp.contains('1'));

    let (_, _) = harness::fake_smb_set(&mut smb, "OUTP OFF", 3);
    let (resp, _) = harness::fake_smb_query(&mut smb, "OUTP?", 4);
    assert!(resp.to_ascii_uppercase().contains("OFF") || resp.contains('0'));
}

#[test]
fn fake_smb_handles_fm_config() {
    let mut smb = harness::create_fake_smb100a();
    harness::fake_smb_set(&mut smb, "FM:SOUR INT", 1);
    harness::fake_smb_set(&mut smb, "FM:DEV 4e6", 2);
    harness::fake_smb_set(&mut smb, "FM:STAT ON", 3);

    let (src, _) = harness::fake_smb_query(&mut smb, "FM:SOUR?", 4);
    assert!(src.to_ascii_uppercase().contains("INT"));
    let (stat, _) = harness::fake_smb_query(&mut smb, "FM:STAT?", 5);
    assert!(stat.to_ascii_uppercase().contains("ON") || stat.contains('1'));
}

// ---------------------------------------------------------------------------
// Deterministic frame generation
// ---------------------------------------------------------------------------

#[test]
fn deterministic_frame_is_12288_bytes() {
    let frame = harness::generate_deterministic_rall_frame(0, 0, 0, 42);
    assert_eq!(frame.len(), 12288);
}

#[test]
fn deterministic_frame_differs_by_index() {
    let f1 = harness::generate_deterministic_rall_frame(0, 0, 0, 42);
    let f2 = harness::generate_deterministic_rall_frame(0, 1, 0, 42);
    // Different step index should produce different B-channel data
    let diff_at_start = f1[0..16] != f2[0..16];
    assert!(
        diff_at_start,
        "frames with different step_index should differ"
    );
}

#[test]
fn deterministic_frame_parseable() {
    let frame = harness::generate_deterministic_rall_frame(0, 0, 0, 42);
    let result = odmr_oe1022d::parse_rall_frame(&frame);
    assert!(result.is_ok(), "deterministic frame should be parseable");
}

// ---------------------------------------------------------------------------
// Command plan
// ---------------------------------------------------------------------------

#[test]
fn command_plan_generates_expected_structure() {
    let recipe = make_valid_recipe();
    let h = recipe::recipe_hash(&recipe).unwrap();
    let resolved = dry_run::build_resolved_recipe(&recipe, &h);
    let (entries, summary) = command_plan::generate_command_plan(&recipe, &resolved);
    assert!(
        entries.len() > 50,
        "plan should have many entries, got {}",
        entries.len()
    );
    assert!(summary.total_commands > 50);
    assert!(summary.set_commands > 0);
    assert!(summary.query_commands > 0);
}

// ---------------------------------------------------------------------------
// Command audit comparison
// ---------------------------------------------------------------------------

#[test]
fn comparison_detects_missing_outp_off() {
    use types::*;
    let expected = vec![CommandPlanEntry {
        sequence_index: 0,
        step_id: "shutdown".into(),
        repeat_index: 0,
        device_id: "smb100a".into(),
        command: "OUTP OFF".into(),
        command_class: "shutdown".into(),
        safety_relevant: true,
    }];
    let cmp = command_audit_compare::compare_plan_vs_audit(&expected, &[]);
    assert!(!cmp.passed);
    assert!(!cmp.missing_expected_commands.is_empty());
}

#[test]
fn comparison_detects_forbidden_command() {
    use types::*;
    let expected = vec![CommandPlanEntry {
        sequence_index: 0,
        step_id: "s1".into(),
        repeat_index: 0,
        device_id: "smb100a".into(),
        command: "OUTP OFF".into(),
        command_class: "shutdown".into(),
        safety_relevant: true,
    }];
    let actual = vec![M3_4CommandAuditEntry {
        timestamp_unix_ms: 0,
        device_id: "smb100a".into(),
        command: "SWE:MODE AUTO".into(),
        command_class: "set".into(),
        allowed: false,
        sent_to_transport: true,
        rejection_reason: None,
        response_preview: None,
        transport_error: None,
        safety_relevant: false,
    }];
    let cmp = command_audit_compare::compare_plan_vs_audit(&expected, &actual);
    assert!(!cmp.forbidden_actual_commands.is_empty());
}

#[test]
fn comparison_passes_for_matching_commands() {
    use types::*;
    let expected = vec![CommandPlanEntry {
        sequence_index: 0,
        step_id: "shutdown".into(),
        repeat_index: 0,
        device_id: "smb100a".into(),
        command: "OUTP OFF".into(),
        command_class: "shutdown".into(),
        safety_relevant: true,
    }];
    let actual = vec![M3_4CommandAuditEntry {
        timestamp_unix_ms: 1,
        device_id: "smb100a".into(),
        command: "OUTP OFF".into(),
        command_class: "shutdown".into(),
        allowed: true,
        sent_to_transport: true,
        rejection_reason: None,
        response_preview: Some("0".into()),
        transport_error: None,
        safety_relevant: true,
    }];
    let cmp = command_audit_compare::compare_plan_vs_audit(&expected, &actual);
    assert!(
        cmp.passed,
        "expected pass, got missing={:?} forbidden={:?}",
        cmp.missing_expected_commands, cmp.forbidden_actual_commands
    );
}

// ---------------------------------------------------------------------------
// Types serialization roundtrips
// ---------------------------------------------------------------------------

#[test]
fn run_result_serializes() {
    let result = types::M3_4RunResult {
        schema_version: "0.2.0".into(),
        kind: "two_device_run_result".into(),
        run_id: "test_run".into(),
        mode: "harness-fake".into(),
        recipe_id: "test_recipe".into(),
        resolved_recipe_id: "resolved_test".into(),
        passed: true,
        steps_completed: 10,
        total_steps: 10,
        frames_requested: 50,
        frames_captured: 50,
        frames_parsed: 49,
        frames_parse_failed: 1,
        parse_failure_rate: 0.02,
        final_rf_off: true,
        final_mod_off: true,
        final_fm_off: true,
        final_syst_err_clean: true,
        command_audit_comparison_passed: true,
        no_forbidden_commands_sent: true,
        emergency_shutdown_triggered: false,
        alignment_count: 50,
        notes: vec![],
    };
    let json = serde_json::to_string(&result).unwrap();
    let back: types::M3_4RunResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.run_id, "test_run");
    assert!(back.passed);
}

#[test]
fn safety_report_serializes() {
    let report = types::M3_4SafetyReport {
        schema_version: "0.2.0".into(),
        kind: "safety_report".into(),
        id: "sr1".into(),
        resolved_recipe_id: "rr1".into(),
        decision: types::SafetyDecision::Allow,
        summary: types::M3_4SafetySummary {
            total_checks: 5,
            passed: 5,
            warnings: 0,
            errors: 0,
        },
        findings: vec![],
    };
    let json = serde_json::to_string(&report).unwrap();
    let back: types::M3_4SafetyReport = serde_json::from_str(&json).unwrap();
    assert_eq!(back.decision, types::SafetyDecision::Allow);
}

#[test]
fn command_audit_comparison_serializes() {
    let cmp = types::CommandAuditComparison {
        schema_version: "0.2.0".into(),
        kind: "command_audit_comparison".into(),
        passed: true,
        expected_command_count: 10,
        actual_command_count: 10,
        missing_expected_commands: vec![],
        unexpected_actual_commands: vec![],
        forbidden_actual_commands: vec![],
        allowed_extra_queries: vec![],
        notes: vec![],
    };
    let json = serde_json::to_string(&cmp).unwrap();
    let back: types::CommandAuditComparison = serde_json::from_str(&json).unwrap();
    assert!(back.passed);
}

#[test]
fn command_plan_entry_serializes() {
    let entry = types::CommandPlanEntry {
        sequence_index: 0,
        step_id: "s1".into(),
        repeat_index: 0,
        device_id: "smb100a".into(),
        command: "OUTP OFF".into(),
        command_class: "set".into(),
        safety_relevant: true,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("OUTP OFF"));
}

#[test]
fn run_stability_summary_serializes() {
    let s = types::RunStabilitySummary {
        schema_version: "0.2.0".into(),
        kind: "run_stability_summary".into(),
        run_id: "test".into(),
        frames_requested: 100,
        frames_captured: 98,
        frames_parsed: 95,
        frames_parse_failed: 3,
        parse_failure_rate: 0.03,
        steps_total: 10,
        steps_passed: 10,
        final_rf_off: true,
        final_mod_off: true,
        final_fm_off: true,
        final_syst_err_clean: true,
        emergency_shutdown_triggered: false,
        no_forbidden_commands_sent: true,
    };
    let json = serde_json::to_string(&s).unwrap();
    let back: types::RunStabilitySummary = serde_json::from_str(&json).unwrap();
    assert!(back.final_rf_off);
}

#[test]
fn step_summary_entry_serializes() {
    let entry = types::RfStepSummaryEntry {
        step_id: "repeat_0_rf_step_000".into(),
        repeat_index: 0,
        frequency_hz: 2.88e9,
        frequency_verified_hz: Some(2.88e9),
        rf_output_on: true,
        frames_requested: 5,
        frames_captured: 5,
        frames_parsed: 5,
        frames_parse_failed: 0,
        step_passed: true,
        b_x_mean: Some(0.5),
        b_x_std: Some(0.1),
        b_y_mean: Some(-0.3),
        b_y_std: Some(0.05),
        duration_ms: 4200,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("repeat_0_rf_step_000"));
}

#[test]
fn magnetic_not_in_scope_serializes() {
    let m = types::MagneticNotInScope {
        schema_version: "0.2.0".into(),
        kind: "magnetic_not_in_scope".into(),
        message: "No magnetic".into(),
        run_id: "test".into(),
    };
    let json = serde_json::to_string(&m).unwrap();
    assert!(json.contains("magnetic_not_in_scope"));
    assert!(!json.to_ascii_lowercase().contains("maynuo"));
}

// ---------------------------------------------------------------------------
// Parse failure injection
// ---------------------------------------------------------------------------

#[test]
fn parse_failure_injection_works() {
    let frames: Vec<Vec<u8>> = (0..10)
        .map(|_| harness::generate_deterministic_rall_frame(0, 0, 0, 42))
        .collect();
    let (good, failed) = harness::inject_parse_failures(&frames, 0.2);
    assert!(good.len() >= 7); // at least 70% good
    assert!(!failed.is_empty() || good.len() == 10); // some may fail, or all pass if rate is low
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_valid_recipe() -> types::M3_4Recipe {
    types::M3_4Recipe {
        schema_version: "0.2.0".into(),
        kind: "two_device_odmr_like_sweep_recipe".into(),
        id: "test_recipe".into(),
        description: None,
        devices: types::RecipeDevices {
            smb100a: types::RecipeDeviceRef {
                device_id: "smb1".into(),
                mode: "real_or_fake_by_runtime".into(),
            },
            oe1022d: types::RecipeDeviceRef {
                device_id: "oe1".into(),
                mode: "real_or_fake_by_runtime".into(),
            },
            magnetic: types::RecipeMagneticRef { in_scope: false },
        },
        rf: types::RecipeRfConfig {
            start_hz: 2.8e9,
            stop_hz: 2.9e9,
            points: 5,
            power_dbm: -30.0,
            max_power_dbm: -20.0,
        },
        modulation: types::RecipeModulationConfig {
            fm_source: "INT".into(),
            fm_deviation_hz: 4e6,
            max_fm_deviation_hz: 5e6,
            internal_lf: None,
        },
        acquisition: types::RecipeAcquisitionConfig {
            frames_per_step: 5,
            repeat_count: 2,
            inter_frame_delay_ms: 20,
        },
        safety: types::RecipeSafetyConfig {
            require_operator_approval: true,
            no_internal_sweep: true,
            no_csv: true,
            no_gui: true,
            no_magnetic: true,
            physical_response_required: false,
        },
    }
}
