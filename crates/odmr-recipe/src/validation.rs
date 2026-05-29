//! Validation logic for recipes and stations.

use crate::types::*;
use odmr_types::{DeviceId, DeviceKind};
use std::collections::HashSet;

/// Errors that can occur during validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    MissingField(String),
    InvalidId {
        field: String,
        reason: String,
    },
    UnknownDeviceKind {
        device_id: String,
        kind: String,
    },
    DuplicateDeviceId(String),
    RequiredDeviceMissing(String),
    SweepStepMismatch {
        sweep_id: String,
        reason: String,
    },
    SafetyViolation {
        field: String,
        value: f64,
        limit: f64,
    },
    InvalidSchemaVersion {
        expected: String,
        got: String,
    },
    Json(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::MissingField(field) => write!(f, "missing required field: {field}"),
            ValidationError::InvalidId { field, reason } => {
                write!(f, "invalid id in field '{field}': {reason}")
            }
            ValidationError::UnknownDeviceKind { device_id, kind } => {
                write!(f, "unknown device kind '{kind}' for device '{device_id}'")
            }
            ValidationError::DuplicateDeviceId(id) => {
                write!(f, "duplicate device_id: {id}")
            }
            ValidationError::RequiredDeviceMissing(id) => {
                write!(f, "required device missing or invalid: {id}")
            }
            ValidationError::SweepStepMismatch { sweep_id, reason } => {
                write!(f, "sweep '{sweep_id}' invalid: {reason}")
            }
            ValidationError::SafetyViolation {
                field,
                value,
                limit,
            } => {
                write!(f, "safety violation: {field}={value} exceeds limit {limit}")
            }
            ValidationError::InvalidSchemaVersion { expected, got } => {
                write!(f, "schema version mismatch: expected {expected}, got {got}")
            }
            ValidationError::Json(msg) => write!(f, "json error: {msg}"),
        }
    }
}

impl std::error::Error for ValidationError {}

impl From<serde_json::Error> for ValidationError {
    fn from(e: serde_json::Error) -> Self {
        ValidationError::Json(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Station validation
// ---------------------------------------------------------------------------

pub fn validate_station(station: &Station) -> Result<(), ValidationError> {
    if station.header.kind != "station" {
        return Err(ValidationError::InvalidId {
            field: "kind".into(),
            reason: format!("expected 'station', got '{}'", station.header.kind),
        });
    }

    if station.devices.is_empty() {
        return Err(ValidationError::MissingField("devices".into()));
    }

    let mut seen_ids = HashSet::new();
    for dev in &station.devices {
        let did = DeviceId::new(&dev.device_id);
        if let Err(e) = did.validate() {
            return Err(ValidationError::InvalidId {
                field: "device_id".into(),
                reason: e.to_string(),
            });
        }
        if !seen_ids.insert(dev.device_id.clone()) {
            return Err(ValidationError::DuplicateDeviceId(dev.device_id.clone()));
        }
        if DeviceKind::from_str_lossy(&dev.device_type).is_none() {
            return Err(ValidationError::UnknownDeviceKind {
                device_id: dev.device_id.clone(),
                kind: dev.device_type.clone(),
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Recipe validation
// ---------------------------------------------------------------------------

pub fn validate_recipe(recipe: &Recipe) -> Result<(), ValidationError> {
    if recipe.header.kind != "recipe" {
        return Err(ValidationError::InvalidId {
            field: "kind".into(),
            reason: format!("expected 'recipe', got '{}'", recipe.header.kind),
        });
    }

    if recipe.station_id.is_empty() {
        return Err(ValidationError::MissingField("station_id".into()));
    }

    for sweep in &recipe.sweeps {
        validate_sweep(sweep)?;
    }

    Ok(())
}

fn validate_sweep(sweep: &SweepDefinition) -> Result<(), ValidationError> {
    if sweep.step == 0.0 {
        return Err(ValidationError::SweepStepMismatch {
            sweep_id: sweep.sweep_id.clone(),
            reason: "step cannot be zero".into(),
        });
    }
    let direction = sweep.stop - sweep.start;
    if direction.signum() != sweep.step.signum() && direction != 0.0 {
        return Err(ValidationError::SweepStepMismatch {
            sweep_id: sweep.sweep_id.clone(),
            reason: "step sign must match start→stop direction".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Safety validation (mock)
// ---------------------------------------------------------------------------

/// Check a recipe against mock safety limits.
///
/// In production this is handled by `odmr-safety`.  For the bootstrap milestone
/// we perform a coarse check on sweep parameters that are known to map to
/// device settings.
pub fn validate_recipe_safety(
    recipe: &Recipe,
    limits: &SafetyLimit,
) -> Result<(), ValidationError> {
    for sweep in &recipe.sweeps {
        if sweep.axis.contains("frequency") {
            let max = sweep.start.max(sweep.stop);
            let min = sweep.start.min(sweep.stop);
            if max > limits.max_frequency_hz {
                return Err(ValidationError::SafetyViolation {
                    field: format!("sweep.{}.frequency", sweep.sweep_id),
                    value: max,
                    limit: limits.max_frequency_hz,
                });
            }
            if min < limits.min_frequency_hz {
                return Err(ValidationError::SafetyViolation {
                    field: format!("sweep.{}.frequency", sweep.sweep_id),
                    value: min,
                    limit: limits.min_frequency_hz,
                });
            }
        }
        // Power checks would require resolved_recipe (knowing actual power levels).
        // We skip them in this mock milestone.
    }
    Ok(())
}
