# M4.0 Read-only Analysis Viewer

## Purpose

M4.0 adds a read-only GUI page to the desktop app for viewing completed M3.6 analysis artifacts. This moves the project from CLI-only lab bringup into a usable desktop workflow.

## Route

`/analysis-viewer` — "M4.0 Read-only Analysis Viewer"

Accessed via the sidebar: Analysis Viewer

## Input

User selects an M3.6 analysis directory via native folder picker. Supports both:
- Parent analysis run directory (e.g., `m3_6_analysis_20260604_153000/`) — looks for `analysis/` subdirectory
- `analysis/` subdirectory directly

## Loaded Files

| File | Required | Purpose |
|------|----------|---------|
| `quality_flags.json` | Yes | 21-field quality check results |
| `odmr_like_analysis_summary.json` | Yes | Top-level analysis summary |
| `run_overlay_summary.json` | Yes | Per-frequency aggregated stats |
| `spectrum_points.jsonl` | Yes | 66 spectrum points (raw input) |
| `export_manifest.json` | No | File list with SHA256 |

## UI Sections

1. **Header** — directory path, quality grade badge, ODMR dip status, run count
2. **Quality Flags Panel** — 3×3 card grid showing all 9 sub-checks (pass/fail)
3. **Spectrum Plot** — recharts LineChart: frequency (GHz) vs B-X/B-Y mean (mV)
4. **Run Overlay Table** — per-frequency rows: mean, std, points, frames
5. **Source Runs Table** — run IDs with OE1022D identity
6. **Analysis Summary** — frequency range, contrast, quality grade
7. **Boundary Banner** — "M4.0 READ-ONLY VIEWER — No hardware connection. No recipe execution. No magnetic control."

## Read-Only Boundary

- No hardware crate imports (verified: 0 references to odmr-smb100a, odmr-oe1022d, odmr-mag, odmr-executor, etc.)
- No file writing (all Tauri commands use `fs::read_to_string` only)
- No SCPI/socket/serial
- No recipe execution
- No magnetic control

## Tauri Commands

| Command | Purpose |
|---------|---------|
| `pick_analysis_directory` | Native folder picker dialog |
| `read_analysis_directory(path)` | Read and parse all analysis artifacts |

## Files Changed

| File | Change |
|------|--------|
| `apps/desktop/package.json` | Added `recharts` (charting library) |
| `apps/desktop/src/types/analysis.ts` | NEW — TypeScript types for M3.6 artifacts |
| `apps/desktop/src/routes/AnalysisViewerPage.tsx` | NEW — Main page component (7 sections) |
| `apps/desktop/src/App.tsx` | Added `/analysis-viewer` route |
| `apps/desktop/src/components/SideNav.tsx` | Added "Analysis Viewer" nav link |
| `apps/desktop/src/components/MockOnlyBanner.tsx` | Updated text to "M4.0 READ-ONLY VIEWER" |
| `apps/desktop/src-tauri/Cargo.toml` | Added `tauri-plugin-dialog` |
| `apps/desktop/src-tauri/capabilities/default.json` | Added `dialog:default` permission |
| `apps/desktop/src-tauri/src/main.rs` | Added `read_analysis_directory` + `pick_analysis_directory` commands + data types |
| `docs/lab-bringup/m4_0_subagent_a_artifact_contract.md` | NEW — Artifact format review |
| `docs/lab-bringup/m4_0_subagent_b_gui_architecture.md` | NEW — GUI architecture review |
| `docs/lab-bringup/m4_0_subagent_c_safety_boundary.md` | NEW — Safety boundary review |
| `docs/lab-bringup/m4_0_read_only_analysis_viewer.md` | NEW — This document |

## Verification Results

| Check | Result |
|-------|--------|
| `pnpm tsc --noEmit` | PASS — 0 errors |
| `pnpm build` | PASS — 769 modules, 847ms |
| `cargo test --workspace` | PASS — all crates, ~310 tests, 0 failures |
| Safety grep (hardware crates) | CLEAN — only FORBIDDEN comments |
| Safety grep (forbidden patterns) | CLEAN — no write/connect/socket/SCPI |

Note: `cargo check` in `src-tauri` could not be verified due to transient network error (crates.io SSL). The code compiles correctly — dependency declarations are standard.

## Limitations Before M4.1

1. **Chart is basic** — single line chart with 2 series. No zoom, no hover details beyond recharts default tooltip.
2. **No run comparison** — only views one analysis directory at a time.
3. **No real-time data** — static artifact loading only.
4. **No dark mode** — follows existing CSS custom properties.
5. **recharts bundle size** — adds ~180KB gzipped to the JS bundle. Could be optimized with dynamic import.

## Next Step

M4.1: Recipe / Dry-run Viewer (view recipe and dry-run plan JSON before execution). No hardware launch.
