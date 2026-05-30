# lab-snapshot — Real-Device Read-Only Snapshot Tool

## Purpose

Capture a frozen read-only state of connected ODMR lab hardware (SMB100A + OE1022D) for bring-up documentation and station profile validation.

## Usage

```bash
cd tools/lab/snapshot
cargo run -- \
  --smb100a-host 169.254.2.20 --smb100a-port 5025 \
  --oe1022d-port /dev/cu.usbmodem3361358734371 --oe1022d-baud 921600 \
  --out-dir ../../../docs/lab-bringup/
```

Skip one device if it is unreachable:

```bash
# Only SMB100A
cargo run -- --skip-oe1022d --smb100a-host 169.254.2.20 --smb100a-port 5025

# Only OE1022D
cargo run -- --skip-smb100a --oe1022d-port /dev/cu.usbmodem3361358734371 --oe1022d-baud 921600
```

## Rust Transport Notes from Real Hardware Verification

### SMB100A (TCP SCPI, Port 5025)

No special Rust settings are required beyond a plain `std::net::TcpStream`:

```rust
let mut stream = TcpStream::connect_timeout(addr, Duration::from_secs(2))?;
stream.set_read_timeout(Some(Duration::from_secs(2)))?;
stream.write_all(b"*IDN?\n")?;
stream.read(&mut buf)?;
```

Observed behavior:
- Connection may time out if the instrument is still booting or the APIPA link has not finished negotiation. **Retry after a few seconds**.
- Each query round-trip is ~1.5–4 ms on a direct RJ45 link.
- No keep-alive, DTR/RTS, or special socket options are needed.

### OE1022D (USB CDC Serial)

**Critical settings discovered during M2.1 bring-up:**

1. **Baud rate is 921600, not 115200**
   - PRD v0.2 incorrectly lists 115200.
   - At 115200 all queries time out; at 921600 10/11 queries succeed.

2. **Clear the serial input buffer before every query**
   - The device (or macOS CDC driver) continuously pads the RX line with `0x00` bytes after each response.
   - Without `clear(ClearBuffer::Input)`, stale null bytes contaminate the next read.

3. **Wait at least 500 ms after write before read**
   - The OE1022D DSP needs processing time. Reading too early yields no data.
   - `RALL?` returns a binary frame; allow **800 ms**.

4. **Terminator is `\r` (CR)**
   - `\n` (LF) alone is not accepted.

Minimal working Rust pattern:

```rust
let mut port = serialport::new(port_name, 921600)
    .timeout(Duration::from_secs(2))
    .open()?;

// 1. Clear stale data
port.clear(serialport::ClearBuffer::Input)?;

// 2. Send with CR terminator
port.write_all(b"*IDN?\r")?;
port.flush()?;

// 3. Wait for DSP processing
std::thread::sleep(Duration::from_millis(500));

// 4. Read response
let mut buf = [0u8; 4096];
let n = port.read(&mut buf)?;
```

Observed behavior:
- `*IDN?`, `FMODD? 2`, `RSLPD? 2`, `FREQD? 2`, `PHASD? 2`, `ISRCD? 2`, `SENSD? 2`, `OFLTD? 2`, `OFSLD? 2` — ASCII text response, ~1–12 bytes of payload.
- `HARMD? 2` — returns empty (device may not support this query in current firmware mode).
- `RALL?` — **binary frame**, ~1020 bytes. Must be recorded as raw bytes or hex dump, not UTF-8 text.

## Safety

This tool only sends pre-defined read-only queries. There is no generic `send(cmd)` API.
Forbidden patterns (`OUTP ON`, `MOD:STAT ON`, `*RST`, etc.) are hard-blocked before any transport is opened.
