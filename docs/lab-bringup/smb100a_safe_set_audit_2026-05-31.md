# SMB100A Safe-Set Audit

> **Safety Audit**: Only pre-approved safe-set commands were sent.
> **Human-in-the-loop**: Each setter required operator confirmation.
> **RF Output**: Kept OFF throughout.
> **Modulation**: Kept OFF throughout.

## Safe-Set Steps

| # | Command | Query Before | Before Value | Query After | After Value | SYST:ERR | Status |
|---|---------|--------------|--------------|-------------|-------------|----------|--------|
| 1 | `OUTP OFF` | `OUTP?` | 0 | `OUTP?` | 0 | 0,"No error" | pass |
| 2 | `MOD:STAT OFF` | `MOD:STAT?` | 0 | `MOD:STAT?` | 0 | 0,"No error" | pass |
| 3 | `FREQ:MODE CW` | `FREQ:MODE?` | CW | `FREQ:MODE?` | CW | 0,"No error" | pass |
| 4 | `FREQ 2.882GHz` | `FREQ?` | 2520000000 | `FREQ?` | 2882000000 | 0,"No error" | pass |
| 5 | `POW -15dBm` | `POW?` | -14 | `POW?` | -15 | 0,"No error" | pass |
| 6 | `POW:ALC AUTO` | `POW:ALC?` | AUTO | `POW:ALC?` | AUTO | 0,"No error" | pass |
| 7 | `LFO:FREQ 500Hz` | `LFO:FREQ?` | 500 | `LFO:FREQ?` | 500 | 0,"No error" | pass |
| 8 | `LFO:VOLT 137mV` | `LFO:VOLT?` | 3 | `LFO:VOLT?` | 0.137 | 0,"No error" | pass |
| 9 | `LFO:SHAP SQU` | `LFO:SHAP?` | SQU | `LFO:SHAP?` | SQU | 0,"No error" | pass |
| 10 | `FM:SOUR INT` | `FM:SOUR?` | INT | `FM:SOUR?` | INT | 0,"No error" | pass |
| 11 | `FM:DEV 4MHz` | `FM:DEV?` | 3500000 | `FM:DEV?` | 4000000 | 0,"No error" | pass |
| 12 | `FM:STAT OFF` | `FM:STAT?` | 0 | `FM:STAT?` | 0 | 0,"No error" | pass |

## Final Validation

| Query | Response | Status |
|-------|----------|--------|
| `OUTP?` | 0 | pass |
| `MOD:STAT?` | 0 | pass |
| `FM:STAT?` | 0 | pass |
| `FREQ:MODE?` | CW | pass |
| `SYST:ERR?` | 0,"No error" | pass |

## Forbidden Command Audit

The following patterns were explicitly blocked by the safe-set tool:
- `OUTP ON`
- `MOD:STAT ON`
- `FM:STAT ON`
- `FREQ:MODE SWE`
- `SWE:EXEC`
- `INIT`
- `RUN`
- `RST`
- `*RST`
