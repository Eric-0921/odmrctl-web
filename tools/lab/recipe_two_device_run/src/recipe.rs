//! Recipe loading and validation for M3.4.

use crate::types::*;
use std::path::Path;

pub fn load_recipe(path: &Path) -> Result<M3_4Recipe, String> {
    let contents = std::fs::read_to_string(path).map_err(|e| format!("read recipe: {}", e))?;
    let recipe: M3_4Recipe =
        serde_json::from_str(&contents).map_err(|e| format!("parse recipe: {}", e))?;
    validate_recipe(&recipe)?;
    Ok(recipe)
}

pub fn validate_recipe(recipe: &M3_4Recipe) -> Result<(), String> {
    if recipe.kind != "two_device_odmr_like_sweep_recipe" {
        return Err(format!(
            "recipe kind must be 'two_device_odmr_like_sweep_recipe', got '{}'",
            recipe.kind
        ));
    }
    if recipe.rf.start_hz <= 0.0 || recipe.rf.stop_hz <= 0.0 {
        return Err("rf start_hz and stop_hz must be positive".into());
    }
    if recipe.rf.start_hz > recipe.rf.stop_hz {
        return Err("rf start_hz must be <= stop_hz".into());
    }
    if recipe.rf.points < 2 {
        return Err("rf points must be >= 2".into());
    }
    if recipe.rf.max_power_dbm > -10.0 {
        return Err("max_rf_power_dbm must be <= -10 dBm".into());
    }
    if recipe.rf.power_dbm > recipe.rf.max_power_dbm {
        return Err("rf_power_dbm must be <= max_rf_power_dbm".into());
    }
    if recipe.modulation.max_fm_deviation_hz > 5_000_000.0 {
        return Err("max_fm_deviation_hz must be <= 5 MHz".into());
    }
    if recipe.modulation.fm_deviation_hz > recipe.modulation.max_fm_deviation_hz {
        return Err("fm_deviation_hz must be <= max_fm_deviation_hz".into());
    }
    if recipe.acquisition.frames_per_step > 10 {
        return Err("frames_per_step must be <= 10".into());
    }
    if recipe.acquisition.repeat_count > 3 {
        return Err("repeat_count must be <= 3".into());
    }
    if recipe.rf.points * recipe.acquisition.frames_per_step * recipe.acquisition.repeat_count > 630
    {
        return Err("total frames (rf_points × frames_per_step × repeat_count) exceeds 630".into());
    }
    if recipe.safety.no_magnetic && recipe.devices.magnetic.in_scope {
        return Err(
            "magnetic devices are not in scope but recipe has magnetic.in_scope=true".into(),
        );
    }
    if recipe.safety.require_operator_approval && recipe.safety.no_internal_sweep {
        // OK - this is the normal case
    }
    if let Some(ref lf) = recipe.modulation.internal_lf {
        validate_lf_shape(&lf.shape)?;
        if lf.lf_output_enabled {
            return Err("LF output must not be enabled (LFO ON is forbidden)".into());
        }
    }
    Ok(())
}

fn validate_lf_shape(shape: &str) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "SIN",
        "SQU",
        "TRI",
        "SAW",
        "ISAW",
        "SINE",
        "SQUARE",
        "TRIANGLE",
        "SAWTOOTH",
        "ISAWTOOTH",
    ];
    let s = shape.trim();
    if s.contains(';') {
        return Err(format!("LF shape '{}' contains semicolon", s));
    }
    if !ALLOWED.contains(&s) {
        return Err(format!(
            "LF shape '{}' is not valid. Allowed: {:?}",
            s, ALLOWED
        ));
    }
    Ok(())
}

/// Compute recipe hash (SHA-256 of canonical JSON).
pub fn recipe_hash(recipe: &M3_4Recipe) -> Result<String, String> {
    let json = serde_json::to_string(recipe).map_err(|e| format!("json: {}", e))?;
    Ok(crate::artifacts::sha256_bytes(json.as_bytes()))
}

/// Generate evenly-spaced frequencies from recipe RF config.
pub fn generate_frequencies(recipe: &M3_4Recipe) -> Vec<f64> {
    let n = recipe.rf.points;
    if n == 1 {
        return vec![recipe.rf.start_hz];
    }
    let step = (recipe.rf.stop_hz - recipe.rf.start_hz) / (n - 1) as f64;
    (0..n)
        .map(|i| recipe.rf.start_hz + step * i as f64)
        .collect()
}
