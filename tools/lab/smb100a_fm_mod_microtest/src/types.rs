//! JSON-serde types and result structs for the M3.1 FM/MOD micro-test tool.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MicrotestConfig {
    pub schema_version: String,
    pub smb_host: String,
    pub smb_port: u16,
    pub smb_query_delay_ms: u64,
    pub smb_timeout_ms: u64,
    pub rf_frequency_hz: f64,
    pub rf_power_dbm: f64,
    pub max_rf_power_dbm: f64,
    pub fm_deviation_hz: f64,
    pub max_fm_deviation_hz: f64,
    pub fm_on_duration_ms: u64,
    pub set_internal_lf: bool,
    pub lf_frequency_hz: f64,
    pub lf_shape: String,
    pub lf_voltage_v: f64,
    pub operator_approves_fm_mod_on: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_approval_note: Option<String>,
    pub leave_fm_config_enabled: bool,
    pub created_at_unix_ms: u64,
}

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
pub struct FmModResult {
    pub passed: bool,
    pub rf_on_command_sent: bool,
    pub rf_off_command_sent: bool,
    pub rf_output_confirmed_on: bool,
    pub rf_output_confirmed_off_after: bool,

    pub mod_on_command_sent: bool,
    pub mod_off_command_sent: bool,
    pub modulation_confirmed_on: bool,
    pub modulation_confirmed_off_after: bool,

    pub fm_enabled: bool,
    pub fm_disabled_after: bool,
    pub fm_source_requested: String,
    pub fm_source_verified: String,
    pub fm_deviation_hz_requested: f64,
    pub fm_deviation_hz_verified: f64,

    pub frequency_hz_requested: f64,
    pub frequency_hz_verified: f64,
    pub power_dbm_requested: f64,
    pub power_dbm_verified: f64,

    pub lf_frequency_hz_requested: f64,
    pub lf_frequency_hz_verified: f64,
    pub lf_shape_requested: String,
    pub lf_shape_verified: String,
    pub lf_voltage_v_requested: f64,
    pub lf_voltage_v_verified: f64,
    pub lf_output_was_not_enabled: bool,

    pub magnetic_devices_touched: bool,
    pub magnetic_commands_sent: usize,

    pub fm_on_duration_ms_requested: u64,
    pub fm_on_duration_ms_measured: u64,

    pub syst_err_before: Vec<ErrorQueueObservation>,
    pub syst_err_after: Vec<ErrorQueueObservation>,
    pub forbidden_commands_sent: usize,
    pub emergency_shutdown_attempted: bool,
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreflightCheck {
    pub passed: bool,
    pub outp_off_before: bool,
    pub mod_stat_off_before: bool,
    pub error_queue_clean_before: bool,
    pub operator_approval_present: bool,
    pub power_within_limit: bool,
    pub fm_deviation_within_limit: bool,
    pub duration_within_limit: bool,
    pub no_magnetic_serial_enumeration: bool,
    pub no_magnetic_commands: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
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
    pub eligible_for_fm_mod_microtest: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub query_interrupted_seen: bool,
    pub smb_query_delay_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SafetyBoundaryNote {
    pub schema_version: String,
    pub real_smb100a_query_only: bool,
    pub real_smb100a_setting_commands_blocked_except_microtest: bool,
    pub rf_on_requires_manual_approval: bool,
    pub no_csv_policy: bool,
    pub no_sweep: bool,
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
    pub smb100a_fm_mod_microtest_config_hash: String,
    pub smb100a_snapshot_before_hash: String,
    pub smb100a_snapshot_during_hash: String,
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

/// Internal aggregate result produced by `run_microtest`.
#[derive(Debug)]
pub struct MicrotestResult {
    pub snapshot_before: Smb100aSnapshot,
    pub snapshot_during: Option<Smb100aSnapshot>,
    pub snapshot_after: Smb100aSnapshot,
    pub audit: Vec<CommandAuditEntry>,
    pub preflight: PreflightCheck,
    pub fm_mod_result: FmModResult,
    pub forbidden_check: ForbiddenCommandCheck,
    pub timeline: Vec<TimelineEvent>,
    pub operator_approval: Option<OperatorApproval>,
    pub emergency_shutdown: Option<EmergencyShutdownEvidence>,
    pub magnetic_not_in_scope: MagneticNotInScope,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}
