//! Safety checks for M3.4 recipe-shaped runs.

use crate::recipe::generate_frequencies;
use crate::types::*;

pub const HARD_MAX_RF_POWER_DBM: f64 = -10.0;
pub const HARD_MAX_POINTS: u64 = 21;
pub const HARD_MAX_FRAMES_PER_STEP: u64 = 10;
pub const HARD_MAX_REPEAT_COUNT: u64 = 3;
pub const HARD_MAX_TOTAL_FRAMES: u64 = 630;
pub const HARD_MAX_FM_DEVIATION_HZ: f64 = 5_000_000.0;

/// Run all safety checks on a recipe and return a safety report.
pub fn check_recipe_safety(
    recipe: &M3_4Recipe,
    resolved_recipe_id: &str,
    operator_approved: bool,
) -> M3_4SafetyReport {
    let mut findings: Vec<M3_4SafetyFinding> = Vec::new();

    // 1. Operator approval
    if recipe.safety.require_operator_approval && !operator_approved {
        findings.push(M3_4SafetyFinding {
            check: "operator_approval".into(),
            severity: "error".into(),
            passed: false,
            detail: "Operator approval is required but not provided".into(),
        });
    } else if recipe.safety.require_operator_approval {
        findings.push(M3_4SafetyFinding {
            check: "operator_approval".into(),
            severity: "info".into(),
            passed: true,
            detail: "Operator approval verified".into(),
        });
    }

    // 2. Power limit
    let power_ok = recipe.rf.power_dbm <= recipe.rf.max_power_dbm
        && recipe.rf.max_power_dbm <= HARD_MAX_RF_POWER_DBM;
    findings.push(M3_4SafetyFinding {
        check: "rf_power_limit".into(),
        severity: if power_ok { "info" } else { "error" }.into(),
        passed: power_ok,
        detail: format!(
            "rf_power={} dBm, max={} dBm, hard_max={} dBm",
            recipe.rf.power_dbm, recipe.rf.max_power_dbm, HARD_MAX_RF_POWER_DBM
        ),
    });

    // 3. Frequency range
    let freq_ok = recipe.rf.start_hz > 0.0
        && recipe.rf.stop_hz > 0.0
        && recipe.rf.start_hz <= recipe.rf.stop_hz;
    findings.push(M3_4SafetyFinding {
        check: "rf_frequency_range".into(),
        severity: if freq_ok { "info" } else { "error" }.into(),
        passed: freq_ok,
        detail: format!(
            "rf range {:.0} - {:.0} Hz",
            recipe.rf.start_hz, recipe.rf.stop_hz
        ),
    });

    // 4. Points limit
    let points_ok = recipe.rf.points <= HARD_MAX_POINTS;
    findings.push(M3_4SafetyFinding {
        check: "rf_points_limit".into(),
        severity: if points_ok { "info" } else { "error" }.into(),
        passed: points_ok,
        detail: format!("rf_points={}, max={}", recipe.rf.points, HARD_MAX_POINTS),
    });

    // 5. Frames per step
    let fps_ok = recipe.acquisition.frames_per_step <= HARD_MAX_FRAMES_PER_STEP;
    findings.push(M3_4SafetyFinding {
        check: "frames_per_step_limit".into(),
        severity: if fps_ok { "info" } else { "error" }.into(),
        passed: fps_ok,
        detail: format!(
            "frames_per_step={}, max={}",
            recipe.acquisition.frames_per_step, HARD_MAX_FRAMES_PER_STEP
        ),
    });

    // 6. Repeat count
    let repeat_ok = recipe.acquisition.repeat_count <= HARD_MAX_REPEAT_COUNT;
    findings.push(M3_4SafetyFinding {
        check: "repeat_count_limit".into(),
        severity: if repeat_ok { "info" } else { "error" }.into(),
        passed: repeat_ok,
        detail: format!(
            "repeat_count={}, max={}",
            recipe.acquisition.repeat_count, HARD_MAX_REPEAT_COUNT
        ),
    });

    // 7. Total frames
    let total_frames =
        recipe.rf.points * recipe.acquisition.frames_per_step * recipe.acquisition.repeat_count;
    let total_ok = total_frames <= HARD_MAX_TOTAL_FRAMES;
    findings.push(M3_4SafetyFinding {
        check: "total_frames_limit".into(),
        severity: if total_ok { "info" } else { "error" }.into(),
        passed: total_ok,
        detail: format!(
            "total_frames={} ({} points × {} fps × {} repeats), max={}",
            total_frames,
            recipe.rf.points,
            recipe.acquisition.frames_per_step,
            recipe.acquisition.repeat_count,
            HARD_MAX_TOTAL_FRAMES
        ),
    });

    // 8. FM deviation
    let fm_ok = recipe.modulation.fm_deviation_hz <= recipe.modulation.max_fm_deviation_hz
        && recipe.modulation.max_fm_deviation_hz <= HARD_MAX_FM_DEVIATION_HZ;
    findings.push(M3_4SafetyFinding {
        check: "fm_deviation_limit".into(),
        severity: if fm_ok { "info" } else { "error" }.into(),
        passed: fm_ok,
        detail: format!(
            "fm_deviation={} Hz, max={} Hz, hard_max={} Hz",
            recipe.modulation.fm_deviation_hz,
            recipe.modulation.max_fm_deviation_hz,
            HARD_MAX_FM_DEVIATION_HZ
        ),
    });

    // 9. No internal sweep
    if recipe.safety.no_internal_sweep {
        findings.push(M3_4SafetyFinding {
            check: "no_internal_sweep".into(),
            severity: "info".into(),
            passed: true,
            detail: "Internal sweep mode is prohibited; software-stepped only".into(),
        });
    }

    // 10. No magnetic
    if recipe.safety.no_magnetic {
        let mag_ok = !recipe.devices.magnetic.in_scope;
        findings.push(M3_4SafetyFinding {
            check: "no_magnetic".into(),
            severity: if mag_ok { "info" } else { "error" }.into(),
            passed: mag_ok,
            detail: format!("magnetic.in_scope={}", recipe.devices.magnetic.in_scope),
        });
    }

    // 11. No CSV
    if recipe.safety.no_csv {
        findings.push(M3_4SafetyFinding {
            check: "no_csv".into(),
            severity: "info".into(),
            passed: true,
            detail: "CSV output is prohibited".into(),
        });
    }

    // 12. LF validation
    if let Some(ref lf) = recipe.modulation.internal_lf {
        if lf.lf_output_enabled {
            findings.push(M3_4SafetyFinding {
                check: "lf_output_disabled".into(),
                severity: "error".into(),
                passed: false,
                detail: "LF output (LFO ON) is forbidden; use internal modulation only".into(),
            });
        } else {
            findings.push(M3_4SafetyFinding {
                check: "lf_output_disabled".into(),
                severity: "info".into(),
                passed: true,
                detail: "LF generator configured in internal mode only".into(),
            });
        }
    }

    // 13. Frequencies are finite
    let freqs = generate_frequencies(recipe);
    let all_finite = freqs.iter().all(|f| f.is_finite());
    findings.push(M3_4SafetyFinding {
        check: "frequencies_finite".into(),
        severity: if all_finite { "info" } else { "error" }.into(),
        passed: all_finite,
        detail: format!("{} frequencies, all finite={}", freqs.len(), all_finite),
    });

    let errors = findings
        .iter()
        .filter(|f| f.severity == "error" && !f.passed)
        .count() as u64;
    let warnings = findings
        .iter()
        .filter(|f| f.severity == "warning" && !f.passed)
        .count() as u64;
    let passed_count = findings.iter().filter(|f| f.passed).count() as u64;

    let decision = if errors > 0 {
        SafetyDecision::Reject
    } else if warnings > 0 {
        SafetyDecision::AllowWithWarnings
    } else {
        SafetyDecision::Allow
    };

    M3_4SafetyReport {
        schema_version: "0.2.0".into(),
        kind: "safety_report".into(),
        id: format!("safety_{}", resolved_recipe_id),
        resolved_recipe_id: resolved_recipe_id.to_string(),
        decision,
        summary: M3_4SafetySummary {
            total_checks: findings.len() as u64,
            passed: passed_count,
            warnings,
            errors,
        },
        findings,
    }
}
