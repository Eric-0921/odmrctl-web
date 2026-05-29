//! odmr-safety — Layer 3 static safety interlock engine.
//!
//! Pure computation: `ResolvedRecipe` + `SafetyLimit` → `SafetyReport`.
//!
//! No hardware access. No fake devices. No executor. No GUI.

use odmr_recipe::{DeviceAction, RecipeStep, ResolvedRecipe, SafetyLimit};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Severity of a single safety finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetySeverity {
    Info,
    Warning,
    Error,
}

/// Overall decision for a safety report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyDecision {
    Allow,
    AllowWithWarnings,
    Reject,
}

/// Classification of what was checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyCheckKind {
    RfFrequency,
    RfPower,
    FmDeviation,
    MagneticField,
    UnknownAction,
    MissingLimit,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// A single safety observation (pass, warning, or violation).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SafetyFinding {
    pub check_kind: SafetyCheckKind,
    pub severity: SafetySeverity,
    pub step_id: String,
    pub device_id: String,
    pub action: String,
    pub field: String,
    pub value: f64,
    pub limit: f64,
    pub message: String,
}

/// Aggregated counts for a safety report.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SafetySummary {
    pub checked_steps: usize,
    pub checked_actions: usize,
    pub info_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
}

/// Full safety report for a resolved recipe.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SafetyReport {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub resolved_recipe_id: String,
    pub decision: SafetyDecision,
    pub summary: SafetySummary,
    pub findings: Vec<SafetyFinding>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check a `ResolvedRecipe` against `SafetyLimit`s and produce a `SafetyReport`.
pub fn check_resolved_recipe(resolved: &ResolvedRecipe, limits: &SafetyLimit) -> SafetyReport {
    let mut findings: Vec<SafetyFinding> = Vec::new();
    let mut checked_steps = 0usize;
    let mut checked_actions = 0usize;

    for step in &resolved.steps {
        checked_steps += 1;
        for action in &step.device_actions {
            checked_actions += 1;
            check_action(action, step, limits, &mut findings);
        }
    }

    let (info_count, warning_count, error_count) = count_severities(&findings);

    let decision = if error_count > 0 {
        SafetyDecision::Reject
    } else if warning_count > 0 {
        SafetyDecision::AllowWithWarnings
    } else {
        SafetyDecision::Allow
    };

    SafetyReport {
        schema_version: "0.2.0".into(),
        kind: "safety_report".into(),
        id: format!("safety_{}", resolved.header.id),
        resolved_recipe_id: resolved.header.id.clone(),
        decision,
        summary: SafetySummary {
            checked_steps,
            checked_actions,
            info_count,
            warning_count,
            error_count,
        },
        findings,
    }
}

/// Return the report ID for a given `SafetyReport`.
///
/// M1.3: deterministic format `safety_{resolved_recipe_id}`.
pub fn safety_report_id(report: &SafetyReport) -> String {
    report.id.clone()
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn check_action(
    action: &DeviceAction,
    step: &RecipeStep,
    limits: &SafetyLimit,
    findings: &mut Vec<SafetyFinding>,
) {
    match action.action.as_str() {
        "set_rf_frequency" => check_rf_frequency(action, step, limits, findings),
        "set_rf_power" => check_rf_power(action, step, limits, findings),
        "set_fm_deviation" => check_fm_deviation(action, step, limits, findings),
        "set_magnetic_field" => check_magnetic_field(action, step, limits, findings),
        _ => {
            // Unknown action → warning
            findings.push(SafetyFinding {
                check_kind: SafetyCheckKind::UnknownAction,
                severity: SafetySeverity::Warning,
                step_id: step.step_id.clone(),
                device_id: action.device_id.clone(),
                action: action.action.clone(),
                field: "action".into(),
                value: 0.0,
                limit: 0.0,
                message: format!(
                    "unrecognized action '{}' for device '{}'",
                    action.action, action.device_id
                ),
            });
        }
    }
}

fn check_rf_frequency(
    action: &DeviceAction,
    step: &RecipeStep,
    limits: &SafetyLimit,
    findings: &mut Vec<SafetyFinding>,
) {
    let Some(params) = &action.params else {
        return;
    };
    let Some(freq) = params.get("frequency_hz").and_then(|v| v.as_f64()) else {
        return;
    };

    if freq < limits.min_frequency_hz {
        findings.push(SafetyFinding {
            check_kind: SafetyCheckKind::RfFrequency,
            severity: SafetySeverity::Error,
            step_id: step.step_id.clone(),
            device_id: action.device_id.clone(),
            action: action.action.clone(),
            field: "frequency_hz".into(),
            value: freq,
            limit: limits.min_frequency_hz,
            message: format!(
                "RF frequency {freq} Hz is below minimum {} Hz",
                limits.min_frequency_hz
            ),
        });
    }

    if freq > limits.max_frequency_hz {
        findings.push(SafetyFinding {
            check_kind: SafetyCheckKind::RfFrequency,
            severity: SafetySeverity::Error,
            step_id: step.step_id.clone(),
            device_id: action.device_id.clone(),
            action: action.action.clone(),
            field: "frequency_hz".into(),
            value: freq,
            limit: limits.max_frequency_hz,
            message: format!(
                "RF frequency {freq} Hz exceeds maximum {} Hz",
                limits.max_frequency_hz
            ),
        });
    }
}

fn check_rf_power(
    action: &DeviceAction,
    step: &RecipeStep,
    limits: &SafetyLimit,
    findings: &mut Vec<SafetyFinding>,
) {
    let Some(params) = &action.params else {
        return;
    };
    let Some(power) = params.get("power_dbm").and_then(|v| v.as_f64()) else {
        return;
    };

    if power > limits.max_power_dbm {
        findings.push(SafetyFinding {
            check_kind: SafetyCheckKind::RfPower,
            severity: SafetySeverity::Error,
            step_id: step.step_id.clone(),
            device_id: action.device_id.clone(),
            action: action.action.clone(),
            field: "power_dbm".into(),
            value: power,
            limit: limits.max_power_dbm,
            message: format!(
                "RF power {power} dBm exceeds maximum {} dBm",
                limits.max_power_dbm
            ),
        });
    }
}

fn check_fm_deviation(
    action: &DeviceAction,
    step: &RecipeStep,
    limits: &SafetyLimit,
    findings: &mut Vec<SafetyFinding>,
) {
    let Some(params) = &action.params else {
        return;
    };
    let Some(dev) = params.get("fm_deviation_hz").and_then(|v| v.as_f64()) else {
        return;
    };

    let Some(limit) = limits.max_fm_deviation_hz else {
        findings.push(SafetyFinding {
            check_kind: SafetyCheckKind::MissingLimit,
            severity: SafetySeverity::Info,
            step_id: step.step_id.clone(),
            device_id: action.device_id.clone(),
            action: action.action.clone(),
            field: "fm_deviation_hz".into(),
            value: dev,
            limit: 0.0,
            message: "FM deviation limit not configured; skipping check".into(),
        });
        return;
    };

    if dev > limit {
        findings.push(SafetyFinding {
            check_kind: SafetyCheckKind::FmDeviation,
            severity: SafetySeverity::Error,
            step_id: step.step_id.clone(),
            device_id: action.device_id.clone(),
            action: action.action.clone(),
            field: "fm_deviation_hz".into(),
            value: dev,
            limit,
            message: format!("FM deviation {dev} Hz exceeds maximum {limit} Hz"),
        });
    }
}

fn check_magnetic_field(
    action: &DeviceAction,
    step: &RecipeStep,
    limits: &SafetyLimit,
    findings: &mut Vec<SafetyFinding>,
) {
    let Some(params) = &action.params else {
        return;
    };
    let Some(field_val) = params.get("b_abs_t").and_then(|v| v.as_f64()) else {
        return;
    };

    let Some(limit) = limits.max_magnetic_field_t else {
        findings.push(SafetyFinding {
            check_kind: SafetyCheckKind::MissingLimit,
            severity: SafetySeverity::Info,
            step_id: step.step_id.clone(),
            device_id: action.device_id.clone(),
            action: action.action.clone(),
            field: "b_abs_t".into(),
            value: field_val,
            limit: 0.0,
            message: "Magnetic field limit not configured; skipping check".into(),
        });
        return;
    };

    if field_val > limit {
        findings.push(SafetyFinding {
            check_kind: SafetyCheckKind::MagneticField,
            severity: SafetySeverity::Error,
            step_id: step.step_id.clone(),
            device_id: action.device_id.clone(),
            action: action.action.clone(),
            field: "b_abs_t".into(),
            value: field_val,
            limit,
            message: format!("Magnetic field {field_val} T exceeds maximum {limit} T"),
        });
    }
}

fn count_severities(findings: &[SafetyFinding]) -> (usize, usize, usize) {
    let mut info = 0usize;
    let mut warning = 0usize;
    let mut error = 0usize;
    for f in findings {
        match f.severity {
            SafetySeverity::Info => info += 1,
            SafetySeverity::Warning => warning += 1,
            SafetySeverity::Error => error += 1,
        }
    }
    (info, warning, error)
}

// ---------------------------------------------------------------------------
// Helpers for building test recipes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test_helpers {
    use super::*;
    use odmr_recipe::{CommonHeader, TimingConfig};

    /// Build a minimal ResolvedRecipe with a single step containing one action.
    pub fn resolved_recipe_with_action(action: DeviceAction) -> ResolvedRecipe {
        ResolvedRecipe {
            header: CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "resolved_recipe".into(),
                id: "test_resolved".into(),
                name: None,
                created_by: None,
                created_at: None,
                description: None,
            },
            source_recipe_id: "test_recipe".into(),
            source_recipe_hash: "abcd".into(),
            station_id: "station_test".into(),
            estimated_duration_s: 1.0,
            safety_report_id: "pending".into(),
            steps: vec![RecipeStep {
                step_id: "step_000001".into(),
                phase: "sweep".into(),
                device_actions: vec![action],
                expected_state: serde_json::Value::Null,
                timing: Some(TimingConfig {
                    default_dwell_ms: Some(500),
                    default_settle_ms: Some(500),
                }),
                acquisition: None,
                sweep_coordinates: None,
                source_block_id: None,
                source_sweep_id: None,
                point_index: None,
                total_points: None,
                estimated_duration_ms: None,
            }],
        }
    }

    /// Standard mock safety limits.
    pub fn mock_safety_limits() -> SafetyLimit {
        SafetyLimit {
            schema_version: "0.2.0".into(),
            kind: "safety_limit".into(),
            id: "mock_limits".into(),
            name: Some("Mock Safety Limits".into()),
            max_power_dbm: 20.0,
            max_frequency_hz: 3_000_000_000.0,
            min_frequency_hz: 1_000_000.0,
            max_fm_deviation_hz: Some(10_000_000.0),
            max_magnetic_field_t: Some(0.01),
            max_mag_ramp_rate_a_per_s: Some(0.1),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::test_helpers::*;
    use super::*;

    #[test]
    fn basic_resolved_recipe_passes_safety() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({ "frequency_hz": 2_820_000_000.0 })),
        };
        let resolved = resolved_recipe_with_action(action);
        let limits = mock_safety_limits();
        let report = check_resolved_recipe(&resolved, &limits);
        assert_eq!(report.decision, SafetyDecision::Allow);
        assert_eq!(report.summary.error_count, 0);
    }

    #[test]
    fn over_max_frequency_is_rejected() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({ "frequency_hz": 4_000_000_000.0 })),
        };
        let resolved = resolved_recipe_with_action(action);
        let limits = mock_safety_limits();
        let report = check_resolved_recipe(&resolved, &limits);
        assert_eq!(report.decision, SafetyDecision::Reject);
        assert!(report.summary.error_count > 0);
        let f = report
            .findings
            .iter()
            .find(|f| f.check_kind == SafetyCheckKind::RfFrequency);
        assert!(f.is_some());
    }

    #[test]
    fn under_min_frequency_is_rejected() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({ "frequency_hz": 500_000.0 })),
        };
        let resolved = resolved_recipe_with_action(action);
        let limits = mock_safety_limits();
        let report = check_resolved_recipe(&resolved, &limits);
        assert_eq!(report.decision, SafetyDecision::Reject);
        assert!(report.summary.error_count > 0);
    }

    #[test]
    fn rf_power_over_max_is_rejected() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_power".into(),
            params: Some(serde_json::json!({ "power_dbm": 25.0 })),
        };
        let resolved = resolved_recipe_with_action(action);
        let limits = mock_safety_limits();
        let report = check_resolved_recipe(&resolved, &limits);
        assert_eq!(report.decision, SafetyDecision::Reject);
        assert!(report
            .findings
            .iter()
            .any(|f| f.check_kind == SafetyCheckKind::RfPower));
    }

    #[test]
    fn fm_deviation_over_max_is_rejected() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_fm_deviation".into(),
            params: Some(serde_json::json!({ "fm_deviation_hz": 20_000_000.0 })),
        };
        let resolved = resolved_recipe_with_action(action);
        let limits = mock_safety_limits();
        let report = check_resolved_recipe(&resolved, &limits);
        assert_eq!(report.decision, SafetyDecision::Reject);
        assert!(report
            .findings
            .iter()
            .any(|f| f.check_kind == SafetyCheckKind::FmDeviation));
    }

    #[test]
    fn magnetic_field_over_max_is_rejected() {
        let action = DeviceAction {
            device_id: "mag_xyz_01".into(),
            action: "set_magnetic_field".into(),
            params: Some(serde_json::json!({ "b_abs_t": 0.02 })),
        };
        let resolved = resolved_recipe_with_action(action);
        let limits = mock_safety_limits();
        let report = check_resolved_recipe(&resolved, &limits);
        assert_eq!(report.decision, SafetyDecision::Reject);
        assert!(report
            .findings
            .iter()
            .any(|f| f.check_kind == SafetyCheckKind::MagneticField));
    }

    #[test]
    fn unknown_action_creates_warning() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_unknown_param".into(),
            params: None,
        };
        let resolved = resolved_recipe_with_action(action);
        let limits = mock_safety_limits();
        let report = check_resolved_recipe(&resolved, &limits);
        assert_eq!(report.decision, SafetyDecision::AllowWithWarnings);
        assert_eq!(report.summary.warning_count, 1);
        assert!(report
            .findings
            .iter()
            .any(|f| f.check_kind == SafetyCheckKind::UnknownAction));
    }

    #[test]
    fn allow_when_no_findings() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({ "frequency_hz": 2_000_000_000.0 })),
        };
        let resolved = resolved_recipe_with_action(action);
        let limits = mock_safety_limits();
        let report = check_resolved_recipe(&resolved, &limits);
        assert_eq!(report.decision, SafetyDecision::Allow);
        assert_eq!(report.summary.error_count, 0);
        assert_eq!(report.summary.warning_count, 0);
    }

    #[test]
    fn allow_with_warnings_when_only_warnings() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_unknown_thing".into(),
            params: None,
        };
        let resolved = resolved_recipe_with_action(action);
        let limits = mock_safety_limits();
        let report = check_resolved_recipe(&resolved, &limits);
        assert_eq!(report.decision, SafetyDecision::AllowWithWarnings);
        assert_eq!(report.summary.error_count, 0);
        assert!(report.summary.warning_count > 0);
    }

    #[test]
    fn reject_when_any_error() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({ "frequency_hz": 4_000_000_000.0 })),
        };
        let resolved = resolved_recipe_with_action(action);
        let limits = mock_safety_limits();
        let report = check_resolved_recipe(&resolved, &limits);
        assert_eq!(report.decision, SafetyDecision::Reject);
    }

    #[test]
    fn report_id_is_deterministic() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({ "frequency_hz": 2_000_000_000.0 })),
        };
        let resolved = resolved_recipe_with_action(action);
        let limits = mock_safety_limits();
        let report1 = check_resolved_recipe(&resolved, &limits);
        let report2 = check_resolved_recipe(&resolved, &limits);
        assert_eq!(safety_report_id(&report1), safety_report_id(&report2));
        assert_eq!(safety_report_id(&report1), "safety_test_resolved");
    }

    #[test]
    fn no_hardware_or_fake_device_dependency() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({ "frequency_hz": 2_000_000_000.0 })),
        };
        let resolved = resolved_recipe_with_action(action);
        let limits = mock_safety_limits();
        let _report = check_resolved_recipe(&resolved, &limits);
        // Pure computation: if we reach here, no external dependency was invoked.
    }

    #[test]
    fn missing_limit_produces_info_not_error() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_fm_deviation".into(),
            params: Some(serde_json::json!({ "fm_deviation_hz": 1_000_000.0 })),
        };
        let resolved = resolved_recipe_with_action(action);
        let mut limits = mock_safety_limits();
        limits.max_fm_deviation_hz = None;
        let report = check_resolved_recipe(&resolved, &limits);
        assert_eq!(report.decision, SafetyDecision::Allow);
        assert!(report
            .findings
            .iter()
            .any(|f| f.severity == SafetySeverity::Info));
    }
}
