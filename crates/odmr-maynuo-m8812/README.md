# odmr-maynuo-m8812

**Layer**: 1 (Drivers)  
**License**: MIT OR Apache-2.0

Serial transport driver for the Maynuo M8812 programmable DC power supply, used
to control magnetic coil currents in the ODMR setup.

## Responsibilities

- Open USB-to-serial (CDC-ACM) ports at 9600 baud, 8N1
- Send SCPI-like commands (`*IDN?`, `SYST:REM`, `CURR`, `OUTP`, `MEAS:CURR?`, `SYST:LOC`)
- Parse responses and handle basic error recovery
- Provide `MaynuoM8812Transport` for use by higher-level tools

## Key Dependencies

- `odmr-types` (Layer 0) — `DeviceId`, error types
- `odmr-device` (Layer 1) — `Device` trait (optional, for future integration)
- `serialport` — cross-platform serial port I/O

## What This Crate Does NOT Do

- Does NOT implement coil constant calculation or field reconstruction
- Does NOT manage multi-axis sequencing (see `odmr-mag`)
- Does NOT perform safety limit enforcement (see `odmr-safety`)
- Does NOT write artifacts or logging (see `odmr-logging`)

## Usage

```rust
use odmr_maynuo_m8812::MaynuoM8812Transport;

let mut transport = MaynuoM8812Transport::open(
    DeviceId::new("maynuo_x"),
    "/dev/cu.usbserial-FTE86EB2",
    MaynuoSerialPortConfig::default(),
)?;

let idn = transport.query_idn()?;
transport.send_set_remote()?;
transport.send_set_current(0.01)?; // 10 mA
transport.send_set_output(true)?;
let current_a = transport.query_meas_current()?;
```

## Safety Notes

- Always send `CURR 0` before `OUTP 0` during cleanup
- Always verify `MEAS:CURR?` reads < 1.0 mA before `SYST:LOC`
- The device reports ~0.04 mA noise floor even at zero current setpoint
