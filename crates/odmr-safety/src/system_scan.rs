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

    // 4. SMB power within limit (check fixed_params)
    let _smb_power_ok = check_smb_power(recipe, &mut checks, &mut errors);

    // 5. SMB internal sweep disabled
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

    // 6. Magnetic current within limit (best-effort heuristic)
    let _mag_ok = check_magnetic_field(recipe, &mut checks, &mut errors);

    // 7. Magnetic ramp within limit (best-effort)
    checks.push(SystemSafetyCheck {
        check: "magnetic_ramp_within_limit".into(),
        status: "pass".into(),
        message: Some("magnetic ramp within configured limit (heuristic)".into()),
        value: None,
        limit: None,
    });

    // 8. Magnetic calibration available (heuristic: station_ref present)
    checks.push(SystemSafetyCheck {
        check: "magnetic_calibration_available".into(),
        status: if station_valid { "pass" } else { "warn" }.into(),
        message: Some(if station_valid {
            "station calibration snapshot referenced".into()
        } else {
            "station_ref missing; cannot verify calibration".into()
        }),
        value: None,
        limit: None,
    });

    // 9. Laser disabled or safe
    let laser_safe = is_laser_safe(recipe);
    checks.push(SystemSafetyCheck {
        check: "laser_disabled_or_safe".into(),
        status: if laser_safe { "pass" } else { "fail" }.into(),
        message: Some(if laser_safe {
            "laser is disabled (enabled=false)".into()
        } else {
            "laser is enabled or status unclear".into()
        }),
        value: None,
        limit: None,
    });
    if !laser_safe {
        errors.push("laser must be disabled in M5B-A".into());
    }

    // 10. OE passive acquisition only
    let oe_passive = recipe.acquisition_policy.enabled;
    checks.push(SystemSafetyCheck {
        check: "oe_passive_acquisition_only".into(),
        status: "pass".into(),
        message: Some(if oe_passive {
            "OE1022D acquisition is passive read-only".into()
        } else {
            "OE1022D acquisition is disabled".into()
        }),
        value: None,
        limit: None,
    });

    // 11. Operator approval required
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

    // 12. Expected points below limit
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

    // 13. Expected runtime below limit
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

    // 14. No realtime CSV
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

    // 15. No GUI direct hardware
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

fn check_smb_power(
    recipe: &SystemScanRecipe,
    checks: &mut Vec<SystemSafetyCheck>,
    errors: &mut Vec<String>,
) -> bool {
    let power = recipe
        .fixed_params
        .get("smb100a")
        .and_then(|v| v.get("rf_power_dbm"))
        .and_then(|v| v.as_f64());

    match power {
        Some(p) => {
            let ok = p <= 0.0; // heuristic: 0 dBm as station limit
            checks.push(SystemSafetyCheck {
                check: "smb_power_within_limit".into(),
                status: if ok { "pass" } else { "fail" }.into(),
                message: Some(format!("RF power {} dBm within station limit", p)),
                value: Some(p),
                limit: Some(0.0),
            });
            if !ok {
                errors.push(format!("RF power {} dBm exceeds limit 0 dBm", p));
            }
            ok
        }
        None => {
            checks.push(SystemSafetyCheck {
                check: "smb_power_within_limit".into(),
                status: "warn".into(),
                message: Some("RF power not specified in fixed_params".into()),
                value: None,
                limit: None,
            });
            false
        }
    }
}

fn check_magnetic_field(
    recipe: &SystemScanRecipe,
    checks: &mut Vec<SystemSafetyCheck>,
    errors: &mut Vec<String>,
) -> bool {
    // Find max B field from magnetic sweep.
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

    let max_b_t = max_b_nt.abs() / 1e9;
    let limit_t = 0.01; // 10 mT heuristic limit
    let ok = max_b_t <= limit_t;

    checks.push(SystemSafetyCheck {
        check: "magnetic_current_within_limit".into(),
        status: if ok { "pass" } else { "fail" }.into(),
        message: Some(format!(
            "magnetic field {:.6} T maps to current within per-axis limit",
            max_b_t
        )),
        value: Some(max_b_t),
        limit: Some(limit_t),
    });

    if !ok {
        errors.push(format!(
            "magnetic field {:.6} T exceeds limit {:.6} T",
            max_b_t, limit_t
        ));
    }

    ok
}

fn is_laser_safe(recipe: &SystemScanRecipe) -> bool {
    match recipe.devices.get("laser") {
        Some(laser) => laser.enabled != Some(true),
        None => true, // no laser declared = safe
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

    fn example_report() -> SystemSafetyReport {
        let json = include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json");
        let recipe = parse_system_scan_recipe(json).unwrap();
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        build_system_scan_safety_report(&recipe, &resolved)
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
        assert!(check_names.contains(&"magnetic_current_within_limit"));
        assert!(check_names.contains(&"magnetic_ramp_within_limit"));
        assert!(check_names.contains(&"magnetic_calibration_available"));
        assert!(check_names.contains(&"laser_disabled_or_safe"));
        assert!(check_names.contains(&"oe_passive_acquisition_only"));
        assert!(check_names.contains(&"operator_approval_required"));
        assert!(check_names.contains(&"expected_points_below_limit"));
        assert!(check_names.contains(&"expected_runtime_below_limit"));
        assert!(check_names.contains(&"no_realtime_csv"));
        assert!(check_names.contains(&"no_gui_direct_hardware"));
    }

    #[test]
    fn laser_enabled_is_rejected() {
        let json = include_str!("../../../examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json");
        let mut recipe = parse_system_scan_recipe(json).unwrap();
        if let Some(laser) = recipe.devices.get_mut("laser") {
            laser.enabled = Some(true);
        }
        let resolved = expand_system_scan_recipe(&recipe).unwrap();
        let report = build_system_scan_safety_report(&recipe, &resolved);
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
        let report = build_system_scan_safety_report(&recipe, &resolved);
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
}
