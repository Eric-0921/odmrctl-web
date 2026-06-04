# Audit B — Device Connection Contract

> Audit date: 2026-05-31  
> Base commit: `4627980`

## Scope

All probes in `tools/lab/common_preflight/src/`:
- `smb_probe.rs` — SMB100A
- `oe_probe.rs` — OE1022D
- `maynuo_probe.rs` — Maynuo M8812
- `cni_laser_probe.rs` — CNI Laser

Plus auto-discovery behavior in each device crate.

## 1. Probe Classification

### Classification Definitions

| Class | Definition |
|-------|------------|
| `identity_only` | Sends only `*IDN?` or equivalent identity query |
| `query_only` | Queries state (error queue, output status, etc.) but never sends set commands |
| `safe_state_probe` | Writes commands, but only to establish or verify a safe/known state |
| `safe_write_probe` | Writes bounded operational parameters under safety limits |
| `operator_approved_probe` | Requires explicit `--operator-approve` flag to run |

### Classification Table

| Device | Probe File | Classification | Writes Commands? | Alters Device Mode? | Requires Op Approval? | Safe for GUI Auto-run? |
|--------|-----------|----------------|------------------|---------------------|----------------------|------------------------|
| **SMB100A** | `smb_probe.rs` | `query_only` | No (queries only) | No | No | ✅ Yes |
| **OE1022D** | `oe_probe.rs` | `identity_only` | No (`*IDN?` only) | No | No | ✅ Yes |
| **Maynuo M8812** | `maynuo_probe.rs` | `safe_state_probe` | Yes (`SYST:REM`, `SYST:LOC`) | Yes (temp remote→local) | No | ✅ Yes |
| **CNI Laser** | `cni_laser_probe.rs` | `safe_state_probe` | Yes (`laser_off`, `set_power(0)`) | Yes (ensures OFF) | No | ✅ Yes |

### Framework-Level Operator Approval

`run_station_preflight()` has `operator_approved: bool`, but this gates **extended preflight mode** (ledger shows prior unsafe state), not per-device approval. Under normal conditions, all probes run without operator approval.

## 2. Per-Device Detailed Analysis

### SMB100A — `query_only`

**Probe sequence:**
1. `*IDN?` — identity
2. `SYST:ERR?` × up to 50 — error queue drain
3. `OUTP?` — RF output state
4. `MOD:STAT?` — modulation state
5. `FM:STAT?` — FM state

**Key observations:**
- No set commands. All traffic is queries.
- `safe_state.confirmed = false` if any state is non-OFF.
- Auto-discovery: TCP connect + `*IDN?` on port 5025 across 42 IPs (2 subnets, 300ms timeout cap).

**Risk:** 42 TCP connection attempts may trigger lab network IDS alerts. No broadcast packets.

### OE1022D — `identity_only`

**Probe sequence:**
1. `*IDN?` at 921600 baud
2. `port.clear(Input)` — flush input buffer
3. No error queue query (OE1022D has no SCPI error queue)
4. `safe_state.confirmed = true` hardcoded

**Key observations:**
- Minimal probe: literally only `*IDN?` is sent.
- No state queries, no safe-state verification.
- Auto-discovery enumerates USB serial ports, filters USB/PL2303/FTDI/CP210x.

**Risk:** Briefly opens every USB serial port. 500ms sleep after write means slow enumeration.

### Maynuo M8812 — `safe_state_probe`

**Probe sequence:**
1. `*IDN?` — identity (strict SN matching)
2. `SYST:REM` — **WRITE** enter remote mode
3. `OUTP?` — query output state
4. `MEAS:CURR?` — query measured current
5. `SYST:LOC` — **WRITE** return to local mode

**Key observations:**
- Writes: `SYST:REM` and `SYST:LOC` are explicit set commands.
- Mode alteration: temporarily remote; guarantees `SYST:LOC` attempt on exit (`let _ =` pattern).
- Safe-state logic: `safe = !output_on && current_ma.abs() < 1.0`.
- `safe_zero_and_local()` utility (not called by probe) does full zeroing.

**Risk:** If process crashes between `SYST:REM` and `SYST:LOC`, device left in remote mode. Mitigation: `let _ =` ensures attempt even on panic path (though not guaranteed).

### CNI Laser — `safe_state_probe`

**Probe sequence (explicit path):**
1. `laser_off` frame — **WRITE** binary frame (`55 AA 03 00 03`)
2. Read echo (optional)

**Probe sequence (auto-discovery):**
1. Skip SCPI-speaking ports (`*IDN?` filter)
2. `laser_off` frame — **WRITE**
3. `set_power(0)` frame — **WRITE** (`55 AA 05 01 00 00 06`)
4. Verify exact byte echo on both frames

**Key observations:**
- Module-level safety contract documented: probe ONLY sends `laser_off`; NEVER sends `laser_on` or nonzero power.
- `laser_on_sent: false`, `nonzero_power_sent: false` hardcoded in report.

**Risk:** During auto-discovery, non-SCPI USB serial ports receive binary frames. Echo-verification reduces false positives.

## 3. Auto-Discovery Risk Table

| Device | Discovery Method | Scope | Side Effects | Mitigation | Risk Level |
|--------|-----------------|-------|-------------|------------|------------|
| SMB100A | TCP scan port 5025 | 42 IPs, 2 subnets | Network scan noise | 300ms timeout; stops at first match | Low |
| OE1022D | Serial enum + `*IDN?` | All USB serial ports | Briefly opens each | Skips non-USB; 2s timeout | Low |
| Maynuo M8812 | Serial enum + `*IDN?` | All USB serial ports | Briefly opens each | SN strict match | Low |
| CNI Laser | Serial enum + frame echo | Non-SCPI USB serial | Sends binary frames | SCPI-filter skip; echo verify | Low |

## 4. Decision

All four probes are **safe for automated GUI preflight** with no per-device operator approval. The station-level `operator_approved` flag gates extended mode (ledger unsafe history), which is the correct design.

Auto-discovery risks are bounded and acceptable for a lab environment. No additional approval gating required.
