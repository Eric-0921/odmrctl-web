# Real-Device Read-Only Station Snapshot

> **Safety Audit**: Only read-only queries were sent.
> No state-changing commands were transmitted.
> All commands validated against hard-coded allow-lists.

- **Date**: 2026-05-30 12:41:17 UTC
- **Operator**: lab-snapshot CLI (human supervised)

## SMB100A

| # | Command | Response | Duration (ms) | Status | Notes |
|---|---------|----------|---------------|--------|-------|
| 1 | `*IDN?` | Rohde&Schwarz,SMB100A,1406.6000k02/101623,3.1.19.15-3.20.390.24 | 3.9 | pass |  |
| 2 | `SYST:ERR?` | 0,"No error" | 1.7 | pass |  |
| 3 | `OUTP?` | 0 | 1.4 | pass |  |
| 4 | `MOD:STAT?` | 0 | 1.5 | pass |  |
| 5 | `FREQ:MODE?` | CW | 1.4 | pass |  |
| 6 | `FREQ?` | 2520000000 | 1.7 | pass |  |
| 7 | `POW?` | -14 | 1.6 | pass |  |
| 8 | `POW:ALC?` | AUTO | 1.4 | pass |  |
| 9 | `FM:STAT?` | 0 | 1.5 | pass |  |
| 10 | `FM:SOUR?` | INT | 1.4 | pass |  |
| 11 | `FM:DEV?` | 3500000 | 1.7 | pass |  |
| 12 | `LFO?` | 0 | 1.5 | pass |  |
| 13 | `LFO:FREQ?` | 500 | 1.6 | pass |  |
| 14 | `LFO:VOLT?` | 3 | 1.6 | pass |  |
| 15 | `LFO:SHAP?` | SQU | 1.4 | pass |  |
| 16 | `SWE:MODE?` | AUTO | 1.4 | pass |  |
| 17 | `SWE:SPAC?` | LIN | 1.5 | pass |  |
| 18 | `SWE:FREQ:STEP?` | 500000 | 1.6 | pass |  |
| 19 | `SWE:FREQ:DWEL?` | 0.5 | 1.6 | pass |  |
| 20 | `FREQ:STAR?` | 2820000000 | 1.6 | pass |  |
| 21 | `FREQ:STOP?` | 2920000000 | 1.6 | pass |  |

## OE1022D

| # | Command | Response | Duration (ms) | Status | Notes |
|---|---------|----------|---------------|--------|-------|
| 1 | `*IDN?` | SSI LIA-OE1022D,SN:D6522078,Version:Ver6.3200831 | 601.2 | pass |  |
| 2 | `FMODD? 2` | 0 | 604.9 | pass |  |
| 3 | `RSLPD? 2` | 0 | 604.9 | pass |  |
| 4 | `FREQD? 2` | 1.58758e+02 | 605.0 | pass |  |
| 5 | `PHASD? 2` | 0.00 | 603.3 | pass |  |
| 6 | `ISRCD? 2` | 0 | 605.0 | pass |  |
| 7 | `SENSD? 2` | 24 | 604.9 | pass |  |
| 8 | `OFLTD? 2` | 9 | 601.3 | pass |  |
| 9 | `OFSLD? 2` | 1 | 605.0 | pass |  |
| 10 | `HARMD? 2` | _(timeout)_ | 2605.3 | timeout | no response |
| 11 | `RALL?` | 3F 0F 38 82 35 CB 1F AC 3F 0F 43 C1 98 26 F9 F9 3F 0F 4E CC 11 44 06 33 3F 0F... | 604.2 | pass | binary frame, 4096 bytes total, first 256b hex shown |

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
