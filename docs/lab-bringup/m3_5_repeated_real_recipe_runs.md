# M3.5 Repeated Real Recipe Runs and Minimal Run Comparison

Date: 2026-06-04

## Summary

Three consecutive real-hardware recipe-shaped runs were executed using the existing M3.4 `recipe_two_device_run` binary with identical recipe, parameters, and hardware configuration. All three runs passed all acceptance criteria with **perfect stability**: 22/22 steps, 110/110 frames captured and parsed, zero parse failures, identical command audit counts, and confirmed final safe state on every run.

## Hardware Configuration

| Device | Connection | Identity |
|--------|-----------|----------|
| SMB100A | TCP `169.254.2.20:5025` | `Rohde&Schwarz,SMB100A,1406.6000k02/101623,3.1.19.15-3.20.390.24` |
| OE1022D | Serial `/dev/cu.usbmodem3361358734371` @ 921600 | `SSI LIA-OE1022D,SN:D6522078,Version:Ver6.3200831` |

Recipe: `examples/recipes/m3_4/m3_4_two_device_sweep.recipe.json`

## Phase 0: Pre-check

| Check | Result |
|-------|--------|
| `cargo test --workspace` | PASS — all crates, 0 failures |
| `cargo fmt --all -- --check` | FAIL — pre-existing in `crates/odmr-mag/` (outside scope) |
| `cargo clippy --workspace --all-targets -- -D warnings` | FAIL — pre-existing `too_many_arguments` in `crates/odmr-mag/` (outside scope) |

No new failures introduced.

## Phase 1: Three Real Recipe Runs

### Run 1

**Run ID:** `m3_5_real_repeat_001_20260604_143946`

| Criterion | Result |
|-----------|--------|
| passed | ✅ true |
| steps | 22/22 |
| frames captured | 110 |
| frames parsed | 110 |
| frames failed | 0 |
| parse failure rate | 0.0000 |
| final RF_OFF | ✅ true |
| final MOD_OFF | ✅ true |
| final FM_OFF | ✅ true |
| final SYST_ERR_CLEAN | ✅ true |
| command_audit_comparison | ✅ passed (100/100) |
| no_forbidden | ✅ true |
| emergency shutdown | false |
| alignment count | 110 |
| raw bin size | 1,351,680 bytes |
| CSV files | 0 |
| magnetic commands | 0 |
| AuditReport.total_commands | 244 |
| AuditReport.smb_set_count | 100 |
| AuditReport.smb_query_count | 33 |
| AuditReport.oe_command_count | 111 |

### Run 2

**Run ID:** `m3_5_real_repeat_002_20260604_144143`

All metrics identical to Run 1.

### Run 3

**Run ID:** `m3_5_real_repeat_003_20260604_144339`

All metrics identical to Run 1 and Run 2.

## Phase 2: Run-to-Run Comparison

Comparison artifacts:
- `../../runs/m3_5_comparison_20260604_144500/run_comparison.json`
- `../../runs/m3_5_comparison_20260604_144500/run_comparison.md`

### Comparison Summary

| Metric | Run 1 | Run 2 | Run 3 | Stable? |
|--------|-------|-------|-------|---------|
| Steps completed | 22 | 22 | 22 | ✅ Yes |
| Frames captured | 110 | 110 | 110 | ✅ Yes |
| Frames parsed | 110 | 110 | 110 | ✅ Yes |
| Frames parse-failed | 0 | 0 | 0 | ✅ Yes |
| Parse failure rate | 0.000000 | 0.000000 | 0.000000 | ✅ Yes |
| Alignment count | 110 | 110 | 110 | ✅ Yes |
| Raw bin size (bytes) | 1,351,680 | 1,351,680 | 1,351,680 | ✅ Yes |
| Audit total commands | 244 | 244 | 244 | ✅ Yes |
| Audit SMB set count | 100 | 100 | 100 | ✅ Yes |
| Audit SMB query count | 33 | 33 | 33 | ✅ Yes |
| Audit OE command count | 111 | 111 | 111 | ✅ Yes |
| B-X mean overall | 0.0000 | 0.0000 | 0.0000 | ✅ Yes |
| B-Y mean overall | 0.0000 | 0.0000 | 0.0000 | ✅ Yes |
| CSV file count | 0 | 0 | 0 | ✅ Yes |
| Magnetic command count | 0 | 0 | 0 | ✅ Yes |

### Cross-Run Conclusions

- **All runs passed**: ✅ Yes
- **Frame counts stable**: ✅ Yes
- **Parse failure rates stable**: ✅ Yes
- **Raw bin sizes stable**: ✅ Yes
- **All final safe states confirmed**: ✅ Yes
- **All command audit comparisons passed**: ✅ Yes
- **No CSV in any run**: ✅ Yes
- **No magnetic in any run**: ✅ Yes
- **No emergency shutdown**: ✅ Yes

## Phase 3: Replay Check

Replay was attempted using the M3.5 real run artifacts. The replay mode requires `index.jsonl` (M3.2/M3.3 format), which M3.4 real runs did not generate. A minimal fix was applied to `real_run.rs` to also write `index.jsonl` alongside the existing `alignment/frame_to_rf_step_alignment.jsonl`. The `index.jsonl` files for the three completed runs were also backfilled from their alignment files.

**Replay Run ID:** `m3_5_replay_verify_20260604_145012`

| Criterion | Result |
|-----------|--------|
| replay does not connect hardware | ✅ Yes (replay mode) |
| replay_report exists | ✅ Yes |
| command_audit_comparison exists | ✅ Yes |
| frames_replayed | 110 |
| frames_parseable | 110 |
| alignment_rebuilt | ✅ Yes |
| statistics_rebuilt | ✅ Yes |
| command_audit_compared | ✅ Yes |
| replay passed | ✅ true |
| notes | [] |

Replay passes cleanly with no mismatches.

## Answers to M3.5 Questions

1. **Can the same recipe-shaped real run complete repeatedly?** ✅ Yes — 3/3 runs completed without error.
2. **Are frame counts stable?** ✅ Yes — 110/110 every run.
3. **Are parse failures stable or rare?** ✅ Yes — 0 failures across all 3 runs (330 frames total).
4. **Are step statistics stable enough to trust the acquisition chain?** ✅ Yes — all counts identical, raw bin sizes identical.
5. **Does command_audit remain consistent across runs?** ✅ Yes — 244 total commands, 100 SMB set, 33 SMB query, 111 OE commands, every run.
6. **Does final hardware safe state always hold?** ✅ Yes — RF off, MOD off, FM off, SYST:ERR clean on all 3 runs.

## Honest Development Notes

| # | Issue | Status |
|---|-------|--------|
| 1 | `cargo fmt` / `cargo clippy` fail on `crates/odmr-mag/` | Pre-existing, outside M3.5 scope |
| 2 | OE1022D IDN response contains trailing null bytes | Pre-existing cosmetic issue; does not affect functionality |
| 3 | Replay blocked on missing `index.jsonl` | Known from M3.4.1; minimal fix applied if needed in Phase 3 |

No compile errors, test failures, hardware failures, communication issues, or command mismatches were encountered during M3.5.

## Files Changed

| File | Change |
|------|--------|
| `tools/lab/recipe_two_device_run/scripts/compare_runs.py` | NEW — lightweight run comparison script |
| `tools/lab/recipe_two_device_run/src/real_run.rs` | MINOR — write `index.jsonl` for replay compatibility |
| `docs/lab-bringup/m3_5_repeated_real_recipe_runs.md` | NEW — this report |
| `../../runs/m3_5_comparison_20260604_144500/` | NEW — comparison artifacts |
| `../../runs/m3_5_real_repeat_001_20260604_143946/` | NEW — Run 1 artifacts |
| `../../runs/m3_5_real_repeat_002_20260604_144143/` | NEW — Run 2 artifacts |
| `../../runs/m3_5_real_repeat_003_20260604_144339/` | NEW — Run 3 artifacts |

## Conclusion

M3.5 repeated real recipe validation is complete. The acquisition chain is stable and trustworthy across back-to-back real hardware executions. No blockers for M3.6.
