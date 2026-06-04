# Mag-M2B: Zero-mode Output-on Readback + Lock-zero Baseline

## Milestone Definition

Mag-M2B is the **first real output-enable milestone** for the magnetic axis line.
It extends Mag-M2A (identity-only) with safe-init commands and zero-mode readback.

Mag-M2B proves that each Maynuo M8812 axis can:
1. Be identified by SN
2. Enter remote mode
3. Enable output at zero commanded current (`CURR 0 + OUTP 1`)
4. Read back the actual zero baseline current via `MEAS:CURR?`
5. Lock the measured baseline as software state
6. Shut down cleanly (`OUTP 0 → CURR 0 → SYST:LOC`)

Mag-M2B does **not** output nonzero recurrent current or target field.

## Architecture

```
odmr-mag (mock-only, no serialport)
    └── MaynuoAxisRunner state machine
         states: Discovered → AxisMapped → InitializedOutputOff
                 → OutputOnZeroMode → ZeroMeasured → ZeroLocked

odmr-maynuo-m8812 (real serial)
    └── M2B expanded transport: *IDN?, SYST:REM, VOLT 75,
        CURR 0.00000, OUTP 0|1, MEAS:CURR?, SYST:LOC

maynuo_m8812_zero_baseline (lab tool)
    └── Bridges transport ↔ state machine
        Identity probe → safe init → output-on → readback → lock → cleanup
```

## Workflow

### Per-Axis Sequence

```
1. Probe identity with *IDN?
2. Match axis by SN
3. SYST:REM
4. VOLT 75
5. CURR 0.00000       → apply_initialized_output_off()
6. OUTP 1             → apply_output_on_zero_mode()
7. Wait settle_ms
8. MEAS:CURR? × N     → collect samples in A, convert to mA
9. Compute mean ± std → apply_zero_measured(mean_ma)
10. Lock zero          → apply_lock_zero()
11. OUTP 0             ┐
12. CURR 0.00000       ├ cleanup (best-effort, always runs)
13. SYST:LOC           ┘
```

### Key Distinction

| Concept | Field | Meaning |
|---------|-------|---------|
| zero_set_current_ma | `AxisZeroBaseline.zero_set_current_ma` | Commanded baseline (always 0.0 in M2B) |
| zero_readback_current_ma | `AxisZeroBaseline.zero_readback_current_ma` | Mean of MEAS:CURR? samples while output on |

These are distinct. The old GUI (`FormMain.cs`) had a similar distinction:
`zeroSetCurr` (commanded) vs `textBoxZeroCurr` (readback display when LockZero off).

## Allowed Commands

| SCPI | Purpose |
|------|---------|
| `*IDN?` | Device identity query |
| `SYST:REM` | Enter remote mode |
| `VOLT 75` | Set voltage limit |
| `CURR 0.00000` | Set zero current |
| `OUTP 1` | Enable output |
| `OUTP 0` | Disable output |
| `MEAS:CURR?` | Read back actual current (A) |
| `SYST:LOC` | Return to local mode |

## Forbidden Commands

| SCPI | Reason |
|------|--------|
| `CURR` nonzero | Not in allowlist; rejected at transport layer |
| `OUTP 2` | Not in allowlist |
| `SYST:REM` variant | Exact-match allowlist rejects typos |
| Any recurrent field/current | Not in M2B scope |
| Any sweep | Not in M2B scope |

## CLI Reference

```
maynuo-m8812-zero-baseline \
    --profile examples/magnetic/maynuo_m8812_axes.example.json \
    --out-dir out/maynuo_zero_baseline \
    --timeout-ms 300 \
    --baudrate 9600 \
    --settle-ms 2000 \
    --zero-samples 5 \
    --sample-interval-ms 200 \
    [--axis-id mag_x] \
    [--dry-run] \
    [--strict] \
    [--include-port COM4] \
    [--exclude-port COM1] \
    [--max-ports 3]
```

| Flag | Default | Purpose |
|------|---------|---------|
| `--profile` | `examples/magnetic/maynuo_m8812_axes.example.json` | Axes profile |
| `--out-dir` | `out/maynuo_zero_baseline` | Output directory |
| `--timeout-ms` | 300 | Read timeout per port |
| `--baudrate` | 9600 | Serial baud rate |
| `--settle-ms` | 2000 | Settle after OUTP 1 |
| `--zero-samples` | 5 | MEAS:CURR? repetitions |
| `--sample-interval-ms` | 200 | Delay between samples |
| `--axis-id` | (all) | Process single axis only |
| `--dry-run` | false | Enumerate only, no open |
| `--strict` | false | Unknown SN → overall failure |
| `--include-port` | (all) | Include only these ports |
| `--exclude-port` | (none) | Exclude these ports |
| `--max-ports` | (unlimited) | Probe at most N ports |

## Artifact Files

| File | Format | Content |
|------|--------|---------|
| `manifest.json` | JSON | Tool run metadata, axes processed, invariants met |
| `zero_baseline_snapshot.json` | JSON | Per-axis `AxisZeroBaseline` with all measurements |
| `zero_baseline_report.json` | JSON | Summary: passed/failed, audit invariants |
| `zero_baseline_events.jsonl` | JSONL | Timeline of all events |
| `command_audit.jsonl` | JSONL | Every SCPI command sent, with metadata |

## Audit Invariants

Computed from the command audit trail:

| Invariant | Requirement |
|-----------|-------------|
| `nonzero_current_sent` | Must be `false` |
| `outp_on_sent` | Must be `true` |
| `outp_on_only_after_curr_zero` | OUTP 1 must appear after CURR 0.00000 |
| `measured_current_queries_sent` | Must be ≥ 1 |
| `zero_readback_current_ma_recorded` | At least one axis must have samples |
| `lock_zero_event_recorded` | At least one axis must have lock applied |
| `recurrent_current_sent` | Always `false` in M2B |
| `recurrent_field_sent` | Always `false` in M2B |
| `final_output_off` | OUTP 0 must appear in audit |
| `final_current_zero_command_sent` | CURR 0.00000 must appear after OUTP 0 |
| `final_local_mode_requested` | SYST:LOC must appear in audit |

All invariants must pass for `audit_invariants_met: true` in the manifest.

## Error Handling

- Each axis is processed independently — one failing does not abort others
- If any step fails, `attempt_cleanup()` runs: OUTP 0 → CURR 0 → SYST:LOC (best-effort)
- If port open fails, no cleanup is needed (port was never connected)
- Readback failures on individual samples are recorded but don't abort — available samples are used for statistics
- If zero samples are collected, the axis is marked failed

## Expected Success Sample

With all three axes connected:

```
$ maynuo-m8812-zero-baseline --zero-samples 3
Zero baseline complete. passed=true. Artifacts written to out/maynuo_zero_baseline
```

Expected `zero_baseline_snapshot.json` entry for one axis:
```json
{
  "axis_id": "mag_x",
  "idn": "MAYNUO,M8812,080020960220402020,V2.7",
  "port_path": "COM4",
  "sn_tail": "2020",
  "zero_set_current_ma": 0.0,
  "zero_readback_samples_ma": [0.15, 0.16, 0.14],
  "zero_readback_current_ma": 0.15,
  "zero_readback_std_ma": 0.008,
  "zero_readback_current_a": 0.00015,
  "coil_constant_nt_per_ma": 143.26,
  "lock_zero_applied": true,
  "output_was_on": true,
  "shutdown_succeeded": true,
  "errors": []
}
```

## Next: Mag-M2C

Mag-M2C will extend with nonzero recurrent current output:
- `CURR {nonzero}` for target field
- `recur_setpoint_planned_from_field()` integration
- Verify `total_current = zero_set + recur_set` matches old GUI behavior
