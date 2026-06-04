# cni-laser-microtest

CNI Laser Low-Power Microtest (M3).

## Purpose

Validate safe laser enable/disable sequence with conservative limits:
- **Power**: ≤ 5 mW (hardcoded)
- **Duration**: ≤ 5 seconds (hardcoded)
- **Operator approval**: mandatory

## Prerequisites

ALL of the following must be true before running:

1. M0 manual checklist completed (key OFF, interlock closed, shutter closed, safety glasses on)
2. M2 preflight passed (`common-preflight` with laser device)
3. Physical shutter can be manually opened/closed
4. Power meter is available and calibrated
5. Beam dump is in place

## Usage

```bash
cargo run -- --operator-approve
```

Optional flags:
```bash
cargo run -- --operator-approve --power-mw 3 --duration-ms 3000
```

## Safety Design

- `MAX_POWER_MW = 5` and `MAX_DURATION_MS = 5000` are compile-time constants
- `--operator-approve` is **required**; without it the tool exits immediately
- `SafeLaserPort` Drop guard sends `laser_off` on **any** exit (normal, panic, or Ctrl+C)
- Manual shutter operations are prompted inline; software never opens/closes the physical shutter

## Sequence

```
1. Send laser_off (redundant safety)
2. Set power to ≤ 5 mW
3. [MANUAL] Open shutter, verify 0 mW on power meter
4. Send laser_on → start timer
5. Hold for ≤ 5 s (monitor power meter)
6. Send laser_off
7. [MANUAL] Close shutter
8. Send redundant laser_off + verify echo
```

## Abort

Press **Ctrl+C** at any time. The Drop guard will immediately send `laser_off`.

## Port

Default: `/dev/cu.usbserial-FTE86EB2` (discovered CNI laser port).  
Override with `--port /dev/cu.OTHER`.
