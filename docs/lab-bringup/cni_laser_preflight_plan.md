# CNI Laser Preflight Plan

**Device**: CNI Laser (DPSS/ Semiconductor Laser)
**Used for**: ODMR optical excitation
**Interface**: RS232, 9600 baud, 8N1, no parity
**Protocol**: Binary frame with checksum (NOT SCPI)
**Date**: 2026-06-04
**Status**: Not yet integrated into real run flow

---

## Safety Assessment

The laser is **the most dangerous device** in the station. Unlike RF (-30 dBm = 1 µW) and magnetic (10 mA, ~1.4 µT), the laser emits:
- Optical power up to ~150–306 mW (existing docs conflict — use **conservative 150 mW upper bound**)
- Invisible wavelength (likely 532 nm or near-IR depending on model)
- Can cause permanent eye damage and skin burns

**Software alone cannot make the laser safe.** Physical safeguards are mandatory:
- Key switch (removes power when off)
- Interlock loop (shuts down if opened)
- Emergency stop button
- Shutter (blocks beam without powering down)
- Warning indicator lights
- Warm-up time required after cold start

---

## M0: Manual Safety Checklist (Before Any Software)

Before any agent/tool touches the laser:

```text
□ Key switch is in OFF position
□ Emergency stop is released (not pressed)
□ Interlock loop is closed (all doors/panels secured)
□ Shutter is CLOSED
□ Warning light is OFF
□ All personnel in laser safety glasses zone
□ Beam path is clear of reflective surfaces
□ Fire extinguisher accessible
```

**Rule**: If ANY checkbox is not ticked, abort. No software override.

---

## M1: Fake Driver + Protocol Tests (No Real Light)

### Goals
1. Understand binary frame protocol
2. Implement command builder with checksum
3. Validate against known-good frame dumps

### Protocol Sketch (From Existing Docs)
- Baud: 9600, 8 data bits, 1 stop bit, no parity
- Frame structure: `[Header] [Command] [Data] [Checksum]`
- Commands:
  - `Laser Off`: fixed frame
  - `Laser On`: fixed frame
  - `Set Power`: frame with power level parameter
  - `Read Status`: query frame

### Fake Implementation
```rust
struct CniLaserFake {
    power_setpoint_mw: u16,
    output_enabled: bool,
    shutter_closed: bool,
    interlock_ok: bool,
}
```

### Tests
- Command frame serialization roundtrip
- Checksum validation (correct vs incorrect)
- Response parsing
- Power limit enforcement (>150 mW rejected)

---

## M2: Real Off-Only Preflight

### Goals
1. Enumerate serial port
2. Identify laser by protocol handshake (if any)
3. Send `Laser Off` command
4. Verify status indicates OFF
5. **Never send `Laser On` or power setpoint**

### Allowed Commands in M2
| Command | Purpose |
|---------|---------|
| Identify/Handshake | Confirm device is laser |
| Laser Off | Ensure output is disabled |
| Read Status | Verify OFF state |

### Forbidden in M2
- `Laser On`
- `Set Power`
- Any command that could open shutter

### Operator Approval Gate
M2 requires **independent** `--operator-approve` with explicit text:
> "I confirm the laser key switch is OFF, interlock is closed, shutter is closed, and all personnel are wearing laser safety glasses."

---

## M3: Low-Power Enable Microtest

### Preconditions (ALL must be true)
1. M0 checklist completed and signed
2. M2 preflight passed (laser confirmed OFF)
3. Physical shutter can be manually opened/closed
4. Power meter is available and calibrated
5. Beam dump is in place

### Goals
1. Set power to **≤ 5 mW** (conservative, eye-safe class 3R limit)
2. Enable output for **≤ 5 seconds**
3. Verify power meter reading matches setpoint ±10%
4. Disable output immediately

### Software Limits (Hardcoded)
```rust
const MAX_POWER_MW: u16 = 5;      // M3 limit
const MAX_DURATION_MS: u64 = 5000; // M3 limit
```

### Sequence
```text
1. Operator approval (second gate, laser-specific)
2. Set power 5 mW
3. Open shutter (manual or software, depending on model)
4. Wait 1 s (warm-up)
5. Enable output
6. Wait 5 s max
7. Disable output
8. Close shutter
9. Verify OFF via status query
```

### Abort Conditions
- Power meter reading > 6 mW → immediate abort
- Any interlock open → immediate abort
- Emergency stop pressed → immediate abort
- Timeout > 5 s → immediate abort

---

## Power Limit Conservatism

Existing documentation contains conflicting numbers:
- One source: 0–150 mW
- Another source: ~306 mW

**Agent decision**: Use **150 mW** as absolute software maximum until a human experimenter explicitly confirms the higher number with device label photo and power meter calibration certificate.

Never let software auto-escalate power limits based on "maybe it's 306 mW".

---

## Integration Timeline

| Phase | Depends On | Deliverable |
|-------|-----------|-------------|
| M0 | Human procedure | `docs/lab-bringup/laser_m0_manual_checklist.md` |
| M1 | Protocol docs | `tools/lab/cni_laser_fake_driver/` + tests |
| M2 | M1 + serial port | `tools/lab/cni_laser_off_only_preflight/` |
| M3 | M2 + power meter | `tools/lab/cni_laser_microtest/` |
| M4+ | M3 + GUI safety | Integration into recipe executor |

**Do NOT integrate laser into M5A or any combined run until M3 is complete and validated.**

---

## Related Documents

- [Device Connection Initialization Audit](device_connection_initialization_audit.md)
- Station schema (laser section)
