# cni_laser_fake_driver

CNI Laser (PSU-SR series) binary frame protocol implementation and fake device.

## Purpose

M1 of the CNI laser preflight integration: implement the protocol without
connecting to real hardware. Used for:

1. Validating frame serialization / deserialization
2. Checksum verification
3. Power limit enforcement (150 mW conservative)
4. Safe-state logic (`laser_on` rejected when setpoint is 0)

## Protocol

- Baud: 9600, 8 data bits, 1 stop bit, no parity
- Binary frame: `[0x55] [0xAA] [Command] [Data...] [Checksum]`
- Checksum = sum(Command + all Data bytes) & 0xFF

### Commands

| Command | Bytes | Description |
|---------|-------|-------------|
| Set Power | `55 AA 05 01 HH LL CS` | Set power setpoint (mW) |
| Laser Off | `55 AA 03 00 03` | Disable output |
| Laser On | `55 AA 03 01 04` | Enable output |

Example: 100 mW → `55 AA 05 01 00 64 6A`

## Tests

```bash
cargo test
```

16 unit tests covering frame roundtrip, checksum validation, power clamping,
and fake device state transitions.

## Safety

- `MAX_POWER_MW = 150` (conservative device label limit)
- `FakeCniLaser::handle_frame` rejects `laser_on` when setpoint is 0
- No real serial I/O in this crate — purely in-memory simulation (M1)

## Integration

`common_preflight` depends on this crate for the `cni_laser_probe` module
(M2: off-only preflight over real serial).
