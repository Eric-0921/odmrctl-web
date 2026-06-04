//! Data types for the Maynuo M8812 identity probe tool.

use odmr_mag::MaynuoIdn;
use serde::{Deserialize, Serialize};

/// Per-port probe classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeClassification {
    MatchedAxis,
    NonTargetDevice,
    MalformedIdn,
    Timeout,
    IoError,
    DuplicateSn,
    UnknownMaynuoSn,
}

impl ProbeClassification {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MatchedAxis => "matched_axis",
            Self::NonTargetDevice => "non_target_device",
            Self::MalformedIdn => "malformed_idn",
            Self::Timeout => "timeout",
            Self::IoError => "io_error",
            Self::DuplicateSn => "duplicate_sn",
            Self::UnknownMaynuoSn => "unknown_maynuo_sn",
        }
    }
}

/// Result for a single probed port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortProbeResult {
    pub port_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb_serial_number: Option<String>,
    pub probe_attempted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idn_raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed: Option<MaynuoIdn>,
    pub classification: ProbeClassification,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_axis_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Per-axis mapping entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisMappingEntry {
    pub axis_id: String,
    pub expected_sn: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_sn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_idn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_port_path: Option<String>,
    pub matched: bool,
}

/// Mapping of all three axes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisMapping {
    pub x: AxisMappingEntry,
    pub y: AxisMappingEntry,
    pub z: AxisMappingEntry,
}

/// Top-level probe report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeReport {
    pub passed: bool,
    pub missing_axes: Vec<String>,
    pub duplicate_axes: Vec<String>,
    pub unknown_sn: Vec<String>,
    pub ports_scanned: usize,
    pub ports_responded: usize,
    pub ports_matched: usize,
    pub strict_mode: bool,
}

/// Identity snapshot containing all port observations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentitySnapshot {
    pub schema_version: String,
    pub observed_ports: Vec<PortProbeResult>,
}

/// Manifest for a probe run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeManifest {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub started_at_utc: String,
    pub completed_at_utc: String,
    pub profile_path: String,
    pub passed: bool,
    pub artifact_files: Vec<String>,
    pub only_idn_queries_sent: bool,
    pub no_current_commands_sent: bool,
    pub no_output_commands_sent: bool,
}

/// Single event for the JSONL timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeEvent {
    pub event_type: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
