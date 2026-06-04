// M4.1 Recipe validation logic — pure TypeScript, no hardware access.
// Constants match tools/lab/recipe_two_device_run/src/safety.rs exactly.

import type { M34Recipe, RecipeValidationResult } from "../types/recipe";

const HARD_MAX_RF_POWER_DBM = -10;
const HARD_MAX_POINTS = 21;
const HARD_MAX_FRAMES_PER_STEP = 10;
const HARD_MAX_REPEAT_COUNT = 3;
const HARD_MAX_TOTAL_FRAMES = 630;
const HARD_MAX_FM_DEVIATION_HZ = 5_000_000;

const ALLOWED_LF_SHAPES = new Set([
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
]);

export function validateRecipe(text: string): RecipeValidationResult {
  // 1. JSON parse
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (e) {
    return {
      parseOk: false,
      parseError: e instanceof Error ? e.message : String(e),
      shapeOk: false,
      shapeErrors: [],
      valueOk: false,
      valueErrors: [],
      warnings: [],
      recipe: null,
    };
  }

  // 2. Shape validation
  const shapeErrors: string[] = [];
  const r = parsed as Record<string, unknown>;

  if (typeof r.kind !== "string") {
    shapeErrors.push("Missing or invalid 'kind' field");
    return {
      parseOk: true,
      shapeOk: false,
      shapeErrors,
      valueOk: false,
      valueErrors: [],
      warnings: [],
      recipe: null,
    };
  }

  // Handle system_scan_recipe as a recognized but not-yet-fully-supported kind
  if (r.kind === "system_scan_recipe") {
    const ssWarnings: string[] = [
      "system_scan_recipe recognized — full GUI preview not yet implemented",
    ];
    if (!isObject(r.devices)) shapeErrors.push("Missing 'devices' block");
    if (!Array.isArray(r.sweeps)) shapeErrors.push("Missing 'sweeps' array");
    if (!Array.isArray(r.sweep_order)) shapeErrors.push("Missing 'sweep_order' array");
    if (!isObject(r.acquisition_policy)) shapeErrors.push("Missing 'acquisition_policy' block");

    if (shapeErrors.length > 0) {
      return {
        parseOk: true,
        shapeOk: false,
        shapeErrors,
        valueOk: false,
        valueErrors: [],
        warnings: [],
        recipe: null,
        kind: r.kind,
      };
    }

    return {
      parseOk: true,
      shapeOk: true,
      shapeErrors: [],
      valueOk: true,
      valueErrors: [],
      warnings: ssWarnings,
      recipe: null,
      kind: r.kind,
    };
  }

  if (r.kind !== "two_device_odmr_like_sweep_recipe") {
    shapeErrors.push(
      `kind must be 'two_device_odmr_like_sweep_recipe' or 'system_scan_recipe', got '${r.kind}'`
    );
  }

  if (!isObject(r.rf)) shapeErrors.push("Missing 'rf' block");
  if (!isObject(r.modulation)) shapeErrors.push("Missing 'modulation' block");
  if (!isObject(r.acquisition))
    shapeErrors.push("Missing 'acquisition' block");
  if (!isObject(r.safety)) shapeErrors.push("Missing 'safety' block");
  if (!isObject(r.devices)) shapeErrors.push("Missing 'devices' block");

  if (shapeErrors.length > 0) {
    return {
      parseOk: true,
      shapeOk: false,
      shapeErrors,
      valueOk: false,
      valueErrors: [],
      warnings: [],
      recipe: null,
      kind: r.kind,
    };
  }

  const recipe = r as unknown as M34Recipe;

  // 3. Value validation
  const valueErrors: string[] = [];
  const warnings: string[] = [];

  // RF range
  if (recipe.rf.start_hz <= 0 || recipe.rf.stop_hz <= 0) {
    valueErrors.push("rf start_hz and stop_hz must be positive");
  }
  if (recipe.rf.start_hz > recipe.rf.stop_hz) {
    valueErrors.push("rf start_hz must be <= stop_hz");
  }

  // Points
  if (recipe.rf.points < 2) {
    valueErrors.push("rf points must be >= 2");
  }
  if (recipe.rf.points > HARD_MAX_POINTS) {
    valueErrors.push(
      `rf points (${recipe.rf.points}) exceeds hard max (${HARD_MAX_POINTS})`
    );
  }

  // Power
  if (recipe.rf.max_power_dbm > HARD_MAX_RF_POWER_DBM) {
    valueErrors.push(
      `max_rf_power_dbm (${recipe.rf.max_power_dbm}) must be <= ${HARD_MAX_RF_POWER_DBM} dBm`
    );
  }
  if (recipe.rf.power_dbm > recipe.rf.max_power_dbm) {
    valueErrors.push(
      `rf_power_dbm (${recipe.rf.power_dbm}) must be <= max_power_dbm (${recipe.rf.max_power_dbm})`
    );
  }

  // FM
  if (recipe.modulation.max_fm_deviation_hz > HARD_MAX_FM_DEVIATION_HZ) {
    valueErrors.push(
      `max_fm_deviation_hz (${recipe.modulation.max_fm_deviation_hz}) must be <= ${HARD_MAX_FM_DEVIATION_HZ} Hz`
    );
  }
  if (
    recipe.modulation.fm_deviation_hz > recipe.modulation.max_fm_deviation_hz
  ) {
    valueErrors.push(
      `fm_deviation_hz (${recipe.modulation.fm_deviation_hz}) must be <= max_fm_deviation_hz (${recipe.modulation.max_fm_deviation_hz})`
    );
  }

  // Acquisition
  if (recipe.acquisition.frames_per_step > HARD_MAX_FRAMES_PER_STEP) {
    valueErrors.push(
      `frames_per_step (${recipe.acquisition.frames_per_step}) exceeds max (${HARD_MAX_FRAMES_PER_STEP})`
    );
  }
  const repeatCount = recipe.acquisition.repeat_count ?? 2;
  if (repeatCount > HARD_MAX_REPEAT_COUNT) {
    valueErrors.push(
      `repeat_count (${repeatCount}) exceeds max (${HARD_MAX_REPEAT_COUNT})`
    );
  }
  const totalFrames =
    recipe.rf.points *
    recipe.acquisition.frames_per_step *
    repeatCount;
  if (totalFrames > HARD_MAX_TOTAL_FRAMES) {
    valueErrors.push(
      `total frames (${totalFrames}) exceeds max (${HARD_MAX_TOTAL_FRAMES})`
    );
  }

  // Magnetic
  if (recipe.devices.magnetic.in_scope) {
    if (recipe.safety.no_magnetic !== false) {
      valueErrors.push(
        "magnetic.in_scope is true but no_magnetic safety flag is set"
      );
    }
  }

  // Policy flags
  if (recipe.safety.no_internal_sweep === false) {
    valueErrors.push("no_internal_sweep must be true (internal sweep forbidden)");
  }
  if (recipe.safety.no_csv === false) {
    valueErrors.push("no_csv must be true (CSV output forbidden)");
  }
  if (recipe.safety.no_gui === false) {
    warnings.push("no_gui is false (GUI output not recommended)");
  }

  // LF validation
  if (recipe.modulation.internal_lf) {
    const lf = recipe.modulation.internal_lf;
    if (lf.lf_output_enabled) {
      valueErrors.push("LF output must not be enabled (LFO ON is forbidden)");
    }
    const shape = lf.shape.trim();
    if (shape.includes(";")) {
      valueErrors.push(`LF shape '${shape}' contains semicolon`);
    } else if (!ALLOWED_LF_SHAPES.has(shape)) {
      valueErrors.push(
        `LF shape '${shape}' is not valid. Allowed: ${Array.from(ALLOWED_LF_SHAPES).join(", ")}`
      );
    }
  }

  return {
    parseOk: true,
    shapeOk: true,
    shapeErrors: [],
    valueOk: valueErrors.length === 0,
    valueErrors,
    warnings,
    recipe,
    kind: recipe.kind,
  };
}

export function generateFrequencyGrid(recipe: M34Recipe): number[] {
  const n = recipe.rf.points;
  if (n === 1) return [recipe.rf.start_hz];
  const step = (recipe.rf.stop_hz - recipe.rf.start_hz) / (n - 1);
  return Array.from({ length: n }, (_, i) => recipe.rf.start_hz + step * i);
}

export function computeTotalFrames(recipe: M34Recipe): number {
  const repeatCount = recipe.acquisition.repeat_count ?? 2;
  return recipe.rf.points * recipe.acquisition.frames_per_step * repeatCount;
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}
