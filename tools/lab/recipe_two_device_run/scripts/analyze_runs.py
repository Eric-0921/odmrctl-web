#!/usr/bin/env python3
"""Generate minimal M3.6 ODMR-like analysis artifacts from completed runs.

This script is analysis-only. It reads existing M3.5 run directories and never
opens hardware transports, parses rawbin payloads, or writes realtime CSV.
"""

import argparse
import hashlib
import json
import statistics
import sys
import tempfile
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path


SCHEMA_VERSION = "0.2.0"
KIND = "m3_6_minimal_odmr_like_analysis"
MV_PER_V = 1000.0


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def read_json(path: Path, default=None):
    if not path.exists():
        return default
    with path.open() as f:
        return json.load(f)


def read_jsonl(path: Path) -> list[dict]:
    rows = []
    if not path.exists():
        return rows
    with path.open() as f:
        for lineno, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError as exc:
                raise ValueError(f"{path}:{lineno}: invalid JSONL: {exc}") from exc
    return rows


def write_json(path: Path, obj: dict):
    path.write_text(json.dumps(obj, indent=2, sort_keys=True) + "\n")


def write_jsonl(path: Path, rows: list[dict]):
    with path.open("w") as f:
        for row in rows:
            f.write(json.dumps(row, sort_keys=True) + "\n")


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def mean(values: list[float]) -> float | None:
    return statistics.mean(values) if values else None


def stdev(values: list[float]) -> float:
    return statistics.stdev(values) if len(values) > 1 else 0.0


def clean_display_text(value: str | None) -> str | None:
    if value is None:
        return None
    cleaned = "".join(ch for ch in value if ch == "\t" or ch == "\n" or ord(ch) >= 32)
    return cleaned.rstrip()


def load_run(run_dir: Path) -> dict:
    run_dir = run_dir.resolve()
    missing = []

    run_result_path = run_dir / "summary" / "run_result.json"
    rf_summary_path = run_dir / "rf" / "rf_step_summary.jsonl"
    audit_path = run_dir / "audit_report.json"
    comparison_path = run_dir / "command_plan" / "command_audit_comparison.json"
    magnetic_path = run_dir / "metadata" / "magnetic_not_in_scope.json"
    oe_identity_path = run_dir / "metadata" / "oe1022d_identity.json"

    for path in [run_result_path, rf_summary_path, audit_path, comparison_path, magnetic_path]:
        if not path.exists():
            missing.append(str(path.relative_to(run_dir)))

    run_result = read_json(run_result_path, {}) or {}
    audit_report = read_json(audit_path, {}) or {}
    command_comparison = read_json(comparison_path, {}) or {}
    magnetic_note = read_json(magnetic_path, {}) or {}
    oe_identity = read_json(oe_identity_path, {}) or {}
    rf_rows = read_jsonl(rf_summary_path)

    run_id = run_result.get("run_id") or run_dir.name
    csv_files = [str(p.relative_to(run_dir)) for p in run_dir.rglob("*.csv")]
    magnetic_command_count = count_magnetic_commands(run_dir / "command_audit.jsonl")

    return {
        "dir": str(run_dir),
        "run_id": run_id,
        "missing_artifacts": missing,
        "run_result": run_result,
        "audit_report": audit_report,
        "command_comparison": command_comparison,
        "magnetic_note": magnetic_note,
        "oe1022d_raw_idn": oe_identity.get("idn"),
        "oe1022d_display_idn": clean_display_text(oe_identity.get("idn")),
        "rf_rows": rf_rows,
        "csv_files": csv_files,
        "magnetic_command_count": magnetic_command_count,
    }


def count_magnetic_commands(audit_jsonl: Path) -> int:
    count = 0
    for entry in read_jsonl(audit_jsonl):
        device_id = str(entry.get("device_id", "")).lower()
        command = str(entry.get("command", "")).lower()
        if "mag" in device_id or "maynuo" in device_id or "mag" in command:
            count += 1
    return count


def point_flags(run: dict, row: dict) -> list[str]:
    flags = []
    if run["missing_artifacts"]:
        flags.append("missing_artifact")
    if not run["run_result"].get("passed", False):
        flags.append("failed_run")
    if row.get("frames_parse_failed", 0) > 0:
        flags.append("parse_failures")
    if not row.get("step_passed", False):
        flags.append("failed_step")
    return flags


def build_spectrum_points(runs: list[dict]) -> list[dict]:
    points = []
    for run in runs:
        for row in run["rf_rows"]:
            b_x = row.get("b_x_mean")
            b_y = row.get("b_y_mean")
            b_x_std = row.get("b_x_std")
            b_y_std = row.get("b_y_std")
            points.append({
                "run_id": run["run_id"],
                "step_id": row.get("step_id"),
                "repeat_index": row.get("repeat_index"),
                "frequency_hz": row.get("frequency_hz"),
                "frequency_verified_hz": row.get("frequency_verified_hz"),
                "b_x_mean_v": b_x,
                "b_y_mean_v": b_y,
                "b_x_mean_mv": b_x * MV_PER_V if b_x is not None else None,
                "b_y_mean_mv": b_y * MV_PER_V if b_y is not None else None,
                "b_x_std_v": b_x_std,
                "b_y_std_v": b_y_std,
                "b_x_std_mv": b_x_std * MV_PER_V if b_x_std is not None else None,
                "b_y_std_mv": b_y_std * MV_PER_V if b_y_std is not None else None,
                "frames_used": row.get("frames_parsed", row.get("frames_captured", 0)),
                "frames_parse_failed": row.get("frames_parse_failed", 0),
                "step_passed": row.get("step_passed", False),
                "quality_flags": point_flags(run, row),
            })
    return points


def aggregate_by_frequency(points: list[dict]) -> dict:
    buckets = defaultdict(list)
    for point in points:
        frequency = point.get("frequency_hz")
        if frequency is not None:
            buckets[frequency].append(point)

    frequencies = []
    for frequency in sorted(buckets):
        rows = buckets[frequency]
        b_x = [r["b_x_mean_v"] for r in rows if r.get("b_x_mean_v") is not None]
        b_y = [r["b_y_mean_v"] for r in rows if r.get("b_y_mean_v") is not None]
        total_frames = sum(r.get("frames_used", 0) for r in rows)
        parse_failed = sum(r.get("frames_parse_failed", 0) for r in rows)
        run_ids = sorted({r["run_id"] for r in rows})
        frequencies.append({
            "frequency_hz": frequency,
            "contributing_run_ids": run_ids,
            "point_count": len(rows),
            "total_frames_used": total_frames,
            "frames_parse_failed": parse_failed,
            "b_x_mean_v": mean(b_x),
            "b_x_std_v": stdev(b_x),
            "b_x_min_v": min(b_x) if b_x else None,
            "b_x_max_v": max(b_x) if b_x else None,
            "b_x_mean_mv": mean(b_x) * MV_PER_V if b_x else None,
            "b_x_std_mv": stdev(b_x) * MV_PER_V if b_x else 0.0,
            "b_x_min_mv": min(b_x) * MV_PER_V if b_x else None,
            "b_x_max_mv": max(b_x) * MV_PER_V if b_x else None,
            "b_y_mean_v": mean(b_y),
            "b_y_std_v": stdev(b_y),
            "b_y_min_v": min(b_y) if b_y else None,
            "b_y_max_v": max(b_y) if b_y else None,
            "b_y_mean_mv": mean(b_y) * MV_PER_V if b_y else None,
            "b_y_std_mv": stdev(b_y) * MV_PER_V if b_y else 0.0,
            "b_y_min_mv": min(b_y) * MV_PER_V if b_y else None,
            "b_y_max_mv": max(b_y) * MV_PER_V if b_y else None,
        })

    return {
        "schema_version": SCHEMA_VERSION,
        "kind": "m3_6_run_overlay_summary",
        "generated_at": utc_now(),
        "frequency_count": len(frequencies),
        "frequencies": frequencies,
    }


def frequency_grid_mismatch(runs: list[dict]) -> bool:
    grids = []
    for run in runs:
        grid = sorted({row.get("frequency_hz") for row in run["rf_rows"] if row.get("frequency_hz") is not None})
        grids.append(grid)
    return any(grid != grids[0] for grid in grids[1:]) if grids else False


def build_quality_flags(runs: list[dict], points: list[dict]) -> dict:
    missing = {run["run_id"]: run["missing_artifacts"] for run in runs if run["missing_artifacts"]}
    failed_runs = [run["run_id"] for run in runs if not run["run_result"].get("passed", False)]
    parse_failures = sum(p.get("frames_parse_failed", 0) for p in points)
    audit_mismatch = [
        run["run_id"]
        for run in runs
        if not run["command_comparison"].get("passed", False)
        or run["command_comparison"].get("missing_expected_commands")
        or run["command_comparison"].get("unexpected_actual_commands")
        or run["command_comparison"].get("forbidden_actual_commands")
    ]
    unsafe_final = [
        run["run_id"]
        for run in runs
        if not (
            run["run_result"].get("final_rf_off", False)
            and run["run_result"].get("final_mod_off", False)
            and run["run_result"].get("final_fm_off", False)
            and run["run_result"].get("final_syst_err_clean", False)
        )
    ]
    csv_present = {run["run_id"]: run["csv_files"] for run in runs if run["csv_files"]}
    magnetic_present = {
        run["run_id"]: run["magnetic_command_count"]
        for run in runs
        if run["magnetic_command_count"] > 0
    }

    flags = {
        "schema_version": SCHEMA_VERSION,
        "kind": "m3_6_quality_flags",
        "generated_at": utc_now(),
        "missing_artifact": bool(missing),
        "missing_artifact_details": missing,
        "failed_run": bool(failed_runs),
        "failed_run_ids": failed_runs,
        "parse_failures": parse_failures > 0,
        "parse_failure_count": parse_failures,
        "audit_mismatch": bool(audit_mismatch),
        "audit_mismatch_run_ids": audit_mismatch,
        "unsafe_final_state": bool(unsafe_final),
        "unsafe_final_state_run_ids": unsafe_final,
        "csv_present": bool(csv_present),
        "csv_present_details": csv_present,
        "magnetic_command_present": bool(magnetic_present),
        "magnetic_command_details": magnetic_present,
        "frequency_grid_mismatch": frequency_grid_mismatch(runs),
        "empty_signal_series": not points,
    }
    flags["passed"] = not any(
        flags[key]
        for key in [
            "missing_artifact",
            "failed_run",
            "parse_failures",
            "audit_mismatch",
            "unsafe_final_state",
            "csv_present",
            "magnetic_command_present",
            "frequency_grid_mismatch",
            "empty_signal_series",
        ]
    )
    return flags


def contrast_from_overlay(overlay: dict, channel: str) -> float | None:
    key = f"{channel}_mean_v"
    values = [row[key] for row in overlay["frequencies"] if row.get(key) is not None]
    if not values:
        return None
    return max(values) - min(values)


def build_summary(runs: list[dict], points: list[dict], overlay: dict, flags: dict) -> dict:
    source_run_ids = [run["run_id"] for run in runs]
    frames_used = sum(point.get("frames_used", 0) for point in points)
    parse_failed = sum(point.get("frames_parse_failed", 0) for point in points)
    parse_failure_rate = parse_failed / frames_used if frames_used else 0.0
    b_x_contrast = contrast_from_overlay(overlay, "b_x")
    b_y_contrast = contrast_from_overlay(overlay, "b_y")
    all_safe = all(
        run["run_result"].get("final_rf_off", False)
        and run["run_result"].get("final_mod_off", False)
        and run["run_result"].get("final_fm_off", False)
        and run["run_result"].get("final_syst_err_clean", False)
        for run in runs
    )

    return {
        "schema_version": SCHEMA_VERSION,
        "kind": KIND,
        "generated_at": utc_now(),
        "source_run_ids": source_run_ids,
        "point_count": len(points),
        "frequency_count": overlay["frequency_count"],
        "frames_used": frames_used,
        "frames_parse_failed": parse_failed,
        "all_runs_passed": all(run["run_result"].get("passed", False) for run in runs),
        "all_safe_states_confirmed": all_safe,
        "no_csv": not flags["csv_present"],
        "no_magnetic": not flags["magnetic_command_present"],
        "parse_failure_rate": parse_failure_rate,
        "contrast_estimate_b_x_v": b_x_contrast,
        "contrast_estimate_b_x_mv": b_x_contrast * MV_PER_V if b_x_contrast is not None else None,
        "contrast_estimate_b_y_v": b_y_contrast,
        "contrast_estimate_b_y_mv": b_y_contrast * MV_PER_V if b_y_contrast is not None else None,
        "physical_odmr_response_required": False,
        "odmr_dip_detected": False,
        "quality_flags_passed": flags["passed"],
        "oe1022d_display_idn_by_run": {
            run["run_id"]: run["oe1022d_display_idn"] for run in runs if run["oe1022d_display_idn"]
        },
    }


def write_summary_markdown(path: Path, summary: dict, overlay: dict, flags: dict):
    def fmt_optional(value, suffix=""):
        if value is None:
            return "N/A"
        return f"{value:.12g}{suffix}"

    lines = [
        "# M3.6 Minimal ODMR-Like Analysis Summary",
        "",
        f"Generated: {summary['generated_at']}",
        "",
        "## Source Runs",
        "",
    ]
    for run_id in summary["source_run_ids"]:
        lines.append(f"- `{run_id}`")
    lines.extend([
        "",
        "## Result",
        "",
        f"- Point count: {summary['point_count']}",
        f"- Frequency count: {summary['frequency_count']}",
        f"- Frames used: {summary['frames_used']}",
        f"- Parse failures: {summary['frames_parse_failed']}",
        f"- Parse failure rate: {summary['parse_failure_rate']:.6f}",
        f"- B-X contrast estimate: {fmt_optional(summary['contrast_estimate_b_x_v'], ' V')} ({fmt_optional(summary['contrast_estimate_b_x_mv'], ' mV')})",
        f"- B-Y contrast estimate: {fmt_optional(summary['contrast_estimate_b_y_v'], ' V')} ({fmt_optional(summary['contrast_estimate_b_y_mv'], ' mV')})",
        f"- ODMR dip detected: {summary['odmr_dip_detected']}",
        "",
        "## Boundary Checks",
        "",
        f"- All runs passed: {summary['all_runs_passed']}",
        f"- All final safe states confirmed: {summary['all_safe_states_confirmed']}",
        f"- No CSV: {summary['no_csv']}",
        f"- No magnetic: {summary['no_magnetic']}",
        f"- Quality flags passed: {summary['quality_flags_passed']}",
        "",
        "## Frequency Overlay",
        "",
        "| Frequency Hz | Points | Frames | B-X mean mV | B-X std mV | B-Y mean mV | B-Y std mV |",
        "|--------------|--------|--------|-------------|------------|-------------|------------|",
    ])
    for row in overlay["frequencies"]:
        lines.append(
            f"| {row['frequency_hz']:.0f} | {row['point_count']} | {row['total_frames_used']} | "
            f"{row['b_x_mean_mv']:.9f} | {row['b_x_std_mv']:.9f} | "
            f"{row['b_y_mean_mv']:.9f} | {row['b_y_std_mv']:.9f} |"
        )
    lines.extend([
        "",
        "## Quality Flags",
        "",
        "```json",
        json.dumps(flags, indent=2, sort_keys=True),
        "```",
        "",
    ])
    path.write_text("\n".join(lines))


def build_export_manifest(out_dir: Path, artifact_paths: list[Path], source_run_ids: list[str]) -> dict:
    files = []
    for path in sorted(artifact_paths):
        files.append({
            "relative_path": str(path.relative_to(out_dir)),
            "size_bytes": path.stat().st_size,
            "sha256": sha256_file(path),
        })
    return {
        "schema_version": SCHEMA_VERSION,
        "kind": "m3_6_export_manifest",
        "generated_at": utc_now(),
        "source_run_ids": source_run_ids,
        "files": files,
    }


def analyze(run_dirs: list[Path], out_dir: Path) -> dict:
    runs = [load_run(path) for path in run_dirs]
    points = build_spectrum_points(runs)
    overlay = aggregate_by_frequency(points)
    flags = build_quality_flags(runs, points)
    summary = build_summary(runs, points, overlay, flags)

    analysis_dir = out_dir / "analysis"
    analysis_dir.mkdir(parents=True, exist_ok=True)

    spectrum_path = analysis_dir / "spectrum_points.jsonl"
    overlay_path = analysis_dir / "run_overlay_summary.json"
    summary_path = analysis_dir / "odmr_like_analysis_summary.json"
    flags_path = analysis_dir / "quality_flags.json"
    markdown_path = analysis_dir / "odmr_like_analysis_summary.md"
    manifest_path = analysis_dir / "export_manifest.json"

    write_jsonl(spectrum_path, points)
    write_json(overlay_path, overlay)
    write_json(flags_path, flags)
    write_json(summary_path, summary)
    write_summary_markdown(markdown_path, summary, overlay, flags)

    manifest = build_export_manifest(
        out_dir,
        [spectrum_path, overlay_path, summary_path, flags_path, markdown_path],
        summary["source_run_ids"],
    )
    write_json(manifest_path, manifest)

    return {
        "runs": runs,
        "points": points,
        "overlay": overlay,
        "flags": flags,
        "summary": summary,
        "manifest": manifest,
        "out_dir": str(out_dir),
    }


def write_lab_report(path: Path, result: dict):
    summary = result["summary"]
    flags = result["flags"]
    manifest = result["manifest"]
    lines = [
        "# M3.6 Minimal ODMR-Like Analysis Artifact",
        "",
        f"Date: {summary['generated_at'][:10]}",
        "",
        "## Summary",
        "",
        "M3.6 generated a minimal ODMR-like analysis artifact from the three stable M3.5 real run directories. The task is analysis-only: it reads run artifacts, aggregates RF-step statistics, and writes machine-readable outputs for later read-only GUI work. It does not connect hardware, parse rawbin payloads, write realtime CSV, or touch magnetic control.",
        "",
        "## Source Runs",
        "",
    ]
    for run_id in summary["source_run_ids"]:
        lines.append(f"- `{run_id}`")
    lines.extend([
        "",
        "## Outputs",
        "",
    ])
    for file_info in manifest["files"]:
        lines.append(
            f"- `{file_info['relative_path']}` — {file_info['size_bytes']} bytes, sha256 `{file_info['sha256']}`"
        )
    lines.extend([
        "",
        "## Acceptance Metrics",
        "",
        f"- Spectrum points: {summary['point_count']} (expected 66)",
        f"- Frequency count: {summary['frequency_count']} (expected 11)",
        f"- Frames used: {summary['frames_used']} (expected 330)",
        f"- Parse failures: {summary['frames_parse_failed']}",
        f"- Parse failure rate: {summary['parse_failure_rate']:.6f}",
        f"- B-X contrast estimate: {summary['contrast_estimate_b_x_v']:.12g} V / {summary['contrast_estimate_b_x_mv']:.12g} mV",
        f"- B-Y contrast estimate: {summary['contrast_estimate_b_y_v']:.12g} V / {summary['contrast_estimate_b_y_mv']:.12g} mV",
        f"- ODMR dip detected: {summary['odmr_dip_detected']} (M3.6 does not infer resonance)",
        "",
        "## Boundary Checks",
        "",
        f"- All runs passed: {summary['all_runs_passed']}",
        f"- All safe states confirmed: {summary['all_safe_states_confirmed']}",
        f"- No CSV: {summary['no_csv']}",
        f"- No magnetic: {summary['no_magnetic']}",
        f"- Quality flags passed: {summary['quality_flags_passed']}",
        "",
        "## Quality Flags",
        "",
        "```json",
        json.dumps(flags, indent=2, sort_keys=True),
        "```",
        "",
        "## Notes",
        "",
        "- Rawbin remains provenance input only; M3.6 uses `rf/rf_step_summary.jsonl` and summary/audit artifacts.",
        "- OE1022D raw IDN metadata is preserved in run artifacts; display text in analysis output trims trailing control/null characters only.",
        "- Physical ODMR response is not required and no dip detector is defined in M3.6.",
        "",
    ])
    path.write_text("\n".join(lines))


def make_synthetic_run(root: Path, run_id: str, offset: float = 0.0, bad_grid: bool = False):
    run = root / run_id
    (run / "summary").mkdir(parents=True)
    (run / "rf").mkdir()
    (run / "command_plan").mkdir()
    (run / "metadata").mkdir()
    write_json(run / "summary" / "run_result.json", {
        "run_id": run_id,
        "passed": True,
        "frames_parse_failed": 0,
        "final_rf_off": True,
        "final_mod_off": True,
        "final_fm_off": True,
        "final_syst_err_clean": True,
    })
    write_json(run / "audit_report.json", {"forbidden_commands_sent": 0})
    write_json(run / "command_plan" / "command_audit_comparison.json", {
        "passed": True,
        "missing_expected_commands": [],
        "unexpected_actual_commands": [],
        "forbidden_actual_commands": [],
    })
    write_json(run / "metadata" / "magnetic_not_in_scope.json", {"kind": "magnetic_not_in_scope"})
    f2 = 2_879_000_000.0 if bad_grid else 2_878_800_000.0
    write_jsonl(run / "rf" / "rf_step_summary.jsonl", [
        {
            "step_id": "repeat_0_rf_step_000",
            "repeat_index": 0,
            "frequency_hz": 2_878_000_000.0,
            "frequency_verified_hz": 2_878_000_000.0,
            "b_x_mean": 0.001 + offset,
            "b_x_std": 0.0001,
            "b_y_mean": 0.002 + offset,
            "b_y_std": 0.0002,
            "frames_parsed": 5,
            "frames_parse_failed": 0,
            "step_passed": True,
        },
        {
            "step_id": "repeat_0_rf_step_001",
            "repeat_index": 0,
            "frequency_hz": f2,
            "frequency_verified_hz": f2,
            "b_x_mean": 0.003 + offset,
            "b_x_std": 0.0001,
            "b_y_mean": 0.004 + offset,
            "b_y_std": 0.0002,
            "frames_parsed": 5,
            "frames_parse_failed": 0,
            "step_passed": True,
        },
    ])
    return run


def self_test():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        run1 = make_synthetic_run(root, "run1")
        run2 = make_synthetic_run(root, "run2", offset=0.001)
        result = analyze([run1, run2], root / "out")
        assert len(result["points"]) == 4
        assert result["overlay"]["frequency_count"] == 2
        assert result["summary"]["frames_used"] == 20
        assert not result["flags"]["frequency_grid_mismatch"]
        assert result["summary"]["contrast_estimate_b_x_v"] is not None

        missing = root / "missing_case"
        missing.mkdir()
        missing_result = analyze([missing], root / "missing_out")
        assert missing_result["flags"]["missing_artifact"]
        assert missing_result["flags"]["empty_signal_series"]

        bad = make_synthetic_run(root, "bad_grid", bad_grid=True)
        mismatch = analyze([run1, bad], root / "mismatch_out")
        assert mismatch["flags"]["frequency_grid_mismatch"]


def main():
    parser = argparse.ArgumentParser(description="Generate M3.6 minimal ODMR-like analysis artifacts")
    parser.add_argument("run_dirs", nargs="*", help="Run directories to analyze")
    parser.add_argument("--out-dir", help="Output directory; artifacts are written under analysis/")
    parser.add_argument("--lab-report", help="Optional markdown report path under docs/lab-bringup/")
    parser.add_argument("--self-test", action="store_true", help="Run built-in focused tests")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        print("self-test passed")
        return

    if not args.run_dirs:
        parser.error("at least one run directory is required unless --self-test is used")

    if not args.out_dir:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        args.out_dir = f"m3_6_analysis_{timestamp}"

    result = analyze([Path(d) for d in args.run_dirs], Path(args.out_dir))
    print(f"Wrote analysis artifacts under {Path(args.out_dir) / 'analysis'}")

    if args.lab_report:
        write_lab_report(Path(args.lab_report), result)
        print(f"Wrote lab report {args.lab_report}")

    if not result["flags"]["passed"]:
        print("quality flags did not pass; inspect quality_flags.json", file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main()
