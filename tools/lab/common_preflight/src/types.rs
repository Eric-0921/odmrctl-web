use serde::{Deserialize, Serialize};

/// Station profile loaded from JSON config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationProfile {
    pub name: String,
    pub devices: Vec<DeviceConfig>,
}

impl StationProfile {
    pub fn load(path: &str) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("read profile: {e}"))?;
        serde_json::from_str(&text)
            .map_err(|e| format!("parse profile: {e}"))
    }
}

/// Per-device configuration in the station profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub device_id: String,
    pub kind: String,
    pub transport: String,
    pub address: String,
    pub expected_sn: Option<String>,
    pub timeout_ms: Option<u64>,
}

/// Aggregate preflight report for the entire station.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationPreflightReport {
    pub schema_version: String,
    pub generated_at: String,
    pub station_profile: String,
    pub all_devices_reachable: bool,
    pub all_identities_verified: bool,
    pub all_safe_states_confirmed: bool,
    pub operator_approved: bool,
    pub elapsed_ms: u64,
    pub devices: Vec<DevicePreflightReport>,
}

impl StationPreflightReport {
    pub fn passed(&self) -> bool {
        self.all_devices_reachable
            && self.all_identities_verified
            && self.all_safe_states_confirmed
    }
}

/// Per-device preflight report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePreflightReport {
    pub device_id: String,
    pub kind: String,
    pub reachability: bool,
    pub identity_raw: Option<String>,
    pub identity_display: Option<String>,
    pub error_queue: Vec<String>,
    pub safe_state: Option<SafeState>,
    pub warnings: Vec<String>,
}

/// Safe state snapshot for a device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeState {
    pub confirmed: bool,
    pub rf_output: Option<String>,
    pub modulation: Option<String>,
    pub fm: Option<String>,
    pub magnetic_output: Option<String>,
    pub magnetic_current_ma: Option<f64>,
}
