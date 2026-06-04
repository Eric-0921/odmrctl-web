# Subagent A — Recipe Preview Contract

## 1. Required Recipe Fields

M3.4 recipe shape (`examples/recipes/m3_4/m3_4_two_device_sweep.recipe.json`):

| Field | Type | Required | Default |
|-------|------|----------|---------|
| `schema_version` | string | yes | `"0.2.0"` |
| `kind` | string | yes | `"two_device_odmr_like_sweep_recipe"` |
| `id` | string | yes | — |
| `description` | string | no | `null` |
| `devices.smb100a.device_id` | string | yes | — |
| `devices.smb100a.mode` | string | no | `"real_or_fake_by_runtime"` |
| `devices.oe1022d.device_id` | string | yes | — |
| `devices.oe1022d.mode` | string | no | `"real_or_fake_by_runtime"` |
| `devices.magnetic.in_scope` | boolean | yes | — |
| `rf.start_hz` | number | yes | — |
| `rf.stop_hz` | number | yes | — |
| `rf.points` | integer | yes | — |
| `rf.power_dbm` | number | yes | — |
| `rf.max_power_dbm` | number | yes | — |
| `modulation.fm_source` | string | no | `"INT"` |
| `modulation.fm_deviation_hz` | number | yes | — |
| `modulation.max_fm_deviation_hz` | number | yes | — |
| `modulation.internal_lf.enabled` | boolean | yes | — |
| `modulation.internal_lf.frequency_hz` | number | yes | — |
| `modulation.internal_lf.shape` | string | yes | — |
| `modulation.internal_lf.voltage_v` | number | yes | — |
| `modulation.internal_lf.lf_output_enabled` | boolean | no | `false` |
| `acquisition.frames_per_step` | integer | yes | — |
| `acquisition.repeat_count` | integer | no | `2` |
| `acquisition.inter_frame_delay_ms` | integer | no | `20` |
| `safety.require_operator_approval` | boolean | no | `true` |
| `safety.no_internal_sweep` | boolean | no | `true` |
| `safety.no_csv` | boolean | no | `true` |
| `safety.no_gui` | boolean | no | `true` |
| `safety.no_magnetic` | boolean | no | `true` |
| `safety.physical_response_required` | boolean | no | `false` |

## 2. Validation Rules

### JSON parse
- Must be valid JSON.

### Shape validation
- `kind` must be `"two_device_odmr_like_sweep_recipe"`.
- `rf`, `modulation`, `acquisition`, `safety` blocks must exist.
- `devices.smb100a`, `devices.oe1022d`, `devices.magnetic` must exist.

### Value validation (hard limits from `safety.rs`)

| Check | Limit | Error if violated |
|-------|-------|-------------------|
| RF start/stop | `> 0`, `start <= stop` | yes |
| RF points | `>= 2`, `<= 21` | yes |
| Max RF power | `<= -10 dBm` | yes |
| RF power | `<= max_power_dbm` | yes |
| Max FM deviation | `<= 5_000_000 Hz` | yes |
| FM deviation | `<= max_fm_deviation_hz` | yes |
| Frames per step | `<= 10` | yes |
| Repeat count | `<= 3` | yes |
| Total frames | `points * fps * repeats <= 630` | yes |
| Magnetic in_scope | must be `false` (if `no_magnetic=true`) | yes |
| LF output | `lf_output_enabled` must be `false` | yes |
| LF shape | must be in allowed list | yes |

### Policy validation
- `no_internal_sweep = false` → reject
- `no_csv = false` → reject
- `no_gui = false` → warning
- `magnetic.in_scope = true` → reject (if `no_magnetic=true`)

## 3. Preview Fields Needed by GUI

### Recipe Summary
- `id`, `kind`, `description`
- RF: `start_hz`, `stop_hz`, `points`, `power_dbm`
- Modulation: `fm_source`, `fm_deviation_hz`
- Acquisition: `frames_per_step`, `repeat_count`
- Safety: `physical_response_required`, `magnetic.in_scope`

### Resolved Preview
- `step_count` = points × repeat_count
- `frequency_grid` = linspace(start, stop, points)
- `total_frames` = points × frames_per_step × repeat_count
- `estimated_duration_s`
- `device_list` = ["smb100a", "oe1022d"]

### Dry-run Preview
- `planned_steps` count
- `smb_command_classes` (set/query counts)
- `oe_frame_count`
- `expected_command_count`
- `expected_frame_count`

### Safety Report Preview
- `decision`: allow / reject / allow_with_warnings
- `findings[]`: { check, severity, passed, detail }
- `errors_count`, `warnings_count`, `passed_count`
- `operator_approval_required`

### Command Plan Preview
- `smb_set_count`, `smb_query_count`
- `oe_count`
- `forbidden_count` (must be 0)
- `internal_sweep_used` (must be false)
- `magnetic_commands` (must be 0)

## 4. Mismatch Risks Between GUI Preview and CLI Real Run

| Risk | Mitigation |
|------|------------|
| Frontend duration estimate drifts from actual | Use same formula as `dry_run.rs`; document as estimate only |
| Frontend validation accepts recipe that CLI rejects | Strictly match `recipe.rs` + `safety.rs` constants; golden test with example recipe |
| Command plan count differs from actual | Use deterministic formula from `command_plan.rs`; count is pre-computed, not runtime |
| Safety decision false positive/negative | Match all 13 checks from `check_recipe_safety` exactly |
| Frequency grid rounding | Use same `linspace` formula; display with `.0` Hz precision |
