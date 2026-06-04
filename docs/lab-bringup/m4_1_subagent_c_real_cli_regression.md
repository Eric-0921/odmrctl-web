# Subagent C — Real CLI Regression Readiness

## 1. Exact CLI to Run

```bash
cd tools/lab/recipe_two_device_run
cargo run --release -- \
  --mode=real \
  --recipe ../../../examples/recipes/m3_4/m3_4_two_device_sweep.recipe.json \
  --smb-host=169.254.2.20 \
  --smb-port=5025 \
  --smb-query-delay-ms=50 \
  --smb-timeout-ms=3000 \
  --oe-port=/dev/cu.usbmodem3361358734371 \
  --oe-baud=921600 \
  --oe-timeout-ms=5000 \
  --run-root=../../../runs \
  --run-id=m4_1_cli_regression_real_$(date +%Y%m%d_%H%M%S) \
  --operator-approves-real-run \
  --operator-approval-note="Human operator approved M4.1 real CLI regression run. GUI does not execute hardware. No magnetic axes are part of this run."
```

## 2. Expected Frames / Steps

| Metric | Expected |
|--------|----------|
| Total steps | 22 (11 points × 2 repeats) |
| Steps completed | 22/22 |
| Frames requested | 110 (11 × 5 × 2) |
| Frames captured | 110 |
| Frames parsed | 110 |
| Frames parse failed | 0 |
| Parse failure rate | 0.0 |

## 3. Final Safety Checks

| Check | Expected |
|-------|----------|
| Final RF OFF | `OUTP? = 0` |
| Final MOD OFF | `MOD:STAT? = 0` |
| Final FM OFF | `FM:STAT? = 0` (unless intentionally configured otherwise) |
| SYST:ERR? clean | Yes (no errors) |
| No internal sweep | Yes |
| No magnetic commands | Yes (0 magnetic commands) |
| No CSV | Yes |

## 4. Command Audit Expectations

- `command_audit_comparison_passed = true`
- `no_forbidden_commands_sent = true`
- Expected command count matches `command_plan.json` count
- No unexpected commands
- No missing expected commands

## 5. What Failure Should Block M4.1

Blocking failures:
1. SMB100A identity NOT verified
2. OE1022D identity NOT verified
3. Safety report decision = reject
4. Steps completed < 22
5. Frames captured < 110
6. Parse failures > 0
7. Command audit comparison FAILED
8. Final RF/MOD/FM NOT in safe state
9. SYST:ERR? shows errors
10. Internal sweep detected
11. Magnetic commands detected
12. CSV output detected

Non-blocking (report but do not block):
- Warnings in safety report (allow_with_warnings)
- Minor timing variance
- Single retry/recovery event

## 6. Run Artifact Preservation

Run directory will be at:
```
runs/m4_1_cli_regression_real_YYYYMMDD_HHMMSS/
```

Must preserve:
- `run_result.json`
- `command_plan.json`
- `command_audit.json`
- `safety_report.json`
- `run_manifest.json`
- `raw_data/` (raw bin + index.jsonl)
- All log files

## 7. Hardware State Before Run

Confirm before starting:
- SMB100A is reachable at 169.254.2.20:5025
- OE1022D is reachable at /dev/cu.usbmodem3361358734371
- No other process is using these devices
- Laser safety protocol followed
