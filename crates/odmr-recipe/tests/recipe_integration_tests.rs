//! Integration tests for recipe loading, validation, and schema examples.

#![allow(deprecated)]

use odmr_recipe::{compute_recipe_hash, load_recipe, load_station, validate_recipe_safety};
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    // Cargo sets CARGO_MANIFEST_DIR to the crate under test.
    // We need to go up to the workspace root.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // tests/ are in workspace root, but CARGO_MANIFEST_DIR here points to
    // the crate being tested (odmr-recipe).  Go up two levels: crate -> crates -> workspace.
    manifest.parent().unwrap().parent().unwrap().to_path_buf()
}

#[test]
fn valid_recipe_loads_successfully() {
    let path = workspace_root().join("examples/recipes/basic_odmr_mock.recipe.json");
    let recipe = load_recipe(&path).expect("valid recipe should load");
    assert_eq!(recipe.header.id, "basic_odmr_mock");
    assert_eq!(recipe.header.kind, "recipe");
}

#[test]
fn station_mock_loads_successfully() {
    let path = workspace_root().join("examples/station.mock.json");
    let station = load_station(&path).expect("station mock should load");
    assert_eq!(station.header.id, "station_nv_lab_01");
    assert_eq!(station.devices.len(), 4);
}

#[test]
fn invalid_recipe_fails() {
    let path = workspace_root().join("examples/recipes/invalid_over_power.recipe.json");
    // The recipe itself is structurally valid JSON, so load_recipe succeeds.
    // The *safety* validation should fail because the sweep exceeds mock limits.
    let recipe = load_recipe(&path).expect("structurally valid recipe should parse");

    // Mock safety limits matching the bootstrap expectations.
    let limits = odmr_recipe::SafetyLimit {
        schema_version: "0.2.0".to_string(),
        kind: "safety_limit".to_string(),
        id: "mock_limits".to_string(),
        name: Some("Mock Safety Limits".to_string()),
        max_power_dbm: 20.0,
        max_frequency_hz: 3_000_000_000.0,
        min_frequency_hz: 1_000_000.0,
        max_fm_deviation_hz: Some(10_000_000.0),
        max_magnetic_field_t: Some(0.01),
        max_mag_ramp_rate_a_per_s: Some(0.1),
    };

    #[allow(deprecated)]
    let result = validate_recipe_safety(&recipe, &limits);
    assert!(
        result.is_err(),
        "recipe exceeding max_frequency_hz should fail safety validation"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("exceeds limit"),
        "error should mention limit: {}",
        msg
    );
}

#[test]
fn recipe_has_deterministic_hash() {
    let path = workspace_root().join("examples/recipes/basic_odmr_mock.recipe.json");
    let recipe = load_recipe(&path).unwrap();
    let h1 = compute_recipe_hash(&recipe).unwrap();
    let h2 = compute_recipe_hash(&recipe).unwrap();
    assert_eq!(h1, h2, "hash must be deterministic for identical content");
    assert!(!h1.is_empty());
}

#[test]
fn all_example_json_files_are_covered_by_tests() {
    // This test checks that all schema-compliant example JSON files
    // under examples/recipes/ and examples/station.mock.json can be parsed.
    // Other JSON files in examples/ (e.g. raw device command dumps) are
    // not required to conform to the recipe/station schema.
    let mut checked = 0usize;

    // Verify station mock
    let station_path = workspace_root().join("examples/station.mock.json");
    load_station(&station_path)
        .unwrap_or_else(|e| panic!("station mock should parse: {}: {e}", station_path.display()));
    checked += 1;

    // Verify all recipes under examples/recipes/
    let recipes_dir = workspace_root().join("examples/recipes");
    for entry in std::fs::read_dir(&recipes_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            load_recipe(&path)
                .unwrap_or_else(|e| panic!("recipe example should parse: {}: {e}", path.display()));
            checked += 1;
        }
    }

    assert!(
        checked >= 3,
        "expected at least 3 schema example files, found {}",
        checked
    );
}
