# M4.1 Recipe / Dry-run Viewer

## Purpose

M4.1 adds a GUI page for loading, inspecting, editing, validating, and previewing M3.4 recipe-shaped runs before execution. The GUI remains dry-run / preview only — no hardware access, no recipe execution.

## Route

`/recipe-viewer` — "M4.1 Recipe / Dry-run Viewer"

Accessed via sidebar: **Recipe Viewer**

Navigation now exposes both:
- Analysis Viewer (M4.0)
- Recipe Viewer (M4.1)

## Input Recipe

M4.1 supports the M3.4 recipe shape (`examples/recipes/m3_4/m3_4_two_device_sweep.recipe.json`).

Users can:
- Load a recipe JSON from file via native file picker
- Paste/edit recipe JSON directly in a text area
- Reset to the bundled example recipe
- See parse and validation errors immediately

## Preview Outputs

### 1. Recipe Summary
- Recipe ID, kind, RF range, power, FM deviation
- Frames per step, repeat count
- Physical response required, magnetic scope

### 2. Resolved Recipe Preview
- Step count (points × repeats)
- Frequency grid (linspace from start to stop Hz)
- Total frames
- Estimated duration
- Device list

### 3. Dry-run Plan Preview
- Planned step count
- SMB command class counts (set/query)
- OE frame acquisition count
- Expected total commands and frames
- "No hardware execution" banner

### 4. Safety Report Preview
- Decision badge: ALLOW / ALLOW WITH WARNINGS / REJECT
- Per-check findings table (13 checks)
- Error count, warning count, passed count
- Operator approval requirement note

### 5. Command Plan Preview
- SMB set/query counts
- OE command count
- Shutdown command count
- Safety-relevant command count
- Forbidden commands: 0
- Internal sweep used: NO
- Magnetic commands: 0

## Safety Boundary

### Allowed Tauri Commands
- `read_recipe_file(path)` — read local JSON file
- `pick_recipe_file()` — native file picker

### Forbidden
- No network sockets
- No serial ports
- No SCPI commands
- No hardware crate imports
- No executor invocation
- No "Run on hardware" button

### Verification
```bash
grep -n "connect_\|send_\|open_serial\|open_tcp\|execute_\|run_hardware\|run_real\|start_executor" apps/desktop/src-tauri/src/main.rs
# Result: empty (no matches)
```

## Manual GUI Test Results

| # | Check | Result |
|---|-------|--------|
| 1 | `/analysis-viewer` still works | PASS |
| 2 | `/recipe-viewer` loads | PASS |
| 3 | Example recipe loads | PASS |
| 4 | Recipe summary displays | PASS |
| 5 | Validation panel shows allow | PASS |
| 6 | Resolved preview: 11 freq × 2 repeats | PASS |
| 7 | Expected frames = 110 | PASS |
| 8 | Safety report shows allow | PASS |
| 9 | Command plan: no forbidden commands | PASS |
| 10 | `magnetic.in_scope=true` → reject/warn | PASS |
| 11 | RF points above limit → reject | PASS |
| 12 | No hardware connection in logs | PASS |
| 13 | No serial/TCP instrument API | PASS |

## Real CLI Regression Run

Run ID: `m4_1_cli_regression_real_20260604_172347`

### Pass Criteria Results

| # | Criterion | Result |
|---|-----------|--------|
| 1 | SMB100A identity verified | PASS |
| 2 | OE1022D identity verified | PASS |
| 3 | Safety report decision = allow | PASS |
| 4 | Command plan written | PASS |
| 5 | 22/22 steps complete | PASS |
| 6 | 110 frames expected | PASS |
| 7 | Frames captured and parsed | PASS (110/110) |
| 8 | Command audit comparison passes | PASS |
| 9 | Final OUTP? = 0 | PASS (final_rf_off=true) |
| 10 | Final MOD:STAT? = 0 | PASS (final_mod_off=true) |
| 11 | Final FM:STAT? = 0 | PASS (final_fm_off=true) |
| 12 | SYST:ERR? clean | PASS (final_syst_err_clean=true) |
| 13 | No internal sweep | PASS |
| 14 | No magnetic commands | PASS |
| 15 | No CSV | PASS |
| 16 | Run artifact preserved | PASS |

### Key Metrics

```json
{
  "run_id": "m4_1_cli_regression_real_20260604_172347",
  "mode": "real",
  "passed": true,
  "steps_completed": 22,
  "total_steps": 22,
  "frames_captured": 110,
  "frames_parsed": 110,
  "frames_parse_failed": 0,
  "parse_failure_rate": 0.0,
  "final_rf_off": true,
  "final_mod_off": true,
  "final_fm_off": true,
  "final_syst_err_clean": true,
  "command_audit_comparison_passed": true,
  "no_forbidden_commands_sent": true,
  "emergency_shutdown_triggered": false
}
```

### Audit Report

```json
{
  "total_commands": 244,
  "allowed_commands": 244,
  "blocked_commands": 0,
  "forbidden_commands_sent": 0,
  "smb_set_count": 100,
  "smb_query_count": 33,
  "oe_command_count": 111,
  "no_internal_sweep_commands": true,
  "no_magnetic_commands": true
}
```

## Code Check Results

| Check | Result |
|-------|--------|
| `pnpm tsc --noEmit` | PASS — 0 errors |
| `pnpm build` | PASS — 773 modules, 1.55s |
| `cargo test --workspace` | PASS — all crates, 0 failures |
| `cargo fmt --check` (M4.1 files) | PASS |
| `check-frontend-hardware.sh` | Pre-existing false positives only (Deserialize, doc text); no M4.1 regressions |

## Files Changed

| File | Change |
|------|--------|
| `apps/desktop/src/types/recipe.ts` | NEW — M3.4 recipe TypeScript types |
| `apps/desktop/src/utils/recipeValidation.ts` | NEW — JSON parse, shape, value validation |
| `apps/desktop/src/utils/recipePreview.ts` | NEW — resolved/dry-run/safety/command plan preview |
| `apps/desktop/src/routes/RecipeViewerPage.tsx` | NEW — M4.1 recipe viewer page (8 panels) |
| `apps/desktop/src/App.tsx` | Added `/recipe-viewer` route |
| `apps/desktop/src/components/SideNav.tsx` | Added "Recipe Viewer" nav link |
| `apps/desktop/src/components/MockOnlyBanner.tsx` | Updated to "M4.1 DRY-RUN VIEWER" |
| `apps/desktop/src-tauri/src/main.rs` | Added `read_recipe_file` + `pick_recipe_file` commands |
| `docs/lab-bringup/m4_1_subagent_a_recipe_preview_contract.md` | NEW |
| `docs/lab-bringup/m4_1_subagent_b_gui_safety_boundary.md` | NEW |
| `docs/lab-bringup/m4_1_subagent_c_real_cli_regression.md` | NEW |
| `docs/lab-bringup/m4_1_recipe_dry_run_viewer.md` | NEW — this document |

## Limitations Before M4.2

1. **GUI cannot approve runs** — operator approval is displayed as info only; actual approval requires CLI `--operator-approves-real-run`.
2. **No executor integration** — M4.1 does not call `odmr-executor` or any hardware crate.
3. **Frontend validation only** — validation logic is duplicated in TypeScript; future M4.2 should use shared Rust validation via Tauri.
4. **No run launching** — M4.1 has no "Run on hardware" button; M4.2 will add an operator-approved GUI run launcher.
5. **M3.4 shape only** — preview logic is hardcoded for the two-device sweep recipe; other recipe kinds will need extension.

## Honest Development Notes

- **Pre-existing issues found**: `cargo fmt --check` reports formatting issues in `crates/odmr-mag/src/lib.rs` (unrelated to M4.1). `check-frontend-hardware.sh` has known false positives on `Deserialize` and doc text.
- **No hardware access from GUI**: Verified via grep — no forbidden command names, no hardware crate imports.
- **No magnetic access**: Magnetic commands count = 0 in both GUI preview and real CLI run.
- **Real CLI regression passed cleanly**: 110/110 frames, 0 parse failures, all safety checks pass.

## Next Step

M4.2: Operator-approved GUI run launcher (integrate executor backend with human confirmation flow).
