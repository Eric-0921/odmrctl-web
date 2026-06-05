//! System scan recipe compiler.
//!
//! Expands a `SystemScanRecipe` into a `ResolvedSystemScan` (concrete steps)
//! and a `SystemDryRunPlan` (human-readable preview).

use odmr_recipe::{CommonHeader, SystemScanRecipe, SystemSweepDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during system-scan compilation.
#[derive(Debug, Clone, PartialEq)]
pub enum SystemScanCompileError {
    Expansion(String),
    MissingSweep(String),
    InvalidSweepOrder(String),
}

impl std::fmt::Display for SystemScanCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemScanCompileError::Expansion(msg) => write!(f, "expansion error: {msg}"),
            SystemScanCompileError::MissingSweep(id) => write!(f, "missing sweep: {id}"),
            SystemScanCompileError::InvalidSweepOrder(msg) => {
                write!(f, "invalid sweep_order: {msg}")
            }
        }
    }
}

impl std::error::Error for SystemScanCompileError {}

// ---------------------------------------------------------------------------
// Resolved types
// ---------------------------------------------------------------------------

/// A resolved system-scan recipe: fully expanded execution plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedSystemScan {
    #[serde(flatten)]
    pub header: CommonHeader,
    pub source_recipe_id: String,
    pub source_recipe_hash: String,
    pub station_id: String,
    pub safety_report_id: String,
    pub estimated_duration_s: f64,
    pub step_count: usize,
    pub steps: Vec<ResolvedSystemStep>,
}

/// A single step in a resolved system-scan recipe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedSystemStep {
    pub step_id: String,
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub point_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sweep_coordinates: Option<serde_json::Value>,
    pub target_device_state: serde_json::Value,
    pub acquisition: AcquisitionStep,
    pub traceability: Traceability,
}

/// Acquisition configuration for a resolved step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcquisitionStep {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_after: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_discard_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frames_expected: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach_device_state_snapshot: Option<bool>,
}

/// Traceability metadata for a resolved step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Traceability {
    pub source_recipe_id: String,
    pub required_state_snapshot: bool,
    pub required_step_hash: bool,
}

// ---------------------------------------------------------------------------
// Dry-run types
// ---------------------------------------------------------------------------

/// Human-readable dry-run plan for a system-scan recipe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemDryRunPlan {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub resolved_recipe_id: String,
    pub summary: SystemDryRunSummary,
    pub phases: Vec<DryRunPhase>,
    pub operator_approval_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemDryRunSummary {
    pub step_count: usize,
    pub total_points: usize,
    pub expected_frames: u64,
    pub estimated_duration_s: f64,
    pub required_devices: Vec<String>,
    pub hazard_actions: usize,
    pub outer_sweep: String,
    pub inner_sweep: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DryRunPhase {
    pub phase: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hazard_note: Option<String>,
    pub steps: Vec<DryRunPhaseStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DryRunPhaseStep {
    pub step_id: String,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Expander
// ---------------------------------------------------------------------------

/// Expand a `SystemScanRecipe` into a `ResolvedSystemScan`.
pub fn expand_system_scan_recipe(
    recipe: &SystemScanRecipe,
) -> Result<ResolvedSystemScan, SystemScanCompileError> {
    let recipe_hash = compute_system_scan_hash(recipe);

    let mut steps: Vec<ResolvedSystemStep> = Vec::new();

    // --- Setup phase ---
    steps.push(make_setup_step("step_setup_000", "preflight", recipe));
    steps.push(make_setup_step("step_setup_001", "rf_configure", recipe));
    steps.push(make_setup_step(
        "step_setup_002",
        "magnetic_baseline",
        recipe,
    ));

    // --- Measure phase: cartesian product of sweeps ---
    let measure_steps = expand_sweep_cartesian_product(recipe)?;
    steps.extend(measure_steps);

    // --- Cleanup phase ---
    steps.push(make_cleanup_step(
        "step_cleanup_000",
        "rf_output_off",
        recipe,
    ));
    steps.push(make_cleanup_step(
        "step_cleanup_001",
        "magnetic_zero",
        recipe,
    ));
    steps.push(make_cleanup_step(
        "step_cleanup_002",
        "magnetic_local",
        recipe,
    ));

    let estimated_duration_s = estimate_duration(recipe, &steps);

    Ok(ResolvedSystemScan {
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
        station_id: recipe.station_ref.clone(),
        safety_report_id: format!("safety_{}", recipe.header.id),
        estimated_duration_s,
        step_count: steps.len(),
        steps,
    })
}

// ---------------------------------------------------------------------------
// Dry-run builder
// ---------------------------------------------------------------------------

/// Build a human-readable dry-run plan from a resolved system scan.
pub fn build_system_scan_dry_run(
    recipe: &SystemScanRecipe,
    resolved: &ResolvedSystemScan,
) -> SystemDryRunPlan {
    let total_points = resolved
        .steps
        .iter()
        .filter(|s| s.phase == "measure")
        .count();

    let expected_frames = if recipe.acquisition_policy.enabled {
        total_points as u64 * recipe.acquisition_policy.frames_per_point
    } else {
        0
    };

    let outer_sweep = recipe.sweep_order.first().cloned().unwrap_or_default();
    let inner_sweep = recipe.sweep_order.get(1).cloned().unwrap_or_default();

    let devices: Vec<String> = recipe
        .devices
        .values()
        .filter(|d| d.required)
        .map(|d| d.device_id.clone())
        .collect();

    let hazard_actions = resolved
        .steps
        .iter()
        .filter(|s| s.phase == "measure")
        .count();

    SystemDryRunPlan {
        schema_version: "0.2.0".into(),
        kind: "dry_run_plan".into(),
        id: format!("dry_run_{}", resolved.header.id),
        resolved_recipe_id: resolved.header.id.clone(),
        summary: SystemDryRunSummary {
            step_count: resolved.steps.len(),
            total_points,
            expected_frames,
            estimated_duration_s: resolved.estimated_duration_s,
            required_devices: devices,
            hazard_actions,
            outer_sweep,
            inner_sweep,
        },
        phases: vec![
            DryRunPhase {
                phase: "setup".into(),
                description: "Device preflight, RF configuration, magnetic baseline zero".into(),
                hazard_note: None,
                steps: vec![
                    DryRunPhaseStep {
                        step_id: "step_setup_000".into(),
                        description: "Station preflight and device lock acquisition".into(),
                    },
                    DryRunPhaseStep {
                        step_id: "step_setup_001".into(),
                        description: "Configure SMB100A RF settings".into(),
                    },
                    DryRunPhaseStep {
                        step_id: "step_setup_002".into(),
                        description: "Set magnetic baseline to B=[0,0,0] nT, settle 500ms".into(),
                    },
                ],
            },
            DryRunPhase {
                phase: "measure".into(),
                description: format!(
                    "Nested sweep: {} magnetic points x {} RF frequencies = {} acquisition points",
                    recipe.sweeps.iter().find(|s| s.device == "magnetic").map(|s| s.point_count()).unwrap_or(0),
                    recipe.sweeps.iter().find(|s| s.device == "smb100a").map(|s| s.point_count()).unwrap_or(0),
                    total_points,
                ),
                hazard_note: Some("RF output ON at each measure point".into()),
                steps: vec![DryRunPhaseStep {
                    step_id: "pt_000..pt_008".into(),
                    description: "For each point: set magnetic vector, set RF frequency, wait settle, RF ON, acquire OE frames".into(),
                }],
            },
            DryRunPhase {
                phase: "cleanup".into(),
                description: "Safe shutdown: RF OFF, magnetic zero, magnetic local mode".into(),
                hazard_note: None,
                steps: vec![
                    DryRunPhaseStep {
                        step_id: "step_cleanup_000".into(),
                        description: "SMB100A RF output OFF".into(),
                    },
                    DryRunPhaseStep {
                        step_id: "step_cleanup_001".into(),
                        description: "Magnetic field ramp to zero, settle 500ms".into(),
                    },
                    DryRunPhaseStep {
                        step_id: "step_cleanup_002".into(),
                        description: "Magnetic controller to LOCAL mode".into(),
                    },
                ],
            },
        ],
        operator_approval_required: recipe.safety.require_operator_approval,
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn compute_system_scan_hash(recipe: &SystemScanRecipe) -> String {
    odmr_recipe::compute_system_scan_hash(recipe)
}

fn make_setup_step(
    step_id: &str,
    setup_kind: &str,
    recipe: &SystemScanRecipe,
) -> ResolvedSystemStep {
    let target_state = match setup_kind {
        "rf_configure" => {
            let mut state = serde_json::Map::new();
            // Include full SMB100A config with output disabled for setup.
            if let Some(smb_value) = recipe.fixed_params.get("smb100a") {
                let mut smb = smb_value.clone();
                if let Some(obj) = smb.as_object_mut() {
                    obj.insert("rf_output_required".into(), serde_json::json!(false));
                }
                state.insert("smb100a".into(), smb);
            }
            // Include full OE1022D config.
            if let Some(oe) = recipe.fixed_params.get("oe1022d") {
                state.insert("oe1022d".into(), oe.clone());
            }
            serde_json::Value::Object(state)
        }
        "magnetic_baseline" => {
            let mut state = serde_json::Map::new();
            let mag = build_magnetic_target_state(recipe, &[0.0, 0.0, 0.0]);
            state.insert("magnetic".into(), mag);
            serde_json::Value::Object(state)
        }
        _ => serde_json::Value::Object(serde_json::Map::new()),
    };

    ResolvedSystemStep {
        step_id: step_id.into(),
        phase: "setup".into(),
        point_index: None,
        sweep_coordinates: None,
        target_device_state: target_state,
        acquisition: AcquisitionStep {
            enabled: false,
            device: None,
            start_after: None,
            pre_discard_ms: None,
            frames_expected: None,
            attach_device_state_snapshot: None,
        },
        traceability: Traceability {
            source_recipe_id: recipe.header.id.clone(),
            required_state_snapshot: false,
            required_step_hash: true,
        },
    }
}

fn make_cleanup_step(
    step_id: &str,
    cleanup_kind: &str,
    recipe: &SystemScanRecipe,
) -> ResolvedSystemStep {
    let target_state = match cleanup_kind {
        "rf_output_off" => {
            let mut state = serde_json::Map::new();
            if let Some(smb_value) = recipe.fixed_params.get("smb100a") {
                let mut smb = smb_value.clone();
                if let Some(obj) = smb.as_object_mut() {
                    obj.insert("rf_output_required".into(), serde_json::json!(false));
                }
                state.insert("smb100a".into(), smb);
            }
            serde_json::Value::Object(state)
        }
        "magnetic_zero" => {
            let mut state = serde_json::Map::new();
            let mag = build_magnetic_target_state(recipe, &[0.0, 0.0, 0.0]);
            state.insert("magnetic".into(), mag);
            serde_json::Value::Object(state)
        }
        "magnetic_local" => serde_json::json!({ "magnetic": { "mode": "local" } }),
        _ => serde_json::Value::Object(serde_json::Map::new()),
    };

    ResolvedSystemStep {
        step_id: step_id.into(),
        phase: "cleanup".into(),
        point_index: None,
        sweep_coordinates: None,
        target_device_state: target_state,
        acquisition: AcquisitionStep {
            enabled: false,
            device: None,
            start_after: None,
            pre_discard_ms: None,
            frames_expected: None,
            attach_device_state_snapshot: None,
        },
        traceability: Traceability {
            source_recipe_id: recipe.header.id.clone(),
            required_state_snapshot: false,
            required_step_hash: true,
        },
    }
}

// ---------------------------------------------------------------------------
// Coil matrix inversion & predicted current
// ---------------------------------------------------------------------------

/// Invert a 3×3 matrix. Returns `None` if the matrix is singular (det < 1e-12).
fn invert_3x3(m: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

    if det.abs() < 1e-12 {
        return None;
    }

    let inv_det = 1.0 / det;

    Some([
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det,
            -(m[0][1] * m[2][2] - m[0][2] * m[2][1]) * inv_det,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det,
        ],
        [
            -(m[1][0] * m[2][2] - m[1][2] * m[2][0]) * inv_det,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det,
            -(m[0][0] * m[1][2] - m[0][2] * m[1][0]) * inv_det,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det,
            -(m[0][0] * m[2][1] - m[0][1] * m[2][0]) * inv_det,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det,
        ],
    ])
}

/// Compute predicted current (A) from B_target (nT), coil matrix (T/A), and zero offsets (A).
fn compute_predicted_current(
    b_target_nt: [f64; 3],
    coil_matrix: [[f64; 3]; 3],
    zero_offsets_a: [f64; 3],
) -> Option<[f64; 3]> {
    let inv = invert_3x3(coil_matrix)?;
    let b_t = [
        b_target_nt[0] / 1e9,
        b_target_nt[1] / 1e9,
        b_target_nt[2] / 1e9,
    ];

    Some([
        inv[0][0] * b_t[0] + inv[0][1] * b_t[1] + inv[0][2] * b_t[2] + zero_offsets_a[0],
        inv[1][0] * b_t[0] + inv[1][1] * b_t[1] + inv[1][2] * b_t[2] + zero_offsets_a[1],
        inv[2][0] * b_t[0] + inv[2][1] * b_t[1] + inv[2][2] * b_t[2] + zero_offsets_a[2],
    ])
}

/// Build magnetic target state JSON for a given B_target vector.
fn build_magnetic_target_state(
    recipe: &SystemScanRecipe,
    b_target_nt: &[f64; 3],
) -> serde_json::Value {
    let mag_params = recipe.fixed_params.get("magnetic");

    // Extract coil matrix and zero offsets.
    let coil_matrix = mag_params
        .and_then(|v| v.get("coil_matrix"))
        .and_then(|v| v.get("matrix"))
        .and_then(|v| v.as_array())
        .map(|rows| {
            let mut m = [[0.0; 3]; 3];
            for (i, row) in rows.iter().enumerate().take(3) {
                if let Some(arr) = row.as_array() {
                    for (j, val) in arr.iter().enumerate().take(3) {
                        m[i][j] = val.as_f64().unwrap_or(0.0);
                    }
                }
            }
            m
        })
        .unwrap_or([[0.0; 3]; 3]);

    let zero_offsets = mag_params
        .and_then(|v| v.get("zero_offsets_a"))
        .map(|v| {
            [
                v.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0),
                v.get("y").and_then(|y| y.as_f64()).unwrap_or(0.0),
                v.get("z").and_then(|z| z.as_f64()).unwrap_or(0.0),
            ]
        })
        .unwrap_or([0.0; 3]);

    let predicted_current = compute_predicted_current(*b_target_nt, coil_matrix, zero_offsets);

    let settle_ms = mag_params
        .and_then(|v| v.get("default_settle_ms"))
        .and_then(|v| v.as_u64())
        .unwrap_or(500);

    let readback_required = mag_params
        .and_then(|v| v.get("readback_required"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let zero_lock_required = mag_params
        .and_then(|v| v.get("zero_lock_required"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut state = serde_json::Map::new();
    state.insert("b_target_nt".into(), serde_json::json!(b_target_nt));
    if let Some(current) = predicted_current {
        state.insert("predicted_current_a".into(), serde_json::json!(current));
    }
    state.insert(
        "zero_offsets_a".into(),
        serde_json::json!({
            "x": zero_offsets[0],
            "y": zero_offsets[1],
            "z": zero_offsets[2],
        }),
    );
    state.insert("settle_ms".into(), serde_json::json!(settle_ms));
    state.insert(
        "readback_required".into(),
        serde_json::json!(readback_required),
    );
    state.insert(
        "zero_lock_required".into(),
        serde_json::json!(zero_lock_required),
    );
    serde_json::Value::Object(state)
}

/// Expand sweeps into a cartesian product of measure steps.
fn expand_sweep_cartesian_product(
    recipe: &SystemScanRecipe,
) -> Result<Vec<ResolvedSystemStep>, SystemScanCompileError> {
    if recipe.sweep_order.is_empty() {
        return Ok(Vec::new());
    }

    // Build a lookup from sweep_id to sweep definition.
    let sweep_map: HashMap<&str, &SystemSweepDefinition> = recipe
        .sweeps
        .iter()
        .map(|s| (s.sweep_id.as_str(), s))
        .collect();

    // Collect varying axes for each sweep in sweep_order.
    type SweepAxisList = Vec<(String, Vec<f64>)>;
    let mut sweep_axes: Vec<(&str, SweepAxisList)> = Vec::new();
    for sweep_id in &recipe.sweep_order {
        let sweep = sweep_map
            .get(sweep_id.as_str())
            .ok_or_else(|| SystemScanCompileError::MissingSweep(sweep_id.clone()))?;
        let axes = sweep.varying_axes();
        if axes.is_empty() {
            return Err(SystemScanCompileError::Expansion(format!(
                "sweep '{}' has no varying axes",
                sweep_id
            )));
        }
        sweep_axes.push((sweep_id.as_str(), axes));
    }

    // Generate cartesian product.
    let mut all_combinations: Vec<Vec<(String, String, f64)>> = vec![vec![]];
    for (sweep_id, axes) in &sweep_axes {
        let mut next_combinations = Vec::new();
        for combo in &all_combinations {
            for (axis_name, values) in axes {
                for &value in values {
                    let mut new_combo = combo.clone();
                    // For coordinate keys, use the short axis name (last segment)
                    let short_axis = axis_name.split('.').next_back().unwrap_or(axis_name);
                    let coord_key = format!("{}.{}", sweep_id, short_axis);
                    new_combo.push((coord_key, axis_name.clone(), value));
                    next_combinations.push(new_combo);
                }
            }
        }
        all_combinations = next_combinations;
    }

    // Build fixed axis values for magnetic sweep.
    let mut fixed_magnetic = serde_json::Map::new();
    if let Some(mag_sweep) = recipe.sweeps.iter().find(|s| s.device == "magnetic") {
        for (name, value) in mag_sweep.fixed_axis_values() {
            fixed_magnetic.insert(name, serde_json::json!(value));
        }
    }

    let mut steps = Vec::with_capacity(all_combinations.len());
    for (point_index, combo) in all_combinations.iter().enumerate() {
        let step_id = format!("pt_{:03}", point_index);

        // Build sweep_coordinates.
        let mut sweep_coords = serde_json::Map::new();
        for (key, _axis_name, value) in combo {
            sweep_coords.insert(key.clone(), serde_json::json!(value));
        }

        // Build target_device_state.
        let mut device_state = serde_json::Map::new();

        // SMB100A state.
        let mut smb_state = serde_json::Map::new();
        if let Some(smb_fixed) = recipe.fixed_params.get("smb100a") {
            if let Some(obj) = smb_fixed.as_object() {
                for (k, v) in obj {
                    smb_state.insert(k.clone(), v.clone());
                }
            }
        }
        // Override with sweep value for RF frequency.
        if let Some((_, _, freq)) = combo.iter().find(|(k, _, _)| k.contains("frequency_hz")) {
            smb_state.insert("frequency_hz".into(), serde_json::json!(freq));
        }
        smb_state.insert("rf_output_required".into(), serde_json::json!(true));
        device_state.insert("smb100a".into(), serde_json::Value::Object(smb_state));

        // Magnetic state.
        let mut b_target = [
            fixed_magnetic
                .get("bx_nt")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            fixed_magnetic
                .get("by_nt")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            fixed_magnetic
                .get("bz_nt")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
        ];
        // Override with sweep values for any magnetic sweep.
        for (key, axis_name, value) in combo {
            if key.contains("mag_") || axis_name.ends_with("_nt") {
                match axis_name.as_str() {
                    "bx_nt" => b_target[0] = *value,
                    "by_nt" => b_target[1] = *value,
                    "bz_nt" => b_target[2] = *value,
                    _ => {}
                }
            }
        }
        let mag_state = build_magnetic_target_state(recipe, &b_target);
        device_state.insert("magnetic".into(), mag_state);

        // OE1022D state.
        if let Some(oe) = recipe.fixed_params.get("oe1022d") {
            device_state.insert("oe1022d".into(), oe.clone());
        }

        // Laser state.
        if let Some(laser) = recipe.fixed_params.get("laser") {
            device_state.insert("laser".into(), laser.clone());
        } else {
            device_state.insert("laser".into(), serde_json::json!({ "enabled": false }));
        }

        // Acquisition.
        let acquisition = if recipe.acquisition_policy.enabled {
            AcquisitionStep {
                enabled: true,
                device: Some(recipe.acquisition_policy.device.clone()),
                start_after: Some(recipe.acquisition_policy.start_after.clone()),
                pre_discard_ms: Some(recipe.acquisition_policy.pre_discard_ms),
                frames_expected: Some(recipe.acquisition_policy.frames_per_point),
                attach_device_state_snapshot: Some(
                    recipe.acquisition_policy.attach_device_state_snapshot,
                ),
            }
        } else {
            AcquisitionStep {
                enabled: false,
                device: None,
                start_after: None,
                pre_discard_ms: None,
                frames_expected: None,
                attach_device_state_snapshot: None,
            }
        };

        steps.push(ResolvedSystemStep {
            step_id,
            phase: "measure".into(),
            point_index: Some(point_index),
            sweep_coordinates: Some(serde_json::Value::Object(sweep_coords)),
            target_device_state: serde_json::Value::Object(device_state),
            acquisition,
            traceability: Traceability {
                source_recipe_id: recipe.header.id.clone(),
                required_state_snapshot: true,
                required_step_hash: true,
            },
        });
    }

    Ok(steps)
}

fn estimate_duration(recipe: &SystemScanRecipe, steps: &[ResolvedSystemStep]) -> f64 {
    let settle_ms = recipe
        .fixed_params
        .get("magnetic")
        .and_then(|v| v.get("default_settle_ms"))
        .and_then(|v| v.as_u64())
        .unwrap_or(500) as f64;

    let mut total_ms = 0.0;
    for step in steps {
        match step.phase.as_str() {
            "setup" | "cleanup" => total_ms += 500.0,
            "measure" => {
                let acquisition_ms = if step.acquisition.enabled {
                    let pre_discard = step.acquisition.pre_discard_ms.unwrap_or(0) as f64;
                    let frames = step.acquisition.frames_expected.unwrap_or(0) as f64;
                    let inter_frame = recipe
                        .fixed_params
                        .get("oe1022d")
                        .and_then(|v| v.get("inter_frame_delay_ms"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(20) as f64;
                    pre_discard + frames * inter_frame + 200.0 // overhead
                } else {
                    0.0
                };
                total_ms += settle_ms + acquisition_ms;
            }
            _ => total_ms += 100.0,
        }
    }
    total_ms / 1000.0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use odmr_recipe::SystemScanRecipe;

    fn example_recipe() -> SystemScanRecipe {
        let json = include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json");
        odmr_recipe::parse_system_scan_recipe(json).unwrap()
    }

    #[test]
    fn expansion_produces_nine_measure_steps() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let measure_steps: Vec<_> = resolved
            .steps
            .iter()
            .filter(|s| s.phase == "measure")
            .collect();
        assert_eq!(measure_steps.len(), 9, "3 mag x 3 RF = 9 points");
    }

    #[test]
    fn first_point_has_correct_coordinates() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let first = resolved
            .steps
            .iter()
            .find(|s| s.step_id == "pt_000")
            .unwrap();
        let coords = first.sweep_coordinates.as_ref().unwrap();
        assert_eq!(coords["mag_z_low_current_points.bz_nt"], -1000.0);
        assert_eq!(coords["rf_frequency_points.frequency_hz"], 2878000000.0);
    }

    #[test]
    fn last_point_has_correct_coordinates() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let last = resolved
            .steps
            .iter()
            .find(|s| s.step_id == "pt_008")
            .unwrap();
        let coords = last.sweep_coordinates.as_ref().unwrap();
        assert_eq!(coords["mag_z_low_current_points.bz_nt"], 1000.0);
        assert_eq!(coords["rf_frequency_points.frequency_hz"], 2886000000.0);
    }

    #[test]
    fn measure_steps_have_acquisition_enabled() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        for step in &resolved.steps {
            if step.phase == "measure" {
                assert!(
                    step.acquisition.enabled,
                    "measure step {} must have acquisition enabled",
                    step.step_id
                );
                assert_eq!(step.acquisition.frames_expected, Some(5));
            }
        }
    }

    #[test]
    fn setup_and_cleanup_have_acquisition_disabled() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        for step in &resolved.steps {
            if step.phase != "measure" {
                assert!(
                    !step.acquisition.enabled,
                    "{} step must not acquire",
                    step.phase
                );
            }
        }
    }

    #[test]
    fn every_measure_step_has_traceability_snapshot_required() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        for step in &resolved.steps {
            if step.phase == "measure" {
                assert!(step.traceability.required_state_snapshot);
            }
        }
    }

    #[test]
    fn resolved_has_stable_step_ids() {
        let recipe = example_recipe();
        let r1 = expand_system_scan_recipe(&recipe).unwrap();
        let r2 = expand_system_scan_recipe(&recipe).unwrap();
        for (a, b) in r1.steps.iter().zip(r2.steps.iter()) {
            assert_eq!(a.step_id, b.step_id);
        }
    }

    #[test]
    fn magnetic_target_state_is_correct() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let pt_000 = resolved
            .steps
            .iter()
            .find(|s| s.step_id == "pt_000")
            .unwrap();
        let mag = pt_000.target_device_state.get("magnetic").unwrap();
        let b_target = mag.get("b_target_nt").unwrap().as_array().unwrap();
        assert_eq!(b_target[0].as_f64(), Some(0.0));
        assert_eq!(b_target[1].as_f64(), Some(0.0));
        assert_eq!(b_target[2].as_f64(), Some(-1000.0));
    }

    #[test]
    fn rf_target_state_is_correct() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let pt_001 = resolved
            .steps
            .iter()
            .find(|s| s.step_id == "pt_001")
            .unwrap();
        let smb = pt_001.target_device_state.get("smb100a").unwrap();
        let rf = smb.get("rf").unwrap();
        assert_eq!(rf.get("frequency_hz").unwrap().as_f64(), Some(2882000000.0));
        assert_eq!(rf.get("power_dbm").unwrap().as_f64(), Some(-30.0));
    }

    #[test]
    fn dry_run_has_correct_summary() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let plan = build_system_scan_dry_run(&recipe, &resolved);
        assert_eq!(plan.summary.total_points, 9);
        assert_eq!(plan.summary.expected_frames, 45);
        assert_eq!(plan.summary.outer_sweep, "mag_z_low_current_points");
        assert_eq!(plan.summary.inner_sweep, "rf_frequency_points");
        assert!(plan.operator_approval_required);
    }

    #[test]
    fn expansion_with_zero_sweeps_returns_empty_measure() {
        let mut recipe = example_recipe();
        recipe.sweeps.clear();
        recipe.sweep_order.clear();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let measure: Vec<_> = resolved
            .steps
            .iter()
            .filter(|s| s.phase == "measure")
            .collect();
        assert!(measure.is_empty());
    }

    #[test]
    fn expansion_preserves_full_smb100a_state() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let setup_rf = resolved
            .steps
            .iter()
            .find(|s| s.step_id == "step_setup_001")
            .unwrap();
        let smb = setup_rf.target_device_state.get("smb100a").unwrap();
        assert!(smb.get("rf").is_some());
        assert!(smb.get("modulation").is_some());
        assert!(smb.get("fm").is_some());
        assert!(smb.get("lf").is_some());
    }

    #[test]
    fn expansion_preserves_full_oe1022d_state() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let pt_000 = resolved
            .steps
            .iter()
            .find(|s| s.step_id == "pt_000")
            .unwrap();
        let oe = pt_000.target_device_state.get("oe1022d").unwrap();
        assert!(oe.get("input").is_some());
        assert!(oe.get("gain").is_some());
        assert!(oe.get("filter").is_some());
        assert!(oe.get("acquisition").is_some());
    }

    #[test]
    fn expansion_computes_magnetic_predicted_current() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let pt_000 = resolved
            .steps
            .iter()
            .find(|s| s.step_id == "pt_000")
            .unwrap();
        let mag = pt_000.target_device_state.get("magnetic").unwrap();
        let current = mag.get("predicted_current_a").unwrap().as_array().unwrap();
        assert_eq!(current.len(), 3);
        // 1000 nT = 1e-6 T, coil constant = 1e-4 T/A => I = 1e-2 A = 0.01 A
        assert!((current[2].as_f64().unwrap() - (-0.01)).abs() < 1e-9);
    }

    #[test]
    fn sweep_order_outer_inner_is_stable() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let measure: Vec<_> = resolved
            .steps
            .iter()
            .filter(|s| s.phase == "measure")
            .collect();
        // Outer sweep (magnetic) should change every 3 steps; inner (RF) every 1 step.
        let bz_first: Vec<_> = measure
            .iter()
            .map(|s| {
                s.sweep_coordinates.as_ref().unwrap()["mag_z_low_current_points.bz_nt"]
                    .as_f64()
                    .unwrap()
            })
            .collect();
        assert_eq!(
            bz_first,
            vec![-1000.0, -1000.0, -1000.0, 0.0, 0.0, 0.0, 1000.0, 1000.0, 1000.0]
        );
    }

    #[test]
    fn every_measure_step_has_complete_target_device_state() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        for step in &resolved.steps {
            if step.phase == "measure" {
                assert!(step.target_device_state.get("smb100a").is_some());
                assert!(step.target_device_state.get("magnetic").is_some());
                assert!(step.target_device_state.get("oe1022d").is_some());
                assert!(step.target_device_state.get("laser").is_some());
            }
        }
    }

    #[test]
    fn resolved_json_serializes() {
        let recipe = example_recipe();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let json = serde_json::to_string_pretty(&resolved).unwrap();
        assert!(json.contains("resolved_m5b_rf_mag_oe_system_scan"));
        assert!(json.contains("step_setup_000"));
        assert!(json.contains("pt_008"));
    }
}
