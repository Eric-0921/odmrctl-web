//! odmr-compiler — Layer 3 pure recipe compiler.
//!
//! Transforms a validated `Recipe` into:
//! - `ResolvedRecipe` (expanded sweep steps)
//! - `DryRunPlan` (human-readable preview)
//!
//! No hardware access. No fake devices. No executor. No GUI.

use odmr_recipe::{CommonHeader, DeviceAction, Recipe, RecipeStep, ResolvedRecipe, TimingConfig};
use std::path::Path;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during recipe compilation.
#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    /// Failed to load the recipe from disk.
    RecipeLoad(String),
    /// Failed to compute recipe hash.
    HashError(String),
    /// Recipe contains no sweeps to expand.
    NoSweeps,
    /// A sweep could not be expanded.
    SweepExpansion { sweep_id: String, reason: String },
    /// A sweep axis is not recognised.
    InvalidAxis { axis: String, reason: String },
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::RecipeLoad(msg) => write!(f, "recipe load failed: {msg}"),
            CompileError::HashError(msg) => write!(f, "hash error: {msg}"),
            CompileError::NoSweeps => write!(f, "recipe contains no sweeps"),
            CompileError::SweepExpansion { sweep_id, reason } => {
                write!(f, "sweep expansion failed for '{sweep_id}': {reason}")
            }
            CompileError::InvalidAxis { axis, reason } => {
                write!(f, "invalid axis '{axis}': {reason}")
            }
        }
    }
}

impl std::error::Error for CompileError {}

impl From<odmr_recipe::RecipeError> for CompileError {
    fn from(e: odmr_recipe::RecipeError) -> Self {
        CompileError::RecipeLoad(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Dry-run plan types
// ---------------------------------------------------------------------------

/// Human-readable preview of a resolved recipe.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DryRunPlan {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub resolved_recipe_id: String,
    pub summary: DryRunSummary,
    pub steps: Vec<DryRunStep>,
}

/// Summary statistics for a dry-run plan.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DryRunSummary {
    pub step_count: usize,
    pub estimated_duration_s: f64,
    pub required_devices: Vec<String>,
    pub hazard_actions: usize,
}

/// One line in the dry-run preview.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DryRunStep {
    pub step_id: String,
    pub sweep_coordinate: serde_json::Value,
    pub device_actions: Vec<String>,
    pub estimated_duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compile a `Recipe` into a `ResolvedRecipe`.
///
/// M1.2 simplification: sweep axis → device_id is inferred heuristically
/// because the recipe does not carry station device inventory.
/// For `"smb100a.rf.frequency_hz"` the inferred device is `"smb100a_01"`.
/// A future `compile_with_station` API will replace this heuristic.
pub fn compile_recipe(recipe: &Recipe) -> Result<ResolvedRecipe, CompileError> {
    if recipe.sweeps.is_empty() {
        return Err(CompileError::NoSweeps);
    }

    let recipe_hash = odmr_recipe::compute_recipe_hash(recipe)
        .map_err(|e| CompileError::HashError(e.to_string()))?;

    let mut steps: Vec<RecipeStep> = Vec::new();

    for sweep in &recipe.sweeps {
        let expanded = expand_sweep(sweep, recipe)?;
        steps.extend(expanded);
    }

    let total_estimated_ms: u64 = steps.iter().filter_map(|s| s.estimated_duration_ms).sum();

    Ok(ResolvedRecipe {
        header: CommonHeader {
            schema_version: recipe.header.schema_version.clone(),
            kind: "resolved_recipe".into(),
            id: format!("resolved_{}", recipe.header.id),
            name: recipe.header.name.clone(),
            created_by: recipe.header.created_by.clone(),
            created_at: recipe.header.created_at.clone(),
            description: recipe.header.description.clone(),
        },
        source_recipe_id: recipe.header.id.clone(),
        source_recipe_hash: recipe_hash,
        station_id: recipe.station_id.clone(),
        estimated_duration_s: total_estimated_ms as f64 / 1000.0,
        safety_report_id: "pending".into(),
        steps,
    })
}

/// Build a human-readable `DryRunPlan` from a `ResolvedRecipe`.
pub fn build_dry_run_plan(resolved: &ResolvedRecipe) -> DryRunPlan {
    let mut devices: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for step in &resolved.steps {
        for action in &step.device_actions {
            if seen.insert(action.device_id.clone()) {
                devices.push(action.device_id.clone());
            }
        }
    }

    let dry_steps: Vec<DryRunStep> = resolved
        .steps
        .iter()
        .map(|s| DryRunStep {
            step_id: s.step_id.clone(),
            sweep_coordinate: s
                .sweep_coordinates
                .clone()
                .unwrap_or(serde_json::Value::Null),
            device_actions: s
                .device_actions
                .iter()
                .map(|a| format!("{}: {}", a.device_id, a.action))
                .collect(),
            estimated_duration_ms: s.estimated_duration_ms.unwrap_or(0),
        })
        .collect();

    DryRunPlan {
        schema_version: "0.2.0".into(),
        kind: "dry_run_plan".into(),
        id: format!("dry_run_{}", resolved.header.id),
        resolved_recipe_id: resolved.header.id.clone(),
        summary: DryRunSummary {
            step_count: resolved.steps.len(),
            estimated_duration_s: resolved.estimated_duration_s,
            required_devices: devices,
            hazard_actions: 0, // M1.2 placeholder; M1.3 safety will populate
        },
        steps: dry_steps,
    }
}

/// Load a recipe from disk and compile it.
pub fn compile_recipe_file(path: &Path) -> Result<ResolvedRecipe, CompileError> {
    let recipe = odmr_recipe::load_recipe(path)?;
    compile_recipe(&recipe)
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Expand a single sweep definition into a vector of `RecipeStep`s.
fn expand_sweep(
    sweep: &odmr_recipe::SweepDefinition,
    recipe: &Recipe,
) -> Result<Vec<RecipeStep>, CompileError> {
    let step_count = ((sweep.stop - sweep.start) / sweep.step).floor() as usize + 1;
    if step_count == 0 || step_count > 1_000_000 {
        return Err(CompileError::SweepExpansion {
            sweep_id: sweep.sweep_id.clone(),
            reason: format!("invalid point count: {step_count}"),
        });
    }

    let device_id = infer_device_id(&sweep.axis)?;

    let timing = recipe.timing.clone();
    let settle_ms = timing
        .as_ref()
        .and_then(|t| t.default_settle_ms)
        .unwrap_or(0);
    let dwell_ms = timing
        .as_ref()
        .and_then(|t| t.default_dwell_ms)
        .unwrap_or(0);
    let step_duration_ms = settle_ms + dwell_ms;

    let acquisition = Some(recipe.acquisition.clone());

    let mut steps = Vec::with_capacity(step_count);
    for i in 0..step_count {
        let freq = sweep.start + i as f64 * sweep.step;
        let step_id = format!("step_{:06}", i + 1);

        let device_action = DeviceAction {
            device_id: device_id.clone(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({ "frequency_hz": freq })),
        };

        let expected_state = serde_json::json!({
            device_id.clone(): {
                "rf_frequency_hz": freq
            }
        });

        let sweep_coordinates = serde_json::json!({
            sweep.axis.clone(): freq
        });

        steps.push(RecipeStep {
            step_id,
            phase: "sweep".into(),
            device_actions: vec![device_action],
            expected_state,
            timing: Some(TimingConfig {
                default_settle_ms: Some(settle_ms),
                default_dwell_ms: Some(dwell_ms),
            }),
            acquisition: acquisition.clone(),
            sweep_coordinates: Some(sweep_coordinates),
            source_block_id: None,
            source_sweep_id: Some(sweep.sweep_id.clone()),
            point_index: Some(i),
            total_points: Some(step_count),
            estimated_duration_ms: Some(step_duration_ms),
        });
    }

    Ok(steps)
}

/// Infer a device_id from a sweep axis string.
///
/// M1.2 heuristic: the first dot-separated segment is treated as the
/// device type prefix, and we append `"_01"`.
///
/// Examples:
/// - `"smb100a.rf.frequency_hz"` → `"smb100a_01"`
fn infer_device_id(axis: &str) -> Result<String, CompileError> {
    let prefix = axis
        .split('.')
        .next()
        .ok_or_else(|| CompileError::InvalidAxis {
            axis: axis.into(),
            reason: "empty axis".into(),
        })?;
    Ok(format!("{prefix}_01"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use odmr_recipe::{AcquisitionConfig, AverageConfig, RecipeIntent, SweepDefinition};

    fn basic_odmr_recipe() -> Recipe {
        Recipe {
            header: CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "recipe".into(),
                id: "basic_odmr_mock".into(),
                name: Some("Basic ODMR Frequency Sweep (Mock)".into()),
                created_by: Some("human".into()),
                created_at: Some("2026-05-28T00:00:00Z".into()),
                description: Some("A minimal CW ODMR recipe for bootstrap validation.".into()),
            },
            station_id: "station_nv_lab_01".into(),
            intent: RecipeIntent {
                experiment_type: "cw_odmr".into(),
                description: Some(
                    "Sweep SMB100A RF frequency around 2.87 GHz and acquire OE1022D Ch-B X signal."
                        .into(),
                ),
            },
            profiles: vec![
                "smb100a_default_fm_500hz".into(),
                "oe1022d_chb_default".into(),
                "mag_xyz_default".into(),
            ],
            blocks: vec!["block_smb_fm_500hz_4mhz".into()],
            sweeps: vec![SweepDefinition {
                sweep_id: "rf_frequency_sweep".into(),
                axis: "smb100a.rf.frequency_hz".into(),
                start: 2_820_000_000.0,
                stop: 2_920_000_000.0,
                step: 500_000.0,
                order: "ascending".into(),
                dwell_ms: None,
                settle_ms: None,
            }],
            acquisition: AcquisitionConfig {
                device_id: "oe1022d_01".into(),
                channel: "B".into(),
                readout: vec![
                    "x".into(),
                    "y".into(),
                    "r".into(),
                    "theta".into(),
                    "freq".into(),
                    "noise".into(),
                ],
                pre_discard_ms: Some(300),
                window_ms: Some(500),
                average: Some(AverageConfig {
                    mode: "mean".into(),
                    repeat: 1,
                }),
            },
            timing: Some(TimingConfig {
                default_dwell_ms: Some(500),
                default_settle_ms: Some(500),
            }),
            metadata: Some(serde_json::json!({
                "sample_id": "diamond_nv_sample_01",
                "operator": "manual",
                "notes": "Initial v0.2 schema example."
            })),
        }
    }

    #[test]
    fn basic_odmr_mock_compiles_successfully() {
        let recipe = basic_odmr_recipe();
        let resolved = compile_recipe(&recipe).unwrap();
        assert_eq!(resolved.source_recipe_id, "basic_odmr_mock");
        assert_eq!(resolved.header.kind, "resolved_recipe");
    }

    #[test]
    fn resolved_recipe_has_exactly_201_steps() {
        let recipe = basic_odmr_recipe();
        let resolved = compile_recipe(&recipe).unwrap();
        assert_eq!(resolved.steps.len(), 201);
    }

    #[test]
    fn first_frequency_is_2820000000_hz() {
        let recipe = basic_odmr_recipe();
        let resolved = compile_recipe(&recipe).unwrap();
        let first = &resolved.steps[0];
        let coords = first.sweep_coordinates.as_ref().unwrap();
        assert_eq!(coords["smb100a.rf.frequency_hz"], 2_820_000_000.0);
    }

    #[test]
    fn last_frequency_is_2920000000_hz() {
        let recipe = basic_odmr_recipe();
        let resolved = compile_recipe(&recipe).unwrap();
        let last = resolved.steps.last().unwrap();
        let coords = last.sweep_coordinates.as_ref().unwrap();
        assert_eq!(coords["smb100a.rf.frequency_hz"], 2_920_000_000.0);
    }

    #[test]
    fn every_step_frequency_increases_by_500000_hz() {
        let recipe = basic_odmr_recipe();
        let resolved = compile_recipe(&recipe).unwrap();
        for window in resolved.steps.windows(2) {
            let a = window[0].sweep_coordinates.as_ref().unwrap()["smb100a.rf.frequency_hz"]
                .as_f64()
                .unwrap();
            let b = window[1].sweep_coordinates.as_ref().unwrap()["smb100a.rf.frequency_hz"]
                .as_f64()
                .unwrap();
            assert!((b - a - 500_000.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn recipe_hash_is_copied_into_resolved_recipe() {
        let recipe = basic_odmr_recipe();
        let expected_hash = odmr_recipe::compute_recipe_hash(&recipe).unwrap();
        let resolved = compile_recipe(&recipe).unwrap();
        assert_eq!(resolved.source_recipe_hash, expected_hash);
        assert_eq!(resolved.source_recipe_hash.len(), 64);
    }

    #[test]
    fn dry_run_plan_has_201_steps() {
        let recipe = basic_odmr_recipe();
        let resolved = compile_recipe(&recipe).unwrap();
        let plan = build_dry_run_plan(&resolved);
        assert_eq!(plan.summary.step_count, 201);
        assert_eq!(plan.steps.len(), 201);
    }

    #[test]
    fn dry_run_plan_lists_required_devices() {
        let recipe = basic_odmr_recipe();
        let resolved = compile_recipe(&recipe).unwrap();
        let plan = build_dry_run_plan(&resolved);
        assert!(plan.summary.required_devices.contains(&"smb100a_01".into()));
    }

    #[test]
    fn invalid_sweep_with_wrong_step_sign_fails_at_validation_not_compiler() {
        // The recipe validator (odmr-recipe) already rejects step sign mismatch.
        // The compiler assumes the recipe has passed validation.
        // We test that a recipe with zero sweeps fails at compile time.
        let mut recipe = basic_odmr_recipe();
        recipe.sweeps.clear();
        let err = compile_recipe(&recipe).unwrap_err();
        assert!(matches!(err, CompileError::NoSweeps));
    }

    #[test]
    fn compiler_does_not_require_hardware_or_fake_device() {
        // compile_recipe is pure: no I/O, no network, no device calls.
        let recipe = basic_odmr_recipe();
        let resolved = compile_recipe(&recipe).unwrap();
        let _plan = build_dry_run_plan(&resolved);
        // If we reach here without any external dependency, the test passes.
    }

    #[test]
    fn step_ids_are_zero_padded() {
        let recipe = basic_odmr_recipe();
        let resolved = compile_recipe(&recipe).unwrap();
        assert_eq!(resolved.steps[0].step_id, "step_000001");
        assert_eq!(resolved.steps[200].step_id, "step_000201");
    }

    #[test]
    fn point_index_and_total_points_are_set() {
        let recipe = basic_odmr_recipe();
        let resolved = compile_recipe(&recipe).unwrap();
        assert_eq!(resolved.steps[0].point_index, Some(0));
        assert_eq!(resolved.steps[0].total_points, Some(201));
        assert_eq!(resolved.steps[200].point_index, Some(200));
    }

    #[test]
    fn source_sweep_id_is_set() {
        let recipe = basic_odmr_recipe();
        let resolved = compile_recipe(&recipe).unwrap();
        assert_eq!(
            resolved.steps[0].source_sweep_id,
            Some("rf_frequency_sweep".into())
        );
    }

    #[test]
    fn estimated_duration_computed_from_timing() {
        let recipe = basic_odmr_recipe();
        let resolved = compile_recipe(&recipe).unwrap();
        // settle_ms=500 + dwell_ms=500 = 1000 ms per step
        assert_eq!(resolved.steps[0].estimated_duration_ms, Some(1000));
        // 201 steps * 1.0 s = 201 s
        assert!((resolved.estimated_duration_s - 201.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compile_recipe_file_loads_and_compiles() {
        // Use the real example file on disk.
        // CARGO_MANIFEST_DIR points to crates/odmr-compiler; go up two levels to workspace root.
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ws_root = manifest.parent().unwrap().parent().unwrap();
        let path = ws_root.join("examples/recipes/basic_odmr_mock.recipe.json");
        let resolved = compile_recipe_file(&path).unwrap();
        assert_eq!(resolved.steps.len(), 201);
    }
}
