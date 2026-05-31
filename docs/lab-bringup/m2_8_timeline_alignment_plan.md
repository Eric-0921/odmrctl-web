# M2.8 Planning: Real SMB100A + Real OE1022D Coordinated Acquisition with Timeline Alignment

## Goal

M2.8 extends M2.7B by adding **timestamp alignment** between the SMB100A query snapshot and the OE1022D acquisition frames.

This is still **not** a real ODMR run. No RF output is turned on. No sweep is executed.

## What M2.8 Includes

```text
M2.8 = real SMB100A query snapshot / state timeline
       + real OE1022D passive acquisition
       + monotonic timestamp alignment
       + no RF ON
       + no sweep
       + no executor
       + no GUI hardware access
```

## What M2.8 Explicitly Does NOT Do

```text
OUTP ON
MOD:STAT ON
FREQ set
POW set
FM set
SWEep set
LFO set
OE1022D setting commands (SENSD, OFSLD, etc.)
executor-controlled real hardware
GUI hardware access
CSV writing
```

## Safety Rules (Same as M2.7B)

| Device | Allowed Commands | Blocked Patterns |
|--------|------------------|------------------|
| SMB100A | Queries ending in `?` only | `OUTP `, `MOD:STAT `, `FREQ `, `POW `, `*RST`, etc. |
| OE1022D | `*IDN?`, `RALL?` only | `*RST`, `SENSD`, `OFLTD`, `OFSLD`, etc. |

## Investigation Areas

### 1. Monotonic Timestamp Alignment

M2.7B captures SMB100A state at `t0` and OE1022D frames at `t1...tn`. M2.8 should:

- Record `t_mono_query_ns` for each SMB100A query
- Record `t_mono_recv_ns` for each OE1022D frame
- Compute `delta_ms = t_oe_first - t_smb_last`
- Verify `delta_ms > 0` (OE acquisition starts strictly after SMB query completes)
- Store alignment metadata in `metadata/timeline_alignment.json`

### 2. SYST:ERR? -410 Recurrence

M2.7B observed `-410,"Query interrupted"` on `SYST:ERR?`. M2.8 should investigate:

- Whether this is benign (rapid query pacing) or indicates a real error
- Whether adding `read_timeout` + drain between queries eliminates it
- Whether `SYST:ERR?` should be queried once at the end instead of after each query

### 3. SMB100A Query Pacing

Current M2.7B sends queries as fast as TCP allows. M2.8 should evaluate:

- Whether a small delay (e.g. 50ms) between queries improves stability
- Whether response draining is needed (some SCPI implementations buffer)
- Whether `SYST:ERR?` should be the final query, not part of the snapshot list

### 4. Station Snapshot Quality Warnings

M2.8 should add `snapshot_quality` to `station_snapshot.json`:

```json
{
  "snapshot_quality": {
    "smb100a_query_count": 14,
    "smb100a_errors": ["-410,Query interrupted"],
    "oe1022d_identity_verified": true,
    "timestamp_alignment_ok": true,
    "warnings": ["SYST:ERR? returned -410"]
  }
}
```

### 5. Real vs Fake SMB100A State Diff

M2.7A used a fake profile with:
- RF OFF, MOD OFF
- FREQ 2.882 GHz, POW -15 dBm
- LF ON 500Hz, 0.137V, SQUARE

M2.7B observed real SMB100A with:
- RF OFF, MOD OFF
- FREQ 2.882 GHz, POW -15 dBm
- LF OFF

M2.8 should document this diff and decide whether the fake profile needs updating.

### 6. Frame Summary / Index State Hash Attachment

Future executor runs will need to prove that each frame was captured under a specific station state. M2.8 should prototype:

- Compute a hash of `smb100a_query_snapshot.json`
- Attach `state_snapshot_hash` to each `index.jsonl` entry
- Attach `state_snapshot_hash` to each `frame_summary.jsonl` row

This is a **prototype only** — full integration happens in M3+.

## Proposed CLI

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
  --smb-query-delay-ms 50 \
  --run-root ../../runs \
  --run-id m2_8_aligned_20260601
```

New flags:
- `--smb-query-delay-ms`: delay between SMB100A queries (default 0)
- `--attach-state-hash`: attach `smb100a_query_snapshot` hash to index entries

## Proposed Output Files

Same as M2.7B, plus:

```text
runs/<run_id>/
  metadata/
    timeline_alignment.json       # NEW
    snapshot_quality.json         # NEW
  parsed/
    state_diff_report.json        # NEW (real vs fake profile)
```

## Success Criteria

1. SMB100A queries execute without `-410` error OR error is documented as benign
2. `timeline_alignment.json` shows positive delta between SMB close and OE first frame
3. `snapshot_quality.json` contains at least one warning about `-410` if it recurs
4. `state_diff_report.json` documents real vs fake profile differences
5. No CSV files in run directory
6. No setting commands sent to SMB100A
7. `command_audit.jsonl` records all attempted commands
8. `cargo test` passes for tool + workspace

## Remaining Blockers Before M2.8

- [ ] Decide whether `SYST:ERR?` should be removed from standard query list
- [ ] Decide whether `--smb-query-delay-ms` default should be > 0
- [ ] Decide whether state hash attachment belongs in M2.8 or deferred to M3
- [ ] Verify `monotonic_ns` availability on macOS (`std::time::Instant`)
