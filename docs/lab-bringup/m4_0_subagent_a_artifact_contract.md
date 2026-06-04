# M4.0 Subagent A — Artifact Contract Review

## Source

M3.6 analysis output directory: `/Users/erictseng/Documents/codex_git/runs/m3_6_analysis_20260604_153000/analysis/`

Generator: `tools/lab/recipe_two_device_run/scripts/analyze_runs.py`

## Required Files (must exist for successful load)

| File | Format | Records | Required |
|------|--------|---------|----------|
| `spectrum_points.jsonl` | JSONL | 66 lines, 14 fields/line | Yes — error if missing |
| `run_overlay_summary.json` | JSON | 1 object, 11 frequency groups | Yes — error if missing |
| `odmr_like_analysis_summary.json` | JSON | 1 object, 20+ fields | Yes — error if missing |
| `quality_flags.json` | JSON | 1 object, 21 fields | Yes — error if missing |

## Optional Files

| File | Format | Purpose | Behavior if missing |
|------|--------|---------|---------------------|
| `export_manifest.json` | JSON | File list with SHA256 | Warning only, not an error |
| `odmr_like_analysis_summary.md` | Markdown | Human-readable report | Ignored by GUI |

## `input_runs.json` — Does Not Exist

The task spec mentions this file but the current M3.6 analysis does not generate it. Source run IDs are embedded inside `odmr_like_analysis_summary.json` (field `source_run_ids`). The GUI should read source run info from the analysis summary, not a separate file.

## Field Contracts

### spectrum_points.jsonl (per-line object)

```
run_id: string
step_id: string
repeat_index: number
frequency_hz: number
frequency_verified_hz: number
b_x_mean_v: number
b_x_mean_mv: number
b_x_std_v: number
b_y_mean_v: number
b_y_mean_mv: number
b_y_std_v: number
frames_used: number
frames_parse_failed: number
step_passed: boolean
quality_flags: string[]
```

Note: `b_x_std_mv` and `b_y_std_mv` are NOT present in spectrum points — they only appear in the overlay summary.

### run_overlay_summary.json

```
schema_version: string
kind: "m3_6_run_overlay_summary"
frequency_count: number
generated_at: string (ISO 8601)
frequencies: FrequencyGroup[]
```

FrequencyGroup:
```
frequency_hz: number
point_count: number
total_frames_used: number
frames_parse_failed: number
contributing_run_ids: string[]
b_x_mean_v, b_x_mean_mv, b_x_std_v, b_x_std_mv, b_x_min_v, b_x_min_mv, b_x_max_v, b_x_max_mv: number
b_y_mean_v, b_y_mean_mv, b_y_std_v, b_y_std_mv, b_y_min_v, b_y_min_mv, b_y_max_v, b_y_max_mv: number
```

### odmr_like_analysis_summary.json

```
schema_version: string
kind: "m3_6_minimal_odmr_like_analysis"
source_run_ids: string[]
frequency_count: number
point_count: number
frames_used: number
frames_parse_failed: number
parse_failure_rate: number
all_runs_passed: boolean
all_safe_states_confirmed: boolean
no_csv: boolean
no_magnetic: boolean
quality_flags_passed: boolean
odmr_dip_detected: boolean (always false in M3.6)
physical_odmr_response_required: boolean (always false in M3.6)
contrast_estimate_b_x_v: number|null
contrast_estimate_b_x_mv: number|null
contrast_estimate_b_y_v: number|null
contrast_estimate_b_y_mv: number|null
oe1022d_display_idn_by_run: Record<string, string>
generated_at: string (ISO 8601)
```

### quality_flags.json

```
schema_version: string
kind: "m3_6_quality_flags"
passed: boolean (composite: all 9 sub-checks false)
missing_artifact: boolean
missing_artifact_details: Record<string, string[]>
failed_run: boolean
failed_run_ids: string[]
parse_failures: boolean
parse_failure_count: number
audit_mismatch: boolean
audit_mismatch_run_ids: string[]
unsafe_final_state: boolean
unsafe_final_state_run_ids: string[]
csv_present: boolean
csv_present_details: Record<string, string[]>
magnetic_command_present: boolean
magnetic_command_details: Record<string, number>
frequency_grid_mismatch: boolean
empty_signal_series: boolean
generated_at: string (ISO 8601)
```

## Path Resolution

Support two input forms:
1. User selects the parent analysis run directory (e.g., `m3_6_analysis_20260604_153000/`) → load from `<path>/analysis/`
2. User selects the `analysis/` subdirectory directly → load from `<path>/`

Detection: if `<path>/analysis/` exists and contains `quality_flags.json`, use that. Otherwise use `<path>/` directly.

## Missing-File Behavior

- Missing required file → return error with filename, frontend shows error state
- Missing optional file → return data without it, frontend shows warning
- Malformed JSON → return parse error with filename and line number for JSONL

## Read-Only Guarantees

- Backend opens files with read-only mode
- No `write`, `create`, `append`, or `truncate`
- No modification of run artifacts
- No creation of new files in the analysis directory
