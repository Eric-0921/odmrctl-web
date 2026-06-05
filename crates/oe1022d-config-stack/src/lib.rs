//! 4-layer (OE1022D / SMB100A / Magnetic / Laser) onion
//! configuration loader and validator.
//!
//! C8 scope. Loads and validates:
//! - 4 individual profile JSONs (one per layer)
//! - 1 onion recipe JSON that ties them together with run-level
//!   acquisition parameters
//!
//! The new project's profile JSON shape mirrors the main repo's
//! `examples/device_profiles/*.json` so that profiles authored
//! against the main repo can be reused here with no edits.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("json parse error in {path}: {source}")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("validation error in {path}: {message}")]
    Validation { path: String, message: String },
    #[error("recipe error: {message}")]
    Recipe { message: String },
}

// ---------------------------------------------------------------------------
// Layer enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Layer {
    Oe1022d,
    Smb100a,
    Magnetic,
    Laser,
}

impl Layer {
    pub const ALL: &'static [Layer] = &[Self::Oe1022d, Self::Smb100a, Self::Magnetic, Self::Laser];
    pub fn required_by_default(self) -> bool {
        matches!(self, Self::Oe1022d)
    }
}

// ---------------------------------------------------------------------------
// Layer 1: OE1022D profile
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oe1022dProfile {
    pub id: String,
    #[serde(default)]
    pub description: String,
    pub primary_channel: PrimaryChannel,
    pub primary_value: PrimaryValue,
    pub input: InputConfig,
    pub reference: ReferenceConfig,
    pub gain: GainConfig,
    pub filter: FilterConfig,
    #[serde(default)]
    pub harmonic: Option<HarmonicConfig>,
    #[serde(default)]
    pub acquisition: Option<AcquisitionConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrimaryChannel {
    A,
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrimaryValue {
    X,
    Y,
    R,
    Theta,
    Freq,
    Noise,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub source: String,
    pub shield_grounding: String,
    pub coupling: String,
    pub notch_filter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceConfig {
    pub source: String,
    pub external_trigger: String,
    pub phase_deg: f64,
    pub auto_phase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GainConfig {
    pub dynamic_reserve: String,
    pub sensitivity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub time_constant_s: f64,
    pub slope_db_oct: String,
    pub sync_filter_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarmonicConfig {
    pub harmonic_1: u32,
    pub harmonic_2: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionConfig {
    #[serde(default = "default_frames_per_point")]
    pub frames_per_point: u32,
    #[serde(default = "default_inter_frame_delay_ms")]
    pub inter_frame_delay_ms: u32,
    #[serde(default = "default_pre_discard_ms")]
    pub pre_discard_ms: u32,
    #[serde(default)]
    pub record_fields: Vec<String>,
}

fn default_frames_per_point() -> u32 { 5 }
fn default_inter_frame_delay_ms() -> u32 { 20 }
fn default_pre_discard_ms() -> u32 { 100 }

// ---------------------------------------------------------------------------
// Layer 2/3/4: minimal profile for the other 3 layers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Smb100aProfile {
    pub id: String,
    #[serde(default)]
    pub description: String,
    /// Carrier frequency in Hz.
    pub frequency_hz: f64,
    /// Output power in dBm.
    pub power_dbm: f64,
    /// "ON" or "OFF".
    pub output_state: String,
    #[serde(default)]
    pub modulation: Option<String>,
    #[serde(default)]
    pub fm_deviation_hz: Option<f64>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagneticProfile {
    pub id: String,
    #[serde(default)]
    pub description: String,
    /// Field vector in nT: { x, y, z }.
    pub field_vector_nt: FieldVectorNt,
    #[serde(default)]
    pub coil_calibration: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldVectorNt {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaserProfile {
    pub id: String,
    #[serde(default)]
    pub description: String,
    /// Power in mW.
    pub power_mw: f64,
    /// Wavelength in nm.
    pub wavelength_nm: f64,
    /// "ON" / "OFF" / "STANDBY".
    pub state: String,
    #[serde(default)]
    pub note: Option<String>,
}

// ---------------------------------------------------------------------------
// Layer enum dispatch for typed profile loading
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum LayerProfile {
    Oe1022d(Oe1022dProfile),
    Smb100a(Smb100aProfile),
    Magnetic(MagneticProfile),
    Laser(LaserProfile),
}

impl LayerProfile {
    pub fn layer(&self) -> Layer {
        match self {
            Self::Oe1022d(_) => Layer::Oe1022d,
            Self::Smb100a(_) => Layer::Smb100a,
            Self::Magnetic(_) => Layer::Magnetic,
            Self::Laser(_) => Layer::Laser,
        }
    }
    pub fn id(&self) -> &str {
        match self {
            Self::Oe1022d(p) => &p.id,
            Self::Smb100a(p) => &p.id,
            Self::Magnetic(p) => &p.id,
            Self::Laser(p) => &p.id,
        }
    }
}

// ---------------------------------------------------------------------------
// Onion recipe
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnionRecipe {
    pub schema_version: String,
    pub kind: OnionRecipeKind,
    pub id: String,
    #[serde(default)]
    pub description: String,
    pub layers: BTreeMap<Layer, LayerRef>,
    pub acquisition: RecipeAcquisition,
    #[serde(default)]
    pub tag: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnionRecipeKind {
    OnionDatasetRecipe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerRef {
    /// "auto" to use the device's USB CDC port, or an explicit
    /// "/dev/cu.usbmodem*" path.
    pub port: String,
    pub profile_ref: String,
    /// Whether this layer is active in the run. Inactive layers
    /// are still written to metadata but not driven.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeAcquisition {
    /// Soft target period in ms (informational; the actual
    /// period is driven by the device's RALL? cycle).
    pub period_ms: u32,
    /// Soft target number of frames per run.
    pub target_frames: u32,
    /// Which fields to expand to ndjson lines.
    pub fields: Vec<String>,
    /// Directory for `runs/<recipe_id>/samples.ndjson`.
    pub ndjson_dir: String,
}

// ---------------------------------------------------------------------------
// Loaders
// ---------------------------------------------------------------------------

fn read_and_parse<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ConfigError> {
    let text = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.display().to_string(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| ConfigError::Json {
        path: path.display().to_string(),
        source,
    })
}

pub fn load_oe1022d_profile(path: &Path) -> Result<Oe1022dProfile, ConfigError> {
    let p: Oe1022dProfile = read_and_parse(path)?;
    validate_oe1022d(&p, path)?;
    Ok(p)
}

pub fn load_smb100a_profile(path: &Path) -> Result<Smb100aProfile, ConfigError> {
    let p: Smb100aProfile = read_and_parse(path)?;
    Ok(p)
}

pub fn load_magnetic_profile(path: &Path) -> Result<MagneticProfile, ConfigError> {
    let p: MagneticProfile = read_and_parse(path)?;
    Ok(p)
}

pub fn load_laser_profile(path: &Path) -> Result<LaserProfile, ConfigError> {
    let p: LaserProfile = read_and_parse(path)?;
    Ok(p)
}

pub fn load_onion_recipe(path: &Path) -> Result<OnionRecipe, ConfigError> {
    let r: OnionRecipe = read_and_parse(path)?;
    validate_recipe(&r, path)?;
    Ok(r)
}

fn validate_oe1022d(p: &Oe1022dProfile, path: &Path) -> Result<(), ConfigError> {
    if p.id.trim().is_empty() {
        return Err(ConfigError::Validation {
            path: path.display().to_string(),
            message: "id must not be empty".into(),
        });
    }
    if !(0.0..=10.0).contains(&p.filter.time_constant_s) {
        return Err(ConfigError::Validation {
            path: path.display().to_string(),
            message: format!(
                "filter.time_constant_s = {} out of range [0, 10]",
                p.filter.time_constant_s
            ),
        });
    }
    Ok(())
}

fn validate_recipe(r: &OnionRecipe, path: &Path) -> Result<(), ConfigError> {
    if r.kind != OnionRecipeKind::OnionDatasetRecipe {
        return Err(ConfigError::Validation {
            path: path.display().to_string(),
            message: format!("unsupported recipe kind: {:?}", r.kind),
        });
    }
    // Layer 1 (OE1022D) must be present and enabled.
    let oe = r.layers.get(&Layer::Oe1022d).ok_or_else(|| {
        ConfigError::Validation {
            path: path.display().to_string(),
            message: "recipe is missing the required OE1022D layer".into(),
        }
    })?;
    if !oe.enabled {
        return Err(ConfigError::Validation {
            path: path.display().to_string(),
            message: "OE1022D layer must be enabled (it is the acquisition source)".into(),
        });
    }
    if r.acquisition.fields.is_empty() {
        return Err(ConfigError::Validation {
            path: path.display().to_string(),
            message: "acquisition.fields must not be empty".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_json(name: &str, body: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "oe1022d_cfg_{}_{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(format!("{name}.json"));
        std::fs::write(&p, body).unwrap();
        p
    }

    #[test]
    fn load_minimal_oe1022d_profile_ok() {
        let p = tmp_json(
            "oe1022d",
            r#"{
                "id": "oe_test",
                "primary_channel": "B",
                "primary_value": "X",
                "input": {"source":"A","shield_grounding":"FLOAT","coupling":"AC","notch_filter":"OFF"},
                "reference": {"source":"EXTERNAL","external_trigger":"TTL_RISING_EDGE","phase_deg":0.0,"auto_phase":false},
                "gain": {"dynamic_reserve":"NORMAL","sensitivity":"100uV"},
                "filter": {"time_constant_s":0.1,"slope_db_oct":"12","sync_filter_enabled":false}
            }"#,
        );
        let profile = load_oe1022d_profile(&p).expect("load");
        assert_eq!(profile.id, "oe_test");
    }

    #[test]
    fn reject_oe1022d_with_empty_id() {
        let p = tmp_json(
            "oe_bad",
            r#"{
                "id": "",
                "primary_channel": "B",
                "primary_value": "X",
                "input": {"source":"A","shield_grounding":"FLOAT","coupling":"AC","notch_filter":"OFF"},
                "reference": {"source":"EXTERNAL","external_trigger":"TTL_RISING_EDGE","phase_deg":0.0,"auto_phase":false},
                "gain": {"dynamic_reserve":"NORMAL","sensitivity":"100uV"},
                "filter": {"time_constant_s":0.1,"slope_db_oct":"12","sync_filter_enabled":false}
            }"#,
        );
        let err = load_oe1022d_profile(&p).unwrap_err();
        assert!(matches!(err, ConfigError::Validation { .. }));
    }

    #[test]
    fn reject_oe1022d_with_out_of_range_time_constant() {
        let p = tmp_json(
            "oe_tc",
            r#"{
                "id": "oe_test",
                "primary_channel": "B",
                "primary_value": "X",
                "input": {"source":"A","shield_grounding":"FLOAT","coupling":"AC","notch_filter":"OFF"},
                "reference": {"source":"EXTERNAL","external_trigger":"TTL_RISING_EDGE","phase_deg":0.0,"auto_phase":false},
                "gain": {"dynamic_reserve":"NORMAL","sensitivity":"100uV"},
                "filter": {"time_constant_s":50.0,"slope_db_oct":"12","sync_filter_enabled":false}
            }"#,
        );
        let err = load_oe1022d_profile(&p).unwrap_err();
        assert!(matches!(err, ConfigError::Validation { .. }));
    }

    #[test]
    fn load_smb100a_profile_ok() {
        let p = tmp_json(
            "smb",
            r#"{
                "id": "smb_test",
                "frequency_hz": 2.882e9,
                "power_dbm": -10.0,
                "output_state": "ON",
                "modulation": "FM",
                "fm_deviation_hz": 500000.0
            }"#,
        );
        let profile = load_smb100a_profile(&p).expect("load");
        assert_eq!(profile.id, "smb_test");
        assert!((profile.frequency_hz - 2.882e9).abs() < 1.0);
    }

    #[test]
    fn load_magnetic_profile_ok() {
        let p = tmp_json(
            "mag",
            r#"{
                "id": "mag_test",
                "field_vector_nt": {"x": 0.0, "y": 0.0, "z": 1000.0}
            }"#,
        );
        let profile = load_magnetic_profile(&p).expect("load");
        assert_eq!(profile.field_vector_nt.z, 1000.0);
    }

    #[test]
    fn load_laser_profile_ok() {
        let p = tmp_json(
            "laser",
            r#"{
                "id": "laser_test",
                "power_mw": 50.0,
                "wavelength_nm": 532.0,
                "state": "ON"
            }"#,
        );
        let profile = load_laser_profile(&p).expect("load");
        assert_eq!(profile.power_mw, 50.0);
    }

    #[test]
    fn load_onion_recipe_minimal_ok() {
        let p = tmp_json(
            "recipe",
            r#"{
                "schema_version": "0.1.0",
                "kind": "onion_dataset_recipe",
                "id": "demo_run_001",
                "layers": {
                    "OE1022D": {"port":"auto","profile_ref":"oe1022d.default.json","enabled":true}
                },
                "acquisition": {
                    "period_ms": 100,
                    "target_frames": 100,
                    "fields": ["BX","BY"],
                    "ndjson_dir": "./runs/demo_run_001"
                }
            }"#,
        );
        let r = load_onion_recipe(&p).expect("load");
        assert_eq!(r.id, "demo_run_001");
        assert!(r.layers.contains_key(&Layer::Oe1022d));
    }

    #[test]
    fn reject_recipe_without_oe1022d_layer() {
        let p = tmp_json(
            "recipe_bad",
            r#"{
                "schema_version": "0.1.0",
                "kind": "onion_dataset_recipe",
                "id": "demo_run_001",
                "layers": {
                    "SMB100A": {"port":"auto","profile_ref":"smb100a.default.json","enabled":true}
                },
                "acquisition": {
                    "period_ms": 100,
                    "target_frames": 100,
                    "fields": ["BX"],
                    "ndjson_dir": "./runs/demo_run_001"
                }
            }"#,
        );
        let err = load_onion_recipe(&p).unwrap_err();
        assert!(matches!(err, ConfigError::Validation { .. }));
    }

    #[test]
    fn reject_recipe_with_disabled_oe1022d() {
        let p = tmp_json(
            "recipe_off",
            r#"{
                "schema_version": "0.1.0",
                "kind": "onion_dataset_recipe",
                "id": "demo",
                "layers": {
                    "OE1022D": {"port":"auto","profile_ref":"oe1022d.default.json","enabled":false}
                },
                "acquisition": {
                    "period_ms": 100,
                    "target_frames": 100,
                    "fields": ["BX"],
                    "ndjson_dir": "./runs/demo"
                }
            }"#,
        );
        let err = load_onion_recipe(&p).unwrap_err();
        assert!(matches!(err, ConfigError::Validation { .. }));
    }

    #[test]
    fn reject_recipe_with_empty_fields() {
        let p = tmp_json(
            "recipe_empty",
            r#"{
                "schema_version": "0.1.0",
                "kind": "onion_dataset_recipe",
                "id": "demo",
                "layers": {
                    "OE1022D": {"port":"auto","profile_ref":"oe1022d.default.json","enabled":true}
                },
                "acquisition": {
                    "period_ms": 100,
                    "target_frames": 100,
                    "fields": [],
                    "ndjson_dir": "./runs/demo"
                }
            }"#,
        );
        let err = load_onion_recipe(&p).unwrap_err();
        assert!(matches!(err, ConfigError::Validation { .. }));
    }

    #[test]
    fn layer_round_trip() {
        for layer in Layer::ALL {
            let json = serde_json::to_string(layer).unwrap();
            let back: Layer = serde_json::from_str(&json).unwrap();
            assert_eq!(*layer, back);
        }
    }
}
