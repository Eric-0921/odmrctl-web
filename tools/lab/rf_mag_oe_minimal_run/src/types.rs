//! Artifact types for Mag-M5A combined run.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Command audit
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandAuditEntry {
    pub seq: u64,
    pub timestamp_unix_ms: u64,
    pub device_id: String,
    pub command: String,
    pub command_class: String,
    pub allowed: bool,
    pub sent_to_transport: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_error: Option<String>,
    pub safety_relevant: bool,
}

// ---------------------------------------------------------------------------
// Timeline event
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedRunEvent {
    pub event_type: String,
    pub timestamp_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

// ---------------------------------------------------------------------------
// Device snapshots
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmbSnapshot {
    pub schema_version: String,
    pub idn: String,
    pub preflight_outp: String,
    pub preflight_mod: String,
    pub preflight_freq: String,
    pub preflight_pow: String,
    pub preflight_err: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OeSnapshot {
    pub schema_version: String,
    pub idn: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagSnapshot {
    pub schema_version: String,
    pub axis_id: String,
    pub expected_sn: String,
    pub observed_sn: String,
    pub idn: String,
    pub port_path: String,
    pub zero_readback_current_ma: f64,
    pub zero_readback_std_ma: f64,
    pub commanded_recur_current_ma: f64,
    pub measured_recur_current_ma: f64,
    pub measured_recur_field_nt: f64,
    pub current_error_ma: f64,
}

// ---------------------------------------------------------------------------
// Combined run report
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RfReportSection {
    pub requested_frequency_hz: u64,
    pub requested_power_dbm: f64,
    pub readback_frequency_hz: Option<f64>,
    pub readback_power_dbm: Option<f64>,
    pub rf_on_window_start_unix_ms: Option<u64>,
    pub rf_on_window_end_unix_ms: Option<u64>,
    pub rf_final_off: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagReportSection {
    pub axis_id: String,
    pub expected_sn: String,
    pub observed_sn: String,
    pub zero_readback_current_ma: f64,
    pub zero_readback_std_ma: f64,
    pub commanded_recur_current_ma: f64,
    pub measured_recur_current_ma: f64,
    pub measured_recur_field_nt: f64,
    pub current_error_ma: f64,
    pub mag_final_output_off: bool,
    pub mag_final_current_zero: bool,
    pub mag_final_local_requested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OeReportSection {
    pub frames_requested: u64,
    pub frames_acquired: u64,
    pub raw_bin_bytes: u64,
    pub frame_size_bytes: u64,
    pub parse_failures: u64,
    pub timeout_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineReportSection {
    pub rf_on_before_oe_capture: bool,
    pub mag_hold_before_oe_capture: bool,
    pub oe_capture_completed_before_cleanup: bool,
    pub cleanup_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedRunReport {
    pub schema_version: String,
    pub run_id: String,
    pub passed: bool,
    pub rf: RfReportSection,
    pub magnetic: MagReportSection,
    pub oe: OeReportSection,
    pub timeline: TimelineReportSection,
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub idn: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedRunManifest {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub run_id: String,
    pub started_at_utc: String,
    pub completed_at_utc: String,
    pub passed: bool,
    pub devices: CombinedRunDevices,
    pub artifact_files: Vec<String>,
    pub raw_first_contract_preserved: bool,
    pub rf_final_off: bool,
    pub mag_final_output_off: bool,
    pub mag_final_current_zero: bool,
    pub mag_final_local_requested: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedRunDevices {
    pub smb100a: DeviceIdentity,
    pub oe1022d: DeviceIdentity,
    pub maynuo: MaynuoDeviceIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaynuoDeviceIdentity {
    pub axis_id: String,
    pub idn: String,
    pub sn: String,
}
