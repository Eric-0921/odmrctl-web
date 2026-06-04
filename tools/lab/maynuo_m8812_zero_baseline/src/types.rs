//! Artifact types for Mag-M2B zero-baseline probe.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Per-axis zero-baseline measurement result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisZeroBaseline {
    pub axis_id: String,
    pub idn: String,
    pub port_path: String,
    pub sn_tail: String,
    /// Commanded zero-set current in mA (always 0.0 in M2B).
    pub zero_set_current_ma: f64,
    /// Individual MEAS:CURR? readback samples in mA.
    pub zero_readback_samples_ma: Vec<f64>,
    /// Mean of zero_readback_samples_ma, the locked baseline.
    pub zero_readback_current_ma: f64,
    /// Standard deviation of samples in mA.
    pub zero_readback_std_ma: f64,
    /// Same mean in A.
    pub zero_readback_current_a: f64,
    pub coil_constant_nt_per_ma: f64,
    pub lock_zero_applied: bool,
    pub output_was_on: bool,
    pub shutdown_succeeded: bool,
    pub errors: Vec<String>,
}

/// Snapshot of all axes processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroBaselineSnapshot {
    pub schema_version: String,
    pub axes: Vec<AxisZeroBaseline>,
    pub timestamp_utc: String,
}

/// Timeline event for zero-baseline operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroBaselineEvent {
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

/// Audit invariants computed from the command trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditInvariants {
    pub nonzero_current_sent: bool,
    pub outp_on_sent: bool,
    pub outp_on_only_after_curr_zero: bool,
    pub measured_current_queries_sent: u32,
    pub zero_set_current_ma: f64,
    pub zero_readback_current_ma_recorded: bool,
    pub lock_zero_event_recorded: bool,
    pub recurrent_current_sent: bool,
    pub recurrent_field_sent: bool,
    pub final_output_off: bool,
    pub final_current_zero_command_sent: bool,
    pub final_local_mode_requested: bool,
    /// Per-axis invariant breakdown. Each processed axis must pass all checks.
    pub per_axis: BTreeMap<String, PerAxisInvariants>,
}

/// Per-axis audit invariant check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerAxisInvariants {
    pub outp_on_sent: bool,
    pub outp_on_only_after_curr_zero: bool,
    pub measured_current_queries_sent: u32,
    pub zero_readback_current_ma_recorded: bool,
    pub lock_zero_event_recorded: bool,
    pub final_output_off: bool,
    pub final_current_zero_command_sent: bool,
    pub final_local_mode_requested: bool,
    pub all_pass: bool,
}

impl PerAxisInvariants {
    pub fn check(&self) -> bool {
        self.outp_on_sent
            && self.outp_on_only_after_curr_zero
            && self.measured_current_queries_sent >= 1
            && self.zero_readback_current_ma_recorded
            && self.lock_zero_event_recorded
            && self.final_output_off
            && self.final_current_zero_command_sent
            && self.final_local_mode_requested
    }
}

impl AuditInvariants {
    pub fn all_pass(&self) -> bool {
        !self.nonzero_current_sent
            && self.outp_on_sent
            && self.outp_on_only_after_curr_zero
            && self.measured_current_queries_sent >= 1
            && self.zero_readback_current_ma_recorded
            && self.lock_zero_event_recorded
            && !self.recurrent_current_sent
            && !self.recurrent_field_sent
            && self.final_output_off
            && self.final_current_zero_command_sent
            && self.final_local_mode_requested
            && !self.per_axis.is_empty()
            && self.per_axis.values().all(|p| p.all_pass)
    }
}

/// Summary report for the zero-baseline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroBaselineReport {
    pub passed: bool,
    pub axes_processed: usize,
    pub axes_passed: usize,
    pub axes_failed: usize,
    pub total_measurements: u32,
    pub audit_invariants: AuditInvariants,
    pub errors: Vec<String>,
}

/// Manifest recording tool run metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroBaselineManifest {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub started_at_utc: String,
    pub completed_at_utc: String,
    pub profile_path: String,
    pub passed: bool,
    pub artifact_files: Vec<String>,
    pub axes_processed: Vec<String>,
    pub audit_invariants_met: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_note: Option<String>,
    pub only_m2b_commands_sent: bool,
}
