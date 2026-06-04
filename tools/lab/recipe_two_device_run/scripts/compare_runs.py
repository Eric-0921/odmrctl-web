#!/usr/bin/env python3
"""Compare M3.5 repeated real recipe runs.

Usage:
    python3 compare_runs.py <run_dir_1> <run_dir_2> <run_dir_3> [--out-dir <dir>]

Produces:
    run_comparison.json  — machine-readable comparison
    run_comparison.md    — human-readable markdown table
"""

import argparse
import json
import os
import statistics
import sys
from datetime import datetime, timezone
from pathlib import Path


def load_run_data(run_dir: str) -> dict:
    """Load all relevant artifacts from a single run directory."""
    d = {"dir": run_dir, "run_id": "unknown"}

    # run_result.json
    rr_path = Path(run_dir) / "summary" / "run_result.json"
    if rr_path.exists():
        with open(rr_path) as f:
            rr = json.load(f)
        d["run_id"] = rr.get("run_id", "unknown")
        d["passed"] = rr.get("passed", False)
        d["steps_completed"] = rr.get("steps_completed", 0)
        d["total_steps"] = rr.get("total_steps", 0)
        d["frames_requested"] = rr.get("frames_requested", 0)
        d["frames_captured"] = rr.get("frames_captured", 0)
        d["frames_parsed"] = rr.get("frames_parsed", 0)
        d["frames_parse_failed"] = rr.get("frames_parse_failed", 0)
        d["parse_failure_rate"] = rr.get("parse_failure_rate", 0.0)
        d["command_audit_comparison_passed"] = rr.get("command_audit_comparison_passed", False)
        d["final_rf_off"] = rr.get("final_rf_off", False)
        d["final_mod_off"] = rr.get("final_mod_off", False)
        d["final_fm_off"] = rr.get("final_fm_off", False)
        d["final_syst_err_clean"] = rr.get("final_syst_err_clean", False)
        d["alignment_count"] = rr.get("alignment_count", 0)
        d["emergency_shutdown"] = rr.get("emergency_shutdown_triggered", False)
        d["notes"] = rr.get("notes", [])

    # run_stability_summary.json
    ss_path = Path(run_dir) / "summary" / "run_stability_summary.json"
    if ss_path.exists():
        with open(ss_path) as f:
            ss = json.load(f)
        d["steps_passed"] = ss.get("steps_passed", 0)
        d["no_forbidden"] = ss.get("no_forbidden_commands_sent", False)

    # audit_report.json
    ar_path = Path(run_dir) / "audit_report.json"
    if ar_path.exists():
        with open(ar_path) as f:
            ar = json.load(f)
        d["audit_total_commands"] = ar.get("total_commands", 0)
        d["audit_smb_set_count"] = ar.get("smb_set_count", 0)
        d["audit_smb_query_count"] = ar.get("smb_query_count", 0)
        d["audit_oe_command_count"] = ar.get("oe_command_count", 0)
        d["audit_forbidden"] = ar.get("forbidden_commands_sent", 0)

    # command_audit_comparison.json
    cp_path = Path(run_dir) / "command_plan" / "command_audit_comparison.json"
    if cp_path.exists():
        with open(cp_path) as f:
            cp = json.load(f)
        d["comparison_expected"] = cp.get("expected_command_count", 0)
        d["comparison_actual"] = cp.get("actual_command_count", 0)
        d["comparison_missing"] = len(cp.get("missing_expected_commands", []))
        d["comparison_unexpected"] = len(cp.get("unexpected_actual_commands", []))
        d["comparison_forbidden"] = len(cp.get("forbidden_actual_commands", []))

    # raw bin size
    raw_path = Path(run_dir) / "raw" / "oe1022d_rall.rawbin"
    d["rawbin_size_bytes"] = os.path.getsize(raw_path) if raw_path.exists() else 0

    # CSV files
    csv_files = list(Path(run_dir).rglob("*.csv"))
    d["csv_file_count"] = len(csv_files)

    # Magnetic commands
    audit_path = Path(run_dir) / "command_audit.jsonl"
    magnetic_count = 0
    if audit_path.exists():
        with open(audit_path) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    entry = json.loads(line)
                    if "mag" in entry.get("device_id", "").lower():
                        magnetic_count += 1
                except json.JSONDecodeError:
                    pass
    d["magnetic_command_count"] = magnetic_count

    # B-channel per-step stats
    rf_path = Path(run_dir) / "rf" / "rf_step_summary.jsonl"
    b_x_means = []
    b_y_means = []
    if rf_path.exists():
        with open(rf_path) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    step = json.loads(line)
                    if step.get("b_x_mean") is not None:
                        b_x_means.append(step["b_x_mean"])
                    if step.get("b_y_mean") is not None:
                        b_y_means.append(step["b_y_mean"])
                except json.JSONDecodeError:
                    pass
    if b_x_means:
        d["b_x_mean_overall"] = statistics.mean(b_x_means)
        d["b_x_std_overall"] = statistics.stdev(b_x_means) if len(b_x_means) > 1 else 0.0
    else:
        d["b_x_mean_overall"] = None
        d["b_x_std_overall"] = None
    if b_y_means:
        d["b_y_mean_overall"] = statistics.mean(b_y_means)
        d["b_y_std_overall"] = statistics.stdev(b_y_means) if len(b_y_means) > 1 else 0.0
    else:
        d["b_y_mean_overall"] = None
        d["b_y_std_overall"] = None

    return d


def compare_runs(runs: list[dict]) -> dict:
    """Build comparison structure from loaded run data."""
    comparison = {
        "schema_version": "0.2.0",
        "kind": "m3_5_run_comparison",
        "generated_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "runs": [],
        "summary": {},
    }

    # Per-run details
    for run in runs:
        comparison["runs"].append({
            "run_id": run["run_id"],
            "passed": run["passed"],
            "steps_requested": run.get("total_steps", 0),
            "steps_completed": run.get("steps_completed", 0),
            "steps_passed": run.get("steps_passed", 0),
            "frames_expected": run.get("frames_requested", 0),
            "frames_captured": run.get("frames_captured", 0),
            "frames_parsed": run.get("frames_parsed", 0),
            "frames_parse_failed": run.get("frames_parse_failed", 0),
            "parse_failure_rate": run.get("parse_failure_rate", 0.0),
            "command_audit_comparison_passed": run.get("command_audit_comparison_passed", False),
            "final_rf_off": run.get("final_rf_off", False),
            "final_mod_off": run.get("final_mod_off", False),
            "final_fm_off": run.get("final_fm_off", False),
            "syst_err_clean_after": run.get("final_syst_err_clean", False),
            "rawbin_size_bytes": run.get("rawbin_size_bytes", 0),
            "alignment_count": run.get("alignment_count", 0),
            "audit_total_commands": run.get("audit_total_commands", 0),
            "audit_smb_set_count": run.get("audit_smb_set_count", 0),
            "audit_smb_query_count": run.get("audit_smb_query_count", 0),
            "audit_oe_command_count": run.get("audit_oe_command_count", 0),
            "b_x_mean_overall": run.get("b_x_mean_overall"),
            "b_y_mean_overall": run.get("b_y_mean_overall"),
            "csv_file_count": run.get("csv_file_count", 0),
            "magnetic_command_count": run.get("magnetic_command_count", 0),
            "emergency_shutdown": run.get("emergency_shutdown", False),
            "notes": run.get("notes", []),
        })

    # Summary statistics
    all_passed = all(r["passed"] for r in comparison["runs"])
    frame_counts = [r["frames_captured"] for r in comparison["runs"]]
    parse_rates = [r["parse_failure_rate"] for r in comparison["runs"]]
    raw_sizes = [r["rawbin_size_bytes"] for r in comparison["runs"]]

    comparison["summary"] = {
        "all_passed": all_passed,
        "run_count": len(runs),
        "frame_count_stable": len(set(frame_counts)) == 1,
        "frame_count_values": frame_counts,
        "parse_failure_rate_stable": len(set(f"{r:.6f}" for r in parse_rates)) == 1,
        "parse_failure_rates": parse_rates,
        "rawbin_size_stable": len(set(raw_sizes)) == 1,
        "rawbin_size_values": raw_sizes,
        "all_final_safe_states_confirmed": all(
            r["final_rf_off"] and r["final_mod_off"] and r["final_fm_off"] and r["syst_err_clean_after"]
            for r in comparison["runs"]
        ),
        "all_command_audit_comparisons_passed": all(
            r["command_audit_comparison_passed"] for r in comparison["runs"]
        ),
        "no_csv_in_any_run": all(r["csv_file_count"] == 0 for r in comparison["runs"]),
        "no_magnetic_in_any_run": all(r["magnetic_command_count"] == 0 for r in comparison["runs"]),
        "no_emergency_shutdown_in_any_run": all(not r["emergency_shutdown"] for r in comparison["runs"]),
    }

    return comparison


def write_markdown(comparison: dict, out_path: str):
    """Write human-readable markdown comparison."""
    lines = []
    lines.append("# M3.5 Repeated Real Recipe Run Comparison")
    lines.append("")
    lines.append(f"Generated: {comparison['generated_at']}")
    lines.append("")
    lines.append("## Per-Run Summary")
    lines.append("")
    lines.append("| # | Run ID | Passed | Steps | Frames (cap/par/fail) | Parse Rate | Audit OK | RF | MOD | FM | SYST | Raw Size | Align |")
    lines.append("|---|--------|--------|-------|----------------------|------------|----------|----|-----|----|------|----------|-------|")
    for i, run in enumerate(comparison["runs"], 1):
        lines.append(
            f"| {i} | `{run['run_id']}` | {'✅' if run['passed'] else '❌'} | "
            f"{run['steps_completed']}/{run['steps_requested']} | "
            f"{run['frames_captured']}/{run['frames_parsed']}/{run['frames_parse_failed']} | "
            f"{run['parse_failure_rate']:.4f} | "
            f"{'✅' if run['command_audit_comparison_passed'] else '❌'} | "
            f"{'✅' if run['final_rf_off'] else '❌'} | "
            f"{'✅' if run['final_mod_off'] else '❌'} | "
            f"{'✅' if run['final_fm_off'] else '❌'} | "
            f"{'✅' if run['syst_err_clean_after'] else '❌'} | "
            f"{run['rawbin_size_bytes']:,} | "
            f"{run['alignment_count']} |"
        )
    lines.append("")

    lines.append("## Detailed Metrics")
    lines.append("")
    lines.append("| Metric | Run 1 | Run 2 | Run 3 | Stable? |")
    lines.append("|--------|-------|-------|-------|---------|")

    metrics = [
        ("Steps completed", "steps_completed", "{}"),
        ("Frames captured", "frames_captured", "{}"),
        ("Frames parsed", "frames_parsed", "{}"),
        ("Frames parse-failed", "frames_parse_failed", "{}"),
        ("Parse failure rate", "parse_failure_rate", "{:.6f}"),
        ("Alignment count", "alignment_count", "{}"),
        ("Raw bin size (bytes)", "rawbin_size_bytes", "{:,}"),
        ("Audit total commands", "audit_total_commands", "{}"),
        ("Audit SMB set count", "audit_smb_set_count", "{}"),
        ("Audit SMB query count", "audit_smb_query_count", "{}"),
        ("Audit OE command count", "audit_oe_command_count", "{}"),
        ("B-X mean overall", "b_x_mean_overall", "{:.4f}"),
        ("B-Y mean overall", "b_y_mean_overall", "{:.4f}"),
        ("CSV file count", "csv_file_count", "{}"),
        ("Magnetic command count", "magnetic_command_count", "{}"),
    ]

    for label, key, fmt in metrics:
        vals = [r.get(key) for r in comparison["runs"]]
        if all(v is None for v in vals):
            continue
        strs = [fmt.format(v) if v is not None else "N/A" for v in vals]
        stable = len(set(strs)) == 1 and "N/A" not in strs
        lines.append(f"| {label} | {strs[0]} | {strs[1]} | {strs[2]} | {'✅ Yes' if stable else '❌ No'} |")

    lines.append("")
    lines.append("## Cross-Run Summary")
    lines.append("")
    s = comparison["summary"]
    lines.append(f"- **All runs passed**: {'✅ Yes' if s['all_passed'] else '❌ No'}")
    lines.append(f"- **Frame counts stable**: {'✅ Yes' if s['frame_count_stable'] else '❌ No'}")
    lines.append(f"- **Parse failure rates stable**: {'✅ Yes' if s['parse_failure_rate_stable'] else '❌ No'}")
    lines.append(f"- **Raw bin sizes stable**: {'✅ Yes' if s['rawbin_size_stable'] else '❌ No'}")
    lines.append(f"- **All final safe states confirmed**: {'✅ Yes' if s['all_final_safe_states_confirmed'] else '❌ No'}")
    lines.append(f"- **All command audit comparisons passed**: {'✅ Yes' if s['all_command_audit_comparisons_passed'] else '❌ No'}")
    lines.append(f"- **No CSV in any run**: {'✅ Yes' if s['no_csv_in_any_run'] else '❌ No'}")
    lines.append(f"- **No magnetic in any run**: {'✅ Yes' if s['no_magnetic_in_any_run'] else '❌ No'}")
    lines.append(f"- **No emergency shutdown**: {'✅ Yes' if s['no_emergency_shutdown_in_any_run'] else '❌ No'}")
    lines.append("")

    # Warnings/notes
    all_notes = []
    for run in comparison["runs"]:
        for note in run.get("notes", []):
            all_notes.append(f"- `{run['run_id']}`: {note}")
    if all_notes:
        lines.append("## Notes / Warnings")
        lines.append("")
        lines.extend(all_notes)
        lines.append("")
    else:
        lines.append("## Notes / Warnings")
        lines.append("")
        lines.append("No warnings or errors across all three runs.")
        lines.append("")

    Path(out_path).write_text("\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser(description="Compare M3.5 repeated real recipe runs")
    parser.add_argument("run_dirs", nargs=3, help="Three run directory paths")
    parser.add_argument("--out-dir", default=".", help="Output directory for comparison files")
    args = parser.parse_args()

    runs = [load_run_data(d) for d in args.run_dirs]
    comparison = compare_runs(runs)

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    json_path = out_dir / "run_comparison.json"
    with open(json_path, "w") as f:
        json.dump(comparison, f, indent=2, default=str)
    print(f"Wrote {json_path}")

    md_path = out_dir / "run_comparison.md"
    write_markdown(comparison, str(md_path))
    print(f"Wrote {md_path}")


if __name__ == "__main__":
    main()
