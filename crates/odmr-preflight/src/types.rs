use odmr_config::{DeviceTransportConfig, StationConfig};
use serde::{Deserialize, Serialize};

/// Station profile loaded from JSON config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationProfile {
    pub name: String,
    pub devices: Vec<DeviceConfig>,
}

impl StationProfile {
    pub fn load(path: &str) -> Result<Self, String> {
        let config = odmr_config::load_station_config(path)
            .map_err(|e| format!("load station config: {e}"))?;
        Ok(Self::from(config))
    }
}

impl From<StationConfig> for StationProfile {
    fn from(value: StationConfig) -> Self {
        Self {
            name: value.name,
            devices: value.devices.into_iter().map(DeviceConfig::from).collect(),
        }
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

impl From<odmr_config::StationDeviceConfig> for DeviceConfig {
    fn from(value: odmr_config::StationDeviceConfig) -> Self {
        let (transport, address, timeout_ms) = match value.transport {
            DeviceTransportConfig::TcpScpi {
                host,
                port,
                timeout_ms,
            } => (
                "tcp".to_string(),
                format!("{host}:{port}"),
                Some(timeout_ms),
            ),
            DeviceTransportConfig::Serial {
                port, timeout_ms, ..
            } => ("serial".to_string(), port, Some(timeout_ms)),
        };
        Self {
            device_id: value.device_id,
            kind: value.device_type,
            transport,
            address,
            expected_sn: value.identity.expected_sn,
            timeout_ms,
        }
    }
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
    pub lock_status: Vec<DeviceLockStatus>,
}

impl StationPreflightReport {
    pub fn passed(&self) -> bool {
        self.all_devices_reachable && self.all_identities_verified && self.all_safe_states_confirmed
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands_sent: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub laser_on_sent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonzero_power_sent: Option<bool>,
}

/// Device lock acquisition status for a single device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLockStatus {
    pub device_id: String,
    pub acquired: bool,
    pub lock_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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

/// Classification of a probe's behavior for safety auditing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeClass {
    /// Sends only identity query (*IDN?). No state change.
    IdentityOnly,
    /// Queries state but never sends set commands.
    QueryOnly,
    /// Writes commands only to establish or verify safe/known state.
    SafeStateProbe,
    /// Writes bounded operational parameters under safety limits.
    SafeWriteProbe,
    /// Requires explicit operator approval before execution.
    OperatorApprovedProbe,
}
