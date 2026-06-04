# Audit D — RF / OE / Mag Integration

> Audit date: 2026-05-31  
> Base commit: `4627980`

## Files Inspected

- `tools/lab/rf_mag_oe_minimal_run/src/app.rs`
- `tools/lab/rf_mag_oe_minimal_run/src/cli.rs`
- `tools/lab/rf_mag_oe_minimal_run/src/main.rs`
- `tools/lab/rf_mag_oe_minimal_run/src/mag_bridge.rs`
- `tools/lab/rf_mag_oe_minimal_run/src/smb_bridge.rs`
- `tools/lab/rf_mag_oe_minimal_run/src/oe_bridge.rs`
- `tools/lab/common_preflight/src/lib.rs`
- `tools/lab/common_preflight/src/ledger.rs`
- `crates/odmr-maynuo-m8812/src/lib.rs`
- `crates/odmr-mag/src/lib.rs`
- `tools/lab/recipe_two_device_run/src/` (all .rs)

## 1. Safety Sequence Assessment

### Success Path (verified correct)

```
1. Preflight (RF off check, Mag zero check)
2. Mag zero-baseline (CURR 0, OUTP 0)
3. Mag recurrent setpoint (CURR <target>)
4. RF setup (FREQ, POW)
5. RF ON (OUTP ON) — AFTER Mag is stable
6. OE capture (RALL frames)
7. RF OFF (OUTP OFF) — BEFORE Mag cleanup
8. Mag cleanup (CURR 0 → OUTP 0 → verify → SYST:LOC)
```

**✅ RF-off during Mag setup enforced.** SMB preflight aborts if `OUTP?` returns `"1"`. RF turned on only after Mag reaches hold current. RF turned off before Mag cleanup begins.

### 🔴 Critical Gap: Error Path RF Shutdown

If SMB RF-ON setup fails (`FREQ`, `POW`, or `OUTP ON`/`OUTP?` error), the code calls `run_cleanup(&mut mag)` and exits, but **never sends `OUTP OFF` to SMB**. The `smb` transport is simply dropped, leaving RF potentially energized.

**Severity: HIGH** — unsupervised automation could leave RF on after a failure.

## 2. Cleanup Ordering Judgment

### Success-Path Maynuo Cleanup

Order: `CURR 0` → `OUTP 0` → 500ms settle → `MEAS:CURR?` verify < 1mA → `SYST:LOC`

**✅ Matches verified-normal shutdown plan** in `odmr-mag::build_verified_normal_shutdown_plan`.

### Error-Path Maynuo Cleanup (`mag_bridge::run_cleanup`)

Same command order but **omits verification and settling**.

**Severity: LOW-MEDIUM** — acceptable for error path but not ideal.

### Misleading Report Fields

`report.magnetic.mag_final_output_off` and `mag_final_local_requested` are set to `true` unconditionally after the commands, even if `send_set_output(false)` or `send_set_local()` failed. The error is logged in `report.errors`, but the boolean flags are unreliable for automated post-run safety checks.

**Severity: MEDIUM** — automated safety checks must read `errors` and `transport_error`, not just boolean flags.

## 3. Preflight Integration Judgment

### Structural Usage

M5A loads `StationProfile`, calls `run_station_preflight()`, forwards `ledger_path` and `operator_approve`, writes preflight artifacts, and gates on `report.passed()`.

**✅ Correct structural integration.**

### Ledger Hygiene Gap

M5A never updates the ledger after the run. If the process crashes between preflight and cleanup, the ledger still records "safe" from the preflight pass. Next session's extended preflight will not trigger.

**Severity: MEDIUM** — creates false-negative for extended preflight.

### Device Lock TOCTOU

Device locks are held only during `run_station_preflight()`. The `DeviceLock` Vec drops on return, releasing locks. The actual run executes without locks, leaving a window where another process could grab the same port.

**Severity: MEDIUM** — in a single-operator lab this is unlikely; in multi-process environment it's a race.

## 4. Command Audit Completeness

All SCPI commands sent to SMB and Maynuo are captured in `CommandAuditEntry` with `sent_to_transport`, `transport_error`, `response_summary`, and `timestamp_ns`.

**✅ Audit trail is complete for the commands that are sent.**

**⚠️ Gap:** The boolean `sent_to_transport` is hardcoded to `true` in some bridge functions even when the underlying write may have failed. Need to propagate actual write success/failure.

## 5. Recipe Compatibility Assessment

### Current State

| Aspect | Status |
|--------|--------|
| M5A is recipe-driven? | **No** — hardcoded execution order, CLI args |
| `recipe_two_device_run` supports Mag? | **No** — enforces `no_magnetic: true` |
| `odmr-mag` integrated with executor? | **No** — mock-only models |
| M5A uses `odmr-mag`? | **No** — bypasses it, uses `odmr-maynuo-m8812` directly |

### Required for Recipe Integration

1. Extend recipe schema for magnetic actions (zero-baseline, recurrent setpoint, axis selection)
2. Add mag safety rules to `odmr-safety` (current limits, coil matrix validation, inter-axis conflicts)
3. Extend `odmr-compiler` to resolve mag steps and interleave with RF/OE steps
4. Extend `odmr-executor` to coordinate three real devices with cross-device safety gates
5. Bridge `odmr-mag` mock models to real `odmr-maynuo-m8812` driver

**Verdict: Low compatibility today. Significant work required.**

## 6. Hardcoded Constants

| Constant | Location | Risk |
|----------|----------|------|
| `VOLT 75` | `mag_bridge.rs` | Should be from station profile |
| 500ms cleanup settle | `app.rs` | Should be configurable |
| 1.0mA current tolerance | `maynuo_probe.rs` | Should be from safety policy |
| `rf_frequency_hz` default | `cli.rs` | CLI overridable, okay |
| `rf_power_dbm` default | `cli.rs` | CLI overridable, okay |

## 7. Emergency Stop / Signal Handler

**❌ No SIGINT handler.** Ctrl+C could leave:
- RF ON (SMB100A)
- Mag current nonzero (Maynuo)
- Maynuo in remote mode

**Severity: MEDIUM** — expected for a lab tool, but must be addressed before unsupervised automation.

## 8. Risk Summary Table

| # | Risk / Blocker | Severity | Evidence |
|---|----------------|----------|----------|
| 1 | **SMB RF may stay ON after RF-ON failure** | 🔴 High | `app.rs` error paths omit `OUTP OFF` |
| 2 | **Maynuo audit entries misleading on failure** | 🟡 Medium | `sent_to_transport` hardcoded `true` |
| 3 | **Ledger becomes stale after run** | 🟡 Medium | No post-run `mark_safe`/`mark_unsafe` |
| 4 | **No device locks during execution** | 🟡 Medium | Locks drop when preflight returns |
| 5 | **Hardcoded constants** | 🟡 Medium | Not runtime-configurable |
| 6 | **No emergency stop / SIGINT handler** | 🟡 Medium | Could leave hardware energized |
| 7 | **Maynuo cleanup lacks verification in error path** | 🟢 Low | `run_cleanup` omits settle/verify |
| 8 | **Mock/real split in `odmr-mag`** | 🟡 Medium | M5A reimplements real logic ad-hoc |
| 9 | **Report fields unreliable for automation** | 🟡 Medium | Booleans set regardless of success |
| 10 | **No command-plan comparison** | 🟢 Low | No `command_plan.jsonl` audit |

## 9. Verdict

M5A is a **functional bring-up tool** with correct success-path safety sequencing. It has **gaps in failure-path RF shutdown, audit fidelity, and ledger hygiene**. It is **not yet compatible** with the recipe-driven executor architecture.

Before promoting M5A concepts to the core pipeline:
- Fix error-path RF shutdown
- Add SIGINT/emergency stop handler
- Improve audit boolean accuracy
- Bridge `odmr-mag` to real hardware
- Extend recipe schema for magnetic steps
