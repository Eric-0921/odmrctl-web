use serde::{Deserialize, Serialize};

/// M3.3 extended sweep configuration, written as metadata/extended_sweep_config.json
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SweepConfig {
    pub schema_version: String,
    pub smb_host: String,
    pub smb_port: u16,
    pub smb_query_delay_ms: u64,
    pub smb_timeout_ms: u64,
    pub oe_port: String,
    pub oe_baud: u32,
    pub oe_timeout_ms: u64,
    pub rf_start_hz: f64,
    pub rf_stop_hz: f64,
    pub rf_points: u64,
    pub rf_power_dbm: f64,
    pub max_rf_power_dbm: f64,
    pub fm_deviation_hz: f64,
    pub max_fm_deviation_hz: f64,
    pub repeat_count: u64,
    pub set_internal_lf: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lf_frequency_hz: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lf_shape: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lf_voltage_v: Option<f64>,
    pub frames_per_step: u64,
    pub inter_frame_delay_ms: u64,
    pub oe_frame_delay_ms: u64,
    pub created_at_unix_ms: u64,
}

/// A single frequency step.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SweepStepDefinition {
    pub step_index: u64,
    pub frequency_hz: f64,
}

/// Full step plan written as rf/step_plan.json
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StepPlan {
    pub schema_version: String,
    pub kind: String,
    pub rf_start_hz: f64,
    pub rf_stop_hz: f64,
    pub rf_points: u64,
    pub repeat_count: u64,
    pub frequencies_hz: Vec<f64>,
    pub frames_per_step: u64,
    pub rf_power_dbm: f64,
    pub fm_deviation_hz: f64,
    pub software_stepped: bool,
    pub smb_internal_sweep_used: bool,
}

/// Per-step result written as rf/rf_step_summary.jsonl
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RfStepResult {
    pub schema_version: String,
    pub step_id: String,
    pub step_index: u64,
    pub repeat_index: u64,
    pub frequency_hz_requested: f64,
    pub frequency_hz_verified: f64,
    pub frequency_set_ok: bool,
    pub rf_on_sent: bool,
    pub rf_off_sent: bool,
    pub rf_on_confirmed: bool,
    pub rf_off_confirmed_after_step: bool,
    pub frames_requested: u64,
    pub frames_captured: usize,
    pub frames_parsed: usize,
    pub frames_failed: usize,
    pub frames_parse_failed: usize,
    pub step_passed: bool,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statistics: Option<StepStatistics>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Step-level B-channel statistics from full 50-sample vectors.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StepStatistics {
    pub repeat_index: u64,
    pub step_id: String,
    pub frequency_hz: f64,
    pub b_x_mean_mv: f64,
    pub b_x_std_mv: f64,
    pub b_x_min_mv: f64,
    pub b_x_max_mv: f64,
    pub b_y_mean_mv: f64,
    pub b_y_std_mv: f64,
    pub b_y_min_mv: f64,
    pub b_y_max_mv: f64,
    pub frames_used: usize,
    pub frames_parse_failed: usize,
}

/// Run-level stability summary.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RunStabilitySummary {
    pub rf_points: usize,
    pub repeat_count: usize,
    pub frames_requested: usize,
    pub frames_captured: usize,
    pub frames_parsed: usize,
    pub frames_parse_failed: usize,
    pub parse_failure_rate: f64,
    pub steps_requested: usize,
    pub steps_completed: usize,
    pub final_rf_off: bool,
    pub final_mod_off: bool,
    pub final_fm_off: bool,
    pub syst_err_clean_after: bool,
}

/// Quarantine entry for parse-failed frames.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParseQuarantineEntry {
    pub frame_seq: u64,
    pub step_id: String,
    pub raw_nbytes: usize,
    pub error_type: String,
    pub error_detail: String,
}

/// Aggregate sweep result (M3.3 extended).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SweepResult {
    pub passed: bool,
    pub total_steps: usize,
    pub steps_completed: usize,
    pub steps_failed: usize,
    pub total_frames_requested: usize,
    pub total_frames_captured: usize,
    pub total_frames_parsed: usize,
    pub step_results: Vec<RfStepResult>,
    pub oe_idn: String,
    pub preflight: PreflightCheck,
    pub forbidden_check: ForbiddenCommandCheck,
    pub emergency_shutdown_attempted: bool,
    pub repeat_count: u64,
    pub parse_failure_rate: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stability: Option<RunStabilitySummary>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Maps an OE1022D frame to the RF step it was acquired during (M3.3 extended).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrameToStepAlignment {
    pub schema_version: String,
    pub frame_seq: u64,
    pub raw_offset: u64,
    pub raw_nbytes: usize,
    pub step_id: String,
    pub step_index: u64,
    pub repeat_index: u64,
    pub frequency_hz: f64,
    pub rf_output_state: String,
    pub mod_state: String,
    pub fm_state: String,
    pub frame_monotonic_ns_since_run_start: u64,
    pub alignment_method: String,
    pub parse_status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlignmentSummary {
    pub schema_version: String,
    pub total_frames: usize,
    pub steps_with_frames: usize,
    pub frames_per_step_map: Vec<FrameCountPerStep>,
    pub alignment_ok: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrameCountPerStep {
    pub repeat_index: u64,
    pub step_index: u64,
    pub frame_count: usize,
}

// ---------------------------------------------------------------------------
// Reused/adapted from M3.2 types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Smb100aSnapshot {
    pub schema_version: String,
    pub device_id: String,
    pub idn: String,
    pub queried_at_unix_ms: u64,
    pub queries: Vec<SmbQueryResult>,
    pub connection_closed: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SmbQueryResult {
    pub command: String,
    pub response: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandAuditEntry {
    pub timestamp_unix_ms: u64,
    pub device_id: String,
    pub command: String,
    pub command_class: String,
    pub allowed: bool,
    pub sent_to_transport: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_approval_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_approval_present: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_relevant: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ErrorQueueObservation {
    pub timestamp_unix_ms: u64,
    pub attempt: usize,
    pub command: String,
    pub response: String,
    pub clean: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreflightCheck {
    pub passed: bool,
    pub outp_off_before: bool,
    pub mod_stat_off_before: bool,
    pub error_queue_clean_before: bool,
    pub operator_approval_present: bool,
    pub power_within_limit: bool,
    pub points_within_limit: bool,
    pub repeat_within_limit: bool,
    pub fm_deviation_within_limit: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForbiddenCommandCheck {
    pub passed: bool,
    pub forbidden_commands_attempted: Vec<String>,
    pub forbidden_commands_sent_to_transport: Vec<String>,
    pub sweep_commands_sent: usize,
    pub lf_output_enable_commands_sent: usize,
    pub unexpected_rf_output_commands_sent: usize,
    pub unexpected_modulation_commands_sent: usize,
    pub unexpected_fm_commands_sent: usize,
    pub magnetic_commands_sent: usize,
    pub oe_setting_commands_sent: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmergencyShutdownEvidence {
    pub shutdown_attempted: bool,
    pub shutdown_timestamp_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outp_command_sent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_command_sent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fm_command_sent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outp_query_after_shutdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_query_after_shutdown: Option<String>,
    pub trigger_reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OperatorApproval {
    pub schema_version: String,
    pub approved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub timestamp_unix_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StationSnapshotQuality {
    pub schema_version: String,
    pub status: String,
    pub eligible_for_extended_sweep: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub query_interrupted_seen: bool,
    pub smb_query_delay_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SafetyBoundaryNote {
    pub schema_version: String,
    pub real_smb100a_query_only: bool,
    pub real_smb100a_setting_commands_blocked_except_sweep: bool,
    pub rf_on_requires_manual_approval: bool,
    pub no_csv_policy: bool,
    pub no_internal_sweep: bool,
    pub no_gui_hardware_access: bool,
    pub no_magnetic_device_access: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MagneticNotInScope {
    pub magnetic_devices_in_scope: bool,
    pub magnetic_serial_enumeration_performed: bool,
    pub magnetic_commands_sent: usize,
    pub reason: String,
    pub known_verified_axis_sns: MagneticAxisSns,
    pub note: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MagneticAxisSns {
    pub x: String,
    pub y: String,
    pub z: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HashManifest {
    pub schema_version: String,
    pub extended_sweep_config_hash: String,
    pub smb100a_snapshot_before_hash: String,
    pub smb100a_snapshot_after_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimelineEvent {
    pub event_type: String,
    pub wall_time_utc: String,
    pub monotonic_ns: u64,
    pub monotonic_ns_since_run_start: u64,
    pub device_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Opaque aggregated result produced by run_sweep, consumed by app.rs.
#[derive(Debug)]
pub struct SweepInternalResult {
    pub passed: bool,
    pub snapshot_before: Smb100aSnapshot,
    pub snapshot_after: Smb100aSnapshot,
    pub oe_idn: String,
    pub audit: Vec<CommandAuditEntry>,
    pub preflight: PreflightCheck,
    pub step_results: Vec<RfStepResult>,
    pub forbidden_check: ForbiddenCommandCheck,
    pub emergency_shutdown: Option<EmergencyShutdownEvidence>,
    pub timeline: Vec<TimelineEvent>,
    pub total_frames_requested: usize,
    pub total_frames_captured: usize,
    pub total_frames_parsed: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}
