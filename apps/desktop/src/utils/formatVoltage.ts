/**
 * Format a voltage value (stored in millivolts) for display.
 * Automatically selects the most readable unit (nV, µV, mV, V).
 *
 * @param mv   Value in millivolts. Null/undefined/NaN → fallback string.
 * @param opts.digits    Decimal places (default 4).
 * @param opts.fallback  String to show when value is invalid (default "N/A").
 * @param opts.unit      Force a specific unit instead of auto-selecting.
 */
export function formatVoltage(
  mv: number | null | undefined,
  opts: {
    digits?: number;
    fallback?: string;
    unit?: "nV" | "µV" | "mV" | "V" | "auto";
  } = {}
): string {
  const { digits = 4, fallback = "N/A", unit = "auto" } = opts;

  if (mv == null || !Number.isFinite(mv)) {
    return fallback;
  }

  const abs = Math.abs(mv);

  if (unit === "auto") {
    if (abs < 1e-6) {
      return `${(mv * 1e9).toFixed(digits)} nV`;
    }
    if (abs < 1e-3) {
      return `${(mv * 1e6).toFixed(digits)} µV`;
    }
    if (abs < 1) {
      return `${(mv * 1e3).toFixed(digits)} mV`;
    }
    return `${mv.toFixed(digits)} V`;
  }

  const value =
    unit === "nV"
      ? mv * 1e9
      : unit === "µV"
        ? mv * 1e6
        : unit === "mV"
          ? mv * 1e3
          : mv;

  return `${value.toFixed(digits)} ${unit}`;
}

/**
 * Pick the best display unit for a set of millivolt values.
 * All values in the set should be rendered with the same unit for consistency.
 */
export function pickVoltageUnit(values: (number | null | undefined)[]): "nV" | "µV" | "mV" | "V" {
  const valid = values.filter((v) => v != null && Number.isFinite(v)) as number[];
  if (valid.length === 0) return "µV";

  const maxAbs = Math.max(...valid.map((v) => Math.abs(v)));

  if (maxAbs < 1e-6) return "nV";
  if (maxAbs < 1e-3) return "µV";
  if (maxAbs < 1) return "mV";
  return "V";
}
