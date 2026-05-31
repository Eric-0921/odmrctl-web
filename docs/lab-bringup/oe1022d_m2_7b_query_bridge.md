# M2.7B Lab Bringup: Real SMB100A Query-only + Real OE1022D Passive Acquisition

## Goal

Create a half-real run that combines:

* Real SMB100A query-only station snapshot (TCP SCPI)
* Real OE1022D passive RALL? acquisition (USB serial)
* Formal run directory artifacts
* Explicit safety evidence that no real SMB100A setting or output command was sent

This is **not** a real ODMR run. No RF output is turned on.

## Hardware State Before Run

| Device | Parameter | Value |
|--------|-----------|-------|
| SMB100A | RF output | **OFF** (OUTP? = 0) |
| SMB100A | Modulation | **OFF** (MOD:STAT? = 0) |
| SMB100A | Frequency | 2.882 GHz |
| SMB100A | Power | -15 dBm |
| SMB100A | FM | OFF, INT source, 4 MHz deviation |
| SMB100A | LF | OFF, 500 Hz, 0.137 V, SQUARE |
| OE1022D | Identity | SSI LIA-OE1022D, SN:D6522078 |

## Tool Location

```text
tools/lab/oe1022d_smb_query_bridge/
```

## CLI Usage

```bash
cd tools/lab/oe1022d_smb_query_bridge
cargo run -- \
  --smb-host 169.254.2.20 \
  --smb-port 5025 \
  --oe-port /dev/cu.usbmodem3361358734371 \
  --oe-baud 921600 \
  --frames 100 \
  --delay-ms 20 \
  --timeout-ms 5000 \
  --run-root ../../runs \
  --run-id m2_7b_full_20260601
```

## SMB100A Queries Sent

All queries are recorded in `command_audit.jsonl`:

| Command | Response |
|---------|----------|
| `*IDN?` | `Rohde&Schwarz,SMB100A,1406.6000k02/101623,3.1.19.15-3.20.390.24` |
| `OUTP?` | `0` (OFF) |
| `MOD:STAT?` | `0` (OFF) |
| `FREQ?` | `2882000000` |
| `POW?` | `-15` |
| `POW:ALC?` | `AUTO` |
| `FM:STAT?` | `0` (OFF) |
| `FM:SOUR?` | `INT` |
| `FM:DEV?` | `4000000` |
| `LFO?` | `0` (OFF) |
| `LFO:FREQ?` | `500` |
| `LFO:VOLT?` | `0.137` |
| `LFO:SHAP?` | `SQU` |
| `SYST:ERR?` | `-410,"Query interrupted"` |

The `SYST:ERR?` response `-410,"Query interrupted"` is benign — it results from rapid sequential queries and does not indicate a hardware fault.

## OE1022D Commands Sent

| Command | Purpose |
|---------|---------|
| `*IDN?` | Identity verification |
| `RALL?` | Frame acquisition (100 frames) |

## Safety Evidence

1. **SMB100A query-only**: `validate_smb_query_only()` rejects any command not ending in `?`
2. **Setting commands blocked**: `OUTP ON`, `FREQ`, `POW`, `*RST`, etc. are rejected before transport
3. **Connection closed before acquisition**: SMB100A TCP socket is dropped before OE1022D serial opens
4. **command_audit.jsonl**: Every attempted command is recorded with `allowed: true/false`
5. **No CSV**: `check-realtime-csv.sh` passes; no `.csv` files in run directory
6. **No executor integration**: Tool is standalone, not wired to `odmr-executor`
7. **No GUI hardware access**: No frontend code modified

## Run Directory Layout

```text
runs/<run_id>/
  manifest.json
  metadata/
    acquisition_config.json
    station_snapshot.json
    parser_version.json
    smb100a_query_snapshot.json
    safety_boundary_note.json
  events.jsonl
  command_audit.jsonl
  index.jsonl
  raw/
    oe1022d_rall.rawbin
  parsed/
    b_channel_preview.jsonl
    frame_summary.jsonl
  audit_report.json
```

## Verification Results

### Smoke Run (10 frames)

| Metric | Value |
|--------|-------|
| Frames requested | 10 |
| Frames OK | **10** |
| Frames fail | 0 |
| Frames timeout | 0 |
| Rawbin size | 122,880 bytes |
| All frames 12,288 bytes | Yes |
| CSV files | None |
| Forbidden commands | None |
| Audit passed | **true** |

### Full Run (100 frames)

| Metric | Value |
|--------|-------|
| Frames requested | 100 |
| Frames OK | **100** |
| Frames fail | 0 |
| Frames timeout | 0 |
| Rawbin size | 1,228,800 bytes |
| All frames 12,288 bytes | Yes |
| CSV files | None |
| Forbidden commands | None |
| Command audit entries | 115 |
| Audit passed | **true** |

## Next Milestone

M2.8: Real SMB100A + Real OE1022D coordinated acquisition (still no executor, no RF sweep, no GUI hardware access).
