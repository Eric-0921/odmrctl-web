# Maynuo M8812 Lab Notes

**Device**: Maynuo M8812 Programmable DC Power Supply
**Used for**: Magnetic coil current control (X/Y/Z axes)
**Interface**: USB-to-Serial (CDC-ACM), SCPI-like protocol
**Date**: 2026-06-04
**Manual Reference**: [M8812 Remote Control Reference](../equipment_manual/maynuo_dc-power-supply/m8812_remote_control_reference.md)

---

## 1. Port Binding Rule

### Problem
macOS/Linux USB CDC-ACM device paths are **not stable**:
- Today: `/dev/cu.usbserial-FTE86EB2`
- Tomorrow (different hub port): `/dev/cu.usbserial-ABCD1234`

### Solution: SN-based Enumeration
Never hardcode port paths. Always:
1. Enumerate all serial ports (`serialport::available_ports()`)
2. Open each port at 9600 baud, 8N1
3. Send `*IDN?` + `\r\n`
4. Parse response: `MAYNUO,M8812,<SN>,<FW_VERSION>`
5. Match SN tail against profile JSON (e.g., tail `2020` → `mag_x`)

### Profile JSON Format
```json
{
  "axes": {
    "mag_x": {
      "expected_sn_tail": "2020",
      "coil_constant_nt_per_ma": 143.26,
      "max_current_ma": 2000
    }
  }
}
```

### Known SNs (Lab Station)
| Axis | Full SN | Tail |
|------|---------|------|
| mag_x | `080020960220402020` | `2020` |

---

## 2. Cleanup Standard Sequence

### Verified Safe Order

```text
1. SYST:REM              (enter remote mode)
2. CURR 0.00000          (set current setpoint to zero)
3. OUTP 0                (disable output)
4. wait 500 ms           (allow current to decay)
5. MEAS:CURR?            (read back actual current)
6. verify < 1.0 mA       (tolerance covers ~0.04 mA noise floor)
7. SYST:LOC              (return to local mode)
```

### Why This Order Matters

| Step | Rationale |
|------|-----------|
| CURR 0 **before** OUTP 0 | Ramp current setpoint down before disabling output. Avoids inductive kickback and ensures smooth decay. |
| OUTP 0 **before** MEAS:CURR? | Output must be OFF before verifying near-zero current. With output ON, MEAS:CURR? reads the old current (~10 mA) even after CURR 0. |
| wait 500 ms | Device needs time to decay from operating current to noise floor. 24 ms is insufficient (measured). |
| MEAS:CURR? **before** SYST:LOC | Once SYST:LOC is sent, device returns to local mode and may reject further queries. |
| Tolerance `< 1.0 mA` | Noise floor is ~0.04 mA (40 µA). 1.0 mA provides 25× margin while catching real failures. |

### Known Failure Signatures

| Symptom | Cause | Action |
|---------|-------|--------|
| MEAS:CURR? returns `0.04 mA` after SYST:LOC | Device in local mode, readback unreliable | Reorder: verify **before** SYST:LOC |
| MEAS:CURR? returns `~10 mA` | Verified before OUTP 0, or wait too short | Ensure OUTP 0 → wait 500ms → MEAS? |
| MEAS:CURR? returns `999.0` | Query failed (device in local mode or comm error) | Retry once, then flag as verify-failed |
| `attempt_cleanup` missing CURR 0 | Bug in M3/M4 error-path cleanup | Fix: add `send_set_current(0.0)` before `send_set_output(false)` |

---

## 3. Current Readback Characteristics

### Noise Floor
- When `CURR 0` + `OUTP 1` (output enabled, zero setpoint): `~0.04 mA`
- When `CURR 0` + `OUTP 0` (output disabled): `~0.00 mA` (sometimes `0.04 mA`)
- This is **ADC offset**, not real current

### Reconstruction Formula
```
zero_baseline_ma = mean(MEAS:CURR? samples at CURR 0 + OUTP 1)
recur_total_ma   = mean(MEAS:CURR? samples at CURR {setpoint} + OUTP 1)
recur_actual_ma  = recur_total_ma - zero_baseline_ma
field_nt         = recur_actual_ma × coil_constant_nt_per_ma
```

### Tolerance Table
| Check | Tolerance | Rationale |
|-------|-----------|-----------|
| Zero baseline | `< 1.0 mA` | Noise floor 0.04 mA |
| Recur setpoint | `±0.5 mA` | Device regulation accuracy |
| Cleanup verify | `< 1.0 mA` | Noise floor + safety margin |

---

## 4. Command Reference

| Command | Direction | Safety Relevant | Notes |
|---------|-----------|-----------------|-------|
| `*IDN?` | Query | No | Response: `MAYNUO,M8812,<SN>,<FW>` |
| `SYST:REM` | Set | No | Enter remote mode. Required before any set command. |
| `SYST:LOC` | Set | No | Return to local mode. **Send LAST** after all verification. |
| `SYST:ERR?` | Query | No | Read error queue: `0, 'No Error'` or `70, 'Invalid Command'` |
| `VOLT 75` | Set | No | Set voltage limit to 75V. Do once at startup. |
| `VOLT:PROT 75` | Set | Yes | Set over-voltage protection to 75V. Hardware safety ceiling. |
| `CURR <A>` | Set | Yes | Set current setpoint in Amperes. `0.01` = 10 mA. |
| `OUTP 1` | Set | Yes | Enable output. **Only after CURR 0 verified.** |
| `OUTP 0` | Set | Yes | Disable output. |
| `MEAS:CURR?` | Query | No | Read actual output current in Amperes. |
| `MEAS:VOLT?` | Query | No | Read actual output voltage in Volts. |
| `MEAS:DVM?` | Query | No | Read built-in 5½-digit voltmeter input. |

### Hardware Limit Corrections (from M8812 manual)

Previous code used **M8811 specs (0-5A)** as the hardware limit. Our actual device is **M8812**:

| Parameter | Old (M8811) | Correct (M8812) |
|-----------|-------------|-----------------|
| Max current | 5000 mA (5 A) | **2000 mA (2 A)** |
| Voltage range | 0-30 V | **0-75 V** |

All `max_current_ma` values in `odmr-mag` have been corrected to **2000**.

### Forbidden Commands
- Never send `CURR` > `max_current_ma / 1000.0` (software limit, now 2.0 A)
- Never send semicolon-separated compound commands
- Never send `*RST` (resets device to unknown state)

---

## 5. Troubleshooting Checklist

| Symptom | Check |
|---------|-------|
| Port not found | `ls /dev/cu.usbserial*` — device powered? USB cable connected? |
| IDN timeout | Wrong baud rate (must be 9600). Try power cycle. |
| SN mismatch | Check profile JSON expected tail vs actual SN. Device may have been swapped. |
| MEAS:CURR? erratic | Check output cable connections. Loose connection causes unstable readback. |
| Cleanup fails verify | Increase wait time to 1000ms. Check if device is in remote mode. |

---

## 6. LIST Mode Discovery (from M8812 manual)

### What We Found

The M8812 manual (Chapter 6.2.5) reveals a **LIST sequential list mode** not used in our original software:

- Pre-program up to **200 steps** into device memory
- Each step sets `VOLT`/`CURR`/`WIDTH` independently
- Execute modes: `CONT` (continuous), `STEP` (single-step), `LOOP`
- Trigger sources: `IMM` (panel), `EXT` (TTL), `BUS` (software)

### Why This Matters

Current M5B-B executor sends **~4 SCPI commands per magnetic point** (`CURR` → `OUTP 1` → `MEAS:CURR?` → `OUTP 0`).

For a 100-point scan:
- **Point-by-point**: ~400 serial round-trips
- **LIST mode**: ~100 write commands to program + 1 trigger (or auto `CONT`)

### Current Stance

**Not implemented in M5B-B**. Reasons:

1. Current scan sizes are small (9-100 points); communication overhead is acceptable
2. LIST mode adds significant complexity: pre-programming verification, interrupt handling (Pause/Stop/E-Stop), error recovery
3. Three-axis coordination (X/Y/Z are independent M8812s) requires synchronized triggers, which needs TTL hardware or tight software timing
4. Safety layer (`odmr-safety`) ramp-limit checks are designed for point-by-point current deltas

### Future Path

```
M5B-B (now)    : point-by-point SCPI, proven safe
M5B-C / Mag-M3 : executor on real hardware, still point-by-point
Mag-M4+        : optional LIST mode as explicit recipe flag
                 `magnetic.execution_mode: "list"`
```

If implemented, LIST should be:
- **Explicit opt-in** (not automatic threshold-based switching)
- Preceded by a **dry-run LIST verification** step that reads back `LIST:CURR? <n>` for every step
- Accompanied by an **abort path** (`OUTP 0` or `ABOR`) that works even during LIST execution

### Relevant Manual Section

See [M8812 Remote Control Reference](../equipment_manual/maynuo_dc-power-supply/m8812_remote_control_reference.md) §6 for full LIST command syntax.

---

## 7. Related Documents

- [M8812 Remote Control Reference](../equipment_manual/maynuo_dc-power-supply/m8812_remote_control_reference.md) — complete SCPI command set extracted from manual
- [Cleanup Audit Matrix](cleanup_audit_matrix.md) — cross-tool cleanup comparison
- [Device Connection Initialization Audit](device_connection_initialization_audit.md) — station-level preflight design
