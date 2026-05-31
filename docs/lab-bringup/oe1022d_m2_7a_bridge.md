# M2.7A: Fake SMB100A + Real OE1022D Bridge

Date: 2026-06-01
Tool: `tools/lab/oe1022d_smb_fake_bridge/`

## Goal

Bridge fake SMB100A state timeline with real OE1022D passive acquisition.

## Safety Boundaries

| Boundary | Value |
|----------|-------|
| Real OE1022D allowed commands | `*IDN?`, `RALL?` only |
| Real SMB100A connected | **false** |
| Fake SMB100A marked mock | **true** |
| CSV created | **false** |
| Executor integration | **false** |
| GUI hardware access | **false** |

## Verified Runs

| Run | Frames | Rawbin Size | Result |
|-----|--------|-------------|--------|
| `m2_7a_smoke_20260601` | 10 | 122,880 B | PASS |
| `m2_7a_full_20260601` | 100 | 1,228,800 B | PASS |

## Run Directory Layout

```text
runs/<run_id>/
  manifest.json
  metadata/
    acquisition_config.json
    station_snapshot.json
    parser_version.json
    fake_smb100a_profile.json
    safety_boundary_note.json
  events.jsonl
  index.jsonl
  raw/oe1022d_rall.rawbin
  parsed/
    b_channel_preview.jsonl
    frame_summary.jsonl
  mock/
    fake_smb100a_state_timeline.jsonl
  audit_report.json
```

## Fake SMB100A Profile

`examples/acquisition/fake_smb100a_odmr_idle_profile.json`

- RF output: OFF
- RF frequency: 2.882 GHz
- RF power: -15 dBm
- LF output: ON, 500 Hz, 0.137 V, SQUARE
- FM: OFF, INT source, 4 MHz deviation

## CLI Usage

```bash
cd tools/lab/oe1022d_smb_fake_bridge
cargo run -- \
  --oe-port /dev/cu.usbmodem3361358734371 \
  --oe-baud 921600 \
  --frames 100 \
  --delay-ms 20 \
  --timeout-ms 5000 \
  --run-root ../../runs \
  --run-id m2_7a_full_20260601 \
  --fake-smb-profile ../../../examples/acquisition/fake_smb100a_odmr_idle_profile.json
```

## Audit Results

Both runs passed audit:

- offsets contiguous: true
- all frames 12288 bytes: true
- no CSV files: true
- no forbidden commands: true
- mock/real boundary machine-readable: true
