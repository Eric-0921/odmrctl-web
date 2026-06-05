//! System scan safety report builder.
//!
//! Checks a `SystemScanRecipe` + `ResolvedSystemScan` against policy rules
//! and produces a `SystemSafetyReport`.

use odmr_recipe::SystemScanRecipe;
use serde::{Deserialize, Serialize};

use crate::SafetyDecision;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Safety report for a system-scan recipe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemSafetyReport {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub resolved_recipe_id: String,
    pub decision: SafetyDecision,
    pub requires_operator_approval: bool,
    pub physical_response_required: bool,
    pub summary: SystemSafetySummary,
    pub checks: Vec<SystemSafetyCheck>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemSafetySummary {
    pub checked_steps: usize,
    pub checked_actions: usize,
    pub info_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemSafetyCheck {
    pub check: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Build a safety report for a system-scan recipe.
pub fn build_system_scan_safety_report(
    recipe: &SystemScanRecipe,
    resolved: &odmr_compiler::system_scan::ResolvedSystemScan,
    station_safety: &odmr_recipe::StationSafety,
) -> SystemSafetyReport {
    let mut checks: Vec<SystemSafetyCheck> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    // 1. Recipe schema valid
    checks.push(SystemSafetyCheck {
        check: "recipe_schema_valid".into(),
        status: "pass".into(),
        message: Some("system_scan_recipe schema validation passed".into()),
        value: None,
        limit: None,
    });

    // 2. Station ref valid (best-effort: non-empty)
    let station_valid = !recipe.station_ref.is_empty();
    checks.push(SystemSafetyCheck {
        check: "station_ref_valid".into(),
        status: if station_valid { "pass" } else { "fail" }.into(),
        message: Some(if station_valid {
            "station_ref is non-empty".into()
        } else {
            "station_ref is empty".into()
        }),
        value: None,
        limit: None,
    });
    if !station_valid {
        errors.push("station_ref is empty".into());
    }

    // 3. Safety limits not overridden
    let no_limit_override = !recipe.fixed_params.contains_key("safety_limits");
    checks.push(SystemSafetyCheck {
        check: "safety_limits_not_overridden".into(),
        status: if no_limit_override { "pass" } else { "fail" }.into(),
        message: Some(if no_limit_override {
            "recipe does not contain safety limit overrides".into()
        } else {
            "recipe contains safety limit overrides".into()
        }),
        value: None,
        limit: None,
    });
    if !no_limit_override {
        errors.push("recipe must not override safety limits".into());
    }

    // 4. SMB100A checks
    check_smb100a(
        recipe,
        station_safety,
        &mut checks,
        &mut errors,
        &mut warnings,
    );

    // 5. OE1022D checks
    check_oe1022d(recipe, station_safety, &mut checks, &mut errors);

    // 6. Magnetic checks
    check_magnetic(
        recipe,
        station_safety,
        resolved,
        &mut checks,
        &mut errors,
        &mut warnings,
    );

    // 7. Laser checks
    check_laser(recipe, station_safety, &mut checks, &mut errors);

    // 8. Recipe integrity checks
    check_recipe_integrity(recipe, &mut checks, &mut errors, &mut warnings);

    // 9. Expected points below limit
    let measure_steps = resolved
        .steps
        .iter()
        .filter(|s| s.phase == "measure")
        .count();
    let points_ok = measure_steps <= 100_000;
    checks.push(SystemSafetyCheck {
        check: "expected_points_below_limit".into(),
        status: if points_ok { "pass" } else { "fail" }.into(),
        message: Some(format!(
            "{} points below maximum point limit",
            measure_steps
        )),
        value: Some(measure_steps as f64),
        limit: Some(100_000.0),
    });
    if !points_ok {
        errors.push(format!("{} points exceeds limit of 100000", measure_steps));
    }

    // 10. Expected runtime below limit
    let runtime_ok = resolved.estimated_duration_s <= 3600.0;
    checks.push(SystemSafetyCheck {
        check: "expected_runtime_below_limit".into(),
        status: if runtime_ok { "pass" } else { "warn" }.into(),
        message: Some(format!(
            "estimated {:.1} s below maximum runtime limit",
            resolved.estimated_duration_s
        )),
        value: Some(resolved.estimated_duration_s),
        limit: Some(3600.0),
    });
    if !runtime_ok {
        warnings.push(format!(
            "estimated runtime {:.1}s exceeds 1 hour",
            resolved.estimated_duration_s
        ));
    }

    let error_count = errors.len();
    let warning_count = warnings.len();

    let decision = if error_count > 0 {
        SafetyDecision::Reject
    } else if warning_count > 0 {
        SafetyDecision::AllowWithWarnings
    } else {
        SafetyDecision::Allow
    };

    SystemSafetyReport {
        schema_version: "0.2.0".into(),
        kind: "safety_report".into(),
        id: format!("safety_{}", resolved.header.id),
        resolved_recipe_id: resolved.header.id.clone(),
        decision,
        requires_operator_approval: recipe.safety.require_operator_approval,
        physical_response_required: recipe.physical_response_required,
        summary: SystemSafetySummary {
            checked_steps: resolved.steps.len(),
            checked_actions: 0,
            info_count: 0,
            warning_count,
            error_count,
        },
        checks,
        warnings,
        errors,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn check_smb100a(
    recipe: &SystemScanRecipe,
    station: &odmr_recipe::StationSafety,
    checks: &mut Vec<SystemSafetyCheck>,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    // Frequency within allowed ranges.
    let freq = recipe
        .fixed_params
        .get("smb100a")
        .and_then(|v| v.get("rf"))
        .and_then(|v| v.get("frequency_hz"))
        .and_then(|v| v.as_f64())
        .or_else(|| {
            recipe
                .fixed_params
                .get("smb100a")
                .and_then(|v| v.get("frequency_hz"))
                .and_then(|v| v.as_f64())
        });

    if let Some(f) = freq {
        let in_range = station
            .smb100a
            .allowed_frequency_ranges_hz
            .iter()
            .any(|range| f >= range[0] && f <= range[1]);
        checks.push(SystemSafetyCheck {
            check: "rf_frequency_within_station_allowed_range".into(),
            status: if in_range { "pass" } else { "fail" }.into(),
            message: Some(format!("RF frequency {} Hz within allowed range", f)),
            value: Some(f),
            limit: None,
        });
        if !in_range {
            errors.push(format!("RF frequency {} Hz outside allowed ranges", f));
        }
    }

    // Power within limit.
    let power = recipe
        .fixed_params
        .get("smb100a")
        .and_then(|v| v.get("rf"))
        .and_then(|v| v.get("power_dbm"))
        .and_then(|v| v.as_f64())
        .or_else(|| {
            recipe
                .fixed_params
                .get("smb100a")
                .and_then(|v| v.get("rf_power_dbm"))
                .and_then(|v| v.as_f64())
        });

    match power {
        Some(p) => {
            let ok = p <= station.smb100a.max_power_dbm;
            checks.push(SystemSafetyCheck {
                check: "smb_power_within_limit".into(),
                status: if ok { "pass" } else { "fail" }.into(),
                message: Some(format!("RF power {} dBm within station limit", p)),
                value: Some(p),
                limit: Some(station.smb100a.max_power_dbm),
            });
            if !ok {
                errors.push(format!(
                    "RF power {} dBm exceeds limit {} dBm",
                    p, station.smb100a.max_power_dbm
                ));
            }
        }
        None => {
            checks.push(SystemSafetyCheck {
                check: "smb_power_within_limit".into(),
                status: "warn".into(),
                message: Some("RF power not specified in fixed_params".into()),
                value: None,
                limit: Some(station.smb100a.max_power_dbm),
            });
        }
    }

    // Internal sweep disabled.
    let no_internal_sweep = recipe.safety.no_internal_smb_sweep;
    checks.push(SystemSafetyCheck {
        check: "smb_internal_sweep_disabled".into(),
        status: if no_internal_sweep { "pass" } else { "warn" }.into(),
        message: Some(if no_internal_sweep {
            "no internal SMB sweep requested".into()
        } else {
            "recipe does not explicitly disable internal SMB sweep".into()
        }),
        value: None,
        limit: None,
    });
    if !no_internal_sweep {
        warnings.push("internal SMB sweep not explicitly disabled".into());
    }

    // Output requires approval.
    let output_approval = station.smb100a.require_operator_approval_for_output_on;
    checks.push(SystemSafetyCheck {
        check: "rf_output_requires_operator_approval".into(),
        status: if output_approval { "pass" } else { "warn" }.into(),
        message: Some(if output_approval {
            "operator approval required for RF output ON".into()
        } else {
            "operator approval not required for RF output".into()
        }),
        value: None,
        limit: None,
    });

    // Modulation approval.
    let mod_approval = station.smb100a.require_modulation_approval;
    checks.push(SystemSafetyCheck {
        check: "modulation_requires_operator_approval".into(),
        status: if mod_approval { "pass" } else { "warn" }.into(),
        message: Some(if mod_approval {
            "operator approval required for modulation".into()
        } else {
            "operator approval not required for modulation".into()
        }),
        value: None,
        limit: None,
    });

    // FM deviation non-negative.
    let fm_dev = recipe
        .fixed_params
        .get("smb100a")
        .and_then(|v| v.get("fm"))
        .and_then(|v| v.get("deviation_hz"))
        .and_then(|v| v.as_f64())
        .or_else(|| {
            recipe
                .fixed_params
                .get("smb100a")
                .and_then(|v| v.get("fm_deviation_hz"))
                .and_then(|v| v.as_f64())
        });

    if let Some(dev) = fm_dev {
        let ok = dev >= 0.0;
        checks.push(SystemSafetyCheck {
            check: "fm_deviation_non_negative".into(),
            status: if ok { "pass" } else { "fail" }.into(),
            message: Some(format!("FM deviation {} Hz", dev)),
            value: Some(dev),
            limit: Some(0.0),
        });
        if !ok {
            errors.push(format!("FM deviation {} Hz is negative", dev));
        }
    }

    // LF frequency positive.
    let lf_freq = recipe
        .fixed_params
        .get("smb100a")
        .and_then(|v| v.get("lf"))
        .and_then(|v| v.get("frequency_hz"))
        .and_then(|v| v.as_f64());

    if let Some(f) = lf_freq {
        let ok = f > 0.0;
        checks.push(SystemSafetyCheck {
            check: "lf_frequency_positive".into(),
            status: if ok { "pass" } else { "fail" }.into(),
            message: Some(format!("LF frequency {} Hz", f)),
            value: Some(f),
            limit: Some(0.0),
        });
        if !ok {
            errors.push(format!("LF frequency {} Hz must be positive", f));
        }
    }

    // LF voltage non-negative.
    let lf_volt = recipe
        .fixed_params
        .get("smb100a")
        .and_then(|v| v.get("lf"))
        .and_then(|v| v.get("voltage_v"))
        .and_then(|v| v.as_f64());

    if let Some(v) = lf_volt {
        let ok = v >= 0.0;
        checks.push(SystemSafetyCheck {
            check: "lf_voltage_non_negative".into(),
            status: if ok { "pass" } else { "fail" }.into(),
            message: Some(format!("LF voltage {} V", v)),
            value: Some(v),
            limit: Some(0.0),
        });
        if !ok {
            errors.push(format!("LF voltage {} V is negative", v));
        }
    }

    // FM enabled but global modulation disabled → warning.
    let fm_enabled = recipe
        .fixed_params
        .get("smb100a")
        .and_then(|v| v.get("fm"))
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let global_mod = recipe
        .fixed_params
        .get("smb100a")
        .and_then(|v| v.get("modulation"))
        .and_then(|v| v.get("global_enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if fm_enabled && !global_mod {
        warnings.push("FM configured but global modulation is disabled".into());
        checks.push(SystemSafetyCheck {
            check: "fm_modulation_consistency".into(),
            status: "warn".into(),
            message: Some("FM enabled but global modulation disabled".into()),
            value: None,
            limit: None,
        });
    }
}

fn check_oe1022d(
    recipe: &SystemScanRecipe,
    station: &odmr_recipe::StationSafety,
    checks: &mut Vec<SystemSafetyCheck>,
    errors: &mut Vec<String>,
) {
    let oe = match odmr_recipe::Oe1022dConfig::try_from_value(
        recipe
            .fixed_params
            .get("oe1022d")
            .unwrap_or(&serde_json::json!({})),
    ) {
        Ok(c) => c,
        Err(_) => {
            checks.push(SystemSafetyCheck {
                check: "oe1022d_config_valid".into(),
                status: "warn".into(),
                message: Some("OE1022D config could not be parsed".into()),
                value: None,
                limit: None,
            });
            return;
        }
    };

    // Primary channel matches station requirement.
    let channel_ok = oe.primary_channel == station.oe1022d.required_primary_channel;
    checks.push(SystemSafetyCheck {
        check: "primary_channel_is_supported".into(),
        status: if channel_ok { "pass" } else { "fail" }.into(),
        message: Some(format!(
            "primary channel {:?} matches station requirement {:?}",
            oe.primary_channel, station.oe1022d.required_primary_channel
        )),
        value: None,
        limit: None,
    });
    if !channel_ok {
        errors.push(format!(
            "primary channel {:?} does not match station requirement {:?}",
            oe.primary_channel, station.oe1022d.required_primary_channel
        ));
    }

    // Reference lock required.
    checks.push(SystemSafetyCheck {
        check: "required_reference_lock_declared".into(),
        status: if station.oe1022d.required_reference_lock {
            "pass".into()
        } else {
            "warn".into()
        },
        message: Some(if station.oe1022d.required_reference_lock {
            "reference lock required by station".into()
        } else {
            "reference lock not required by station".into()
        }),
        value: None,
        limit: None,
    });

    // Time constant positive.
    let tc_ok = oe.filter.time_constant_s > 0.0;
    checks.push(SystemSafetyCheck {
        check: "time_constant_positive".into(),
        status: if tc_ok { "pass" } else { "fail" }.into(),
        message: Some(format!("time constant {} s", oe.filter.time_constant_s)),
        value: Some(oe.filter.time_constant_s),
        limit: Some(0.0),
    });
    if !tc_ok {
        errors.push(format!(
            "time constant {} s must be positive",
            oe.filter.time_constant_s
        ));
    }

    // Filter slope supported (already enforced by enum).
    checks.push(SystemSafetyCheck {
        check: "filter_slope_supported".into(),
        status: "pass".into(),
        message: Some(format!(
            "filter slope {:?} dB/oct supported",
            oe.filter.slope_db_oct
        )),
        value: None,
        limit: None,
    });

    // Record fields supported (already enforced by enum).
    checks.push(SystemSafetyCheck {
        check: "record_fields_supported".into(),
        status: "pass".into(),
        message: Some(format!(
            "{} record fields supported",
            oe.acquisition.record_fields.len()
        )),
        value: None,
        limit: None,
    });
}

fn check_magnetic(
    recipe: &SystemScanRecipe,
    station: &odmr_recipe::StationSafety,
    resolved: &odmr_compiler::system_scan::ResolvedSystemScan,
    checks: &mut Vec<SystemSafetyCheck>,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    let mag_params = recipe.fixed_params.get("magnetic");

    // Coil matrix present.
    let coil_present = mag_params
        .and_then(|v| v.get("coil_matrix"))
        .and_then(|v| v.get("matrix"))
        .is_some();
    checks.push(SystemSafetyCheck {
        check: "coil_matrix_present".into(),
        status: if coil_present { "pass" } else { "fail" }.into(),
        message: Some(if coil_present {
            "coil matrix present in recipe".into()
        } else {
            "coil matrix missing from recipe".into()
        }),
        value: None,
        limit: None,
    });
    if !coil_present {
        errors.push("coil matrix is required for magnetic control".into());
        return;
    }

    // Coil matrix not singular: check by looking at predicted_current in resolved steps.
    let mut has_predicted_current = false;
    for step in &resolved.steps {
        if let Some(mag) = step.target_device_state.get("magnetic") {
            if mag.get("predicted_current_a").is_some() {
                has_predicted_current = true;
                break;
            }
        }
    }

    checks.push(SystemSafetyCheck {
        check: "coil_matrix_not_singular".into(),
        status: if has_predicted_current {
            "pass"
        } else {
            "fail"
        }
        .into(),
        message: Some(if has_predicted_current {
            "coil matrix is invertible (predicted current computed)".into()
        } else {
            "coil matrix appears singular (no predicted current)".into()
        }),
        value: None,
        limit: None,
    });
    if !has_predicted_current {
        errors.push("coil matrix is singular or all-zero; cannot compute predicted current".into());
    }

    // Max B field from sweep.
    let mut max_b_nt: f64 = 0.0;
    if let Some(mag_sweep) = recipe.sweeps.iter().find(|s| s.device == "magnetic") {
        use odmr_recipe::SweepAxisValue;
        if let odmr_recipe::SweepShape::CartesianGrid { axes, .. } = &mag_sweep.shape {
            for val in axes.values() {
                if let SweepAxisValue::Values { values } = val {
                    for &v in values {
                        max_b_nt = max_b_nt.max(v.abs());
                    }
                }
            }
        }
    }
    let max_b_t = max_b_nt / 1e9;
    let b_ok = max_b_t <= station.magnetic.max_b_vector_t;
    checks.push(SystemSafetyCheck {
        check: "b_vector_within_station_limit".into(),
        status: if b_ok { "pass" } else { "fail" }.into(),
        message: Some(format!("max B field {:.6} T within limit", max_b_t)),
        value: Some(max_b_t),
        limit: Some(station.magnetic.max_b_vector_t),
    });
    if !b_ok {
        errors.push(format!(
            "magnetic field {:.6} T exceeds limit {:.6} T",
            max_b_t, station.magnetic.max_b_vector_t
        ));
    }

    // Predicted current within axis limits.
    for step in resolved.steps.iter().filter(|s| s.phase == "measure") {
        if let Some(mag) = step.target_device_state.get("magnetic") {
            if let Some(current) = mag.get("predicted_current_a").and_then(|v| v.as_array()) {
                let ix = current.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                let iy = current.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let iz = current.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0);

                let x_ok = ix.abs() <= station.magnetic.max_current_a_per_axis.x;
                let y_ok = iy.abs() <= station.magnetic.max_current_a_per_axis.y;
                let z_ok = iz.abs() <= station.magnetic.max_current_a_per_axis.z;

                if !x_ok || !y_ok || !z_ok {
                    checks.push(SystemSafetyCheck {
                        check: "predicted_current_within_axis_limits".into(),
                        status: "fail".into(),
                        message: Some(format!(
                            "step {} predicted current [{:.3}, {:.3}, {:.3}] A exceeds limit",
                            step.step_id, ix, iy, iz
                        )),
                        value: Some(ix.abs().max(iy.abs()).max(iz.abs())),
                        limit: Some(station.magnetic.max_current_a_per_axis.x),
                    });
                    errors.push(format!(
                        "step {} predicted current exceeds axis limit",
                        step.step_id
                    ));
                    break; // Only report first violation
                }
            }
        }
    }

    // Current ramp within limits (best-effort: check adjacent points).
    let measure_steps: Vec<_> = resolved
        .steps
        .iter()
        .filter(|s| s.phase == "measure")
        .collect();
    let mut ramp_ok = true;
    for i in 1..measure_steps.len() {
        let prev = &measure_steps[i - 1];
        let curr = &measure_steps[i];
        if let (Some(p_mag), Some(c_mag)) = (
            prev.target_device_state.get("magnetic"),
            curr.target_device_state.get("magnetic"),
        ) {
            if let (Some(p_cur), Some(c_cur)) = (
                p_mag.get("predicted_current_a").and_then(|v| v.as_array()),
                c_mag.get("predicted_current_a").and_then(|v| v.as_array()),
            ) {
                let dx =
                    (c_cur[0].as_f64().unwrap_or(0.0) - p_cur[0].as_f64().unwrap_or(0.0)).abs();
                let dy =
                    (c_cur[1].as_f64().unwrap_or(0.0) - p_cur[1].as_f64().unwrap_or(0.0)).abs();
                let dz =
                    (c_cur[2].as_f64().unwrap_or(0.0) - p_cur[2].as_f64().unwrap_or(0.0)).abs();

                if dx > station.magnetic.max_ramp_a_per_s.x
                    || dy > station.magnetic.max_ramp_a_per_s.y
                    || dz > station.magnetic.max_ramp_a_per_s.z
                {
                    ramp_ok = false;
                    break;
                }
            }
        }
    }
    checks.push(SystemSafetyCheck {
        check: "current_ramp_within_limits".into(),
        status: if ramp_ok { "pass" } else { "warn" }.into(),
        message: Some(if ramp_ok {
            "current ramp within axis limits".into()
        } else {
            "current ramp exceeds axis limits (warning)".into()
        }),
        value: None,
        limit: None,
    });
    if !ramp_ok {
        warnings.push("current ramp between adjacent points exceeds axis limit".into());
    }

    // Zero lock / readback required.
    checks.push(SystemSafetyCheck {
        check: "zero_lock_required".into(),
        status: if station.magnetic.require_zero_lock {
            "pass".into()
        } else {
            "warn".into()
        },
        message: Some(if station.magnetic.require_zero_lock {
            "zero lock required by station".into()
        } else {
            "zero lock not required by station".into()
        }),
        value: None,
        limit: None,
    });

    checks.push(SystemSafetyCheck {
        check: "readback_required".into(),
        status: if station.magnetic.require_readback {
            "pass".into()
        } else {
            "warn".into()
        },
        message: Some(if station.magnetic.require_readback {
            "readback required by station".into()
        } else {
            "readback not required by station".into()
        }),
        value: None,
        limit: None,
    });

    // Axis serial numbers present.
    let sn_present = mag_params
        .and_then(|v| v.get("axes"))
        .and_then(|v| v.get("x"))
        .and_then(|v| v.get("serial_number"))
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    checks.push(SystemSafetyCheck {
        check: "axis_serial_numbers_present".into(),
        status: if sn_present { "pass" } else { "warn" }.into(),
        message: Some(if sn_present {
            "axis serial numbers present".into()
        } else {
            "axis serial numbers missing".into()
        }),
        value: None,
        limit: None,
    });
}

fn check_laser(
    recipe: &SystemScanRecipe,
    station: &odmr_recipe::StationSafety,
    checks: &mut Vec<SystemSafetyCheck>,
    errors: &mut Vec<String>,
) {
    let laser_enabled = recipe
        .fixed_params
        .get("laser")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Laser disabled.
    let laser_ok = !laser_enabled;
    checks.push(SystemSafetyCheck {
        check: "laser_disabled".into(),
        status: if laser_ok { "pass" } else { "fail" }.into(),
        message: Some(if laser_ok {
            "laser is disabled".into()
        } else {
            "laser is enabled".into()
        }),
        value: None,
        limit: None,
    });
    if !laser_ok {
        errors.push("laser must be disabled".into());
    }

    // Laser power within limit.
    let power = recipe
        .fixed_params
        .get("laser")
        .and_then(|v| v.get("power_mw"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let power_ok = power <= station.laser.max_power_mw;
    checks.push(SystemSafetyCheck {
        check: "laser_power_within_limit".into(),
        status: if power_ok { "pass" } else { "fail" }.into(),
        message: Some(format!("laser power {} mW within limit", power)),
        value: Some(power),
        limit: Some(station.laser.max_power_mw),
    });
    if !power_ok {
        errors.push(format!(
            "laser power {} mW exceeds limit {} mW",
            power, station.laser.max_power_mw
        ));
    }
}

fn check_recipe_integrity(
    recipe: &SystemScanRecipe,
    checks: &mut Vec<SystemSafetyCheck>,
    _errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    // Operator approval.
    let op_approval = recipe.safety.require_operator_approval;
    checks.push(SystemSafetyCheck {
        check: "operator_approval_required".into(),
        status: if op_approval { "pass" } else { "warn" }.into(),
        message: Some(if op_approval {
            "operator approval is required by recipe safety block".into()
        } else {
            "recipe does not require operator approval".into()
        }),
        value: None,
        limit: None,
    });
    if !op_approval {
        warnings.push("operator approval not required by recipe".into());
    }

    // No realtime CSV.
    let no_csv = recipe.safety.no_realtime_csv;
    checks.push(SystemSafetyCheck {
        check: "no_realtime_csv".into(),
        status: if no_csv { "pass" } else { "warn" }.into(),
        message: Some(if no_csv {
            "recipe does not request realtime CSV output".into()
        } else {
            "recipe does not explicitly disable realtime CSV".into()
        }),
        value: None,
        limit: None,
    });
    if !no_csv {
        warnings.push("realtime CSV not explicitly disabled".into());
    }

    // No GUI direct hardware.
    let no_gui_hw = recipe.safety.no_gui_direct_hardware;
    checks.push(SystemSafetyCheck {
        check: "no_gui_direct_hardware".into(),
        status: if no_gui_hw { "pass" } else { "warn" }.into(),
        message: Some(if no_gui_hw {
            "recipe does not request GUI direct hardware control".into()
        } else {
            "recipe does not explicitly disable GUI direct hardware".into()
        }),
        value: None,
        limit: None,
    });
    if !no_gui_hw {
        warnings.push("GUI direct hardware not explicitly disabled".into());
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use odmr_compiler::system_scan::expand_system_scan_recipe;
    use odmr_recipe::parse_system_scan_recipe;

    fn default_station() -> odmr_recipe::StationSafety {
        odmr_recipe::StationSafety::default()
    }

    fn example_report() -> SystemSafetyReport {
        let json = include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json");
        let recipe = parse_system_scan_recipe(json).unwrap();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        build_system_scan_safety_report(&recipe, &resolved, &default_station())
    }

    #[test]
    fn safety_report_allows_example_recipe() {
        let report = example_report();
        assert_eq!(report.decision, SafetyDecision::Allow);
        assert_eq!(report.summary.error_count, 0);
    }

    #[test]
    fn safety_report_requires_operator_approval() {
        let report = example_report();
        assert!(report.requires_operator_approval);
    }

    #[test]
    fn all_expected_checks_present() {
        let report = example_report();
        let check_names: Vec<_> = report.checks.iter().map(|c| c.check.as_str()).collect();
        assert!(check_names.contains(&"recipe_schema_valid"));
        assert!(check_names.contains(&"station_ref_valid"));
        assert!(check_names.contains(&"safety_limits_not_overridden"));
        assert!(check_names.contains(&"smb_power_within_limit"));
        assert!(check_names.contains(&"smb_internal_sweep_disabled"));
        assert!(check_names.contains(&"rf_frequency_within_station_allowed_range"));
        assert!(check_names.contains(&"b_vector_within_station_limit"));
        assert!(check_names.contains(&"operator_approval_required"));
        assert!(check_names.contains(&"expected_points_below_limit"));
        assert!(check_names.contains(&"expected_runtime_below_limit"));
    }

    #[test]
    fn laser_enabled_is_rejected() {
        let json = include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json");
        let mut recipe = parse_system_scan_recipe(json).unwrap();
        // Update both device ref and fixed_params.
        if let Some(laser) = recipe.devices.get_mut("laser") {
            laser.enabled = Some(true);
        }
        if let Some(laser_params) = recipe.fixed_params.get_mut("laser") {
            if let Some(obj) = laser_params.as_object_mut() {
                obj.insert("enabled".into(), serde_json::json!(true));
            }
        }
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let report = build_system_scan_safety_report(&recipe, &resolved, &default_station());
        assert_eq!(report.decision, SafetyDecision::Reject);
        assert!(report.errors.iter().any(|e| e.contains("laser")));
    }

    #[test]
    fn safety_limit_override_is_rejected() {
        let json = include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json");
        let mut recipe = parse_system_scan_recipe(json).unwrap();
        recipe.fixed_params.insert(
            "safety_limits".into(),
            serde_json::json!({ "max_power_dbm": 50 }),
        );
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let report = build_system_scan_safety_report(&recipe, &resolved, &default_station());
        assert_eq!(report.decision, SafetyDecision::Reject);
        assert!(report.errors.iter().any(|e| e.contains("safety limit")));
    }

    #[test]
    fn report_serializes_to_json() {
        let report = example_report();
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("safety_report"));
        assert!(json.contains("allow"));
    }

    #[test]
    fn singular_coil_matrix_fails() {
        let json = include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json");
        let mut recipe = parse_system_scan_recipe(json).unwrap();
        let mag = recipe.fixed_params.get_mut("magnetic").unwrap();
        let obj = mag.as_object_mut().unwrap();
        obj.insert(
            "coil_matrix".into(),
            serde_json::json!({
                "unit": "T_per_A",
                "matrix": [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [3.0, 6.0, 9.0]]
            }),
        );
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let report = build_system_scan_safety_report(&recipe, &resolved, &default_station());
        assert_eq!(report.decision, SafetyDecision::Reject);
        assert!(report.errors.iter().any(|e| e.contains("singular")));
    }

    #[test]
    fn rf_frequency_out_of_range_fails() {
        let json = include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json");
        let mut recipe = parse_system_scan_recipe(json).unwrap();
        let smb = recipe.fixed_params.get_mut("smb100a").unwrap();
        let obj = smb.as_object_mut().unwrap();
        let rf = obj.get_mut("rf").unwrap().as_object_mut().unwrap();
        rf.insert("frequency_hz".into(), serde_json::json!(4_000_000_000.0));
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let report = build_system_scan_safety_report(&recipe, &resolved, &default_station());
        assert_eq!(report.decision, SafetyDecision::Reject);
        assert!(report.errors.iter().any(|e| e.contains("frequency")));
    }

    #[test]
    fn rf_power_too_high_fails() {
        let json = include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json");
        let mut recipe = parse_system_scan_recipe(json).unwrap();
        let smb = recipe.fixed_params.get_mut("smb100a").unwrap();
        let obj = smb.as_object_mut().unwrap();
        let rf = obj.get_mut("rf").unwrap().as_object_mut().unwrap();
        rf.insert("power_dbm".into(), serde_json::json!(10.0));
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let report = build_system_scan_safety_report(&recipe, &resolved, &default_station());
        assert_eq!(report.decision, SafetyDecision::Reject);
        assert!(report.errors.iter().any(|e| e.contains("power")));
    }

    #[test]
    fn magnetic_current_limit_exceeded_fails() {
        let json = include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json");
        let recipe = parse_system_scan_recipe(json).unwrap();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let mut tight_station = default_station();
        tight_station.magnetic.max_current_a_per_axis.x = 0.001;
        tight_station.magnetic.max_current_a_per_axis.y = 0.001;
        tight_station.magnetic.max_current_a_per_axis.z = 0.001;
        let report = build_system_scan_safety_report(&recipe, &resolved, &tight_station);
        assert_eq!(report.decision, SafetyDecision::Reject);
        assert!(report
            .errors
            .iter()
            .any(|e| e.contains("current") || e.contains("limit")));
    }

    #[test]
    fn magnetic_ramp_limit_exceeded_fails_or_warns() {
        let json = include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json");
        let recipe = parse_system_scan_recipe(json).unwrap();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let mut tight_station = default_station();
        tight_station.magnetic.max_ramp_a_per_s.x = 1e-9;
        tight_station.magnetic.max_ramp_a_per_s.y = 1e-9;
        tight_station.magnetic.max_ramp_a_per_s.z = 1e-9;
        let report = build_system_scan_safety_report(&recipe, &resolved, &tight_station);
        // Ramp limit violation is currently a warning, not a reject.
        assert!(report.warnings.iter().any(|w| w.contains("ramp")));
    }

    #[test]
    fn missing_operator_approval_warns() {
        let json = include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json");
        let mut recipe = parse_system_scan_recipe(json).unwrap();
        recipe.safety.require_operator_approval = false;
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let report = build_system_scan_safety_report(&recipe, &resolved, &default_station());
        assert_eq!(report.decision, SafetyDecision::AllowWithWarnings);
        assert!(report.warnings.iter().any(|w| w.contains("approval")));
    }
}
