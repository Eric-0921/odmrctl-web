# Mag-M2A+: Maynuo M8812 Identity-Only Hardware Discovery

## Milestone Definition

Mag-M2A+ is the **first real-hardware milestone** for the magnetic axis line.
It introduces:

1. A new real-driver crate: `crates/odmr-maynuo-m8812`
2. A lab bring-up tool: `tools/lab/maynuo_m8812_identity_probe`
3. Real serial-port enumeration and `*IDN?` probing
4. Exact SN-based axis mapping
5. Structured artifact output for auditability

Mag-M2A+ is **identity-only**.  No current or output commands are sent.

## Architecture

```
odmr-mag (mock-only, no serialport)
    └── types, MaynuoIdn parser, SN extraction
         │
odmr-maynuo-m8812 (real serial)
    └── serialport enumeration, port open, *IDN? query
         │
maynuo_m8812_identity_probe (lab tool)
    └── CLI, probe loop, classification, matching, artifact output
```

## Workflow

1. Parse CLI args (profile path, timeout, baudrate, filters, dry-run, strict)
2. Load `MaynuoAxesProfile` from JSON
3. Enumerate serial ports, apply include/exclude/max-ports filters
4. For each candidate (non-dry-run):
   a. Open port with 9600/8/N/1, DTR=true
   b. Send `*IDN?`
   c. Read response line
   d. Parse via `odmr-mag::parse_maynuo_idn()`
   e. Classify: matched_axis / unknown_maynuo_sn / malformed_idn / timeout / io_error
5. Match results to logical axes by exact SN equality
6. Write artifact files

## Allowed Command

| SCPI | Purpose |
|------|---------|
| `*IDN?` | Device identity query |

**Only `*IDN?` is permitted.**  The transport layer rejects all other commands.

## Forbidden Commands

| SCPI | Reason |
|------|--------|
| `SYST:REM` | Changes device to remote mode |
| `SYST:LOC` | Changes device to local mode |
| `VOLT <v>` | Sets voltage limit |
| `CURR <a>` | Sets output current |
| `OUTP 0/1` | Enables/disables output |
| `MEAS:CURR?` | Queries current (requires output to be meaningful) |

## Binding Rules

- Axis mapping uses **only** the SN from `*IDN?` response.
- `last_known_port_name` is an operator hint — never used for binding.
- Port paths (`/dev/cu.*`, `COM*`) are dynamic per session.
- Known expected SN mapping:
  - X → `080020960220402020`
  - Y → `080020960220402022`
  - Z → `080020960220402003`

## Artifact Files

| File | Content |
|------|---------|
| `manifest.json` | Tool run metadata, passed/failed, artifact list |
| `maynuo_identity_snapshot.json` | Per-port probe results with classifications |
| `maynuo_identity_events.jsonl` | Timeline of probe events |
| `maynuo_axis_mapping.json` | X/Y/Z expected vs observed SN |
| `maynuo_probe_report.json` | Summary: passed, missing/duplicate/unknown axes |

## CLI Reference

```
maynuo-m8812-identity-probe \
    --profile examples/magnetic/maynuo_m8812_axes.example.json \
    --out-dir out/maynuo_identity_probe \
    --timeout-ms 300 \
    --baudrate 9600 \
    [--dry-run] \
    [--strict] \
    [--include-port COM4] \
    [--exclude-port COM1] \
    [--max-ports 3]
```

## Expected Success Sample

With all three axes connected:

```
$ maynuo-m8812-identity-probe --dry-run
Probe complete. passed=true. Artifacts written to out/maynuo_identity_probe
```

Expected `maynuo_axis_mapping.json`:
```json
{
  "x": {
    "axis_id": "mag_x",
    "expected_sn": "080020960220402020",
    "observed_sn": "080020960220402020",
    "observed_idn": "MAYNUO,M8812,080020960220402020,V2.7",
    "observed_port_path": "COM4",
    "matched": true
  },
  "y": { "...": "..." },
  "z": { "...": "..." }
}
```

## Next: Mag-M2B

Mag-M2B will extend `odmr-maynuo-m8812` with safe-init commands and a
controlled test plan execution path.  It will NOT yet enable the GUI or the
executor.
