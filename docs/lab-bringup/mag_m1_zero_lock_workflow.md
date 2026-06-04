# Mag-M1: Magnetic Axis Operational State Machine + Zero-Lock Workflow

## Milestone Definition

Mag-M1 is a **mock-only operational state machine** for the Maynuo M8812 magnetic axes. It builds on Mag-M0.5/M0.6 types (profiles, commands, plans) and adds:

1. Deterministic state transitions per axis
2. SN-based discovery and axis mapping
3. Zero-current measurement and lock-zero workflow
4. Recurrent field/current setpoint planning
5. Readback reconstruction from measured total current
6. Normal and emergency shutdown paths

Mag-M1 is still **mock-only**. No serial port is opened, no SCPI command is sent, and no Tauri hardware command is enabled. All operations are pure computation producing structured events and command plan artifacts.

## Source Materials

| Source | Purpose |
|--------|---------|
| `reverse_application/reverse_output/FormMain.cs` | LockZero / zeroSetCurr / recurSetCurr / recurSetMag logic |
| `reverse_application/reverse_output/verify_protocol.py` | Verified SCPI sequences |
| `crates/odmr-mag/src/lib.rs` (M0.5/M0.6) | Maynuo types, command plans, profiles |
| `memory/device-fingerprint-mapping.md` | Verified SN-to-axis mapping |

## Axis State Machine

### States

```
Unknown
  → Discovered { idn }
    → AxisMapped { axis_id, idn }
      → InitializedOutputOff { axis_id, idn }
        → OutputOnZeroMode { axis_id, idn }
          → ZeroMeasured { axis_id, idn, zero_current_ma }
            → ZeroLocked { axis_id, idn, zero_current_ma }
              → RecurSetpointPlanned { axis_id, idn, zero, recur_ma, recur_nt }
                → RecurSetpointAppliedMock { axis_id, idn, zero, recur_ma, recur_nt, total }
Any state → ShutdownNormal { axis_id }
Any state → ShutdownEmergency { axis_id }
Any state → Fault { axis_id, reason }
```

### Transition Validation

| Transition | Prerequisite | Error on Violation |
|-----------|-------------|-------------------|
| Discover | Must be Unknown | `InvalidStateTransition` |
| Map to axis | Must be Discovered | `InvalidStateTransition` |
| Safe init | Must be AxisMapped | `InvalidStateTransition` |
| Output on | Must be InitializedOutputOff | `OutputBeforeInit` |
| Measure zero | Must be OutputOnZeroMode | `InvalidStateTransition` |
| Lock zero | Must be ZeroMeasured or ZeroLocked | `LockZeroBeforeMeasurement` |
| Plan recur field | Must be ZeroLocked/RecurSetpoint* | `RecurBeforeLockZero` |
| Apply setpoint | Must be RecurSetpointPlanned | `InvalidStateTransition` |
| Shutdown | Any state | (always valid) |
| Emergency stop | Any state | (always valid) |

SN validation is enforced at discovery time: only known `expected_idn` strings are accepted. Duplicate SNs and missing axes are rejected.

## Discovery by IDN

### SN-to-Axis Binding

| Axis | SN | Coil Constant (nT/mA) |
|------|-----|---------------------|
| X | `080020960220402020` | 143.26 |
| Y | `080020960220402022` | 141.77 |
| Z | `080020960220402003` | 156.15 |

### Matching Logic

```rust
match_axes_by_idn(profile, observed_idns) -> Result<BTreeMap<axis_id, idn>, MagError>
```

1. For each observed IDN, extract the SN (third comma-separated field)
2. Find which axis profile contains that SN in its `expected_idn`
3. Reject if: unknown SN, duplicate SN, missing axis
4. Return map of axis_id → full IDN string for identity verification

**`last_known_port_name` is a hint only.** Port paths (`/dev/cu.PL2303G-*`, `COM3`/`COM4`/`COM6`) are dynamically assigned per session and must not be used as binding keys.

## Zero-Lock Workflow

### State Model (from FormMain.cs)

```
zeroSetCurr:   per-axis zero-field bias current [mA]
recurSetCurr:  per-axis recurrence current [mA]
recurSetMag:   per-axis recurrence magnetic field [nT]
coilConstant:  per-axis gain [nT/mA]
LockZero:      bool (per-axis toggle)
Output:        bool (per-axis toggle)
```

### Mode A: LockZero = OFF

- **Output current** = `zeroSetCurr`
- `recurSetCurr` and `recurSetMag` are display/calculation only
- When `zeroSetCurr` changes and output is ON → send new CURR value

### Mode B: LockZero = ON + Output = ON

- **Output current** = `zeroSetCurr + recurSetCurr`
- When `recurSetCurr` changes → recalculate total, send CURR
- When `recurSetMag` changes → `recurSetCurr = recurSetMag / coilConstant`, recalculate total, send CURR
- When `zeroSetCurr` changes → recalculate total, send CURR

### Workflow Steps (Mag-M1 mock)

| Step | State Transition | Action (in real execution) | Mag-M1 |
|------|------------------|---------------------------|--------|
| 1 | Unknown → Discovered | `*IDN?` → verify SN | `MaynuoAxisRunner::apply_discovered(idn)` |
| 2 | Discovered → AxisMapped | Match SN to axis | `apply_axis_mapped()` |
| 3 | AxisMapped → InitializedOutputOff | `SYST:REM`, `VOLT 75`, `CURR 0`, `OUTP 0` | `apply_initialized_output_off()` |
| 4 | InitializedOutputOff → OutputOnZeroMode | `OUTP 1` at zero current | `apply_output_on_zero_mode()` |
| 5 | OutputOnZeroMode → ZeroMeasured | `MEAS:CURR?` → capture zero | `apply_zero_measured(ma)` |
| 6 | ZeroMeasured → ZeroLocked | Lock-zero enabled (state only) | `apply_lock_zero()` |
| 7 | ZeroLocked → RecurSetpointPlanned | Compute `recur_ma = field_nt / coil_constant` | `apply_recur_setpoint_planned_from_field(nt)` |
| 8 | RecurSetpointPlanned → RecurSetpointAppliedMock | `CURR {total_a:.5}` (dry-run) | `apply_recur_setpoint_applied_mock()` |

### Current Computation

```
recur_current_ma = target_field_nt / coil_constant_nt_per_ma
total_current_ma = zero_current_ma + recur_current_ma
current_command = CURR {total_current_ma / 1000.0:.5}
```

### Validation

- Total current must be ≥ 0 (M8812 positive-only)
- Total current must be ≤ `max_current_ma` (5000 mA)
- Field and current must be finite
- Lock-zero must be enabled before planning recurrent field

## Readback Model

### Reconstruction from Measured Total

```
If lock_zero = true:
    recur_current_ma = max(0, measured_total_ma - zero_current_ma)
    recur_field_nt = recur_current_ma * coil_constant_nt_per_ma

If lock_zero = false:
    zero_current_ma = measured_total_ma  (capture new zero)
    recur_current_ma = 0
```

Field reconstruction accuracy is limited by:
- Current measurement resolution (±0.01 mA on M8812 display / ±0.001 mA in SCPI)
- Coil constant uncertainty (para.xml values, not individually calibrated per coil)

### API

```rust
runner.readback(measured_total_ma: f64) -> Result<(recur_ma, recur_nt), MagError>
runner.readback_recur_current_ma(measured_total_ma: f64) -> Result<f64, MagError>
runner.readback_recur_field_nt(recur_current_ma: f64) -> f64
runner.current_command_preview() -> Option<String>
```

## Command Plans (Mag-M1)

### New Plan Builders

| Builder | Purpose | Commands |
|---------|---------|----------|
| `build_output_on_zero_mode_plan` | Turn output on at zero current | `OUTP 1` |
| `build_measure_zero_current_plan` | Capture zero baseline | `MEAS:CURR?` |
| `build_lock_zero_event` | Lock-zero state transition (pure event) | (none) |
| `build_recur_field_setpoint_plan` | Set recur field via target nT | `CURR {total_a:.5}` |
| `build_recur_current_setpoint_plan` | Set recur current directly | `CURR {total_a:.5}` |
| `build_readback_recur_state_plan` | Query current for reconstruction | `MEAS:CURR?` |

All plans have `executable: false` with explanatory `executable_reason` strings.

## Normal vs Emergency Shutdown

| Mode | Sequence | Use Case | Builder |
|------|----------|----------|---------|
| Verified Normal | `CURR 0` → `OUTP 0` → `SYST:LOC` | Normal disconnect | `build_verified_normal_shutdown_plan` |
| Emergency | `OUTP 0` → `CURR 0` → `SYST:LOC` | Fast output kill | `build_emergency_shutdown_plan` |

In the state machine, `apply_shutdown_normal()` and `apply_shutdown_emergency()` are valid from any state. Emergency sets output=false with zero delay.

## GUI Relationship

### Contract

The GUI receives display-only payloads (`ZeroLockAxisGuiPayload`):

```json
{
  "axis_id": "mag_x",
  "expected_idn": "MAYNUO,M8812,080020960220402020,V2.7",
  "state": "ZeroLocked",
  "output_state": "off",
  "zero_current_ma": 0.15,
  "lock_zero": true,
  "target_field_nt": null,
  "total_current_ma": null,
  "current_command_preview": null,
  "allowed_next_actions": ["plan_recur_field", "shutdown_normal", "shutdown_emergency"],
  "disabled_actions_with_reasons": {
    "map_to_axis": "Must be Discovered first",
    "safe_init": "Must be AxisMapped first"
  }
}
```

### What React CAN do (M1 mock):
- Display axis cards with state, identity, calibration
- Show zero/recur/total current values
- Show command preview strings
- Show allowed/disabled action lists
- Navigate through mock workflow steps (mock state transitions)

### What React CANNOT do (M1 mock):
- Open serial ports
- Send SCPI commands
- Enable output (real)
- Read measured current from hardware
- Anything requiring `serialport`, `rusb`, or TCP

### Example File

`examples/magnetic/maynuo_m8812_zero_lock_gui_contract.example.json` — full three-axis display contract with example workflow walkthrough.

## What Remains Mock-Only

- `MaynuoAxisRunner` is a pure state machine — no I/O
- All plan builders return `MaynuoCommandPlan` with `executable: false`
- `ZeroLockAxisGuiPayload::from_runner()` builds display data from mock state
- No `serialport` dependency
- `cargo tree -p odmr-mag` confirms only `serde` + `serde_json`

## What Is Postponed to Mag-M2/M3

| Feature | Postponed To | Reason |
|---------|--------------|--------|
| Real serial port enumeration | Mag-M2A | Requires `serialport` crate + device registry |
| Real `*IDN?` query and SN verification | Mag-M2A | Requires hardware connection |
| Real `CURR` / `OUTP` / `MEAS` dispatch | Mag-M2B/M3 | Requires executor + safety gate |
| Real current readback with timestamp | Mag-M3 | Requires active output |
| GUI enable real controls | Mag-M3 | Requires full backend + safety chain |
| LockZero auto-recalculation on zero change | Mag-M3 | Requires real output to be meaningful |
| Cross-axis coupling | Mag-M4+ | Requires field probe measurement |

## Compliance with ADR-008

Mag-M1 does not weaken ADR-008:
- `cargo tree -p odmr-mag` shows no `serialport`, `rusb`, `hidapi`, or `visa-rs`
- `grep` for hardware keywords returns only documentation references
- `MaynuoAxisRunner` produces events and state — no I/O
- All plans are `executable: false`
- The only test current is 10 mA (micro-test) or mock zero (0 mA)

## Types Added in Mag-M1

| Type | Purpose |
|------|---------|
| `MaynuoAxisState` | Enum of 12 operational states |
| `MaynuoAxisStateEvent` | Timeline event record |
| `MaynuoAxisRunner` | In-memory state machine runner |
| `ZeroLockAxisGuiPayload` | Display-only GUI payload |
| `match_axes_by_idn()` | SN-based discovery matching |
| `extract_sn_from_idn()` | Parse SN from *IDN? response (replaced by `parse_maynuo_idn()` in M1.1) |
| `build_output_on_zero_mode_plan()` | Output-on plan |
| `build_measure_zero_current_plan()` | Zero measurement plan |
| `build_lock_zero_event()` | Lock-zero event |
| `build_recur_field_setpoint_plan()` | Field setpoint plan |
| `build_recur_current_setpoint_plan()` | Current setpoint plan |
| `build_readback_recur_state_plan()` | Readback query plan |

## Error Variants Added in Mag-M1

| Variant | When |
|---------|------|
| `InvalidStateTransition` | State machine transition rejected |
| `UnknownSerialNumber` | IDN SN not in any axis profile |
| `DuplicateSerialNumber` | Same SN matched twice |
| `AxisNotDiscovered` | Required axis missing from scan |
| `LockZeroBeforeMeasurement` | Lock-zero before zero measured |
| `RecurBeforeLockZero` | Recur setpoint before lock-zero |
| `TotalCurrentOverLimit` | Zero + recur exceeds hardware max |
| `OutputBeforeInit` | Output on before safe init |

## Test Coverage

138 tests in `odmr-mag`:
- SN matching (complete, unknown, duplicate, missing)
- State machine transitions (valid paths and invalid)
- Zero measurement and lock-zero workflow
- Field-to-current on X/Y/Z with real coil constants
- Total current = zero + recur
- Command preview formatting
- Readback reconstruction
- Limit enforcement (total current, non-finite values)
- Serialization round-trips (state, runner)
- All plans not executable
- Existing M0/M0.5/M0.6 tests preserved

## Mag-M1.1 Hardening Patch

A narrow correctness patch applied before Mag-M2A hardware discovery.

### Fixes Applied

| Fix | Detail |
|-----|--------|
| **Strict IDN parser** | Added `MaynuoIdn` struct + `parse_maynuo_idn()`. Validates manufacturer, model, non-empty SN. Rejects malformed responses with `MagError::MalformedIdn` instead of silently matching empty strings. |
| **Exact SN matching** | `match_axes_by_idn()` now uses exact `serial_number` equality via `expected_sn_from_idn()`. No substring `contains()` — empty or partial SNs cannot match. |
| **Event timeline deduplication** | Removed `emit()` helper. Each state transition now creates exactly 1 `MaynuoAxisStateEvent` with `from_state` = old state, `to_state` = new state. No more duplicate events or `from == to` artifacts. |
| **Readback sign preservation** | `readback_recur_current_ma()` no longer clamps negative recur with `.max(0.0)`. When `measured_total < zero` with `lock_zero = true`, recur is negative — preserving directional field information. |
| **Verified normal shutdown consistency** | `build_10ma_microtest_plan()` now uses `CURR 0 → OUTP 0 → SYST:LOC` (verified normal). Previously used emergency order (`OUTP 0 → CURR 0 → SYST:LOC`) despite claiming `shutdown_mode = verified_normal`. |

### Types Added in M1.1

| Type | Purpose |
|------|---------|
| `MaynuoIdn` | Structured parsed *IDN? response: manufacturer, model, serial_number, firmware |
| `parse_maynuo_idn()` | Strict IDN parser returning `Result<MaynuoIdn, MagError>` |
| `expected_sn_from_idn()` | Helper to extract SN from expected_idn strings |

### Error Variant Added in M1.1

| Variant | When |
|---------|------|
| `MalformedIdn { idn, reason }` | IDN string fails structural or semantic validation |
