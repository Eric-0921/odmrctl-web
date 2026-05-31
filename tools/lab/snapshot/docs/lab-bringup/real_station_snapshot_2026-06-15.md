# Real-Device Read-Only Station Snapshot

> **Safety Audit**: Only read-only queries were sent.
> No state-changing commands were transmitted.
> All commands validated against hard-coded allow-lists.

## SMB100A

| # | Command | Response | Duration (ms) | Status | Notes |
|---|---------|----------|---------------|--------|-------|
| 1 | `*IDN?` | Rohde&Schwarz,SMB100A,1406.6000k02/101623,3.1.19.15-3.20.390.24 | 7 | pass |  |
| 2 | `SYST:ERR?` | 0,"No error" | 2 | pass |  |
| 3 | `OUTP?` | 0 | 1 | pass |  |
| 4 | `MOD:STAT?` | 0 | 1 | pass |  |
| 5 | `FREQ:MODE?` | CW | 1 | pass |  |
| 6 | `FREQ?` | 2520000000 | 1 | pass |  |
| 7 | `POW?` | -14 | 1 | pass |  |
| 8 | `POW:ALC?` | AUTO | 1 | pass |  |
| 9 | `FM:STAT?` | 0 | 1 | pass |  |
| 10 | `FM:SOUR?` | INT | 1 | pass |  |
| 11 | `FM:DEV?` | 3500000 | 1 | pass |  |
| 12 | `LFO?` | 0 | 1 | pass |  |
| 13 | `LFO:FREQ?` | 500 | 1 | pass |  |
| 14 | `LFO:VOLT?` | 3 | 1 | pass |  |
| 15 | `LFO:SHAP?` | SQU | 1 | pass |  |
| 16 | `SWE:MODE?` | AUTO | 1 | pass |  |
| 17 | `SWE:SPAC?` | LIN | 1 | pass |  |
| 18 | `SWE:FREQ:STEP?` | 500000 | 1 | pass |  |
| 19 | `SWE:FREQ:DWEL?` | 0.5 | 1 | pass |  |
| 20 | `FREQ:STAR?` | 2820000000 | 1 | pass |  |
| 21 | `FREQ:STOP?` | 2920000000 | 1 | pass |  |

## OE1022D

| # | Command | Response | Duration (ms) | Status | Notes |
|---|---------|----------|---------------|--------|-------|

## Forbidden Command Audit

The following patterns were explicitly blocked by the snapshot tool:
- `OUTP ON`
- `MOD:STAT ON`
- `FM:STAT ON`
- `FREQ:MODE SWE`
- `SWE:EXEC`
- `INIT`
- `RUN`
- `RST`
- `*RST`
- `SSETD`
- `RSETD`
