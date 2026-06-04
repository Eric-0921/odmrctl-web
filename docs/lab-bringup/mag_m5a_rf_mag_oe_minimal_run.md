# Mag-M5A: RF + Single-Axis Mag + OE Minimal Combined Run

**Milestone**: Mag-M5A
**Status**: ✅ Verified
**Previous**: [Mag-M4](mag_m4_sequential_axis_run.md)
**Next**: Mag-M5B (multi-axis RF+Mag+OE) — *future*

## Milestone Definition

Mag-M5A is the first **combined three-modality** artifact. It exercises:

- **RF** (SMB100A): set frequency / power, enable output, verify readback, disable
- **Magnetic** (Maynuo M8812, single axis): identity, zero-baseline, recurrent current, cleanup
- **OE** (OE1022D): identity, `RALL?` frame capture during RF+Mag hold

The goal is to prove that all three device families can be orchestrated in a single
tool invocation with correct sequencing, safety cleanup, and artifact generation.

## Sequencing Invariant

```
RF ON  ──→  Mag HOLD  ──→  OE CAPTURE  ──→  RF OFF  ──→  Mag CLEANUP
```

1. **RF must be ON before OE capture** (`rf_on_before_oe_capture`)
2. **Mag must be at recurrent setpoint before OE capture** (`mag_hold_before_oe_capture`)
3. **OE capture must complete before any cleanup** (`oe_capture_completed_before_cleanup`)
4. **Cleanup must complete** (`cleanup_completed`)

## Allowed Commands

### SMB100A (TCP)

| SCPI | Purpose |
|------|---------|
| `*IDN?` | Device identity |
| `FREQ <hz>` | Set RF frequency |
| `POW <dbm>` | Set RF power |
| `OUTP ON/OFF` | RF output switch |
| `OUTP?` | Readback output state |
| `MOD:STAT?` | Readback modulation state |
| `SYST:ERR?` | Error queue check |

### Maynuo M8812 (Serial)

| SCPI | Purpose |
|------|---------|
| `*IDN?` | Device identity |
| `SYST:REM` | Enter remote mode |
| `VOLT 75` | Set voltage limit |
| `CURR <float>` | Set current (A) |
| `OUTP 1/0` | Output enable/disable |
| `MEAS:CURR?` | Readback actual current (A) |
| `SYST:LOC` | Return to local mode |

### OE1022D (Serial)

| Command | Purpose |
|---------|---------|
| `*IDN?` | Device identity |
| `RALL?` | Capture 12288-byte frame |

## Workflow (10 Steps)

```
1. Create run directory (odmr-logging RunDirectory)
2. Load magnetic axis profile (JSON)
3. SMB preflight: connect → *IDN? → OUTP? → MOD:STAT? → SYST:ERR? → FREQ? → POW?
4. OE preflight: connect → *IDN?
5. Maynuo identity: enumerate ports → *IDN? → SN match
6. Maynuo zero-baseline: SYST:REM → VOLT 75 → CURR 0 → OUTP 1 → settle → MEAS:CURR? × N
7. Maynuo recur: CURR {zero+recur} → settle → MEAS:CURR? × N → compute field
8. SMB RF ON: FREQ → POW → OUTP ON → verify readback
9. OE acquisition: RALL? × frames → raw.bin append → index.jsonl
10. Cleanup:
    SMB: OUTP OFF → OUTP? → SYST:ERR?
    Maynuo: CURR 0 → OUTP 0 → settle 500ms → MEAS:CURR? → SYST:LOC
```

## Pass Criteria

| Field | Requirement |
|-------|-------------|
| `rf_final_off` | `true` (SMB OUTP OFF confirmed) |
| `mag_final_output_off` | `true` (Maynuo OUTP 0 sent) |
| `mag_final_current_zero` | `true` (final MEAS:CURR? < 1.0 mA) |
| `mag_final_local_requested` | `true` (SYST:LOC sent) |
| `frames_acquired` | == `frames_requested` |
| `errors` | empty |

## Artifact Inventory

| File | Description |
|------|-------------|
| `combined_run_report.json` | Full report with all sections |
| `manifest.json` | Device identities, artifact list, safety flags |
| `combined_events.jsonl` | Timestamped event timeline |
| `smb_command_audit.jsonl` | Per-SMB-command audit trail |
| `maynuo_command_audit.jsonl` | Per-Maynuo-command audit trail |
| `oe_command_audit.jsonl` | Per-OE-command audit trail |
| `raw.bin` | Concatenated RALL frames (12288 bytes each) |
| `frame_index.jsonl` | Offset/length index for each frame |
| `frame_summary.jsonl` | Per-frame capture metadata |
| `rf_snapshot.json` | RF section snapshot |
| `magnetic_snapshot.json` | Magnetic section snapshot |
| `oe_snapshot.json` | OE section snapshot |

## Known Hardware Behaviours

### Maynuo M8812 Noise Floor

When `CURR 0` is set and `OUTP 1`, the device reports ~0.04 mA (40 µA) as a residual
ADC offset. This is **not** a real current — it is the noise floor of the instrument.
The zero-baseline phase captures this and subtracts it from all subsequent measurements.

### Maynuo M8812 Cleanup Timing

After `CURR 0` is sent while output is ON, the device may still source current for
tens of milliseconds. The cleanup sequence must:

1. Send `CURR 0.00000`
2. Send `OUTP 0`
3. **Wait ≥ 500 ms** for output to decay
4. Query `MEAS:CURR?` to verify near-zero
5. Send `SYST:LOC`

Querying `MEAS:CURR?` **before** `OUTP 0` will read the old current (~10 mA).
Querying **after** `SYST:LOC` may fail because the device returns to local mode.

## Verification History

| Run ID | Date | Result | Notes |
|--------|------|--------|-------|
| `mag_m5a_20260620612_122334` | 2025-06-04 | ❌ `passed=false` | `mag_final_current_zero=false` because cleanup queried current **after** `SYST:LOC`; device in local mode returned 0.04 mA noise floor, which exceeded 0.001 mA threshold. |
| `mag_m5a_20260620612_123201` | 2025-06-04 | ✅ `passed=true` | Fixed cleanup sequence: `CURR 0 → OUTP 0 → settle 500ms → MEAS:CURR? → SYST:LOC`. Final readback 0.000000 A. All 10/10 OE frames acquired. RF readback verified. |

## CLI Usage

```bash
cd tools/lab/rf_mag_oe_minimal_run
cargo build --release

# Dry run (fake transports, no hardware)
./target/release/rf-mag-oe-minimal-run --dry-run

# Real run (requires --operator-approve)
./target/release/rf-mag-oe-minimal-run \
  --operator-approve \
  --smb-host 169.254.2.20 \
  --oe-port /dev/cu.usbmodem3361358734371 \
  --mag-profile examples/magnetic/maynuo_m8812_axes.example.json \
  --mag-axis-id mag_x \
  --mag-recur-current-ma 10.0 \
  --rf-frequency-hz 2882000000 \
  --rf-power-dbm -30 \
  --frames 10 \
  --out-dir out/rf_mag_oe_minimal_run
```

## Safety Notes

- `--operator-approve` is **mandatory** for real runs; omitting it aborts immediately.
- RF power is limited to `-30 dBm` (1 µW) for this minimal run.
- Magnetic current is limited to **10 mA** (low-current regime).
- All three modalities have independent cleanup paths; a failure in any step triggers
  the appropriate device-specific safe-state routine.
- No CSV files are created in the real-time path (raw-first architecture).
