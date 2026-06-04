// M4.1 Recipe preview computation — pure TypeScript, no hardware access.
// All formulas match tools/lab/recipe_two_device_run/src/dry_run.rs, safety.rs, command_plan.rs.

import type {
  M34Recipe,
  ResolvedPreview,
  DryRunPreview,
  SafetyPreview,
  SafetyFinding,
  CommandPlanPreview,
} from "../types/recipe";
import { generateFrequencyGrid, computeTotalFrames } from "./recipeValidation";

const HARD_MAX_RF_POWER_DBM = -10;
const HARD_MAX_POINTS = 21;
const HARD_MAX_FRAMES_PER_STEP = 10;
const HARD_MAX_REPEAT_COUNT = 3;
const HARD_MAX_TOTAL_FRAMES = 630;
const HARD_MAX_FM_DEVIATION_HZ = 5_000_000;

export function buildResolvedPreview(recipe: M34Recipe): ResolvedPreview {
  const frequencies = generateFrequencyGrid(recipe);
  const repeatCount = recipe.acquisition.repeat_count ?? 2;
  const stepCount = frequencies.length * repeatCount;
  const totalFrames = computeTotalFrames(recipe);
  const delayMs = recipe.acquisition.inter_frame_delay_ms ?? 20;
  const estimatedDurationS =
    totalFrames * (0.8 + delayMs / 1000) +
    frequencies.length * repeatCount * 0.2;

  return {
    step_count: stepCount,
    frequencies,
    total_frames: totalFrames,
    estimated_duration_s: estimatedDurationS,
    device_list: ["smb100a", "oe1022d"],
    physical_response_required: recipe.safety.physical_response_required ?? false,
  };
}

export function buildDryRunPreview(recipe: M34Recipe): DryRunPreview {
  const resolved = buildResolvedPreview(recipe);
  const repeatCount = recipe.acquisition.repeat_count ?? 2;
  const rfPoints = recipe.rf.points;
  const framesPerStep = recipe.acquisition.frames_per_step;

  // Per-step: 5 SMB commands + frames_per_step OE + 1 SMB shutdown
  const smbPerStep = 6; // OUTP OFF, FREQ, FREQ?, OUTP ON, OUTP?, OUTP OFF
  const totalSmbSet =
    smbPerStep * rfPoints * repeatCount +
    5 + // config: POW, POW:ALC, FM:SOUR, FM:DEV, FM:STAT, MOD:STAT
    3; // shutdown: OUTP OFF, MOD:STAT OFF, FM:STAT OFF
  const totalSmbQuery =
    10 + // preflight
    rfPoints * repeatCount + // FREQ? + OUTP? per step
    3; // shutdown SYST:ERR?
  const totalOe = rfPoints * repeatCount * framesPerStep + 1; // +1 for *IDN?

  return {
    step_count: resolved.step_count,
    total_frames: resolved.total_frames,
    repeat_count: repeatCount,
    rf_points: rfPoints,
    estimated_duration_s: resolved.estimated_duration_s,
    required_devices: ["smb100a", "oe1022d"],
    smb_set_count: totalSmbSet,
    smb_query_count: totalSmbQuery,
    oe_frame_count: totalOe,
  };
}

export function buildSafetyPreview(recipe: M34Recipe): SafetyPreview {
  const findings: SafetyFinding[] = [];

  // 1. Operator approval
  if (recipe.safety.require_operator_approval !== false) {
    findings.push({
      check: "operator_approval",
      severity: "info",
      passed: true,
      detail: "Operator approval is required (GUI cannot approve)",
    });
  }

  // 2. Power limit
  const powerOk =
    recipe.rf.power_dbm <= recipe.rf.max_power_dbm &&
    recipe.rf.max_power_dbm <= HARD_MAX_RF_POWER_DBM;
  findings.push({
    check: "rf_power_limit",
    severity: powerOk ? "info" : "error",
    passed: powerOk,
    detail: `rf_power=${recipe.rf.power_dbm} dBm, max=${recipe.rf.max_power_dbm} dBm, hard_max=${HARD_MAX_RF_POWER_DBM} dBm`,
  });

  // 3. Frequency range
  const freqOk =
    recipe.rf.start_hz > 0 &&
    recipe.rf.stop_hz > 0 &&
    recipe.rf.start_hz <= recipe.rf.stop_hz;
  findings.push({
    check: "rf_frequency_range",
    severity: freqOk ? "info" : "error",
    passed: freqOk,
    detail: `rf range ${recipe.rf.start_hz.toFixed(0)} - ${recipe.rf.stop_hz.toFixed(0)} Hz`,
  });

  // 4. Points limit
  const pointsOk = recipe.rf.points <= HARD_MAX_POINTS;
  findings.push({
    check: "rf_points_limit",
    severity: pointsOk ? "info" : "error",
    passed: pointsOk,
    detail: `rf_points=${recipe.rf.points}, max=${HARD_MAX_POINTS}`,
  });

  // 5. Frames per step
  const fpsOk = recipe.acquisition.frames_per_step <= HARD_MAX_FRAMES_PER_STEP;
  findings.push({
    check: "frames_per_step_limit",
    severity: fpsOk ? "info" : "error",
    passed: fpsOk,
    detail: `frames_per_step=${recipe.acquisition.frames_per_step}, max=${HARD_MAX_FRAMES_PER_STEP}`,
  });

  // 6. Repeat count
  const repeatCount = recipe.acquisition.repeat_count ?? 2;
  const repeatOk = repeatCount <= HARD_MAX_REPEAT_COUNT;
  findings.push({
    check: "repeat_count_limit",
    severity: repeatOk ? "info" : "error",
    passed: repeatOk,
    detail: `repeat_count=${repeatCount}, max=${HARD_MAX_REPEAT_COUNT}`,
  });

  // 7. Total frames
  const totalFrames = computeTotalFrames(recipe);
  const totalOk = totalFrames <= HARD_MAX_TOTAL_FRAMES;
  findings.push({
    check: "total_frames_limit",
    severity: totalOk ? "info" : "error",
    passed: totalOk,
    detail: `total_frames=${totalFrames}, max=${HARD_MAX_TOTAL_FRAMES}`,
  });

  // 8. FM deviation
  const fmOk =
    recipe.modulation.fm_deviation_hz <= recipe.modulation.max_fm_deviation_hz &&
    recipe.modulation.max_fm_deviation_hz <= HARD_MAX_FM_DEVIATION_HZ;
  findings.push({
    check: "fm_deviation_limit",
    severity: fmOk ? "info" : "error",
    passed: fmOk,
    detail: `fm_deviation=${recipe.modulation.fm_deviation_hz} Hz, max=${recipe.modulation.max_fm_deviation_hz} Hz, hard_max=${HARD_MAX_FM_DEVIATION_HZ} Hz`,
  });

  // 9. No internal sweep
  if (recipe.safety.no_internal_sweep !== false) {
    findings.push({
      check: "no_internal_sweep",
      severity: "info",
      passed: true,
      detail: "Internal sweep mode is prohibited; software-stepped only",
    });
  }

  // 10. No magnetic
  if (recipe.safety.no_magnetic !== false) {
    const magOk = !recipe.devices.magnetic.in_scope;
    findings.push({
      check: "no_magnetic",
      severity: magOk ? "info" : "error",
      passed: magOk,
      detail: `magnetic.in_scope=${recipe.devices.magnetic.in_scope}`,
    });
  }

  // 11. No CSV
  if (recipe.safety.no_csv !== false) {
    findings.push({
      check: "no_csv",
      severity: "info",
      passed: true,
      detail: "CSV output is prohibited",
    });
  }

  // 12. LF validation
  if (recipe.modulation.internal_lf) {
    const lf = recipe.modulation.internal_lf;
    if (lf.lf_output_enabled) {
      findings.push({
        check: "lf_output_disabled",
        severity: "error",
        passed: false,
        detail: "LF output (LFO ON) is forbidden; use internal modulation only",
      });
    } else {
      findings.push({
        check: "lf_output_disabled",
        severity: "info",
        passed: true,
        detail: "LF generator configured in internal mode only",
      });
    }
  }

  // 13. Frequencies finite
  const freqs = generateFrequencyGrid(recipe);
  const allFinite = freqs.every((f) => Number.isFinite(f));
  findings.push({
    check: "frequencies_finite",
    severity: allFinite ? "info" : "error",
    passed: allFinite,
    detail: `${freqs.length} frequencies, all finite=${allFinite}`,
  });

  const errors = findings.filter((f) => f.severity === "error" && !f.passed).length;
  const warnings = findings.filter(
    (f) => f.severity === "warning" && !f.passed
  ).length;
  const passedCount = findings.filter((f) => f.passed).length;

  let decision: "allow" | "reject" | "allow_with_warnings";
  if (errors > 0) {
    decision = "reject";
  } else if (warnings > 0) {
    decision = "allow_with_warnings";
  } else {
    decision = "allow";
  }

  return {
    decision,
    findings,
    total_checks: findings.length,
    passed_count: passedCount,
    warnings_count: warnings,
    errors_count: errors,
    operator_approval_required:
      recipe.safety.require_operator_approval !== false,
  };
}

export function buildCommandPlanPreview(recipe: M34Recipe): CommandPlanPreview {
  const repeatCount = recipe.acquisition.repeat_count ?? 2;
  const rfPoints = recipe.rf.points;
  const framesPerStep = recipe.acquisition.frames_per_step;
  const lfEnabled = recipe.modulation.internal_lf?.enabled ?? false;

  // Preflight queries
  let seq = 0;
  const preflightCount = 10;
  seq += preflightCount;

  // Config commands
  const configCount = 5 + (lfEnabled ? 3 : 0);
  seq += configCount;

  // Per-step commands
  const perStepSmb = 6; // OUTP OFF, FREQ, FREQ?, OUTP ON, OUTP?, OUTP OFF
  const perStepOe = framesPerStep; // RALL? frames
  const totalStepCommands =
    rfPoints * repeatCount * (perStepSmb + perStepOe);
  seq += totalStepCommands;

  // Shutdown
  const shutdownCount = 3; // OUTP OFF, MOD:STAT OFF, FM:STAT OFF
  seq += shutdownCount;

  // Final SYST:ERR? checks
  const finalErrChecks = 3;
  seq += finalErrChecks;

  // OE identity
  const oeIdentity = 1;
  seq += oeIdentity;

  const total = seq;
  const smbSetCount =
    configCount +
    rfPoints * repeatCount * perStepSmb +
    shutdownCount;
  const smbQueryCount = preflightCount + rfPoints * repeatCount * 2 + finalErrChecks; // 2 queries per step: FREQ? + OUTP?
  const oeCount = rfPoints * repeatCount * framesPerStep + oeIdentity;

  return {
    total_commands: total,
    smb_set_count: smbSetCount,
    smb_query_count: smbQueryCount,
    oe_count: oeCount,
    shutdown_count: shutdownCount,
    safety_relevant_count:
      rfPoints * repeatCount * 3 + // OUTP OFF, OUTP ON, OUTP OFF per step
      shutdownCount + // OUTP OFF, MOD:STAT OFF, FM:STAT OFF
      finalErrChecks + // SYST:ERR?
      2, // OUTP? + MOD:STAT? in preflight
    forbidden_count: 0,
    internal_sweep_used: false,
    magnetic_commands: 0,
  };
}
