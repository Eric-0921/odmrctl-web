# Preflight Checklist Template

**Run ID**: `________`  
**Operator**: `________`  
**Date/Time**: `________`  
**Station Profile**: `________`

---

## Phase A: Passive Preflight (Automated)

Run: `common-preflight --station-profile <profile> --preflight-only`

| Check | Required | Result | Evidence |
|-------|----------|--------|----------|
| All devices reachable | YES | ☐ PASS / ☐ FAIL | `station_preflight_report.json` |
| All identities verified | YES | ☐ PASS / ☐ FAIL | SN match in report |
| All safe states confirmed | YES | ☐ PASS / ☐ FAIL | RF=OFF, Mag=OFF, Laser=OFF |
| Error queues empty | YES | ☐ PASS / ☐ FAIL | `error_queue: []` in report |
| Device locks acquired | YES | ☐ PASS / ☐ FAIL | No `DeviceBusy` errors |

**If ANY check fails → STOP. Do not proceed to Phase B.**

---

## Phase B: Operator Approval (Manual)

| Check | Required | Initials |
|-------|----------|----------|
| I have read the experiment plan and understand the target parameters | YES | `____` |
| I have confirmed all physical safeguards are in place | YES | `____` |
| I understand the emergency shutdown procedure | YES | `____` |
| I agree to proceed with this experiment | YES | `____` |

**Approval command**: `--operator-approve`

---

## Phase C: Armed Execution (Automated)

Once Phase A + B pass:

1. Preload target state (FREQ, POW, CURR setpoints) **without enabling outputs**
2. Verify setpoints with readback queries
3. Enable outputs ONLY inside bounded step window
4. Immediately verify and record final safe state

---

## Emergency Procedures

| Scenario | Action |
|----------|--------|
| RF stuck ON | `recipe_two_device_run` sends `OUTP OFF`; if fails, power cycle SMB100A |
| Mag stuck ON | `common_preflight` `safe_zero_and_local()` sends `CURR 0 → OUTP 0 → SYST:LOC` |
| Laser stuck ON | Key switch OFF → Emergency stop → Interlock open |
| Software unresponsive | Physical power switches, then contact safety officer |

---

## Post-Run Verification

| Check | Result |
|-------|--------|
| RF output confirmed OFF | ☐ |
| Magnetic output confirmed OFF (< 1.0 mA) | ☐ |
| Laser output confirmed OFF | ☐ |
| All devices returned to local mode (if applicable) | ☐ |
| Artifacts written and checksum verified | ☐ |

---

*This checklist must be completed before every real-hardware run. No exceptions.*
