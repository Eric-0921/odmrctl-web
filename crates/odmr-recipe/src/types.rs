//! Core data types for recipes, stations, and resolved recipes.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Common header
// ---------------------------------------------------------------------------

/// Fields present in every top-level JSON object.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CommonHeader {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Station
// ---------------------------------------------------------------------------

/// Describes the current experimental station: device inventory, connections,
/// safety profile reference, and calibration snapshot.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Station {
    #[serde(flatten)]
    pub header: CommonHeader,
    pub devices: Vec<StationDevice>,
    pub safety_profile_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration_snapshot_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct StationDevice {
    pub device_id: String,
    pub device_type: String,
    pub transport: TransportConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind")]
pub enum TransportConfig {
    #[serde(rename = "tcp_scpi")]
    TcpScpi { host: String, port: u16 },
    #[serde(rename = "serial")]
    Serial {
        port: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        baud_rate: Option<u32>,
    },
    #[serde(rename = "serial_group")]
    SerialGroup { ports: Vec<String> },
}

// ---------------------------------------------------------------------------
// Recipe
// ---------------------------------------------------------------------------

/// Human- or AI-authored experiment plan.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Recipe {
    #[serde(flatten)]
    pub header: CommonHeader,
    pub station_id: String,
    pub intent: RecipeIntent,
    #[serde(default)]
    pub profiles: Vec<String>,
    #[serde(default)]
    pub blocks: Vec<String>,
    #[serde(default)]
    pub sweeps: Vec<SweepDefinition>,
    pub acquisition: AcquisitionConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<TimingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RecipeIntent {
    pub experiment_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SweepDefinition {
    pub sweep_id: String,
    pub axis: String,
    pub start: f64,
    pub stop: f64,
    pub step: f64,
    #[serde(default = "default_order")]
    pub order: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dwell_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settle_ms: Option<u64>,
}

fn default_order() -> String {
    "ascending".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AcquisitionConfig {
    pub device_id: String,
    pub channel: String,
    #[serde(default)]
    pub readout: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_discard_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average: Option<AverageConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AverageConfig {
    pub mode: String,
    pub repeat: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TimingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_dwell_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_settle_ms: Option<u64>,
}

// ---------------------------------------------------------------------------
// ResolvedRecipe
// ---------------------------------------------------------------------------

/// Compiler-generated final execution plan.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ResolvedRecipe {
    #[serde(flatten)]
    pub header: CommonHeader,
    pub source_recipe_id: String,
    pub source_recipe_hash: String,
    pub station_id: String,
    pub estimated_duration_s: f64,
    pub safety_report_id: String,
    pub steps: Vec<RecipeStep>,
}

// ---------------------------------------------------------------------------
// RecipeStep
// ---------------------------------------------------------------------------

/// A single explicit execution step inside a resolved recipe.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RecipeStep {
    pub step_id: String,
    pub phase: String,
    #[serde(default)]
    pub device_actions: Vec<DeviceAction>,
    pub expected_state: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<TimingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acquisition: Option<AcquisitionConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sweep_coordinates: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_block_id: Option<String>,
    /// Which sweep definition this step was expanded from.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_sweep_id: Option<String>,
    /// Zero-based index within the sweep.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub point_index: Option<usize>,
    /// Total number of points in the parent sweep.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub total_points: Option<usize>,
    /// Estimated duration of this step in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub estimated_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DeviceAction {
    pub device_id: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// SafetyLimit
// ---------------------------------------------------------------------------

/// Mock representation of safety limits for the bootstrap milestone.
///
/// In production these live in `station` / safety-profile and are enforced by
/// `odmr-safety`.  Here we keep a minimal struct so recipes can be validated
/// against representative bounds.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SafetyLimit {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub max_power_dbm: f64,
    pub max_frequency_hz: f64,
    pub min_frequency_hz: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fm_deviation_hz: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_magnetic_field_t: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_mag_ramp_rate_a_per_s: Option<f64>,
}
