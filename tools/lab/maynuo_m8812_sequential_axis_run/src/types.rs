//! Artifact types for Mag-M4 sequential multi-axis run.

use serde::{Deserialize, Serialize};

/// Per-axis step result within the sequential run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialAxisStepResult {
    pub step_index: u32,
    pub axis_id: String,
    pub idn: String,
    pub port_path: String,
    pub sn_tail: String,
    pub coil_constant_nt_per_ma: f64,

    // Zero-baseline phase
    pub zero_set_current_ma: f64,
    pub zero_readback_samples_ma: Vec<f64>,
    pub zero_readback_current_ma: f64,
    pub zero_readback_std_ma: f64,
    pub lock_zero_applied: bool,

    // Recurrent setpoint
    pub recur_current_ma_requested: f64,
    pub total_current_ma_commanded: f64,
    pub command_string: String,

    // Recur phase readback
    pub recur_readback_samples_ma: Vec<f64>,
    pub measured_total_current_ma: f64,
    pub measured_total_std_ma: f64,
    pub measured_recur_current_ma: f64,
    pub measured_recur_field_nt: f64,

    // Error vs setpoint
    pub current_error_ma: f64,
    pub field_error_nt: f64,

    // Cleanup
    pub output_final_off: bool,
    pub current_final_zero: bool,
    pub local_mode_requested: bool,

    // Timing
    pub started_at_utc: String,
    pub completed_at_utc: String,

    pub errors: Vec<String>,
}

/// Snapshot of the full sequential run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialAxisRunSnapshot {
    pub schema_version: String,
    pub axes: Vec<SequentialAxisStepResult>,
    pub timestamp_utc: String,
}

/// Timeline event for sequential run operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialAxisRunEvent {
    pub event_type: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub axis_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Per-command audit trail entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandAuditEntry {
    pub seq: u64,
    pub timestamp: String,
    pub axis_id: String,
    pub command: String,
    pub command_class: String,
    pub expects_response: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_error: Option<String>,
    pub allowed: bool,
    pub nonzero_current_attempted: bool,
}

/// Per-axis entry in the summary report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialAxisReportEntry {
    pub step_index: u32,
    pub axis_id: String,
    pub passed: bool,
    pub expected_sn: String,
    pub observed_sn: String,
    pub zero_readback_current_ma: f64,
    pub commanded_recur_current_ma: f64,
    pub measured_total_current_ma: f64,
    pub measured_recur_current_ma: f64,
    pub measured_recur_field_nt: f64,
    pub current_error_ma: f64,
    pub field_error_nt: f64,
    pub output_final_off: bool,
    pub current_final_zero: bool,
    pub local_mode_requested: bool,
    pub errors: Vec<String>,
}

/// Summary report for the sequential run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialAxisRunReport {
    pub passed: bool,
    pub axes_processed: u32,
    pub axes_passed: u32,
    pub recur_current_ma_requested: f64,
    pub per_axis: Vec<SequentialAxisReportEntry>,
    pub no_axis_overlap: bool,
    pub errors: Vec<String>,
}

/// Manifest recording tool run metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialAxisRunManifest {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub started_at_utc: String,
    pub completed_at_utc: String,
    pub profile_path: String,
    pub axes_requested: Vec<String>,
    pub recur_current_ma_requested: f64,
    pub passed: bool,
    pub artifact_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_note: Option<String>,
    pub only_m4_commands_sent: bool,
}
