# SMB100A Safe-Set Audit

Human-in-the-loop tool that safely configures an SMB100A RF signal generator
to the ODMR baseline state while keeping RF output OFF and modulation OFF.

## Safety Model

This tool enforces a **two-layer defense**:

1. **Allow-list**: Only 12 pre-defined safe-set commands can be sent.
2. **Forbidden-pattern gate**: Dangerous substrings (`OUTP ON`, `*RST`, etc.)
   are rejected even if they somehow reach the validator.

There is **no generic `send(cmd)` API**. The library only exposes:

- `Smb100aSafeSet::run(confirm_callback)` — runs the 12-step sequence
- `Smb100aSafeSet::run_final_validation()` — read-only final checks

Every setter requires **operator confirmation** via stdin prompt.

## Allowed Commands

```text
OUTP OFF
MOD:STAT OFF
FREQ:MODE CW
FREQ 2.882GHz
POW -15dBm
POW:ALC AUTO
LFO:FREQ 500Hz
LFO:VOLT 137mV
LFO:SHAP SQU
FM:SOUR INT
FM:DEV 4MHz
FM:STAT OFF
```

## Forbidden Patterns

```text
OUTP ON
MOD:STAT ON
FM:STAT ON
FREQ:MODE SWE
SWE:EXEC
INIT
RUN
RST
*RST
```

## Usage

```bash
cd tools/lab/smb100a_safe_set
cargo run -- --host 169.254.2.20 --port 5025
```

The tool will prompt for confirmation before each step:

```
Step  1/12: OUTP OFF
  Before: OUTP? = 0
  Confirm? [Y/n/abort] >
```

Type:

- `Y` or Enter → send the command
- `n` → skip this step
- `abort` → stop and send safe-disconnect (`OUTP OFF`, `MOD:STAT OFF`)

## Output

- `docs/lab-bringup/smb100a_safe_set_audit_YYYY-MM-DD.md`
- `examples/verification/smb100a_safe_set_observed_YYYY-MM-DD.jsonl`

## Tests

```bash
cargo test
```

Safety gate tests prove:

- `OUTP ON` is rejected
- `MOD:STAT ON` is rejected
- `FREQ:MODE SWE` is rejected
- `SWE:EXEC` is rejected
- All 12 safe-set commands are accepted
- No generic arbitrary-send API exists

## Transport Notes

- SMB100A: TCP/5025, newline (`\n`) terminator
- Timeout: 2s connect, 2s read
- If the instrument is still booting, the first connection may time out;
  wait ~30s and retry.
