//! Strongly-typed device configuration parameters for system-scan recipes.
//!
//! Each device kind (SMB100A, OE1022D, Magnetic, Laser) has a dedicated config
//! struct that can be parsed from the loose `serde_json::Value` stored in
//! `SystemScanRecipe::fixed_params`.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// SMB100A
// ---------------------------------------------------------------------------

/// Full SMB100A configuration for a system-scan recipe.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct Smb100aConfig {
    #[serde(default)]
    pub rf: Smb100aRfConfig,
    #[serde(default)]
    pub modulation: Smb100aModulationConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fm: Option<Smb100aFmConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lf: Option<Smb100aLfConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Smb100aRfConfig {
    pub frequency_hz: f64,
    pub power_dbm: f64,
    #[serde(default)]
    pub output_enabled: bool,
    #[serde(default = "default_frequency_mode")]
    pub frequency_mode: Smb100aFrequencyMode,
    #[serde(default = "default_alc_state")]
    pub alc_state: Smb100aAlcState,
}

impl Default for Smb100aRfConfig {
    fn default() -> Self {
        Self {
            frequency_hz: 2_882_000_000.0,
            power_dbm: -30.0,
            output_enabled: false,
            frequency_mode: Smb100aFrequencyMode::Cw,
            alc_state: Smb100aAlcState::Auto,
        }
    }
}

fn default_frequency_mode() -> Smb100aFrequencyMode {
    Smb100aFrequencyMode::Cw
}

fn default_alc_state() -> Smb100aAlcState {
    Smb100aAlcState::Auto
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct Smb100aModulationConfig {
    #[serde(default)]
    pub global_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Smb100aFmConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_fm_source")]
    pub source: Smb100aFmSource,
    #[serde(default = "default_fm_mode")]
    pub mode: Smb100aFmMode,
    pub deviation_hz: f64,
}

impl Default for Smb100aFmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            source: Smb100aFmSource::Internal,
            mode: Smb100aFmMode::Normal,
            deviation_hz: 0.0,
        }
    }
}

fn default_fm_source() -> Smb100aFmSource {
    Smb100aFmSource::Internal
}

fn default_fm_mode() -> Smb100aFmMode {
    Smb100aFmMode::Normal
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Smb100aLfConfig {
    #[serde(default)]
    pub output_enabled: bool,
    pub frequency_hz: f64,
    #[serde(default = "default_lf_shape")]
    pub shape: Smb100aLfShape,
    pub voltage_v: f64,
    #[serde(default = "default_lf_impedance")]
    pub source_impedance: Smb100aLfImpedance,
}

impl Default for Smb100aLfConfig {
    fn default() -> Self {
        Self {
            output_enabled: false,
            frequency_hz: 500.0,
            shape: Smb100aLfShape::Sine,
            voltage_v: 0.0,
            source_impedance: Smb100aLfImpedance::Low,
        }
    }
}

fn default_lf_shape() -> Smb100aLfShape {
    Smb100aLfShape::Sine
}

fn default_lf_impedance() -> Smb100aLfImpedance {
    Smb100aLfImpedance::Low
}

// --- SMB100A enums ---

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Smb100aFrequencyMode {
    #[default]
    Cw,
    Sweep,
    List,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Smb100aAlcState {
    #[default]
    Auto,
    On,
    Off,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Smb100aFmSource {
    #[default]
    Internal,
    External,
    #[serde(rename = "INTERNAL_EXTERNAL")]
    InternalExternal,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Smb100aFmMode {
    #[default]
    Normal,
    #[serde(rename = "HIGH_DEVIATION")]
    HighDeviation,
    #[serde(rename = "LOW_NOISE")]
    LowNoise,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Smb100aLfShape {
    #[default]
    Sine,
    Square,
    Triangle,
    Sawtooth,
    #[serde(rename = "INVERTED_SAWTOOTH")]
    InvertedSawtooth,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Smb100aLfImpedance {
    #[default]
    Low,
    #[serde(rename = "G600")]
    G600,
}

// ---------------------------------------------------------------------------
// OE1022D
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Oe1022dConfig {
    #[serde(default = "default_primary_channel")]
    pub primary_channel: Oe1022dChannel,
    #[serde(default = "default_primary_value")]
    pub primary_value: Oe1022dPrimaryValue,
    #[serde(default)]
    pub input: Oe1022dInputConfig,
    #[serde(default)]
    pub reference: Oe1022dReferenceConfig,
    #[serde(default)]
    pub gain: Oe1022dGainConfig,
    #[serde(default)]
    pub filter: Oe1022dFilterConfig,
    #[serde(default)]
    pub harmonic: Oe1022dHarmonicConfig,
    #[serde(default)]
    pub acquisition: Oe1022dAcquisitionConfig,
}

impl Default for Oe1022dConfig {
    fn default() -> Self {
        Self {
            primary_channel: Oe1022dChannel::B,
            primary_value: Oe1022dPrimaryValue::X,
            input: Oe1022dInputConfig::default(),
            reference: Oe1022dReferenceConfig::default(),
            gain: Oe1022dGainConfig::default(),
            filter: Oe1022dFilterConfig::default(),
            harmonic: Oe1022dHarmonicConfig::default(),
            acquisition: Oe1022dAcquisitionConfig::default(),
        }
    }
}

fn default_primary_channel() -> Oe1022dChannel {
    Oe1022dChannel::B
}

fn default_primary_value() -> Oe1022dPrimaryValue {
    Oe1022dPrimaryValue::X
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Oe1022dInputConfig {
    #[serde(default = "default_input_source")]
    pub source: Oe1022dInputSource,
    #[serde(default = "default_shield_grounding")]
    pub shield_grounding: Oe1022dShieldGrounding,
    #[serde(default = "default_coupling")]
    pub coupling: Oe1022dCoupling,
    #[serde(default = "default_notch_filter")]
    pub notch_filter: Oe1022dNotchFilter,
}

impl Default for Oe1022dInputConfig {
    fn default() -> Self {
        Self {
            source: Oe1022dInputSource::SingleEndedVoltage,
            shield_grounding: Oe1022dShieldGrounding::Ground,
            coupling: Oe1022dCoupling::Ac,
            notch_filter: Oe1022dNotchFilter::Off,
        }
    }
}

fn default_input_source() -> Oe1022dInputSource {
    Oe1022dInputSource::SingleEndedVoltage
}

fn default_shield_grounding() -> Oe1022dShieldGrounding {
    Oe1022dShieldGrounding::Ground
}

fn default_coupling() -> Oe1022dCoupling {
    Oe1022dCoupling::Ac
}

fn default_notch_filter() -> Oe1022dNotchFilter {
    Oe1022dNotchFilter::Off
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Oe1022dReferenceConfig {
    #[serde(default = "default_ref_source")]
    pub source: Oe1022dReferenceSource,
    #[serde(default = "default_external_trigger")]
    pub external_trigger: Oe1022dExternalTrigger,
    #[serde(default)]
    pub phase_deg: f64,
    #[serde(default)]
    pub auto_phase: bool,
}

impl Default for Oe1022dReferenceConfig {
    fn default() -> Self {
        Self {
            source: Oe1022dReferenceSource::External,
            external_trigger: Oe1022dExternalTrigger::TtlRisingEdge,
            phase_deg: 0.0,
            auto_phase: false,
        }
    }
}

fn default_ref_source() -> Oe1022dReferenceSource {
    Oe1022dReferenceSource::External
}

fn default_external_trigger() -> Oe1022dExternalTrigger {
    Oe1022dExternalTrigger::TtlRisingEdge
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Oe1022dGainConfig {
    #[serde(default = "default_dynamic_reserve")]
    pub dynamic_reserve: Oe1022dDynamicReserve,
    #[serde(default = "default_sensitivity")]
    pub sensitivity: Oe1022dSensitivity,
}

impl Default for Oe1022dGainConfig {
    fn default() -> Self {
        Self {
            dynamic_reserve: Oe1022dDynamicReserve::Normal,
            sensitivity: Oe1022dSensitivity::S100uV,
        }
    }
}

fn default_dynamic_reserve() -> Oe1022dDynamicReserve {
    Oe1022dDynamicReserve::Normal
}

fn default_sensitivity() -> Oe1022dSensitivity {
    Oe1022dSensitivity::S100uV
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Oe1022dFilterConfig {
    #[serde(default = "default_time_constant")]
    pub time_constant_s: f64,
    #[serde(default = "default_slope")]
    pub slope_db_oct: Oe1022dFilterSlope,
    #[serde(default)]
    pub sync_filter_enabled: bool,
}

impl Default for Oe1022dFilterConfig {
    fn default() -> Self {
        Self {
            time_constant_s: 0.1,
            slope_db_oct: Oe1022dFilterSlope::Db12,
            sync_filter_enabled: false,
        }
    }
}

fn default_time_constant() -> f64 {
    0.1
}

fn default_slope() -> Oe1022dFilterSlope {
    Oe1022dFilterSlope::Db12
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Oe1022dHarmonicConfig {
    #[serde(default = "default_harmonic_1")]
    pub harmonic_1: u16,
    #[serde(default = "default_harmonic_2")]
    pub harmonic_2: u16,
}

impl Default for Oe1022dHarmonicConfig {
    fn default() -> Self {
        Self {
            harmonic_1: 1,
            harmonic_2: 2,
        }
    }
}

fn default_harmonic_1() -> u16 {
    1
}

fn default_harmonic_2() -> u16 {
    2
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Oe1022dAcquisitionConfig {
    #[serde(default = "default_frames_per_point")]
    pub frames_per_point: u64,
    #[serde(default = "default_inter_frame_delay_ms")]
    pub inter_frame_delay_ms: u64,
    #[serde(default)]
    pub pre_discard_ms: u64,
    #[serde(default = "default_record_fields")]
    pub record_fields: Vec<Oe1022dRecordField>,
}

impl Default for Oe1022dAcquisitionConfig {
    fn default() -> Self {
        Self {
            frames_per_point: 5,
            inter_frame_delay_ms: 20,
            pre_discard_ms: 100,
            record_fields: vec![
                Oe1022dRecordField::X,
                Oe1022dRecordField::Y,
                Oe1022dRecordField::R,
                Oe1022dRecordField::Theta,
                Oe1022dRecordField::Freq,
                Oe1022dRecordField::Noise,
            ],
        }
    }
}

fn default_frames_per_point() -> u64 {
    5
}

fn default_inter_frame_delay_ms() -> u64 {
    20
}

fn default_record_fields() -> Vec<Oe1022dRecordField> {
    vec![
        Oe1022dRecordField::X,
        Oe1022dRecordField::Y,
        Oe1022dRecordField::R,
        Oe1022dRecordField::Theta,
        Oe1022dRecordField::Freq,
        Oe1022dRecordField::Noise,
    ]
}

// --- OE1022D enums ---

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oe1022dChannel {
    A,
    #[default]
    B,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oe1022dPrimaryValue {
    #[default]
    X,
    Y,
    R,
    Theta,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oe1022dInputSource {
    #[serde(rename = "SINGLE_ENDED_VOLTAGE")]
    #[default]
    SingleEndedVoltage,
    #[serde(rename = "DIFFERENTIAL_VOLTAGE")]
    DifferentialVoltage,
    #[serde(rename = "CURRENT_GAIN_1M")]
    CurrentGain1M,
    #[serde(rename = "CURRENT_GAIN_100M")]
    CurrentGain100M,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oe1022dShieldGrounding {
    #[default]
    Ground,
    Float,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oe1022dCoupling {
    #[default]
    Ac,
    Dc,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oe1022dNotchFilter {
    #[default]
    Off,
    Line,
    #[serde(rename = "TWO_X_LINE")]
    TwoXLine,
    Both,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oe1022dReferenceSource {
    #[default]
    External,
    Internal,
    #[serde(rename = "INTERNAL_SWEEP")]
    InternalSweep,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oe1022dExternalTrigger {
    #[serde(rename = "TTL_RISING_EDGE")]
    #[default]
    TtlRisingEdge,
    #[serde(rename = "SINE_ZERO_CROSSING")]
    SineZeroCrossing,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oe1022dDynamicReserve {
    #[serde(rename = "LOW_NOISE")]
    LowNoise,
    #[default]
    Normal,
    #[serde(rename = "HIGH_RESERVE")]
    HighReserve,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oe1022dSensitivity {
    #[serde(rename = "1nV")]
    S1nV,
    #[serde(rename = "2nV")]
    S2nV,
    #[serde(rename = "5nV")]
    S5nV,
    #[serde(rename = "10nV")]
    S10nV,
    #[serde(rename = "20nV")]
    S20nV,
    #[serde(rename = "50nV")]
    S50nV,
    #[serde(rename = "100nV")]
    #[default]
    S100nV,
    #[serde(rename = "200nV")]
    S200nV,
    #[serde(rename = "500nV")]
    S500nV,
    #[serde(rename = "1uV")]
    S1uV,
    #[serde(rename = "2uV")]
    S2uV,
    #[serde(rename = "5uV")]
    S5uV,
    #[serde(rename = "10uV")]
    S10uV,
    #[serde(rename = "20uV")]
    S20uV,
    #[serde(rename = "50uV")]
    S50uV,
    #[serde(rename = "100uV")]
    S100uV,
    #[serde(rename = "200uV")]
    S200uV,
    #[serde(rename = "500uV")]
    S500uV,
    #[serde(rename = "1mV")]
    S1mV,
    #[serde(rename = "2mV")]
    S2mV,
    #[serde(rename = "5mV")]
    S5mV,
    #[serde(rename = "10mV")]
    S10mV,
    #[serde(rename = "20mV")]
    S20mV,
    #[serde(rename = "50mV")]
    S50mV,
    #[serde(rename = "100mV")]
    S100mV,
    #[serde(rename = "200mV")]
    S200mV,
    #[serde(rename = "500mV")]
    S500mV,
    #[serde(rename = "1V")]
    S1V,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oe1022dFilterSlope {
    #[serde(rename = "6")]
    Db6,
    #[serde(rename = "12")]
    #[default]
    Db12,
    #[serde(rename = "18")]
    Db18,
    #[serde(rename = "24")]
    Db24,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oe1022dRecordField {
    X,
    Y,
    R,
    Theta,
    Freq,
    Noise,
    #[serde(rename = "XH1")]
    Xh1,
    #[serde(rename = "YH1")]
    Yh1,
    #[serde(rename = "RH1")]
    Rh1,
    #[serde(rename = "THETAH1")]
    Thetah1,
    #[serde(rename = "XH2")]
    Xh2,
    #[serde(rename = "YH2")]
    Yh2,
    #[serde(rename = "RH2")]
    Rh2,
    #[serde(rename = "THETAH2")]
    Thetah2,
}

// ---------------------------------------------------------------------------
// Magnetic
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MagneticConfig {
    #[serde(default = "default_magnetic_mode")]
    pub mode: String,
    #[serde(default = "default_magnetic_basis")]
    pub basis: String,
    #[serde(default = "default_field_unit")]
    pub field_unit: String,
    #[serde(default)]
    pub axes: MagneticAxesConfig,
    #[serde(default)]
    pub coil_matrix: CoilMatrix,
    #[serde(default)]
    pub zero_offsets_a: MagneticZeroOffsets,
    #[serde(default = "default_settle_ms")]
    pub default_settle_ms: u64,
    #[serde(default = "default_true")]
    pub readback_required: bool,
    #[serde(default = "default_true")]
    pub zero_lock_required: bool,
}

impl Default for MagneticConfig {
    fn default() -> Self {
        Self {
            mode: "field_vector".into(),
            basis: "lab_cartesian".into(),
            field_unit: "nT".into(),
            axes: MagneticAxesConfig::default(),
            coil_matrix: CoilMatrix::default(),
            zero_offsets_a: MagneticZeroOffsets::default(),
            default_settle_ms: 500,
            readback_required: true,
            zero_lock_required: true,
        }
    }
}

fn default_magnetic_mode() -> String {
    "field_vector".into()
}

fn default_magnetic_basis() -> String {
    "lab_cartesian".into()
}

fn default_field_unit() -> String {
    "nT".into()
}

fn default_settle_ms() -> u64 {
    500
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct MagneticAxesConfig {
    #[serde(default)]
    pub x: MagneticAxisConfig,
    #[serde(default)]
    pub y: MagneticAxisConfig,
    #[serde(default)]
    pub z: MagneticAxisConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct MagneticAxisConfig {
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub serial_number: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// 3×3 coil matrix in T/A (tesla per ampere).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CoilMatrix {
    #[serde(default = "default_matrix")]
    pub matrix: [[f64; 3]; 3],
}

impl Default for CoilMatrix {
    fn default() -> Self {
        Self {
            matrix: [[0.0; 3]; 3],
        }
    }
}

fn default_matrix() -> [[f64; 3]; 3] {
    [[0.0; 3]; 3]
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct MagneticZeroOffsets {
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub z: f64,
}

// ---------------------------------------------------------------------------
// Laser
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LaserConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub power_mw: f64,
    #[serde(default = "default_wavelength")]
    pub wavelength_nm: f64,
    #[serde(default)]
    pub warmup_ms: u64,
    #[serde(default)]
    pub settle_ms: u64,
    #[serde(default)]
    pub emission_required: bool,
}

impl Default for LaserConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            power_mw: 0.0,
            wavelength_nm: 532.0,
            warmup_ms: 0,
            settle_ms: 0,
            emission_required: false,
        }
    }
}

fn default_wavelength() -> f64 {
    532.0
}

// ---------------------------------------------------------------------------
// Station Safety
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct StationSafety {
    #[serde(default)]
    pub smb100a: Smb100aSafetyLimits,
    #[serde(default)]
    pub oe1022d: Oe1022dSafetyLimits,
    #[serde(default)]
    pub magnetic: MagneticSafetyLimits,
    #[serde(default)]
    pub laser: LaserSafetyLimits,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Smb100aSafetyLimits {
    #[serde(default = "default_allowed_frequency_ranges")]
    pub allowed_frequency_ranges_hz: Vec<[f64; 2]>,
    #[serde(default = "default_max_power")]
    pub max_power_dbm: f64,
    #[serde(default)]
    pub default_output_enabled: bool,
    #[serde(default = "default_true")]
    pub require_operator_approval_for_output_on: bool,
    #[serde(default = "default_true")]
    pub require_modulation_approval: bool,
}

impl Default for Smb100aSafetyLimits {
    fn default() -> Self {
        Self {
            allowed_frequency_ranges_hz: vec![[2_800_000_000.0, 2_950_000_000.0]],
            max_power_dbm: -10.0,
            default_output_enabled: false,
            require_operator_approval_for_output_on: true,
            require_modulation_approval: true,
        }
    }
}

fn default_allowed_frequency_ranges() -> Vec<[f64; 2]> {
    vec![[2_800_000_000.0, 2_950_000_000.0]]
}

fn default_max_power() -> f64 {
    -10.0
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct Oe1022dSafetyLimits {
    #[serde(default = "default_required_channel")]
    pub required_primary_channel: Oe1022dChannel,
    #[serde(default = "default_true")]
    pub required_reference_lock: bool,
    #[serde(default = "default_overload_policy")]
    pub overload_policy: String,
}

fn default_required_channel() -> Oe1022dChannel {
    Oe1022dChannel::B
}

fn default_overload_policy() -> String {
    "REJECT_POINT".into()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MagneticSafetyLimits {
    #[serde(default = "default_max_current")]
    pub max_current_a_per_axis: MaxCurrentPerAxis,
    #[serde(default = "default_max_ramp")]
    pub max_ramp_a_per_s: MaxRampPerAxis,
    #[serde(default = "default_max_b")]
    pub max_b_vector_t: f64,
    #[serde(default = "default_true")]
    pub require_zero_lock: bool,
    #[serde(default = "default_true")]
    pub require_readback: bool,
}

impl Default for MagneticSafetyLimits {
    fn default() -> Self {
        Self {
            max_current_a_per_axis: MaxCurrentPerAxis::default(),
            max_ramp_a_per_s: MaxRampPerAxis::default(),
            max_b_vector_t: 0.001,
            require_zero_lock: true,
            require_readback: true,
        }
    }
}

fn default_max_current() -> MaxCurrentPerAxis {
    MaxCurrentPerAxis::default()
}

fn default_max_ramp() -> MaxRampPerAxis {
    MaxRampPerAxis::default()
}

fn default_max_b() -> f64 {
    0.001
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MaxCurrentPerAxis {
    #[serde(default = "default_axis_limit")]
    pub x: f64,
    #[serde(default = "default_axis_limit")]
    pub y: f64,
    #[serde(default = "default_axis_limit")]
    pub z: f64,
}

impl Default for MaxCurrentPerAxis {
    fn default() -> Self {
        Self {
            x: 0.1,
            y: 0.1,
            z: 0.1,
        }
    }
}

fn default_axis_limit() -> f64 {
    0.1
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MaxRampPerAxis {
    #[serde(default = "default_ramp_limit")]
    pub x: f64,
    #[serde(default = "default_ramp_limit")]
    pub y: f64,
    #[serde(default = "default_ramp_limit")]
    pub z: f64,
}

impl Default for MaxRampPerAxis {
    fn default() -> Self {
        Self {
            x: 0.02,
            y: 0.02,
            z: 0.02,
        }
    }
}

fn default_ramp_limit() -> f64 {
    0.02
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LaserSafetyLimits {
    #[serde(default)]
    pub max_power_mw: f64,
    #[serde(default)]
    pub default_enabled: bool,
    #[serde(default = "default_true")]
    pub require_operator_approval: bool,
}

impl Default for LaserSafetyLimits {
    fn default() -> Self {
        Self {
            max_power_mw: 0.0,
            default_enabled: false,
            require_operator_approval: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers (TryFrom serde_json::Value)
// ---------------------------------------------------------------------------

impl Smb100aConfig {
    /// Parse from a loose `serde_json::Value`.
    pub fn try_from_value(value: &serde_json::Value) -> Result<Self, crate::RecipeError> {
        serde_json::from_value(value.clone()).map_err(crate::RecipeError::Json)
    }
}

impl Oe1022dConfig {
    pub fn try_from_value(value: &serde_json::Value) -> Result<Self, crate::RecipeError> {
        serde_json::from_value(value.clone()).map_err(crate::RecipeError::Json)
    }
}

impl MagneticConfig {
    pub fn try_from_value(value: &serde_json::Value) -> Result<Self, crate::RecipeError> {
        serde_json::from_value(value.clone()).map_err(crate::RecipeError::Json)
    }
}

impl LaserConfig {
    pub fn try_from_value(value: &serde_json::Value) -> Result<Self, crate::RecipeError> {
        serde_json::from_value(value.clone()).map_err(crate::RecipeError::Json)
    }
}

impl StationSafety {
    pub fn try_from_value(value: &serde_json::Value) -> Result<Self, crate::RecipeError> {
        serde_json::from_value(value.clone()).map_err(crate::RecipeError::Json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smb100a_config_roundtrip() {
        let config = Smb100aConfig::default();
        let json = serde_json::to_value(&config).unwrap();
        let parsed = Smb100aConfig::try_from_value(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn oe1022d_config_roundtrip() {
        let config = Oe1022dConfig::default();
        let json = serde_json::to_value(&config).unwrap();
        let parsed = Oe1022dConfig::try_from_value(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn magnetic_config_roundtrip() {
        let config = MagneticConfig::default();
        let json = serde_json::to_value(&config).unwrap();
        let parsed = MagneticConfig::try_from_value(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn laser_config_roundtrip() {
        let config = LaserConfig::default();
        let json = serde_json::to_value(&config).unwrap();
        let parsed = LaserConfig::try_from_value(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn station_safety_roundtrip() {
        let config = StationSafety::default();
        let json = serde_json::to_value(&config).unwrap();
        let parsed = StationSafety::try_from_value(&json).unwrap();
        assert_eq!(config, parsed);
    }
}
