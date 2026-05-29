//! Integration test that generates deterministic example outputs.
//!
//! Run with: cargo test -p odmr-compiler --test generate_examples -- --nocapture

use odmr_compiler::{build_dry_run_plan, compile_recipe_file};
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().parent().unwrap().to_path_buf()
}

#[test]
fn generate_resolved_recipe_example() {
    let recipe_path = workspace_root().join("examples/recipes/basic_odmr_mock.recipe.json");
    let resolved = compile_recipe_file(&recipe_path).unwrap();

    let out_path = workspace_root().join("examples/resolved/basic_odmr_mock.resolved_recipe.json");
    let json = serde_json::to_string_pretty(&resolved).unwrap();
    std::fs::write(&out_path, json).unwrap();

    println!("Wrote {}", out_path.display());
    assert_eq!(resolved.steps.len(), 201);
}

#[test]
fn generate_dry_run_plan_example() {
    let recipe_path = workspace_root().join("examples/recipes/basic_odmr_mock.recipe.json");
    let resolved = compile_recipe_file(&recipe_path).unwrap();
    let plan = build_dry_run_plan(&resolved);

    let out_path = workspace_root().join("examples/resolved/basic_odmr_mock.dry_run_plan.json");
    let json = serde_json::to_string_pretty(&plan).unwrap();
    std::fs::write(&out_path, json).unwrap();

    println!("Wrote {}", out_path.display());
    assert_eq!(plan.summary.step_count, 201);
}
