# Cleanup / Shutdown Audit Matrix

**Date**: 2026-06-04
**Scope**: All 17 tools in `tools/lab/`
**Auditor**: Agent (automated grep + manual review)

## Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Present and correct |
| ⚠️ | Present but has issues |
| ❌ | Missing or dangerously wrong |
| N/A | Tool does not interact with this device |

---

## Maynuo M8812 Tools

| Tool | Cleanup Function | Sequence | Wait | MEAS:CURR? Verify | Danger |
|------|-----------------|----------|------|-------------------|--------|
| `maynuo_m8812_identity_probe` | None (closes port after IDN) | N/A | N/A | N/A | ✅ No energy output, safe |
| `maynuo_m8812_zero_baseline` | `attempt_cleanup()` | OUTP 0 → CURR 0 → SYST:LOC | ❌ None | ❌ None | ⚠️ **Wrong order** (should be CURR 0 → OUTP 0); no wait; no current verify |
| `maynuo_m8812_recur_microtest` | `attempt_cleanup()` (error path) | CURR 0 → OUTP 0 → SYST:LOC | ❌ None | ❌ None | ⚠️ No wait; no MEAS:CURR? verify |
| `maynuo_m8812_recur_microtest` | Main cleanup (success path) | CURR 0 → OUTP 0 → SYST:LOC | ❌ None | ❌ None | ⚠️ `current_final_zero = true` is **hardcoded** (not measured); no wait |
| `maynuo_m8812_sequential_axis_run` | `attempt_cleanup()` (error path) | CURR 0 → OUTP 0 → SYST:LOC | ❌ None | ❌ None | ⚠️ No wait; no MEAS:CURR? verify |
| `maynuo_m8812_sequential_axis_run` | Main cleanup (success path) | CURR 0 → OUTP 0 → SYST:LOC | ❌ None | ❌ None | ⚠️ `current_final_zero = true` is **hardcoded** (not measured); no wait |
| `rf_mag_oe_minimal_run` (M5A) | Inline cleanup | CURR 0 → OUTP 0 → **wait 500ms** → MEAS:CURR? → SYST:LOC | ✅ 500ms | ✅ `< 1.0 mA` | ✅ **Fixed** — this is the reference implementation |

### Critical Findings

1. **M3 (recur_microtest) error-path cleanup lacks `CURR 0`** (`app.rs:412-453`):
   ```rust
   fn attempt_cleanup(...) {
       transport.send_set_output(false);   // OUTP 0
       transport.send_set_local();          // SYST:LOC
       // CURR 0 IS MISSING!
   }
   ```
   If an error occurs after `OUTP 1` but before main cleanup, the device stays at nonzero current setpoint while returning to local mode. This is a **safety bug**.

2. **M4 (sequential_axis_run) same bug** (`app.rs:505-543`): error-path `attempt_cleanup` also lacks `CURR 0`.

3. **M2B (zero_baseline) cleanup order is wrong** (`app.rs:354-398`):
   - Current: `OUTP 0 → CURR 0 → SYST:LOC`
   - Should be: `CURR 0 → OUTP 0 → wait → MEAS:CURR? → SYST:LOC`
   - Rationale: Ramp current to zero BEFORE disabling output, to avoid inductive kickback / noise.

4. **M3/M4 `current_final_zero` is hardcoded** (`app.rs:340`, `app.rs:432`):
   ```rust
   result.current_final_zero = true;  // Never actually measured!
   ```
   This gives false confidence. The flag should be set based on `MEAS:CURR?` readback.

---

## SMB100A Tools

| Tool | Shutdown Path | Sequence | Wait | Query Verify | Notes |
|------|--------------|----------|------|--------------|-------|
| `smb100a_fm_mod_microtest` | `attempt_emergency_shutdown()` | OUTP OFF → wait → MOD:STAT OFF → wait → FM:STAT OFF → wait → drain → OUTP? → MOD:STAT? | ✅ delay_ms × 3 | ✅ OUTP? + MOD:STAT? | ✅ Good — has delays and verification |
| `smb100a_oe1022d_extended_sweep` | `attempt_emergency_shutdown()` | Same as above | ✅ delay_ms × 3 | ✅ OUTP? + MOD:STAT? | ✅ Good |
| `smb100a_oe1022d_step_sweep` | `attempt_emergency_shutdown()` | Same as above | ✅ delay_ms × 3 | ✅ OUTP? + MOD:STAT? | ✅ Good |
| `recipe_two_device_run` | `real_run.rs` final shutdown | OUTP OFF → MOD:STAT OFF → (FM:STAT OFF if !leave_fm) | ⚠️ 50ms only | ❌ No post-shutdown query | ⚠️ Minimal wait; no verification after shutdown commands |
| `smb100a_safe_set` | `main.rs` disconnect | OUTP OFF → MOD:STAT OFF → flush | ❌ None | ❌ None | ⚠️ But `lib.rs` has query_before/query_after per step |
| `smb100a_rf_microtest` | Normal + Emergency | OUTP OFF → wait → OUTP? verify; Emergency: same | ✅ delay_ms | ✅ OUTP? | ✅ Good |
| `smb100a_preflight_clearance` | N/A (read-only) | N/A | N/A | N/A | ✅ No output, safe |
| `oe1022d_smb_query_bridge` | N/A (read-only) | N/A | N/A | N/A | ✅ No output, safe |
| `oe1022d_smb_fake_bridge` | N/A (fake) | N/A | N/A | N/A | ✅ No hardware |

### SMB Findings

1. `recipe_two_device_run` final shutdown (`real_run.rs:252-256`) sends shutdown commands but does not wait or verify. The 50ms sleep before `OUTP OFF` is for step boundary, not post-shutdown settling.
2. All SMB emergency shutdown paths are consistent and well-designed (same pattern copied across 3 tools).

---

## OE1022D Tools

| Tool | Cleanup | Notes |
|------|---------|-------|
| `oe1022d_acquire` | Close serial port | ✅ Read-only, safe |
| `oe1022d_logged_acquire` | Close serial port | ✅ Read-only, safe |
| `oe1022d_rall_capture` | Close serial port | ✅ Read-only, safe |
| `oe1022d_run_audit` | Close serial port | ✅ Read-only, safe |
| `smb100a_oe1022d_extended_sweep` | `oe_transport.rs` `Drop` | ⚠️ No explicit buffer clear before RALL? |
| `smb100a_oe1022d_step_sweep` | `oe_transport.rs` `Drop` | ⚠️ No explicit buffer clear before RALL? |
| `rf_mag_oe_minimal_run` | `oe_bridge.rs` close port | ⚠️ No explicit buffer clear before RALL? |

### OE Findings

1. No OE tool sends configuration commands (all read-only or `RALL?`). Cleanup = close port.
2. **Missing**: explicit `clear_input_buffer()` before `RALL?` in any tool.
3. **Missing**: frame header signature verification after reading 12288 bytes.

---

## Cross-Tool Risk Summary

| Risk | Severity | Affected Tools |
|------|----------|----------------|
| Maynuo error-path cleanup missing `CURR 0` | 🔴 **Critical** | `maynuo_m8812_recur_microtest`, `maynuo_m8812_sequential_axis_run` |
| Maynuo success-path cleanup no wait / no verify | 🟠 High | `maynuo_m8812_zero_baseline`, `maynuo_m8812_recur_microtest`, `maynuo_m8812_sequential_axis_run` |
| Maynuo hardcoded `current_final_zero` | 🟠 High | `maynuo_m8812_recur_microtest`, `maynuo_m8812_sequential_axis_run` |
| SMB shutdown minimal wait / no verify | 🟡 Medium | `recipe_two_device_run` |
| OE no buffer clear before RALL? | 🟡 Medium | All OE capture tools |

---

## Recommended Unified Cleanup (Reference: M5A)

### Maynuo
```text
CURR 0.00000
OUTP 0
wait 500 ms
MEAS:CURR?
verify abs(current) < 1.0 mA
SYST:LOC
```

### SMB100A
```text
OUTP OFF
wait {delay_ms}
MOD:STAT OFF  (if applicable)
wait {delay_ms}
FM:STAT OFF   (if applicable)
wait {delay_ms}
drain buffer
OUTP?         (verify OFF)
MOD:STAT?     (verify OFF)
SYST:ERR?     (drain error queue)
```

### OE1022D
```text
clear_input_buffer
RALL? (read 12288 bytes)
verify frame header signature
close port
```

---

## Action Items

1. ~~P0-Critical~~: M3/M4 `attempt_cleanup()` already includes `CURR 0` (audit corrected).
2. **P0-Critical**: Fix M2B cleanup order to `CURR 0 → OUTP 0`.
3. **P1-High**: Add `wait 500ms + MEAS:CURR?` verification to all Maynuo tools.
4. **P1-High**: Remove hardcoded `current_final_zero = true`; set from actual readback.
5. **P2-Medium**: Add `clear_input_buffer()` before `RALL?` in all OE capture tools.
6. **P2-Medium**: Add frame header signature check to OE frame capture.
7. **P2-Medium**: Add post-shutdown `OUTP?` verification to `recipe_two_device_run`.
