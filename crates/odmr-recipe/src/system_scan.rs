//! System scan recipe types and parser.
//!
//! This module defines `SystemScanRecipe` — a recipe kind that supports
//! multi-device parameter scanning (RF frequency, magnetic field vector,
//! OE1022D acquisition) via nested sweep expansion.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::CommonHeader;
use crate::validation::ValidationError;

// ---------------------------------------------------------------------------
// Top-level recipe
// ---------------------------------------------------------------------------

/// A system-level scan recipe that declares fixed parameters, sweep axes,
/// sweep order, and acquisition policy for multi-device experiments.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SystemScanRecipe {
    #[serde(flatten)]
    pub header: CommonHeader,
    pub station_ref: String,
    #[serde(default)]
    pub physical_response_required: bool,
    pub devices: HashMap<String, DeviceRef>,
    pub fixed_params: HashMap<String, serde_json::Value>,
    pub sweeps: Vec<SystemSweepDefinition>,
    pub sweep_order: Vec<String>,
    pub acquisition_policy: AcquisitionPolicy,
    #[serde(default)]
    pub safety: SafetyFlags,
}

// ---------------------------------------------------------------------------
// Device reference
// ---------------------------------------------------------------------------

/// Reference to a device instance used by the recipe.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DeviceRef {
    pub device_id: String,
    #[serde(default)]
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

// ---------------------------------------------------------------------------
// Sweep definitions
// ---------------------------------------------------------------------------

/// A sweep definition for system-scan recipes.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SystemSweepDefinition {
    pub sweep_id: String,
    pub device: String,
    #[serde(flatten)]
    pub shape: SweepShape,
}

/// Discriminated union of supported sweep shapes.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum SweepShape {
    /// Cartesian grid sweep for magnetic field vectors.
    CartesianGrid {
        axis_group: String,
        unit: String,
        axes: HashMap<String, SweepAxisValue>,
    },
    /// Discrete value list sweep for scalar parameters (e.g. RF frequency).
    ValuesList { axis: String, values: Vec<f64> },
}

/// Per-axis value specification: either a single fixed value or a list of
/// swept values.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum SweepAxisValue {
    Value { value: f64 },
    Values { values: Vec<f64> },
}

// ---------------------------------------------------------------------------
// Acquisition policy
// ---------------------------------------------------------------------------

/// Declares when and how OE1022D (or other) acquisition should occur.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AcquisitionPolicy {
    pub enabled: bool,
    pub device: String,
    pub mode: String,
    pub start_after: Vec<String>,
    pub pre_discard_ms: u64,
    pub frames_per_point: u64,
    #[serde(default)]
    pub attach_device_state_snapshot: bool,
}

// ---------------------------------------------------------------------------
// Safety flags
// ---------------------------------------------------------------------------

/// Policy switches declared by the recipe.  Numeric limits are **not**
/// allowed here — they come from the station/safety profile.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct SafetyFlags {
    #[serde(default)]
    pub require_operator_approval: bool,
    #[serde(default)]
    pub no_internal_smb_sweep: bool,
    #[serde(default)]
    pub no_realtime_csv: bool,
    #[serde(default)]
    pub no_gui_direct_hardware: bool,
    #[serde(default)]
    pub laser_default_disabled: bool,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parse a `SystemScanRecipe` from a JSON string.
pub fn parse_system_scan_recipe(json: &str) -> Result<SystemScanRecipe, crate::RecipeError> {
    let recipe: SystemScanRecipe = serde_json::from_str(json)?;
    validate_system_scan_recipe(&recipe)?;
    Ok(recipe)
}

/// Load a `SystemScanRecipe` from a file path.
pub fn load_system_scan_recipe(
    path: &std::path::Path,
) -> Result<SystemScanRecipe, crate::RecipeError> {
    let contents = std::fs::read_to_string(path)?;
    parse_system_scan_recipe(&contents)
}

/// Compute a deterministic SHA-256 hash for a `SystemScanRecipe`.
pub fn compute_system_scan_hash(recipe: &SystemScanRecipe) -> String {
    match serde_json::to_string(recipe) {
        Ok(json) => {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(json.as_bytes());
            hex::encode(hasher.finalize())
        }
        Err(_) => "invalid_hash".into(),
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate a `SystemScanRecipe` without loading external files.
pub fn validate_system_scan_recipe(recipe: &SystemScanRecipe) -> Result<(), ValidationError> {
    if recipe.header.kind != "system_scan_recipe" {
        return Err(ValidationError::InvalidId {
            field: "kind".into(),
            reason: format!(
                "expected 'system_scan_recipe', got '{}'",
                recipe.header.kind
            ),
        });
    }

    if recipe.station_ref.is_empty() {
        return Err(ValidationError::MissingField("station_ref".into()));
    }

    // Required devices must be present.
    let required_device_keys = ["smb100a", "oe1022d", "magnetic"];
    for key in &required_device_keys {
        match recipe.devices.get(*key) {
            Some(dev) if dev.required => {}
            _ => {
                return Err(ValidationError::RequiredDeviceMissing(format!(
                    "required device '{}' missing or not marked required",
                    key
                )));
            }
        }
    }

    // Laser must be disabled for M5B-A.
    if let Some(laser) = recipe.devices.get("laser") {
        if laser.enabled == Some(true) {
            return Err(ValidationError::InvalidId {
                field: "devices.laser.enabled".into(),
                reason: "laser enabled is not supported in M5B-A".into(),
            });
        }
    }

    // Sweep order must reference existing sweeps.
    let sweep_ids: std::collections::HashSet<_> =
        recipe.sweeps.iter().map(|s| s.sweep_id.as_str()).collect();
    for ordered_id in &recipe.sweep_order {
        if !sweep_ids.contains(ordered_id.as_str()) {
            return Err(ValidationError::SweepStepMismatch {
                sweep_id: ordered_id.clone(),
                reason: "sweep_order references unknown sweep_id".into(),
            });
        }
    }

    // Every sweep must have at least one varying axis.
    for sweep in &recipe.sweeps {
        match &sweep.shape {
            SweepShape::CartesianGrid { axes, .. } => {
                let varying = axes
                    .values()
                    .any(|v| matches!(v, SweepAxisValue::Values { .. }));
                if !varying {
                    return Err(ValidationError::SweepStepMismatch {
                        sweep_id: sweep.sweep_id.clone(),
                        reason: "cartesian_grid sweep has no varying axes".into(),
                    });
                }
            }
            SweepShape::ValuesList { values, .. } => {
                if values.is_empty() {
                    return Err(ValidationError::SweepStepMismatch {
                        sweep_id: sweep.sweep_id.clone(),
                        reason: "values_list sweep has empty values".into(),
                    });
                }
            }
        }
    }

    // Acquisition policy must be valid when enabled.
    if recipe.acquisition_policy.enabled && recipe.acquisition_policy.frames_per_point == 0 {
        return Err(ValidationError::InvalidId {
            field: "acquisition_policy.frames_per_point".into(),
            reason: "frames_per_point must be > 0 when acquisition is enabled".into(),
        });
    }

    // Reject raw command arrays anywhere in fixed_params.
    for (device_key, params) in &recipe.fixed_params {
        if let Some(arr) = params.as_array() {
            if !arr.is_empty() && arr.iter().all(|v| v.is_string()) {
                return Err(ValidationError::InvalidId {
                    field: format!("fixed_params.{}", device_key),
                    reason: "command arrays are not allowed in system_scan_recipe".into(),
                });
            }
        }
    }

    // Reject safety limit overrides.
    if recipe.fixed_params.contains_key("safety_limits") {
        return Err(ValidationError::InvalidId {
            field: "fixed_params.safety_limits".into(),
            reason: "recipe cannot override safety limits".into(),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers: sweep expansion metadata
// ---------------------------------------------------------------------------

impl SystemSweepDefinition {
    /// Return the number of points this sweep contributes.
    pub fn point_count(&self) -> usize {
        match &self.shape {
            SweepShape::CartesianGrid { axes, .. } => axes
                .values()
                .filter_map(|v| match v {
                    SweepAxisValue::Values { values } => Some(values.len()),
                    _ => None,
                })
                .product(),
            SweepShape::ValuesList { values, .. } => values.len(),
        }
    }

    /// Return the varying axis names and their value lists for cartesian expansion.
    pub fn varying_axes(&self) -> Vec<(String, Vec<f64>)> {
        match &self.shape {
            SweepShape::CartesianGrid { axes, .. } => axes
                .iter()
                .filter_map(|(name, val)| match val {
                    SweepAxisValue::Values { values } => Some((name.clone(), values.clone())),
                    _ => None,
                })
                .collect(),
            SweepShape::ValuesList { axis, values } => {
                vec![(axis.clone(), values.clone())]
            }
        }
    }

    /// Return the fixed axis values for cartesian grids.
    pub fn fixed_axis_values(&self) -> HashMap<String, f64> {
        let mut result = HashMap::new();
        if let SweepShape::CartesianGrid { axes, .. } = &self.shape {
            for (name, val) in axes {
                if let SweepAxisValue::Value { value } = val {
                    result.insert(name.clone(), *value);
                }
            }
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn example_recipe_json() -> &'static str {
        include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json")
    }

    #[test]
    fn example_system_scan_recipe_parses() {
        let recipe = parse_system_scan_recipe(example_recipe_json()).unwrap();
        assert_eq!(recipe.header.kind, "system_scan_recipe");
        assert_eq!(recipe.header.id, "m5b_rf_mag_oe_system_scan");
        assert_eq!(recipe.sweeps.len(), 2);
        assert_eq!(recipe.sweep_order.len(), 2);
    }

    #[test]
    fn magnetic_sweep_has_three_points() {
        let recipe = parse_system_scan_recipe(example_recipe_json()).unwrap();
        let mag_sweep = recipe
            .sweeps
            .iter()
            .find(|s| s.sweep_id == "mag_z_low_current_points")
            .unwrap();
        assert_eq!(mag_sweep.point_count(), 3);
    }

    #[test]
    fn rf_sweep_has_three_points() {
        let recipe = parse_system_scan_recipe(example_recipe_json()).unwrap();
        let rf_sweep = recipe
            .sweeps
            .iter()
            .find(|s| s.sweep_id == "rf_frequency_points")
            .unwrap();
        assert_eq!(rf_sweep.point_count(), 3);
    }

    #[test]
    fn laser_enabled_is_rejected() {
        let json = r#"{
            "schema_version": "0.2.0",
            "kind": "system_scan_recipe",
            "id": "test",
            "description": "test",
            "station_ref": "station.json",
            "physical_response_required": false,
            "devices": {
                "smb100a": { "device_id": "smb", "required": true },
                "oe1022d": { "device_id": "oe", "required": true },
                "magnetic": { "device_id": "mag", "required": true },
                "laser": { "device_id": "laser", "required": false, "enabled": true }
            },
            "fixed_params": {},
            "sweeps": [
                { "sweep_id": "s1", "device": "magnetic", "type": "cartesian_grid", "axis_group": "mag", "unit": "nT", "axes": { "bz_nt": { "values": [0.0] } } }
            ],
            "sweep_order": ["s1"],
            "acquisition_policy": { "enabled": false, "device": "oe", "mode": "per_final_sweep_point", "start_after": [], "pre_discard_ms": 0, "frames_per_point": 0 }
        }"#;
        let err = parse_system_scan_recipe(json).unwrap_err();
        assert!(err.to_string().contains("laser enabled"));
    }

    #[test]
    fn command_array_in_fixed_params_is_rejected() {
        let json = r#"{
            "schema_version": "0.2.0",
            "kind": "system_scan_recipe",
            "id": "test",
            "description": "test",
            "station_ref": "station.json",
            "physical_response_required": false,
            "devices": {
                "smb100a": { "device_id": "smb", "required": true },
                "oe1022d": { "device_id": "oe", "required": true },
                "magnetic": { "device_id": "mag", "required": true }
            },
            "fixed_params": {
                "smb100a": ["FREQ 2.8GHz", "OUTP ON"]
            },
            "sweeps": [
                { "sweep_id": "s1", "device": "magnetic", "type": "cartesian_grid", "axis_group": "mag", "unit": "nT", "axes": { "bz_nt": { "values": [0.0] } } }
            ],
            "sweep_order": ["s1"],
            "acquisition_policy": { "enabled": false, "device": "oe", "mode": "per_final_sweep_point", "start_after": [], "pre_discard_ms": 0, "frames_per_point": 0 }
        }"#;
        let err = parse_system_scan_recipe(json).unwrap_err();
        assert!(err.to_string().contains("command arrays"));
    }

    #[test]
    fn safety_limit_override_is_rejected() {
        let json = r#"{
            "schema_version": "0.2.0",
            "kind": "system_scan_recipe",
            "id": "test",
            "description": "test",
            "station_ref": "station.json",
            "physical_response_required": false,
            "devices": {
                "smb100a": { "device_id": "smb", "required": true },
                "oe1022d": { "device_id": "oe", "required": true },
                "magnetic": { "device_id": "mag", "required": true }
            },
            "fixed_params": {
                "safety_limits": { "max_power_dbm": 50 }
            },
            "sweeps": [
                { "sweep_id": "s1", "device": "magnetic", "type": "cartesian_grid", "axis_group": "mag", "unit": "nT", "axes": { "bz_nt": { "values": [0.0] } } }
            ],
            "sweep_order": ["s1"],
            "acquisition_policy": { "enabled": false, "device": "oe", "mode": "per_final_sweep_point", "start_after": [], "pre_discard_ms": 0, "frames_per_point": 0 }
        }"#;
        let err = parse_system_scan_recipe(json).unwrap_err();
        assert!(err.to_string().contains("safety limits"));
    }

    #[test]
    fn missing_sweep_in_order_is_rejected() {
        let json = r#"{
            "schema_version": "0.2.0",
            "kind": "system_scan_recipe",
            "id": "test",
            "description": "test",
            "station_ref": "station.json",
            "physical_response_required": false,
            "devices": {
                "smb100a": { "device_id": "smb", "required": true },
                "oe1022d": { "device_id": "oe", "required": true },
                "magnetic": { "device_id": "mag", "required": true }
            },
            "fixed_params": {},
            "sweeps": [
                { "sweep_id": "s1", "device": "magnetic", "type": "cartesian_grid", "axis_group": "mag", "unit": "nT", "axes": { "bz_nt": { "values": [0.0] } } }
            ],
            "sweep_order": ["s1", "nonexistent"],
            "acquisition_policy": { "enabled": false, "device": "oe", "mode": "per_final_sweep_point", "start_after": [], "pre_discard_ms": 0, "frames_per_point": 0 }
        }"#;
        let err = parse_system_scan_recipe(json).unwrap_err();
        assert!(err.to_string().contains("unknown sweep_id"));
    }

    #[test]
    fn empty_values_list_is_rejected() {
        let json = r#"{
            "schema_version": "0.2.0",
            "kind": "system_scan_recipe",
            "id": "test",
            "description": "test",
            "station_ref": "station.json",
            "physical_response_required": false,
            "devices": {
                "smb100a": { "device_id": "smb", "required": true },
                "oe1022d": { "device_id": "oe", "required": true },
                "magnetic": { "device_id": "mag", "required": true }
            },
            "fixed_params": {},
            "sweeps": [
                { "sweep_id": "s1", "device": "smb100a", "type": "values_list", "axis": "smb100a.rf.frequency_hz", "values": [] }
            ],
            "sweep_order": ["s1"],
            "acquisition_policy": { "enabled": false, "device": "oe", "mode": "per_final_sweep_point", "start_after": [], "pre_discard_ms": 0, "frames_per_point": 0 }
        }"#;
        let err = parse_system_scan_recipe(json).unwrap_err();
        assert!(err.to_string().contains("empty values"));
    }
}
