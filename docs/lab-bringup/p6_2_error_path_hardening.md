# P6.2 — Error-Path Hardening + common_preflight Workspace Extraction

> Date: 2026-05-31  
> Commit: (to be filled after push)

## 1. Purpose

Fix M5A runtime safety blockers before GUI run launcher (M4.2) and multi-axis ODMR (M5B):

- RF may stay ON after setup failure
- No SIGINT / emergency stop
- Audit booleans are optimistic (hardcoded true)
- StationLedger becomes stale after run
- Device locks drop after preflight
- `common_preflight` not yet a workspace crate

## 2. Failures Fixed

### 2.1 SMB RF stays ON after failure

**Before:** Error paths after Step 8 (RF ON) called Maynuo cleanup but never sent `OUTP OFF` to SMB.

**After:** All 4 error paths after Step 8 now send `OUTP OFF` before returning:
- FREQ failure
- POW failure
- OUTP ON failure
- OUTP? verification failure

### 2.2 No SIGINT handler

**Before:** Ctrl+C would terminate the process immediately, potentially leaving RF ON and Mag current nonzero.

**After:** Unix SIGINT handler sets `ABORT_FLAG` (atomic bool). `check_abort()` is called after every major step (preflight, RF ON, each OE frame). If set, the run exits cleanly through the normal error path, which triggers cleanup.

### 2.3 Audit booleans misleading

**Before:** `mag_final_output_off`, `mag_final_local_requested`, and `cleanup_completed` were hardcoded to `true` regardless of actual command success.

**After:**
- `mag_final_output_off = outp_ok` (result of `send_set_output(false)`)
- `mag_final_local_requested = loc_ok` (result of `send_set_local()`)
- `cleanup_completed = false` in `maynuo_cleanup_and_exit` (error paths don't claim success)
- Audit entries record `sent_to_transport` and `transport_error` truthfully

### 2.4 StationLedger stale after run

**Before:** Ledger was only updated during preflight. A failed run would leave it showing "safe" from the preflight pass.

**After:** Post-run ledger update at the end of `run()`. If `report.passed`, all devices marked safe. If failed, all marked unsafe.

### 2.5 Device locks drop after preflight

**Before:** `run_station_preflight()` acquired locks and dropped them on return, leaving a TOCTOU window during execution.

**After:**
- New API: `run_station_preflight_with_locks()` returns `(StationPreflightReport, Vec<DeviceLock>)`
- M5A holds the `Vec<DeviceLock>` until `run()` returns (after cleanup)
- Original `run_station_preflight()` is preserved for backward compatibility

## 3. common_preflight → odmr-preflight Extraction

### New workspace crate

```
crates/odmr-preflight/
  Cargo.toml
  README.md
  src/lib.rs
  src/types.rs          (+ ProbeClass enum)
  src/error.rs
  src/device_lock.rs
  src/ledger.rs
  src/station_report.rs
  src/smb_probe.rs
  src/oe_probe.rs
  src/maynuo_probe.rs
  src/cni_laser_probe.rs
```

### API surface

- `run_station_preflight(...)` — backward-compatible
- `run_station_preflight_with_locks(...)` — returns locks
- `DeviceLock`, `LockError`
- `StationLedger`, `mark_safe`, `mark_unsafe`
- `StationPreflightReport`, `DevicePreflightReport`, `DeviceLockStatus`
- `ProbeClass` enum (identity_only, query_only, safe_state_probe, etc.)

### Compatibility

- `tools/lab/common_preflight/` is now a thin CLI wrapper depending on `odmr-preflight`
- `tools/lab/rf_mag_oe_minimal_run/` depends on `odmr-preflight` instead of `common_preflight`
- `cni_laser_fake_driver` removed `[workspace]` from its `Cargo.toml` so workspace crates can depend on it

## 4. Cleanup Guarantees

| Scenario | SMB OUTP OFF | Mag CURR 0 | Mag OUTP 0 | Mag SYST:LOC | Ledger Updated |
|----------|-------------|-----------|-----------|-------------|---------------|
| Normal completion | ✅ Verified | ✅ Verified | ✅ Verified | ✅ Attempted | ✅ |
| FREQ/POW/OUTP ON fail | ✅ Emergency | ✅ | ✅ | ✅ | ✅ |
| OUTP? verification fail | ✅ Emergency | ✅ | ✅ | ✅ | ✅ |
| Maynuo zero/recur fail | N/A (RF not on) | ✅ | ✅ | ✅ | ✅ |
| OE acquisition fail | ✅ (after loop) | ✅ | ✅ | ✅ | ✅ |
| SIGINT | ✅ (abort path) | ✅ | ✅ | ✅ | ✅ |

## 5. SIGINT Behavior

1. Signal handler sets `ABORT_FLAG = true`
2. Next `check_abort()` call returns `Err("Run aborted by operator (SIGINT)")`
3. Error is pushed to `report.errors`
4. `report.interrupted = true`
5. Current step exits → normal error path cleanup runs
6. `maynuo_cleanup_and_exit()` writes artifacts with `cleanup_completed = false`
7. Ledger marks devices unsafe
8. Process exits with error code

## 6. Fake/Fault-Injection Tests Added

| Test | What it verifies |
|------|-----------------|
| `test_smb_emergency_off_on_rf_failure` | Fake SMB sends `OUTP OFF` on emergency path |
| `test_cleanup_booleans_truthful_on_failure` | Fake Maynuo booleans reflect actual state |
| `test_abort_flag_triggers_exit` | Atomic abort flag logic |
| `test_report_interrupted_field_exists` | `interrupted` field serializes in report |

Plus existing tests: `test_happy_path_fake_transports`, `test_no_csv_files_created`, etc.

## 7. Real Hardware Regression

**Status:** Not run. Hardware was not explicitly requested for this session.

Fake/fault-injection tests provide coverage for the new error paths. Real hardware regression should be run before M5B.

## 8. Checks Results

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |
| `cargo test --workspace` | ✅ |
| `cargo test -p odmr-maynuo-m8812` | ✅ 13 passed |
| `cargo test -p odmr-mag` | ✅ 138 passed |
| `cargo test -p odmr-preflight` | ✅ 8 passed + 1 doc-test |
| `cargo test --manifest-path tools/lab/common_preflight/Cargo.toml` | ✅ |
| `cargo test --manifest-path tools/lab/rf_mag_oe_minimal_run/Cargo.toml` | ✅ 16 passed |
| `cargo test --manifest-path tools/lab/recipe_two_device_run/Cargo.toml` | ✅ 40 passed |
| `cargo test --manifest-path tools/lab/cni_laser_fake_driver/Cargo.toml` | ✅ 16 passed |
| `pnpm tsc --noEmit` (desktop) | ✅ |
| `check-consistency.sh` | ✅ |
| `check-frontend-hardware.sh` | ✅ |
| `check-realtime-csv.sh` | ✅ |

## 9. Remaining Blockers Before Recipe-M5B

1. **M4.2 GUI run launcher** — requires typed Rust runtime API, not yet implemented
2. **Recipe schema magnetic support** — no magnetic step types in recipe yet
3. **Executor multi-device coordination** — `odmr-executor` handles RF+OE only
4. **`odmr-mag` real bridge** — mock-only; M5A bypasses it for real hardware
5. **Emergency stop physical button** — SIGINT is software-only
6. **Command-plan audit comparison** — no `command_plan.jsonl` like `recipe_two_device_run`

## 10. Honest Notes

| # | Issue | Action |
|---|-------|--------|
| 1 | `cargo fmt` found unformatted files in copied `odmr-preflight` sources | Fixed by running `cargo fmt --all` |
| 2 | `cni_laser_fake_driver` had `[workspace]` which blocked workspace crate dependency | Removed `[workspace]` from its `Cargo.toml` |
| 3 | `ctrlc` crate download failed (network issue) | Switched to `libc` + raw Unix signal handler |
| 4 | `FakeSmbTransport` had no failure injection | Added `fail_on: Option<String>` field |
| 5 | 6 pre-existing clippy warnings in `rf-mag-oe-minimal-run` | Left unchanged (unrelated to P6.2) |
| 6 | `common_preflight` CLI wrapper has 0 tests | Acceptable — it's a thin wrapper around tested crate |
