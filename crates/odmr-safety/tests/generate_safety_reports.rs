//! Integration tests that generate deterministic safety report examples.

use odmr_recipe::{DeviceAction, RecipeStep, ResolvedRecipe, SafetyLimit};
use odmr_safety::check_resolved_recipe;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().parent().unwrap().to_path_buf()
}

#[test]
fn generate_basic_odmr_mock_safety_report() {
    let resolved_path =
        workspace_root().join("examples/resolved/basic_odmr_mock.resolved_recipe.json");
    let contents = std::fs::read_to_string(&resolved_path).unwrap();
    let resolved: ResolvedRecipe = serde_json::from_str(&contents).unwrap();

    let limits = SafetyLimit {
        schema_version: "0.2.0".into(),
        kind: "safety_limit".into(),
        id: "safety_nv_station_default".into(),
        name: Some("NV Station Default".into()),
        max_power_dbm: 20.0,
        max_frequency_hz: 3_000_000_000.0,
        min_frequency_hz: 1_000_000.0,
        max_fm_deviation_hz: Some(10_000_000.0),
        max_magnetic_field_t: Some(0.01),
        max_mag_ramp_rate_a_per_s: Some(0.1),
    };

    let report = check_resolved_recipe(&resolved, &limits);
    assert_eq!(report.decision, odmr_safety::SafetyDecision::Allow);
    assert_eq!(report.summary.error_count, 0);

    let out_path = workspace_root().join("examples/safety/basic_odmr_mock.safety_report.json");
    let json = serde_json::to_string_pretty(&report).unwrap();
    std::fs::write(&out_path, json).unwrap();
    println!("Wrote {}", out_path.display());
}

#[test]
fn generate_invalid_over_frequency_safety_report() {
    // Build a resolved recipe with a single over-limit frequency step.
    let resolved = ResolvedRecipe {
        header: odmr_recipe::CommonHeader {
            schema_version: "0.2.0".into(),
            kind: "resolved_recipe".into(),
            id: "resolved_invalid_over_freq".into(),
            name: None,
            created_by: None,
            created_at: None,
            description: None,
        },
        source_recipe_id: "invalid_over_freq".into(),
        source_recipe_hash: "0000".into(),
        station_id: "station_nv_lab_01".into(),
        estimated_duration_s: 1.0,
        safety_report_id: "pending".into(),
        steps: vec![RecipeStep {
            step_id: "step_000001".into(),
            phase: "sweep".into(),
            device_actions: vec![DeviceAction {
                device_id: "smb100a_01".into(),
                action: "set_rf_frequency".into(),
                params: Some(serde_json::json!({ "frequency_hz": 4_000_000_000.0 })),
            }],
            expected_state: serde_json::Value::Null,
            timing: None,
            acquisition: None,
            sweep_coordinates: None,
            source_block_id: None,
            source_sweep_id: None,
            point_index: None,
            total_points: None,
            estimated_duration_ms: None,
        }],
    };

    let limits = SafetyLimit {
        schema_version: "0.2.0".into(),
        kind: "safety_limit".into(),
        id: "safety_nv_station_default".into(),
        name: Some("NV Station Default".into()),
        max_power_dbm: 20.0,
        max_frequency_hz: 3_000_000_000.0,
        min_frequency_hz: 1_000_000.0,
        max_fm_deviation_hz: Some(10_000_000.0),
        max_magnetic_field_t: Some(0.01),
        max_mag_ramp_rate_a_per_s: Some(0.1),
    };

    let report = check_resolved_recipe(&resolved, &limits);
    assert_eq!(report.decision, odmr_safety::SafetyDecision::Reject);
    assert!(report.summary.error_count > 0);

    let out_path =
        workspace_root().join("examples/safety/invalid_over_frequency.safety_report.json");
    let json = serde_json::to_string_pretty(&report).unwrap();
    std::fs::write(&out_path, json).unwrap();
    println!("Wrote {}", out_path.display());
}
