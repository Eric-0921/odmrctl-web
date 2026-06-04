# Mag-M3: Single-axis Recurrent Current / Field Micro-test

## Milestone Definition

Mag-M3 is the **first nonzero recurrent current milestone**. It verifies the full
old-GUI logic end-to-end:

```text
zero baseline → lock-zero → CURR nonzero → readback reconstruction → cleanup
```

M3 is **single-axis only**. Multi-axis coordination is a future milestone.

## Architecture

```
odmr-mag (mock-only)
    └── types, profile, SN extraction, coil constants

odmr-maynuo-m8812 (real serial)
    └── M3 expanded transport: CURR <float> pattern matching
        Allows any properly-formatted CURR command; current limit
        enforcement is at the tool layer

maynuo_m8812_recur_microtest (lab tool)
    └── Single-axis: IDN → zero-baseline → lock-zero →
        CURR nonzero → readback reconstruction → cleanup
```

## Workflow

```
1. Enumerate and filter ports
2. Identity probe with *IDN?
3. Match target axis by SN
4. SYST:REM
5. VOLT 75
6. ─── Zero-baseline phase ───
7. CURR 0.00000
8. OUTP 1
9. Wait settle_ms
10. MEAS:CURR? × N  → zero_readback_current_ma
11. Lock-zero (software)
12. ─── Recur phase ───
13. Compute recur current:
      if --target-field-nt:  recur_ma = target_nt / coil_constant
      if --recur-current-ma: recur_ma = explicit
14. total_ma = zero_set + recur_ma
15. CURR {total_ma / 1000:.5}
16. Wait settle_ms
17. MEAS:CURR? × N  → measured_total_current_ma
18. Reconstruct:
      measured_recur_ma = measured_total_ma - zero_readback_ma
      measured_recur_nt = measured_recur_ma × coil_constant
19. ─── Cleanup ───
20. CURR 0.00000
21. OUTP 0
22. SYST:LOC
```

## Allowed Commands

| SCPI | Purpose |
|------|---------|
| `*IDN?` | Device identity query |
| `SYST:REM` | Enter remote mode |
| `VOLT 75` | Set voltage limit |
| `CURR <float>` | Set current (any valid float; limit enforced at tool layer) |
| `OUTP 1` | Enable output |
| `OUTP 0` | Disable output |
| `MEAS:CURR?` | Read back actual current (A) |
| `SYST:LOC` | Return to local mode |

## CLI Reference

```
maynuo-m8812-recur-microtest \
    --profile examples/magnetic/maynuo_m8812_axes.example.json \
    --axis-id mag_x \
    --recur-current-ma 10.0 \
    --out-dir out/maynuo_recur_microtest \
    --timeout-ms 500 \
    --settle-ms 2000 \
    --samples 5 \
    --sample-interval-ms 200 \
    --max-current-error-ma 2.0 \
    --max-current-std-ma 0.5 \
    [--target-field-nt 1432.6] \
    [--dry-run] \
    [--include-port /dev/cu.PL2303G-USBtoUART1320]
```

| Flag | Default | Purpose |
|------|---------|---------|
| `--profile` | `examples/magnetic/maynuo_m8812_axes.example.json` | Axes profile |
| `--axis-id` | (required) | Axis to test (mag_x/mag_y/mag_z) |
| `--recur-current-ma` | 10.0 | Recurrent current in mA |
| `--target-field-nt` | (none) | Target field in nT (overrides --recur-current-ma) |
| `--out-dir` | `out/maynuo_recur_microtest` | Output directory |
| `--settle-ms` | 2000 | Settle time after CURR commands |
| `--samples` | 5 | Number of MEAS:CURR? per phase |
| `--sample-interval-ms` | 200 | Delay between MEAS:CURR? queries |
| `--max-current-error-ma` | 2.0 | Maximum allowed current error |
| `--max-current-std-ma` | 0.5 | Maximum allowed current std deviation |
| `--dry-run` | false | Enumerate only, no open |
| `--include-port` | (all) | Include only these ports |
| `--exclude-port` | (none) | Exclude these ports |

## Artifact Files

| File | Format | Content |
|------|--------|---------|
| `manifest.json` | JSON | Tool run metadata |
| `maynuo_recur_microtest_snapshot.json` | JSON | Full per-phase measurements |
| `maynuo_recur_microtest_report.json` | JSON | Summary: passed, error, tolerances |
| `maynuo_recur_microtest_events.jsonl` | JSONL | Timeline of all events |
| `maynuo_command_audit.jsonl` | JSONL | Every SCPI command sent |

## Expected Coil Constants

| Axis | Coil constant (nT/mA) | 10 mA field (nT) |
|------|----------------------|-------------------|
| X | 143.26 | 1432.6 |
| Y | 141.77 | 1417.7 |
| Z | 156.15 | 1561.5 |

## Recur Current Reconstruction

```text
measured_recur_current_ma = measured_total_current_ma - zero_readback_current_ma
measured_recur_field_nt   = measured_recur_current_ma × coil_constant_nt_per_ma
```

This matches the old GUI pattern:
```csharp
// LockZero ON + output ON:
double recur_ma = total_readback_ma - zeroSetCurr;
double recur_nt = recur_ma * coilConstant;
```

## Next: Mag-M4

Mag-M4 will extend to X/Y/Z sequential low-current micro-tests, then Mag-M5 will
integrate RF + Mag + OE combined experiments.
