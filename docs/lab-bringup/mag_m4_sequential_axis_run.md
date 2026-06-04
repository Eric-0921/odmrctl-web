# Mag-M4: Sequential Multi-Axis Low-current Magnetic Run

**Milestone**: Mag-M4
**Status**: Ready
**Previous**: [Mag-M3](mag_m3_recur_microtest.md)
**Next**: Mag-M5 (RF + Mag + OE integration)

## Milestone Definition

Mag-M4 is the first **multi-axis** magnetic artifact. It runs all three axes
(X, Y, Z) sequentially at low recurrent current (default 10 mA), proving that:

1. Each axis can be independently identified, energized, measured, and shut down
2. Only one axis is output-enabled at any time (**no axis overlap**)
3. The full three-axis bring-up is repeatable as a single tool invocation

Mag-M4 is **not** simultaneous three-axis output. It is sequential
basis-vector testing:
```
X 10mA → cleanup → Y 10mA → cleanup → Z 10mA → cleanup
```

## Allowed Commands

| SCPI | Purpose |
|------|---------|
| `*IDN?` | Device identity query |
| `SYST:REM` | Enter remote mode |
| `VOLT 75` | Set voltage limit |
| `CURR <float>` | Set current (bounded low current) |
| `OUTP 1` | Enable output |
| `OUTP 0` | Disable output |
| `MEAS:CURR?` | Read back actual current (A) |
| `SYST:LOC` | Return to local mode |

## Workflow (Per Step)

Each axis step performs the full M3 workflow independently:

```
1. IDN match (port → SN → axis)
2. SYST:REM
3. VOLT 75
4. ─── Zero-baseline phase ───
5. CURR 0.00000
6. OUTP 1
7. Wait settle_ms
8. MEAS:CURR? × N  → zero_readback_current_ma
9. Lock-zero (software)
10. ─── Recur phase ───
11. CURR {recur_ma / 1000:.5}
12. Wait settle_ms
13. MEAS:CURR? × N  → measured_total_current_ma
14. Reconstruct: recur = total - zero, field = recur × coil_const
15. ─── Cleanup ───
16. CURR 0.00000
17. OUTP 0
18. SYST:LOC
```

**No overlap guarantee**: Each axis completes step 18 before the next axis
starts step 2. This is structurally enforced by the tool's sequential code.

## CLI Reference

```
maynuo-m8812-sequential-axis-run \
    --profile examples/magnetic/maynuo_m8812_axes.example.json \
    --recur-current-ma 10.0 \
    --out-dir out/maynuo_sequential_axis_run \
    --timeout-ms 500 \
    --settle-ms 2000 \
    --samples 5 \
    --sample-interval-ms 200 \
    --max-current-error-ma 2.0 \
    --max-current-std-ma 0.5 \
    [--axis-id mag_x] [--axis-id mag_y] [--axis-id mag_z] \
    [--dry-run] \
    [--include-port /dev/cu.PL2303G-USBtoUART1320]
```

| Flag | Default | Purpose |
|------|---------|---------|
| `--profile` | `examples/magnetic/maynuo_m8812_axes.example.json` | Axes profile |
| `--out-dir` | `out/maynuo_sequential_axis_run` | Output directory |
| `--timeout-ms` | 300 | Read timeout |
| `--baudrate` | 9600 | Serial baud rate |
| `--recur-current-ma` | 10.0 | Recurrent current in mA (all axes) |
| `--settle-ms` | 2000 | Settle time after CURR commands |
| `--samples` | 5 | Number of MEAS:CURR? per phase |
| `--sample-interval-ms` | 200 | Delay between MEAS:CURR? queries |
| `--max-current-error-ma` | 2.0 | Maximum allowed current error |
| `--max-current-std-ma` | 0.5 | Maximum allowed current std deviation |
| `--axis-id` | mag_x,mag_y,mag_z | Axes to process (repeatable, order preserved) |
| `--dry-run` | false | Enumerate only, no hardware access |
| `--operator-note` | (none) | Note in manifest |
| `--include-port` | (all) | Include only these ports |
| `--exclude-port` | (none) | Exclude these ports |
| `--max-ports` | (none) | Maximum ports to probe |

## Artifact Files

| File | Format | Content |
|------|--------|---------|
| `manifest.json` | JSON | Tool run metadata |
| `maynuo_sequential_axis_snapshot.json` | JSON | Per-axis step results with samples |
| `maynuo_sequential_axis_report.json` | JSON | Summary: passed, per-axis metrics, no_axis_overlap |
| `maynuo_sequential_axis_events.jsonl` | JSONL | Timeline of all events |
| `maynuo_command_audit.jsonl` | JSONL | Every SCPI command sent |

## Report Fields

The `maynuo_sequential_axis_report.json` includes, per axis:

| Field | Type | Purpose |
|-------|------|---------|
| `passed` | bool | Overall run passed |
| `axes_processed` | u32 | Number of axes processed |
| `axes_passed` | u32 | Number of axes passing all checks |
| `per_axis[].step_index` | u32 | 1-based step order |
| `per_axis[].axis_id` | string | mag_x / mag_y / mag_z |
| `per_axis[].expected_sn` | string | Expected SN tail |
| `per_axis[].observed_sn` | string | Actual SN tail from IDN |
| `per_axis[].zero_readback_current_ma` | f64 | Mean zero baseline (mA) |
| `per_axis[].commanded_recur_current_ma` | f64 | Requested recur current (mA) |
| `per_axis[].measured_total_current_ma` | f64 | Mean total readback (mA) |
| `per_axis[].measured_recur_current_ma` | f64 | Reconstructed recur (mA) |
| `per_axis[].measured_recur_field_nt` | f64 | Reconstructed field (nT) |
| `per_axis[].current_error_ma` | f64 | Absolute error (mA) |
| `per_axis[].field_error_nt` | f64 | Field error (nT) |
| `per_axis[].output_final_off` | bool | Final OUTP 0 sent |
| `per_axis[].current_final_zero` | bool | Final CURR 0 sent |
| `per_axis[].local_mode_requested` | bool | Final SYST:LOC sent |
| `no_axis_overlap` | bool | Always true (structural guarantee) |

## Expected Coil Constants

| Axis | Coil constant (nT/mA) | 10 mA field (nT) |
|------|----------------------|-------------------|
| X | 143.26 | 1432.6 |
| Y | 141.77 | 1417.7 |
| Z | 156.15 | 1561.5 |

## Cross-axis Safety

The tool enforces no axis overlap structurally — each axis completes its
full cleanup (CURR 0 → OUTP 0 → SYST:LOC) before the next axis begins.
There is no simultaneous multi-axis output in Mag-M4.

## Next: Mag-M5

Mag-M5 will integrate RF + Mag + OE into combined experiments with
simultaneous multi-axis output.
