//! odmr-recipe — Layer 2 recipe data structures, JSON loading and validation.

pub mod types;
pub mod validation;

pub use types::*;
pub use validation::*;

use std::io;

/// Load a `Station` from a JSON file path.
pub fn load_station(path: &std::path::Path) -> Result<Station, RecipeError> {
    let contents = std::fs::read_to_string(path)?;
    let station: Station = serde_json::from_str(&contents)?;
    validation::validate_station(&station)?;
    Ok(station)
}

/// Load a `Recipe` from a JSON file path.
pub fn load_recipe(path: &std::path::Path) -> Result<Recipe, RecipeError> {
    let contents = std::fs::read_to_string(path)?;
    let recipe: Recipe = serde_json::from_str(&contents)?;
    validation::validate_recipe(&recipe)?;
    Ok(recipe)
}

/// Load a `SafetyLimit` from a JSON file path.
pub fn load_safety_limits(path: &std::path::Path) -> Result<SafetyLimit, RecipeError> {
    let contents = std::fs::read_to_string(path)?;
    let limits: SafetyLimit = serde_json::from_str(&contents)?;
    Ok(limits)
}

/// Compute a deterministic hash for a `Recipe`.
///
/// The hash is SHA-256 of the canonical JSON representation (sorted keys,
/// no extra whitespace).  This gives a stable identifier for a given recipe
/// content regardless of formatting or field ordering in the source file.
pub fn compute_recipe_hash(recipe: &Recipe) -> Result<String, RecipeError> {
    let canonical = serde_json::to_string(recipe)?;
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let hash = hasher.finalize();
    Ok(hex::encode(hash))
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum RecipeError {
    Io(io::Error),
    Json(serde_json::Error),
    Validation(ValidationError),
}

impl std::fmt::Display for RecipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecipeError::Io(e) => write!(f, "io error: {e}"),
            RecipeError::Json(e) => write!(f, "json error: {e}"),
            RecipeError::Validation(e) => write!(f, "validation error: {e}"),
        }
    }
}

impl std::error::Error for RecipeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RecipeError::Io(e) => Some(e),
            RecipeError::Json(e) => Some(e),
            RecipeError::Validation(e) => Some(e),
        }
    }
}

impl From<io::Error> for RecipeError {
    fn from(e: io::Error) -> Self {
        RecipeError::Io(e)
    }
}

impl From<serde_json::Error> for RecipeError {
    fn from(e: serde_json::Error) -> Self {
        RecipeError::Json(e)
    }
}

impl From<ValidationError> for RecipeError {
    fn from(e: ValidationError) -> Self {
        RecipeError::Validation(e)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_json(content: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    #[test]
    fn load_valid_station() {
        let json = r#"{
            "schema_version": "0.2.0",
            "kind": "station",
            "id": "station_test",
            "devices": [
                {
                    "device_id": "smb100a_01",
                    "device_type": "smb100a",
                    "transport": { "kind": "tcp_scpi", "host": "192.168.0.20", "port": 5025 },
                    "required": true
                }
            ],
            "safety_profile_id": "safety_default"
        }"#;
        let file = temp_json(json);
        let station = load_station(file.path()).unwrap();
        assert_eq!(station.header.id, "station_test");
        assert_eq!(station.devices.len(), 1);
    }

    #[test]
    fn load_valid_recipe() {
        let json = r#"{
            "schema_version": "0.2.0",
            "kind": "recipe",
            "id": "recipe_test",
            "station_id": "station_test",
            "intent": { "experiment_type": "cw_odmr" },
            "acquisition": { "device_id": "oe1022d_01", "channel": "B", "readout": ["x"] }
        }"#;
        let file = temp_json(json);
        let recipe = load_recipe(file.path()).unwrap();
        assert_eq!(recipe.header.id, "recipe_test");
    }

    #[test]
    fn reject_station_with_duplicate_device_ids() {
        let json = r#"{
            "schema_version": "0.2.0",
            "kind": "station",
            "id": "station_dup",
            "devices": [
                { "device_id": "smb100a_01", "device_type": "smb100a", "transport": { "kind": "tcp_scpi", "host": "h", "port": 1 }, "required": true },
                { "device_id": "smb100a_01", "device_type": "oe1022d", "transport": { "kind": "serial", "port": "COM1" }, "required": true }
            ],
            "safety_profile_id": "safety_default"
        }"#;
        let file = temp_json(json);
        let err = load_station(file.path()).unwrap_err();
        assert!(err.to_string().contains("duplicate device_id"));
    }

    #[test]
    fn reject_recipe_with_bad_sweep_step() {
        let json = r#"{
            "schema_version": "0.2.0",
            "kind": "recipe",
            "id": "recipe_bad_sweep",
            "station_id": "station_test",
            "intent": { "experiment_type": "cw_odmr" },
            "sweeps": [
                { "sweep_id": "s1", "axis": "smb100a.rf.frequency_hz", "start": 10, "stop": 0, "step": 1 }
            ],
            "acquisition": { "device_id": "oe1022d_01", "channel": "B", "readout": ["x"] }
        }"#;
        let file = temp_json(json);
        let err = load_recipe(file.path()).unwrap_err();
        assert!(err.to_string().contains("step sign"));
    }

    #[test]
    fn recipe_hash_is_deterministic() {
        let recipe = Recipe {
            header: CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "recipe".into(),
                id: "r1".into(),
                name: None,
                created_by: None,
                created_at: None,
                description: None,
            },
            station_id: "s1".into(),
            intent: RecipeIntent {
                experiment_type: "cw_odmr".into(),
                description: None,
            },
            profiles: vec![],
            blocks: vec![],
            sweeps: vec![],
            acquisition: AcquisitionConfig {
                device_id: "oe1022d_01".into(),
                channel: "B".into(),
                readout: vec!["x".into()],
                pre_discard_ms: None,
                window_ms: None,
                average: None,
            },
            timing: None,
            metadata: None,
        };
        let h1 = compute_recipe_hash(&recipe).unwrap();
        let h2 = compute_recipe_hash(&recipe).unwrap();
        assert_eq!(h1, h2);
        assert!(!h1.is_empty());
    }

    #[test]
    fn hash_length_is_64_chars() {
        let recipe = Recipe {
            header: CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "recipe".into(),
                id: "r1".into(),
                name: None,
                created_by: None,
                created_at: None,
                description: None,
            },
            station_id: "s1".into(),
            intent: RecipeIntent {
                experiment_type: "cw_odmr".into(),
                description: None,
            },
            profiles: vec![],
            blocks: vec![],
            sweeps: vec![],
            acquisition: AcquisitionConfig {
                device_id: "oe1022d_01".into(),
                channel: "B".into(),
                readout: vec!["x".into()],
                pre_discard_ms: None,
                window_ms: None,
                average: None,
            },
            timing: None,
            metadata: None,
        };
        let h = compute_recipe_hash(&recipe).unwrap();
        assert_eq!(h.len(), 64, "SHA-256 hex string must be 64 characters");
    }

    #[test]
    fn same_recipe_same_hash() {
        let recipe = Recipe {
            header: CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "recipe".into(),
                id: "r1".into(),
                name: None,
                created_by: None,
                created_at: None,
                description: None,
            },
            station_id: "s1".into(),
            intent: RecipeIntent {
                experiment_type: "cw_odmr".into(),
                description: None,
            },
            profiles: vec![],
            blocks: vec![],
            sweeps: vec![],
            acquisition: AcquisitionConfig {
                device_id: "oe1022d_01".into(),
                channel: "B".into(),
                readout: vec!["x".into()],
                pre_discard_ms: None,
                window_ms: None,
                average: None,
            },
            timing: None,
            metadata: None,
        };
        let h1 = compute_recipe_hash(&recipe).unwrap();
        let h2 = compute_recipe_hash(&recipe).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_recipe_different_hash() {
        let recipe_a = Recipe {
            header: CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "recipe".into(),
                id: "r1".into(),
                name: None,
                created_by: None,
                created_at: None,
                description: None,
            },
            station_id: "s1".into(),
            intent: RecipeIntent {
                experiment_type: "cw_odmr".into(),
                description: None,
            },
            profiles: vec![],
            blocks: vec![],
            sweeps: vec![],
            acquisition: AcquisitionConfig {
                device_id: "oe1022d_01".into(),
                channel: "B".into(),
                readout: vec!["x".into()],
                pre_discard_ms: None,
                window_ms: None,
                average: None,
            },
            timing: None,
            metadata: None,
        };
        let mut recipe_b = recipe_a.clone();
        recipe_b.header.id = "r2".into();
        let h_a = compute_recipe_hash(&recipe_a).unwrap();
        let h_b = compute_recipe_hash(&recipe_b).unwrap();
        assert_ne!(h_a, h_b, "changing recipe id must change hash");
    }
}
