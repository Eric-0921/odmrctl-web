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

## Decisions Made

### 1. SYST:ERR? Usage (Decided)

**Decision: Keep `SYST:ERR?` as health check, but change placement.**

- `SYST:ERR?` is **not** removed from the query list.
- It is moved to the **end of the query sequence** (after all other queries).
- Each query is followed by **full response drain** before sending the next query.
- If `-410,"Query interrupted"` recurs, it is recorded as a **warning**, not a failure:
  - `station_snapshot_quality.warnings` contains `"SYST:ERR? returned -410"`
  - `-410 Query interrupted` does **not** fail M2.8
  - But before any future RF ON milestone, the error queue must be cleared.

Rationale: `-410` is a benign side effect of rapid sequential queries, not a hardware fault. But the error queue must be clean before dangerous commands.

### 2. SMB100A Query Delay (Decided)

**Decision: `--smb-query-delay-ms` default = 100ms.**

| Environment | Default |
|-------------|---------|
| Real hardware | `100ms` |
| Test / CI | `0ms` (overridable) |

Rationale: SMB100A query snapshot is not a high-frequency path. A 100ms delay has negligible cost but significantly reduces query overlap / buffer undrained issues.

### 3. State Hash Attachment Scope (Decided)

**Decision: M2.8 builds a prototype; M3 freezes the schema contract.**

M2.8 produces:

```text
metadata/hash_manifest.json
```

Containing:

```json
{
  "station_snapshot_hash": "sha256:...",
  "smb100a_query_snapshot_hash": "sha256:...",
  "acquisition_config_hash": "sha256:..."
}
```

Optionally attached to:
- `parsed/frame_summary.jsonl` (recommended)
- `index.jsonl` (optional, may bloat index)

M3 will decide whether hash attachment becomes a mandatory schema field.

### 4. Monotonic Timestamp Fields (Decided)

**Decision: Use `std::time::Instant` for intra-run monotonic timestamps. Do not attempt cross-process or cross-reboot alignment.**

Required fields per run:

```json
{
  "run_started_wall_time_utc": "2026-06-01T12:00:00Z",
  "run_started_monotonic_anchor_ns": 123456789000,
  "frames": [
    {
      "frame_index": 0,
      "frame_monotonic_ns_since_run_start": 850000000,
      "frame_wall_time_utc": "2026-06-01T12:00:00.850Z"
    }
  ]
}
```

Rationale: A single run's internal alignment is sufficient for M2.8. Cross-run comparison uses wall time.

## Investigation Areas

### 1. Monotonic Timestamp Alignment

M2.7B captures SMB100A state at `t0` and OE1022D frames at `t1...tn`. M2.8 should:

- Record `t_mono_query_ns` for each SMB100A query
- Record `t_mono_recv_ns` for each OE1022D frame
- Compute `delta_ms = t_oe_first - t_smb_last`
- Verify `delta_ms > 0` (OE acquisition starts strictly after SMB query completes)
- Store alignment metadata in `metadata/timeline_alignment.json`

### 2. SYST:ERR? -410 Recurrence

M2.7B observed `-410,"Query interrupted"` on `SYST:ERR?`. With the new 100ms delay + drain strategy, M2.8 should verify:

- Whether `-410` still recurs
- Whether moving `SYST:ERR?` to the end of the sequence eliminates it
- Whether full response drain between queries helps

Expected outcome: `-410` may still appear occasionally; if so, it is logged as a warning in `snapshot_quality.json`.

### 3. SMB100A Query Pacing

With `--smb-query-delay-ms = 100ms`, the query sequence takes ~1.4s instead of ~50ms. This is acceptable because:

- Query snapshot is a one-time per-run operation
- 1.4s is negligible compared to a 100-frame acquisition (~10s)
- The delay significantly reduces SCPI buffer contention

### 4. Station Snapshot Quality Warnings

M2.8 writes `metadata/snapshot_quality.json`:

```json
{
  "schema_version": "0.2.0",
  "smb100a_query_count": 14,
  "smb100a_query_delay_ms": 100,
  "smb100a_errors": [],
  "oe1022d_identity_verified": true,
  "timestamp_alignment_ok": true,
  "warnings": [],
  "connection_closed_before_acquisition": true
}
```

If `-410` recurs:

```json
{
  "warnings": ["SYST:ERR? returned -410,Query interrupted"],
  "smb100a_errors": ["-410,Query interrupted"]
}
```

### 5. Real vs Fake SMB100A State Diff

M2.7A fake profile:

```json
{
  "rf_output_enabled": false,
  "modulation_global_enabled": false,
  "rf_frequency_hz": 2882000000,
  "rf_power_dbm": -15,
  "lf_output_enabled": true,
  "lf_frequency_hz": 500,
  "lf_voltage_v": 0.137,
  "lf_shape": "SQUARE"
}
```

M2.7B real observation:

```json
{
  "rf_output_enabled": false,
  "modulation_global_enabled": false,
  "rf_frequency_hz": 2882000000,
  "rf_power_dbm": -15,
  "lf_output_enabled": false,
  "lf_frequency_hz": 500,
  "lf_voltage_v": 0.137,
  "lf_shape": "SQU"
}
```

Diffs:
- `lf_output_enabled`: true (fake) vs false (real)
- `lf_shape`: "SQUARE" (fake) vs "SQU" (real device truncates)

M2.8 should document this in `parsed/state_diff_report.json` and decide whether to update the fake profile.

### 6. Frame Summary / Index State Hash Attachment (Prototype)

M2.8 computes SHA-256 hashes of:

- `metadata/station_snapshot.json` → `station_snapshot_hash`
- `metadata/smb100a_query_snapshot.json` → `smb100a_query_snapshot_hash`
- `metadata/acquisition_config.json` → `acquisition_config_hash`

These are written to `metadata/hash_manifest.json`.

Prototype attachment to `parsed/frame_summary.jsonl`:

```json
{
  "frame_index": 0,
  "state_snapshot_hash": "sha256:abc123...",
  "run_id": "m2_8_aligned_20260601"
}
```

This is a **prototype** — full schema contract in M3.

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
  --smb-query-delay-ms 100 \
  --attach-state-hash \
  --run-root ../../runs \
  --run-id m2_8_aligned_20260601
```

New flags:
- `--smb-query-delay-ms`: delay between SMB100A queries (default `100`)
- `--attach-state-hash`: attach `smb100a_query_snapshot` hash to frame_summary rows

## Proposed Output Files

Same as M2.7B, plus:

```text
runs/<run_id>/
  metadata/
    timeline_alignment.json       # NEW
    snapshot_quality.json         # NEW
    hash_manifest.json            # NEW (prototype)
  parsed/
    state_diff_report.json        # NEW (real vs fake profile)
```

## Success Criteria

1. SMB100A queries execute with `--smb-query-delay-ms = 100`
2. `-410` does not recur, OR recurs and is documented as a warning in `snapshot_quality.json`
3. `timeline_alignment.json` shows positive delta between SMB close and OE first frame
4. `snapshot_quality.json` contains `connection_closed_before_acquisition: true`
5. `hash_manifest.json` contains valid SHA-256 hashes
6. `state_diff_report.json` documents real vs fake profile differences
7. No CSV files in run directory
8. No setting commands sent to SMB100A
9. `command_audit.jsonl` records all attempted commands
10. `cargo test` passes for tool + workspace

## Remaining Blockers Before M2.8

- [x] Decide whether `SYST:ERR?` should be removed from standard query list → **Keep, move to end, drain between queries**
- [x] Decide whether `--smb-query-delay-ms` default should be > 0 → **Default 100ms**
- [x] Decide whether state hash attachment belongs in M2.8 or deferred to M3 → **M2.8 prototype, M3 contract**
- [x] Verify `monotonic_ns` availability on macOS → **Use `std::time::Instant`, intra-run only**
