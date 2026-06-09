//! odmr-config — canonical JSON configuration for ODMR stations and runtime.
//!
//! This crate is the single configuration entrypoint for:
//! - app/runtime defaults
//! - station/device topology
//! - manual-derived transport and safety defaults
//! - compatibility loading of legacy station profile JSON
//!
//! Manual sources fixed into the defaults below:
//! - `docs/equipment_manual/smb100a/05_remote_control_basics.md`
//! - `docs/equipment_manual/smb100a/06l_source_subsystem.md`
//! - `docs/equipment_manual/oe1022d/05_oe1022d_rall_global_data_config_reading.md`
//! - `docs/equipment_manual/oe1022d/02_fundamentals.md`
//! - `docs/equipment_manual/oe1022d/oe1022d_reference_signal_remote.md`
//! - `docs/equipment_manual/maynuo_dc-power-supply/m8812_remote_control_reference.md`
//! - `docs/equipment_manual/CNI Laser psu-sr/RS232语言协议_恒功率.md`

use odmr_types::DeviceId;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CURRENT_SCHEMA_VERSION: &str = "0.1.0";

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Validation(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Json(e) => write!(f, "json error: {e}"),
            Self::Validation(msg) => write!(f, "validation error: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Validation(_) => None,
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    #[serde(default = "schema_version")]
    pub schema_version: String,
    #[serde(default = "app_kind")]
    pub kind: String,
    #[serde(default = "default_run_root")]
    pub run_root: String,
    #[serde(default)]
    pub artifact_policy: ArtifactPolicy,
    #[serde(default)]
    pub replay_defaults: ReplayDefaults,
    #[serde(default)]
    pub feature_flags: FeatureFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArtifactPolicy {
    #[serde(default = "default_canonical_run_format")]
    pub canonical_run_format: String,
    #[serde(default = "default_true")]
    pub keep_events_jsonl: bool,
    #[serde(default = "default_true")]
    pub keep_index_jsonl: bool,
    #[serde(default = "default_true")]
    pub keep_step_scoped_rall_files: bool,
    #[serde(default = "default_true")]
    pub retain_legacy_rawbin_adapter: bool,
}

impl Default for ArtifactPolicy {
    fn default() -> Self {
        Self {
            canonical_run_format: default_canonical_run_format(),
            keep_events_jsonl: true,
            keep_index_jsonl: true,
            keep_step_scoped_rall_files: true,
            retain_legacy_rawbin_adapter: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayDefaults {
    #[serde(default)]
    pub mode: ReplayModeDefault,
    #[serde(default = "default_replay_speed")]
    pub speed: f64,
    #[serde(default = "default_true")]
    pub allow_legacy_rawbin: bool,
}

impl Default for ReplayDefaults {
    fn default() -> Self {
        Self {
            mode: ReplayModeDefault::AsFastAsPossible,
            speed: default_replay_speed(),
            allow_legacy_rawbin: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplayModeDefault {
    #[default]
    AsFastAsPossible,
    OriginalTimestampPaced,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeatureFlags {
    #[serde(default = "default_true")]
    pub allow_hardware_runs: bool,
    #[serde(default = "default_true")]
    pub enable_replay_api: bool,
    #[serde(default = "default_true")]
    pub enable_workbench_default_values: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            allow_hardware_runs: true,
            enable_replay_api: true,
            enable_workbench_default_values: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StationConfig {
    #[serde(default = "schema_version")]
    pub schema_version: String,
    #[serde(default = "station_kind")]
    pub kind: String,
    #[serde(default)]
    pub station_id: String,
    pub name: String,
    pub devices: Vec<StationDeviceConfig>,
    #[serde(default)]
    pub safety: StationSafetyConfig,
    #[serde(default)]
    pub cleanup: StationCleanupPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StationDeviceConfig {
    pub device_id: String,
    pub device_type: String,
    pub transport: DeviceTransportConfig,
    #[serde(default)]
    pub identity: DeviceIdentityConfig,
    #[serde(default = "default_true")]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeviceTransportConfig {
    TcpScpi {
        host: String,
        #[serde(default = "default_smb_port")]
        port: u16,
        #[serde(default = "default_timeout_ms")]
        timeout_ms: u64,
    },
    Serial {
        port: String,
        #[serde(default = "default_serial_baud")]
        baud_rate: u32,
        #[serde(default = "default_data_bits")]
        data_bits: u8,
        #[serde(default = "default_stop_bits")]
        stop_bits: u8,
        #[serde(default = "default_parity")]
        parity: String,
        #[serde(default)]
        dtr: bool,
        #[serde(default = "default_timeout_ms")]
        timeout_ms: u64,
        #[serde(default)]
        line_terminator: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DeviceIdentityConfig {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub expected_contains: Vec<String>,
    #[serde(default)]
    pub expected_sn: Option<String>,
    #[serde(default)]
    pub manual_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StationSafetyConfig {
    #[serde(default = "default_smb_max_power_dbm")]
    pub smb100a_max_power_dbm: f64,
    #[serde(default = "default_smb_min_freq_hz")]
    pub smb100a_min_freq_hz: f64,
    #[serde(default = "default_smb_max_freq_hz")]
    pub smb100a_max_freq_hz: f64,
    #[serde(default = "default_mag_max_current_a")]
    pub mag_max_current_a_per_axis: f64,
    #[serde(default = "default_laser_max_power_mw")]
    pub laser_max_power_mw: u16,
}

impl Default for StationSafetyConfig {
    fn default() -> Self {
        Self {
            smb100a_max_power_dbm: default_smb_max_power_dbm(),
            smb100a_min_freq_hz: default_smb_min_freq_hz(),
            smb100a_max_freq_hz: default_smb_max_freq_hz(),
            mag_max_current_a_per_axis: default_mag_max_current_a(),
            laser_max_power_mw: default_laser_max_power_mw(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StationCleanupPolicy {
    #[serde(default = "default_true")]
    pub smb_rf_off: bool,
    #[serde(default = "default_true")]
    pub smb_modulation_off: bool,
    #[serde(default = "default_true")]
    pub smb_fm_off: bool,
    #[serde(default = "default_true")]
    pub laser_emergency_off: bool,
    #[serde(default = "default_true")]
    pub maynuo_zero_current_before_output_off: bool,
    #[serde(default = "default_maynuo_cleanup_wait_ms")]
    pub maynuo_cleanup_wait_ms: u64,
}

impl Default for StationCleanupPolicy {
    fn default() -> Self {
        Self {
            smb_rf_off: true,
            smb_modulation_off: true,
            smb_fm_off: true,
            laser_emergency_off: true,
            maynuo_zero_current_before_output_off: true,
            maynuo_cleanup_wait_ms: default_maynuo_cleanup_wait_ms(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DeviceDefaults {
    #[serde(default)]
    pub smb100a: Smb100aDefaults,
    #[serde(default)]
    pub oe1022d: Oe1022dDefaults,
    #[serde(default)]
    pub maynuo_m8812: MaynuoM8812Defaults,
    #[serde(default)]
    pub cni_laser: CniLaserDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Smb100aDefaults {
    #[serde(default = "default_smb_port")]
    pub raw_socket_port: u16,
    #[serde(default = "default_true")]
    pub require_remote_scpi_session: bool,
    #[serde(default = "default_true")]
    pub cleanup_rf_output_off: bool,
    #[serde(default = "default_true")]
    pub cleanup_modulation_off: bool,
    #[serde(default = "default_true")]
    pub cleanup_fm_off: bool,
    #[serde(default = "default_smb_lf_voltage_is_peak")]
    pub lf_voltage_is_peak: bool,
    #[serde(default = "default_smb_reference_frequency_hz")]
    pub default_lf_reference_frequency_hz: f64,
}

impl Default for Smb100aDefaults {
    fn default() -> Self {
        Self {
            raw_socket_port: default_smb_port(),
            require_remote_scpi_session: true,
            cleanup_rf_output_off: true,
            cleanup_modulation_off: true,
            cleanup_fm_off: true,
            lf_voltage_is_peak: default_smb_lf_voltage_is_peak(),
            default_lf_reference_frequency_hz: default_smb_reference_frequency_hz(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Oe1022dDefaults {
    #[serde(default = "default_oe_baud")]
    pub baud_rate: u32,
    #[serde(default = "default_rall_frame_bytes")]
    pub rall_frame_bytes: usize,
    #[serde(default = "default_oe_frame_points")]
    pub frame_points: usize,
    #[serde(default = "default_oe_refresh_ms")]
    pub refresh_ms: u64,
    #[serde(default = "default_supported_acquisition_transport")]
    pub acquisition_transport: String,
    #[serde(default)]
    pub pll_reference: Oe1022dPllReferenceDefaults,
}

impl Default for Oe1022dDefaults {
    fn default() -> Self {
        Self {
            baud_rate: default_oe_baud(),
            rall_frame_bytes: default_rall_frame_bytes(),
            frame_points: default_oe_frame_points(),
            refresh_ms: default_oe_refresh_ms(),
            acquisition_transport: default_supported_acquisition_transport(),
            pll_reference: Oe1022dPllReferenceDefaults::default(),
        }
    }
}

/// OE1022D PLL reference thresholds copied from the equipment manual.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Oe1022dPllReferenceDefaults {
    #[serde(default = "default_oe_reference_channel")]
    pub channel: u8,
    #[serde(default)]
    pub source: Oe1022dReferenceSource,
    #[serde(default)]
    pub external_trigger: Oe1022dExternalTrigger,
    #[serde(default = "default_oe_ttl_high_min_v")]
    pub ttl_high_min_v: f64,
    #[serde(default = "default_oe_ttl_low_max_v")]
    pub ttl_low_max_v: f64,
    #[serde(default = "default_oe_sine_min_vpp")]
    pub sine_min_vpp: f64,
    #[serde(default = "default_oe_ttl_required_below_hz")]
    pub ttl_required_below_hz: f64,
}

impl Default for Oe1022dPllReferenceDefaults {
    fn default() -> Self {
        Self {
            channel: default_oe_reference_channel(),
            source: Oe1022dReferenceSource::External,
            external_trigger: Oe1022dExternalTrigger::TtlRisingEdge,
            ttl_high_min_v: default_oe_ttl_high_min_v(),
            ttl_low_max_v: default_oe_ttl_low_max_v(),
            sine_min_vpp: default_oe_sine_min_vpp(),
            ttl_required_below_hz: default_oe_ttl_required_below_hz(),
        }
    }
}

/// Manual-derived OE1022D reference source mode.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Oe1022dReferenceSource {
    #[default]
    External,
    Internal,
    InternalSweep,
}

/// Manual-derived OE1022D external reference trigger mode.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Oe1022dExternalTrigger {
    #[default]
    TtlRisingEdge,
    SineZeroCrossing,
}

/// SMB100A LF output shape used when the LF output is physically wired as OE reference.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Smb100aLfShape {
    #[default]
    Sine,
    Square,
    Triangle,
    Sawtooth,
    InvertedSawtooth,
}

/// Pure-software contract for determining whether an OE1022D external reference can PLL lock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PllReferenceContract {
    pub reference_frequency_hz: f64,
    pub oe_reference_source: Oe1022dReferenceSource,
    pub oe_external_trigger: Oe1022dExternalTrigger,
    pub source_signal: ReferenceSignalContract,
}

/// Reference signal source model used by `validate_pll_reference_contract`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReferenceSignalContract {
    Ttl {
        high_v: f64,
        low_v: f64,
    },
    Sine {
        vpp: f64,
    },
    Smb100aLfOutput {
        voltage_peak_v: f64,
        shape: Smb100aLfShape,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaynuoM8812Defaults {
    #[serde(default = "default_maynuo_baud")]
    pub baud_rate: u32,
    #[serde(default = "default_data_bits")]
    pub data_bits: u8,
    #[serde(default = "default_stop_bits")]
    pub stop_bits: u8,
    #[serde(default = "default_parity")]
    pub parity: String,
    #[serde(default = "default_line_feed")]
    pub line_terminator: String,
    #[serde(default = "default_true")]
    pub dtr: bool,
    #[serde(default = "default_maynuo_remote_command")]
    pub remote_command: String,
    #[serde(default = "default_maynuo_local_command")]
    pub local_command: String,
    #[serde(default = "default_maynuo_current_command")]
    pub set_current_command: String,
    #[serde(default = "default_maynuo_output_command")]
    pub set_output_command: String,
    #[serde(default = "default_maynuo_meas_current_command")]
    pub measure_current_command: String,
}

impl Default for MaynuoM8812Defaults {
    fn default() -> Self {
        Self {
            baud_rate: default_maynuo_baud(),
            data_bits: default_data_bits(),
            stop_bits: default_stop_bits(),
            parity: default_parity(),
            line_terminator: default_line_feed(),
            dtr: true,
            remote_command: default_maynuo_remote_command(),
            local_command: default_maynuo_local_command(),
            set_current_command: default_maynuo_current_command(),
            set_output_command: default_maynuo_output_command(),
            measure_current_command: default_maynuo_meas_current_command(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CniLaserDefaults {
    #[serde(default = "default_laser_baud")]
    pub baud_rate: u32,
    #[serde(default = "default_data_bits")]
    pub data_bits: u8,
    #[serde(default = "default_stop_bits")]
    pub stop_bits: u8,
    #[serde(default = "default_parity")]
    pub parity: String,
    #[serde(default = "default_laser_max_power_mw")]
    pub software_max_power_mw: u16,
    #[serde(default = "default_true")]
    pub require_emergency_off_cleanup: bool,
    #[serde(default = "default_true")]
    pub require_interlock_confirmation: bool,
}

impl Default for CniLaserDefaults {
    fn default() -> Self {
        Self {
            baud_rate: default_laser_baud(),
            data_bits: default_data_bits(),
            stop_bits: default_stop_bits(),
            parity: default_parity(),
            software_max_power_mw: default_laser_max_power_mw(),
            require_emergency_off_cleanup: true,
            require_interlock_confirmation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectiveRuntimeDefaults {
    pub app: AppConfig,
    pub station: StationConfig,
    pub device_defaults: DeviceDefaults,
}

#[derive(Debug, Deserialize)]
struct LegacyStationProfile {
    pub name: String,
    pub devices: Vec<LegacyDeviceConfig>,
    #[serde(default)]
    pub safety: Option<StationSafetyConfig>,
}

#[derive(Debug, Deserialize)]
struct LegacyDeviceConfig {
    pub device_id: String,
    pub kind: String,
    pub transport: String,
    pub address: String,
    pub expected_sn: Option<String>,
    pub timeout_ms: Option<u64>,
}

pub fn load_app_config(path: impl AsRef<Path>) -> Result<AppConfig, ConfigError> {
    let text = fs::read_to_string(path)?;
    let config: AppConfig = serde_json::from_str(&text)?;
    validate_app_config(&config)?;
    Ok(config)
}

pub fn load_station_config(path: impl AsRef<Path>) -> Result<StationConfig, ConfigError> {
    let text = fs::read_to_string(path)?;
    load_station_config_str(&text)
}

pub fn load_station_config_str(text: &str) -> Result<StationConfig, ConfigError> {
    if let Ok(config) = serde_json::from_str::<StationConfig>(text) {
        validate_station_config(&config)?;
        return Ok(config);
    }

    let legacy: LegacyStationProfile = serde_json::from_str(text)?;
    let config = legacy_to_station_config(legacy);
    validate_station_config(&config)?;
    Ok(config)
}

pub fn load_device_defaults(path: impl AsRef<Path>) -> Result<DeviceDefaults, ConfigError> {
    let text = fs::read_to_string(path)?;
    let defaults: DeviceDefaults = serde_json::from_str(&text)?;
    validate_device_defaults(&defaults)?;
    Ok(defaults)
}

pub fn validate_app_config(config: &AppConfig) -> Result<(), ConfigError> {
    if config.run_root.trim().is_empty() {
        return Err(ConfigError::Validation(
            "app.run_root must not be empty".into(),
        ));
    }
    if config.replay_defaults.speed <= 0.0 {
        return Err(ConfigError::Validation(
            "app.replay_defaults.speed must be > 0".into(),
        ));
    }
    Ok(())
}

pub fn validate_station_config(config: &StationConfig) -> Result<(), ConfigError> {
    if config.name.trim().is_empty() {
        return Err(ConfigError::Validation(
            "station.name must not be empty".into(),
        ));
    }
    if config.devices.is_empty() {
        return Err(ConfigError::Validation(
            "station.devices must not be empty".into(),
        ));
    }
    for device in &config.devices {
        DeviceId::new(device.device_id.replace('.', "_"))
            .validate()
            .map_err(|e| ConfigError::Validation(format!("invalid device_id: {e}")))?;
        validate_station_device(device)?;
    }
    if config.safety.laser_max_power_mw > default_laser_max_power_mw() {
        return Err(ConfigError::Validation(format!(
            "station.safety.laser_max_power_mw exceeds manual-derived ceiling {} mW",
            default_laser_max_power_mw()
        )));
    }
    Ok(())
}

pub fn validate_device_defaults(defaults: &DeviceDefaults) -> Result<(), ConfigError> {
    if defaults.smb100a.raw_socket_port != 5025 {
        return Err(ConfigError::Validation(
            "SMB100A manual default raw socket port must remain 5025".into(),
        ));
    }
    if defaults.oe1022d.rall_frame_bytes != 12288 {
        return Err(ConfigError::Validation(
            "OE1022D RALL frame size must remain 12288 bytes".into(),
        ));
    }
    if defaults.oe1022d.frame_points != 50 {
        return Err(ConfigError::Validation(
            "OE1022D RALL frame point count must remain 50".into(),
        ));
    }
    validate_oe_pll_reference_defaults(&defaults.oe1022d.pll_reference)?;
    if defaults.maynuo_m8812.baud_rate != 9600 {
        return Err(ConfigError::Validation(
            "Maynuo M8812 default baud must remain 9600".into(),
        ));
    }
    if defaults.cni_laser.baud_rate != 9600 {
        return Err(ConfigError::Validation(
            "CNI laser default baud must remain 9600".into(),
        ));
    }
    Ok(())
}

pub fn validate_pll_reference_contract(contract: &PllReferenceContract) -> Result<(), ConfigError> {
    let defaults = Oe1022dPllReferenceDefaults::default();
    if contract.reference_frequency_hz <= 0.0 {
        return Err(ConfigError::Validation(
            "PLL reference frequency must be > 0 Hz".into(),
        ));
    }
    if contract.oe_reference_source != Oe1022dReferenceSource::External {
        return Err(ConfigError::Validation(
            "OE1022D PLL lock requires external reference mode (FMODD 2,0); internal reference does not use PLL"
                .into(),
        ));
    }
    if contract.reference_frequency_hz < defaults.ttl_required_below_hz
        && contract.oe_external_trigger != Oe1022dExternalTrigger::TtlRisingEdge
    {
        return Err(ConfigError::Validation(format!(
            "OE1022D reference below {} Hz must use TTL trigger",
            defaults.ttl_required_below_hz
        )));
    }

    match &contract.source_signal {
        ReferenceSignalContract::Ttl { high_v, low_v } => {
            if contract.oe_external_trigger != Oe1022dExternalTrigger::TtlRisingEdge {
                return Err(ConfigError::Validation(
                    "TTL reference signal must use OE1022D TTL rising-edge trigger".into(),
                ));
            }
            if *high_v <= defaults.ttl_high_min_v || *low_v >= defaults.ttl_low_max_v {
                return Err(ConfigError::Validation(format!(
                    "OE1022D TTL reference requires high > {} V and low < {} V",
                    defaults.ttl_high_min_v, defaults.ttl_low_max_v
                )));
            }
        }
        ReferenceSignalContract::Sine { vpp } => {
            if contract.oe_external_trigger != Oe1022dExternalTrigger::SineZeroCrossing {
                return Err(ConfigError::Validation(
                    "sine reference signal must use OE1022D sine zero-crossing trigger".into(),
                ));
            }
            if *vpp <= defaults.sine_min_vpp {
                return Err(ConfigError::Validation(format!(
                    "OE1022D sine reference requires amplitude > {} Vpp",
                    defaults.sine_min_vpp
                )));
            }
        }
        ReferenceSignalContract::Smb100aLfOutput {
            voltage_peak_v,
            shape,
        } => validate_smb_lf_reference(*voltage_peak_v, *shape, contract.oe_external_trigger)?,
    }

    Ok(())
}

pub fn resolve_runtime_defaults(
    app: Option<AppConfig>,
    station: StationConfig,
    device_defaults: Option<DeviceDefaults>,
) -> Result<EffectiveRuntimeDefaults, ConfigError> {
    let app = app.unwrap_or_else(default_app_config);
    let device_defaults = device_defaults.unwrap_or_default();
    validate_app_config(&app)?;
    validate_station_config(&station)?;
    validate_device_defaults(&device_defaults)?;
    Ok(EffectiveRuntimeDefaults {
        app,
        station,
        device_defaults,
    })
}

pub fn default_app_config() -> AppConfig {
    AppConfig {
        schema_version: schema_version(),
        kind: app_kind(),
        run_root: default_run_root(),
        artifact_policy: ArtifactPolicy::default(),
        replay_defaults: ReplayDefaults::default(),
        feature_flags: FeatureFlags::default(),
    }
}

fn legacy_to_station_config(legacy: LegacyStationProfile) -> StationConfig {
    let devices = legacy
        .devices
        .into_iter()
        .map(|device| {
            let transport = match device.transport.as_str() {
                "tcp" | "tcp_scpi" | "raw_socket" => {
                    let (host, port) = split_host_port(&device.address);
                    DeviceTransportConfig::TcpScpi {
                        host,
                        port: port.unwrap_or(default_smb_port()),
                        timeout_ms: device.timeout_ms.unwrap_or(default_timeout_ms()),
                    }
                }
                _ => {
                    let baud = match normalize_device_type(&device.kind).as_str() {
                        "oe1022d" => default_oe_baud(),
                        "laser" => default_laser_baud(),
                        _ => default_maynuo_baud(),
                    };
                    let dtr = normalize_device_type(&device.kind) == "magnet_xyz";
                    DeviceTransportConfig::Serial {
                        port: device.address,
                        baud_rate: baud,
                        data_bits: 8,
                        stop_bits: 1,
                        parity: "none".into(),
                        dtr,
                        timeout_ms: device.timeout_ms.unwrap_or(default_timeout_ms()),
                        line_terminator: if normalize_device_type(&device.kind) == "magnet_xyz" {
                            Some(default_line_feed())
                        } else {
                            None
                        },
                    }
                }
            };
            StationDeviceConfig {
                device_id: device.device_id,
                device_type: normalize_device_type(&device.kind),
                transport,
                identity: DeviceIdentityConfig {
                    query: default_identity_query_for_kind(&device.kind),
                    expected_contains: default_identity_contains_for_kind(
                        &device.kind,
                        device.expected_sn.clone(),
                    ),
                    expected_sn: device.expected_sn,
                    manual_verified: matches!(
                        normalize_device_type(&device.kind).as_str(),
                        "laser"
                    ),
                },
                required: true,
            }
        })
        .collect();

    StationConfig {
        schema_version: schema_version(),
        kind: station_kind(),
        station_id: legacy.name.clone(),
        name: legacy.name,
        devices,
        safety: legacy.safety.unwrap_or_default(),
        cleanup: StationCleanupPolicy::default(),
    }
}

fn validate_station_device(device: &StationDeviceConfig) -> Result<(), ConfigError> {
    match (
        &device.transport,
        normalize_device_type(&device.device_type).as_str(),
    ) {
        (DeviceTransportConfig::TcpScpi { port, .. }, "smb100a") if *port != 5025 => {
            return Err(ConfigError::Validation(format!(
                "{} must use SMB100A manual default port 5025",
                device.device_id
            )));
        }
        (
            DeviceTransportConfig::Serial {
                baud_rate,
                line_terminator,
                ..
            },
            "magnet_xyz",
        ) => {
            if *baud_rate != 9600 {
                return Err(ConfigError::Validation(format!(
                    "{} must use Maynuo 9600 8N1",
                    device.device_id
                )));
            }
            if line_terminator.as_deref() != Some("\n") {
                return Err(ConfigError::Validation(format!(
                    "{} must use Maynuo LF line terminator",
                    device.device_id
                )));
            }
        }
        (DeviceTransportConfig::Serial { baud_rate, .. }, "laser") if *baud_rate != 9600 => {
            return Err(ConfigError::Validation(format!(
                "{} must use CNI laser 9600 8N1",
                device.device_id
            )));
        }
        (DeviceTransportConfig::Serial { baud_rate, .. }, "oe1022d") if *baud_rate == 0 => {
            return Err(ConfigError::Validation(format!(
                "{} must declare a non-zero OE1022D baud rate",
                device.device_id
            )));
        }
        _ => {}
    }
    Ok(())
}

fn validate_oe_pll_reference_defaults(
    defaults: &Oe1022dPllReferenceDefaults,
) -> Result<(), ConfigError> {
    if defaults.channel != 2 {
        return Err(ConfigError::Validation(
            "OE1022D default ODMR PLL reference channel must remain Ch-B (2)".into(),
        ));
    }
    if defaults.source != Oe1022dReferenceSource::External {
        return Err(ConfigError::Validation(
            "OE1022D PLL defaults must use external reference".into(),
        ));
    }
    if defaults.ttl_high_min_v != 3.0 {
        return Err(ConfigError::Validation(
            "OE1022D TTL high threshold must remain 3.0 V".into(),
        ));
    }
    if defaults.ttl_low_max_v != 0.5 {
        return Err(ConfigError::Validation(
            "OE1022D TTL low threshold must remain 0.5 V".into(),
        ));
    }
    if defaults.sine_min_vpp != 0.4 {
        return Err(ConfigError::Validation(
            "OE1022D sine reference threshold must remain 0.4 Vpp".into(),
        ));
    }
    if defaults.ttl_required_below_hz != 1.0 {
        return Err(ConfigError::Validation(
            "OE1022D TTL-required-below threshold must remain 1 Hz".into(),
        ));
    }
    Ok(())
}

fn validate_smb_lf_reference(
    voltage_peak_v: f64,
    shape: Smb100aLfShape,
    oe_trigger: Oe1022dExternalTrigger,
) -> Result<(), ConfigError> {
    if voltage_peak_v <= 0.0 {
        return Err(ConfigError::Validation(
            "SMB100A LF reference voltage must be > 0 V".into(),
        ));
    }
    let defaults = Oe1022dPllReferenceDefaults::default();
    match oe_trigger {
        Oe1022dExternalTrigger::TtlRisingEdge => {
            if shape != Smb100aLfShape::Square {
                return Err(ConfigError::Validation(
                    "SMB100A LF TTL reference must use square waveform".into(),
                ));
            }
            if voltage_peak_v <= defaults.ttl_high_min_v {
                return Err(ConfigError::Validation(format!(
                    "SMB100A LF square reference at {voltage_peak_v} V peak cannot satisfy OE1022D TTL high > {} V",
                    defaults.ttl_high_min_v
                )));
            }
        }
        Oe1022dExternalTrigger::SineZeroCrossing => {
            if shape != Smb100aLfShape::Sine {
                return Err(ConfigError::Validation(
                    "SMB100A LF sine reference must use sine waveform for zero crossing".into(),
                ));
            }
            let vpp = 2.0 * voltage_peak_v;
            if vpp <= defaults.sine_min_vpp {
                return Err(ConfigError::Validation(format!(
                    "SMB100A LF sine reference at {vpp} Vpp cannot satisfy OE1022D sine > {} Vpp",
                    defaults.sine_min_vpp
                )));
            }
        }
    }
    Ok(())
}

fn normalize_device_type(input: &str) -> String {
    match input {
        "rf_source" => "smb100a",
        "lock_in" => "oe1022d",
        "magnetic" | "maynuo" => "magnet_xyz",
        "cni" | "cni_laser" => "laser",
        other => other,
    }
    .to_string()
}

fn split_host_port(address: &str) -> (String, Option<u16>) {
    if let Some((host, port)) = address.rsplit_once(':') {
        if let Ok(port) = port.parse::<u16>() {
            return (host.to_string(), Some(port));
        }
    }
    (address.to_string(), None)
}

fn default_identity_query_for_kind(kind: &str) -> Option<String> {
    match normalize_device_type(kind).as_str() {
        "laser" => None,
        _ => Some("*IDN?".into()),
    }
}

fn default_identity_contains_for_kind(kind: &str, expected_sn: Option<String>) -> Vec<String> {
    let mut values = match normalize_device_type(kind).as_str() {
        "smb100a" => vec!["ROHDE".into(), "SCHWARZ".into(), "SMB100A".into()],
        "oe1022d" => vec!["OE1022D".into()],
        "magnet_xyz" => vec!["MAYNUO".into(), "M8812".into()],
        "laser" => Vec::new(),
        _ => Vec::new(),
    };
    if let Some(sn) = expected_sn {
        values.push(sn);
    }
    values
}

fn schema_version() -> String {
    CURRENT_SCHEMA_VERSION.into()
}

fn app_kind() -> String {
    "app_config".into()
}

fn station_kind() -> String {
    "station_config".into()
}

fn default_run_root() -> String {
    "apps/desktop/src-tauri/target/odmr-runs".into()
}

fn default_canonical_run_format() -> String {
    "step_scoped_rall_v1".into()
}

fn default_replay_speed() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_smb_port() -> u16 {
    5025
}

fn default_timeout_ms() -> u64 {
    5000
}

fn default_serial_baud() -> u32 {
    9600
}

fn default_data_bits() -> u8 {
    8
}

fn default_stop_bits() -> u8 {
    1
}

fn default_parity() -> String {
    "none".into()
}

fn default_line_feed() -> String {
    "\n".into()
}

fn default_smb_max_power_dbm() -> f64 {
    -10.0
}

fn default_smb_min_freq_hz() -> f64 {
    2_800_000_000.0
}

fn default_smb_max_freq_hz() -> f64 {
    2_950_000_000.0
}

fn default_mag_max_current_a() -> f64 {
    0.1
}

fn default_laser_max_power_mw() -> u16 {
    150
}

fn default_maynuo_cleanup_wait_ms() -> u64 {
    500
}

fn default_oe_baud() -> u32 {
    921_600
}

fn default_rall_frame_bytes() -> usize {
    12_288
}

fn default_oe_frame_points() -> usize {
    50
}

fn default_oe_refresh_ms() -> u64 {
    50
}

fn default_supported_acquisition_transport() -> String {
    "usb_serial_rall".into()
}

fn default_smb_lf_voltage_is_peak() -> bool {
    true
}

fn default_smb_reference_frequency_hz() -> f64 {
    500.0
}

fn default_oe_reference_channel() -> u8 {
    2
}

fn default_oe_ttl_high_min_v() -> f64 {
    3.0
}

fn default_oe_ttl_low_max_v() -> f64 {
    0.5
}

fn default_oe_sine_min_vpp() -> f64 {
    0.4
}

fn default_oe_ttl_required_below_hz() -> f64 {
    1.0
}

fn default_maynuo_baud() -> u32 {
    9600
}

fn default_maynuo_remote_command() -> String {
    "SYST:REM".into()
}

fn default_maynuo_local_command() -> String {
    "SYST:LOC".into()
}

fn default_maynuo_current_command() -> String {
    "CURR".into()
}

fn default_maynuo_output_command() -> String {
    "OUTP".into()
}

fn default_maynuo_meas_current_command() -> String {
    "MEAS:CURR?".into()
}

fn default_laser_baud() -> u32 {
    9600
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_defaults_match_locked_values() {
        let defaults = DeviceDefaults::default();
        assert_eq!(defaults.smb100a.raw_socket_port, 5025);
        assert!(defaults.smb100a.lf_voltage_is_peak);
        assert_eq!(defaults.smb100a.default_lf_reference_frequency_hz, 500.0);
        assert_eq!(defaults.oe1022d.rall_frame_bytes, 12288);
        assert_eq!(defaults.oe1022d.frame_points, 50);
        assert_eq!(defaults.oe1022d.refresh_ms, 50);
        assert_eq!(defaults.oe1022d.pll_reference.channel, 2);
        assert_eq!(
            defaults.oe1022d.pll_reference.source,
            Oe1022dReferenceSource::External
        );
        assert_eq!(
            defaults.oe1022d.pll_reference.external_trigger,
            Oe1022dExternalTrigger::TtlRisingEdge
        );
        assert_eq!(defaults.oe1022d.pll_reference.ttl_high_min_v, 3.0);
        assert_eq!(defaults.oe1022d.pll_reference.ttl_low_max_v, 0.5);
        assert_eq!(defaults.oe1022d.pll_reference.sine_min_vpp, 0.4);
        assert_eq!(defaults.oe1022d.pll_reference.ttl_required_below_hz, 1.0);
        assert_eq!(defaults.maynuo_m8812.baud_rate, 9600);
        assert_eq!(defaults.maynuo_m8812.line_terminator, "\n");
        assert_eq!(defaults.cni_laser.baud_rate, 9600);
        assert_eq!(defaults.cni_laser.software_max_power_mw, 150);
    }

    #[test]
    fn legacy_station_profile_maps_to_canonical() {
        let input = r#"
        {
          "name": "nv_lab_with_laser",
          "devices": [
            {
              "device_id": "smb100a.main",
              "kind": "rf_source",
              "transport": "tcp",
              "address": "169.254.2.20:5025"
            },
            {
              "device_id": "mag.x",
              "kind": "magnetic",
              "transport": "serial",
              "address": "/dev/cu.PL2303G-USBtoUART1320",
              "expected_sn": "080020960220402020"
            }
          ],
          "safety": {
            "laser_max_power_mw": 100
          }
        }"#;
        let config = load_station_config_str(input).unwrap();
        assert_eq!(config.kind, "station_config");
        assert_eq!(config.devices.len(), 2);
        assert_eq!(config.devices[0].device_id, "smb100a.main");
        match &config.devices[0].transport {
            DeviceTransportConfig::TcpScpi { port, .. } => assert_eq!(*port, 5025),
            _ => panic!("expected tcp scpi transport"),
        }
        match &config.devices[1].transport {
            DeviceTransportConfig::Serial {
                baud_rate,
                line_terminator,
                ..
            } => {
                assert_eq!(*baud_rate, 9600);
                assert_eq!(line_terminator.as_deref(), Some("\n"));
            }
            _ => panic!("expected serial transport"),
        }
        assert_eq!(config.safety.laser_max_power_mw, 100);
    }

    #[test]
    fn canonical_station_config_validates_manual_constraints() {
        let config = StationConfig {
            schema_version: schema_version(),
            kind: station_kind(),
            station_id: "lab".into(),
            name: "lab".into(),
            devices: vec![StationDeviceConfig {
                device_id: "laser_main".into(),
                device_type: "laser".into(),
                transport: DeviceTransportConfig::Serial {
                    port: "/dev/cu.usbserial".into(),
                    baud_rate: 115200,
                    data_bits: 8,
                    stop_bits: 1,
                    parity: "none".into(),
                    dtr: false,
                    timeout_ms: 1000,
                    line_terminator: None,
                },
                identity: DeviceIdentityConfig::default(),
                required: true,
            }],
            safety: StationSafetyConfig::default(),
            cleanup: StationCleanupPolicy::default(),
        };
        let err = validate_station_config(&config).unwrap_err().to_string();
        assert!(err.contains("9600 8N1"));
    }

    #[test]
    fn current_smb_lf_137mv_square_cannot_satisfy_oe_ttl_lock() {
        let contract = PllReferenceContract {
            reference_frequency_hz: 500.0,
            oe_reference_source: Oe1022dReferenceSource::External,
            oe_external_trigger: Oe1022dExternalTrigger::TtlRisingEdge,
            source_signal: ReferenceSignalContract::Smb100aLfOutput {
                voltage_peak_v: 0.137,
                shape: Smb100aLfShape::Square,
            },
        };
        let err = validate_pll_reference_contract(&contract)
            .unwrap_err()
            .to_string();
        assert!(err.contains("cannot satisfy OE1022D TTL high"));
    }

    #[test]
    fn oe_ttl_reference_contract_accepts_manual_thresholds() {
        let contract = PllReferenceContract {
            reference_frequency_hz: 500.0,
            oe_reference_source: Oe1022dReferenceSource::External,
            oe_external_trigger: Oe1022dExternalTrigger::TtlRisingEdge,
            source_signal: ReferenceSignalContract::Ttl {
                high_v: 5.0,
                low_v: 0.0,
            },
        };
        validate_pll_reference_contract(&contract).unwrap();
    }

    #[test]
    fn oe_sine_reference_contract_rejects_below_400mvpp() {
        let contract = PllReferenceContract {
            reference_frequency_hz: 500.0,
            oe_reference_source: Oe1022dReferenceSource::External,
            oe_external_trigger: Oe1022dExternalTrigger::SineZeroCrossing,
            source_signal: ReferenceSignalContract::Sine { vpp: 0.3 },
        };
        let err = validate_pll_reference_contract(&contract)
            .unwrap_err()
            .to_string();
        assert!(err.contains("sine reference requires amplitude"));
    }

    #[test]
    fn oe_reference_below_1hz_requires_ttl() {
        let contract = PllReferenceContract {
            reference_frequency_hz: 0.5,
            oe_reference_source: Oe1022dReferenceSource::External,
            oe_external_trigger: Oe1022dExternalTrigger::SineZeroCrossing,
            source_signal: ReferenceSignalContract::Sine { vpp: 1.0 },
        };
        let err = validate_pll_reference_contract(&contract)
            .unwrap_err()
            .to_string();
        assert!(err.contains("below 1 Hz must use TTL"));
    }
}
