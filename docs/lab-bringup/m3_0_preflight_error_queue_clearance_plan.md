# M3.0: Pre-flight Error Queue Clearance Plan

## Purpose

Before any RF ON micro-test or real ODMR run, the SMB100A instrument error queue must be verified clean. This document describes the required manually-approved clearance procedure.

## Background

During M2.8 and M2.9, `SYST:ERR?` returned `-410,"Query interrupted"` (SCPI Query INTERRUPTED). This is a benign timing artifact from sending queries before the previous response fully transmitted. However, it leaves the error queue non-empty, which blocks `eligible_for_rf_on_microtest`.

## Clearance Procedure

### Step 1: Manual Approval

A human operator must explicitly approve entering the error-queue-clearance diagnostic mode. This is a deliberate safety gate — no automated script may perform this step unsupervised.

### Step 2: Send `*CLS`

Clear the instrument status and error queue:

```scpi
*CLS
```

> **Warning**: `*CLS` clears all status registers and the error queue. It is normally forbidden in M2.8/M2.9 shadow mode, but it is required before RF ON.

### Step 3: Verify Clearance

Send `SYST:ERR?` repeatedly until it returns:

```scpi
SYST:ERR?
0,"No error"
```

If any other error is returned, document it and resolve before proceeding.

### Step 4: Re-query Snapshot

After clearance, run the standard SMB100A query-only snapshot to confirm:

```scpi
*IDN?
OUTP?
MOD:STAT?
FREQ?
POW?
POW:ALC?
FM:STAT?
FM:SOUR?
FM:DEV?
LFO?
LFO:FREQ?
LFO:VOLT?
LFO:SHAP?
SYST:ERR?
```

The final `SYST:ERR?` must return `0,"No error"`.

### Step 5: Update Station Snapshot Quality

If clearance succeeds:

- `station_snapshot_quality.status = "passed"`
- `eligible_for_rf_on_microtest = true`
- `query_interrupted_seen = false`

## Blockers Before M3.0 RF ON Micro-Test

| Blocker | Status | Resolution |
|---------|--------|------------|
| `-410` in error queue | ❌ Open | Execute clearance procedure above |
| `eligible_for_rf_on_microtest` false | ❌ Open | Requires clean error queue |
| Human approval for `*CLS` | ❌ Open | Manual operator sign-off required |

## Safety Notes

- Do **not** send `*CLS` during a running acquisition.
- Do **not** send `*CLS` inside the M2.9 default shadow mode.
- `*CLS` must only be used in a dedicated, manually-approved diagnostic sub-mode.
- After clearance, the standard M2.8/M2.9 query-only safety boundary remains in effect until the RF ON micro-test explicitly enables output.
