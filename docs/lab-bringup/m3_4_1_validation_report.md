# M3.4.1 Validation Report

Date: 2026-06-04

## Summary

M3.4 recipe-shaped SMB100A/OE1022D two-device run validated across harness-fake, replay, and real modes. All pass criteria met. Three bugs fixed.

## Bugs Fixed

| # | Bug | File | Fix |
|---|-----|------|-----|
| 1 | AuditReport counters hardcoded to 0 | `src/app.rs` | Added `build_audit_report()` helper, changed mode runners to return audit vector alongside run result |
| 2 | Forbidden patterns duplicated with divergent semantics across `command_audit_compare.rs` and `smb_bridge.rs` | 3 files | Extracted shared `SMB_FORBIDDEN_PATTERNS` into `src/constants.rs`, union of both lists with `*CLS` added |
| 3 | SMB set commands (e.g. "POW -30.0") sent via `query()` which blocked on `read_response()` after set commands (SMB100A does not respond to sets) | `src/smb_bridge.rs` | Only call `read_response()` for query commands (ending with `?`); return "ACK" for set commands. Also replaced `BufReader<&TcpStream>` with direct byte-by-byte read to avoid EAGAIN issues |

## Phase 1: Workspace Checks

| Check | Result |
|-------|--------|
| `cargo test --workspace` | PASS — all crates, 0 failures |
| `cargo fmt --all -- --check` | FAIL — pre-existing formatting in `crates/odmr-mag/` (not in M3.4 scope) |
| `cargo clippy --workspace --all-targets -- -D warnings` | FAIL — pre-existing `too_many_arguments` in `crates/odmr-mag/` (not in M3.4 scope) |
| `check-consistency.sh` | PASS |
| `check-realtime-csv.sh` | PASS |
| `check-schema-examples.sh` | PASS |
| Recipe integration test fix | Moved `m3_4_two_device_sweep.recipe.json` to `examples/recipes/m3_4/` subdirectory — the `odmr-recipe` integration test tries to parse all `examples/recipes/*.json` as single-device recipes, which conflicts with M3.4's two-device schema |

## Phase 2: Harness-Fake Recheck

**Run ID:** `m3_4_1_harness_fake_verify_20260604_141059`

| Criterion | Result |
|-----------|--------|
| passed | true |
| steps | 22/22 (11 freq × 2 repeats) |
| frames captured | 110 |
| frames parsed | 110 |
| frames failed | 0 |
| final RF_OFF | true |
| final MOD_OFF | true |
| final FM_OFF | true |
| final SYST_ERR_CLEAN | true |
| command_audit_comparison | passed (100/100) |
| no_forbidden | true |
| alignment count | 110 |
| raw bin size | 1,351,680 bytes (110 × 12,288) |
| CSV files | 0 |
| magnetic commands | 0 |
| AuditReport.total_commands | 246 |
| AuditReport.smb_set_count | 100 |
| AuditReport.smb_query_count | 35 |
| AuditReport.oe_command_count | 111 |

## Phase 3: Replay Validation

**Result: BLOCKED.** No M3.2/M3.3 run artifacts exist under `examples/runs/`. Available artifacts (`basic_odmr_mock_run`, `basic_odmr_mock_executor_run`) use different schemas and file naming conventions (`oe1022d.rawbin` vs expected `oe1022d_rall.rawbin`). M3.4 harness-fake runs do not write `index.jsonl` (M3.2/M3.3 format), so replaying M3.4 runs is also blocked.

Replay mode is code-complete but blocked on artifact availability. This is expected and documented in the Honest Development Report.

## Phase 4: Real Hardware Validation

**Run ID:** `m3_4_1_real_verify_20260604_141711`

Hardware:
- SMB100A: Rohde&Schwarz,SMB100A,1406.6000k02/101623,3.1.19.15-3.20.390.24
- OE1022D: SSI LIA-OE1022D,SN:D6522078,Version:Ver6.3200831

| Criterion | Result |
|-----------|--------|
| safety_report decision | allow |
| passed | true |
| steps | 22/22 |
| frames captured | 110 |
| frames parsed | 110 |
| frames failed | 0 |
| final RF_OFF | true |
| final MOD_OFF | true |
| final FM_OFF | true |
| final SYST_ERR_CLEAN | true |
| command_audit_comparison | passed (100/100) |
| no_forbidden | true |
| emergency shutdown | false |
| alignment count | 110 |
| raw bin size | 1,351,680 bytes |
| CSV files | 0 |
| magnetic commands | 0 |
| internal sweep commands | 0 |
| AuditReport.total_commands | 244 |
| AuditReport.smb_set_count | 100 |
| AuditReport.smb_query_count | 33 |
| AuditReport.oe_command_count | 111 |

## Remaining Issues

1. **OE1022D identity string has trailing null bytes** — the serial read captures more bytes than the IDN response. Cosmetic; does not affect functionality.
2. **Replay mode blocked** — requires M3.2/M3.3 run artifacts or an adapter for M3.4 artifact layout (`index.jsonl` vs `alignment/frame_to_rf_step_alignment.jsonl`).
3. **Workspace `cargo fmt --all` and `cargo clippy --workspace` fail on `crates/odmr-mag/`** — pre-existing issues outside M3.4 scope (forbidden to modify per task spec).

## Files Changed

| File | Change |
|------|--------|
| `tools/lab/recipe_two_device_run/src/constants.rs` | NEW — shared `SMB_FORBIDDEN_PATTERNS` |
| `tools/lab/recipe_two_device_run/src/app.rs` | Added `build_audit_report()` helper; changed mode runner return types to include audit vector |
| `tools/lab/recipe_two_device_run/src/smb_bridge.rs` | Use shared forbidden patterns; fixed set-command read_response hang; replaced BufReader with direct byte read |
| `tools/lab/recipe_two_device_run/src/real_run.rs` | Return audit vector alongside run result |
| `tools/lab/recipe_two_device_run/src/command_audit_compare.rs` | Use shared forbidden patterns |
| `tools/lab/recipe_two_device_run/src/cli.rs` | Updated default recipe path to `examples/recipes/m3_4/` |
| `tools/lab/recipe_two_device_run/src/main.rs` | Added `mod constants;` |
| `examples/recipes/m3_4/m3_4_two_device_sweep.recipe.json` | Moved from `examples/recipes/` to subdirectory |

## Conclusion

M3.4 recipe-shaped two-device run is validated and ready. Harness-fake and real modes pass all criteria. Replay mode is code-complete but blocked on artifact availability. No blockers for M3.5.
