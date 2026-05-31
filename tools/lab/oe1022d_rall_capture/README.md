# OE1022D RALL? Raw Capture and Parser Probe

Captures raw binary `RALL?` frames from an OE1022D lock-in amplifier over serial,
then runs an offline parser probe to inspect candidate encodings.

## Safety Model

This tool sends **only two commands**:

- `*IDN?` — once, for identity verification
- `RALL?` — repeated, for frame capture

All other commands are rejected. No OE1022D settings are changed.

## Usage

```bash
cd tools/lab/oe1022d_rall_capture

# Quick test: 10 frames
cargo run -- --frames 10 --delay-ms 20

# Full capture: 100 frames
cargo run -- --frames 100 --delay-ms 20
```

## CLI Options

```text
--port       Serial port path (default: /dev/cu.usbmodem3361358734371)
--baud       Baud rate (default: 921600)
--frames     Number of frames to capture, <= 1000 (default: 10)
--delay-ms   Delay between frames in ms, >= 0 (default: 20)
--timeout-ms Serial read timeout in ms, <= 5000 (default: 2000)
--out-dir    Output directory (default: docs/lab-bringup)
```

## Output Files

```text
docs/lab-bringup/rall_capture_YYYY-MM-DD/
  capture_report.md           # Human-readable capture report
  rall_frames.rawbin          # Length-prefixed raw frames
  rall_index.jsonl            # Per-frame metadata
  observed_commands.jsonl     # *IDN? response record
  parser_probe_summary.json   # Structured parser probe results
  parser_probe_summary.md     # Human-readable parser probe report
```

## Parser Probe Candidates

For each captured frame, the tool probes:

1. `be_f64` — big-endian IEEE-754 double
2. `le_f64` — little-endian IEEE-754 double
3. `be_f32` — big-endian IEEE-754 single
4. `le_f32` — little-endian IEEE-754 single

## Tests

```bash
cargo test
```

## Transport Notes

- OE1022D serial: USB CDC, 921600 baud, 8N1
- Command terminator: `\r` (CR)
- Must clear input buffer before each query
- `RALL?` requires ~800ms wait before reading response
- Response is binary, not ASCII
