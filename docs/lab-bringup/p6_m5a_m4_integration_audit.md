# P6 / M5A / M4 Integration Audit — Final Report

> Audit date: 2026-05-31  
> Auditor: AI agent (compatibility freeze)  
> Base commit: `4627980` — "feat(lab): P6.1 preflight contract patch + GUI-M5A artifact viewer prep"

---

## 1. Current Latest Commit

`4627980` — P6.1 preflight contract patch + GUI-M5A artifact viewer prep.

Parent: `42e715d` — P6 hardening + CI fix (`libudev-dev`).

---

## 2. New Crates / Tools Since M4.1

### Workspace Crates (1 new)

| Crate | Added When | Status |
|-------|-----------|--------|
| `odmr-maynuo-m8812` | Mag-M2A | Real driver for Maynuo M8812 DC power supply |

### Lab Tools (8 new since M4.1 baseline)

| Tool | Stage | Type |
|------|-------|------|
| `common_preflight` | P6 | Unified station preflight |
| `maynuo_m8812_identity_probe` | Mag-M2A | Identity query |
| `maynuo_m8812_zero_baseline` | Mag-M2B | Zero-baseline setup |
| `maynuo_m8812_recur_microtest` | Mag-M3 | Recurrent setpoint test |
| `maynuo_m8812_sequential_axis_run` | Mag-M4 | Sequential axis run |
| `rf_mag_oe_minimal_run` | Mag-M5A | **RF + Mag + OE combined** |
| `cni_laser_fake_driver` | Laser-M1 | Protocol validation |
| `cni_laser_microtest` | Laser-M3 | Low-power microtest |
| `visa_probe` | Connection | VISA A/B benchmark |

### GUI Additions

| Addition | Status |
|----------|--------|
| `read_m5a_run_directory` Tauri command | Read-only, no hardware |
| `pick_m5a_run_directory` Tauri command | Read-only, no hardware |
| `m5aRun.ts` TypeScript types | Pure type definitions |
| `examples/gui/m5a_combined_run_viewer/` | Fixture data for viewer dev |

---

## 3. Dependency Risks

| Dependency | Risk Level | Mitigation |
|-----------|------------|------------|
| `serialport` in workspace (via `odmr-maynuo-m8812`) | 🟡 Low | `libudev-dev` in CI; macOS uses IOKit |
| `rsvisa` / `visa-rs` | 🟢 **None** | Not in workspace; isolated to `visa_probe` |
| `fs2` | 🟢 **None** | Only in `common_preflight` (lab tool) |
| `tauri-plugin-dialog` | 🟢 **None** | GUI-only, read-only |
| New crate count | 🟢 Low | Only 1 new workspace crate |

**No dependency conflicts detected.**

---

## 4. rsvisa / VISA Status

| Question | Answer |
|----------|--------|
| Is `rsvisa` used? | **No.** Not in any `Cargo.toml` or `Cargo.lock`. |
| Is `visa-rs` used? | **Yes**, only in `tools/lab/visa_probe/` (standalone). |
| Is VISA in the workspace? | **No.** |
| Default path or diagnostic only? | **Diagnostic only.** `visa_probe` is a benchmark tool, not on any critical path. |
| CI requirement? | **No.** CI does not build `tools/lab/visa_probe/`. |
| Feature-gate needed? | **No.** Nothing to feature-gate in the workspace. |

**Decision:** VISA remains fully isolated. No action required.

---

## 5. Device Connection Classification Table

| Device | Classification | Writes? | Alters Mode? | Op Approval? | GUI Auto-run Safe? |
|--------|---------------|---------|-------------|-------------|-------------------|
| SMB100A | `query_only` | No | No | No | ✅ Yes |
| OE1022D | `identity_only` | No | No | No | ✅ Yes |
| Maynuo M8812 | `safe_state_probe` | Yes (REM/LOC) | Yes (temp) | No | ✅ Yes |
| CNI Laser | `safe_state_probe` | Yes (off/0) | Yes (ensures OFF) | No | ✅ Yes |

**Framework note:** `operator_approved` gates extended preflight (ledger history), not per-device.

---

## 6. Auto-Discovery Risk Table

| Device | Method | Scope | Side Effects | Risk |
|--------|--------|-------|-------------|------|
| SMB100A | TCP scan port 5025 | 42 IPs, 2 subnets | Network scan noise | Low |
| OE1022D | Serial enum + `*IDN?` | All USB serial | Briefly opens each | Low |
| Maynuo M8812 | Serial enum + `*IDN?` | All USB serial | Briefly opens each | Low |
| CNI Laser | Serial enum + frame echo | Non-SCPI USB serial | Sends binary frames | Low |

**Decision:** Bounded scans, acceptable for lab environment. No operator approval required per-device.

---

## 7. GUI Tauri Command Boundary Table

| Command | Read-Only? | Hardware? | Forbidden Imports? |
|---------|-----------|-----------|-------------------|
| `app_metadata` | ✅ | ❌ | ❌ |
| `read_analysis_directory` | ✅ | ❌ | ❌ |
| `pick_analysis_directory` | ✅ | ❌ | ❌ |
| `read_recipe_file` | ✅ | ❌ | ❌ |
| `pick_recipe_file` | ✅ | ❌ | ❌ |
| `read_m5a_run_directory` | ✅ | ❌ | ❌ |
| `pick_m5a_run_directory` | ✅ | ❌ | ❌ |

**Decision:** GUI boundary is clean. 7 commands, all read-only, no hardware access.

---

## 8. `common_preflight` Compatibility Judgment

| Aspect | Judgment |
|--------|----------|
| API stability | Good — single entry point `run_station_preflight()` |
| Test coverage | 8 tests pass (lock contention, identity, laser frames) |
| Crate boundary | Currently a lab tool; not yet a workspace crate |
| GUI integration | **Must NOT be called directly from Tauri commands** |
| Future path | Extract to workspace crate after M5B stabilization |

**Decision:** Stable for lab use. Extraction to workspace crate is a post-M5B task. GUI must access it only through a Layer 3/4 runtime API, never directly.

---

## 9. StationLedger / DeviceLock Judgment

| Aspect | Judgment |
|--------|----------|
| `DeviceLock` (fs2 / flock) | Works. Cross-process isolation verified by tests. |
| `StationLedger` | JSON-based persistence. Simple but functional. |
| Ledger hygiene | **Gap:** M5A never updates ledger post-run. Stale safe flags possible. |
| Lock lifetime | **Gap:** Locks held only during preflight, not during execution. |
| Extended mode gate | Correct design — requires `--operator-approve` for unsafe history. |

**Decision:** Core mechanisms are sound. Two gaps (ledger hygiene, lock lifetime) are acceptable for lab tools but must be addressed before unsupervised automation.

---

## 10. RF / OE / Mag Merge Readiness

| Criterion | Status | Notes |
|-----------|--------|-------|
| Success-path safety | ✅ Ready | RF off → Mag setup → RF on → OE → RF off → Mag cleanup |
| Error-path safety | ❌ **Not ready** | RF may stay ON after RF-ON setup failure |
| Recipe schema Mag support | ❌ **Not ready** | No magnetic step types in recipe |
| Executor multi-device coord | ❌ **Not ready** | `odmr-executor` handles RF+OE only |
| `odmr-mag` real integration | ❌ **Not ready** | Mock-only; M5A bypasses it |
| Audit completeness | 🟡 Partial | Boolean flags can be misleading on failure |
| Emergency stop | ❌ **Not ready** | No SIGINT handler |

**Verdict:** Success-path is real-hardware verified. Error-path and automation readiness require work before merge into core pipeline.

---

## 11. CNI Laser Isolation Decision

| Aspect | Decision |
|--------|----------|
| Default recipe workflow | **NOT integrated** — CNI remains lab-only |
| `common_preflight` probe | Off-only (`laser_off`, `set_power(0)`); safe |
| Auto-discovery | Sends binary frames to non-SCPI ports; bounded risk |
| Future integration | Requires ADR, safety policy, and recipe schema extensions |

**Decision:** CNI Laser stays isolated. No default workflow integration.

---

## 12. Blockers Before M4.2 (GUI Run Launcher)

1. **Architecture decision:** GUI run launcher must call typed Rust runtime API, not lab tool subprocesses.
2. **Executor extension:** `odmr-executor` needs multi-device coordination (RF + Mag + OE).
3. **Recipe schema:** Magnetic step types needed.
4. **Safety rules:** Mag current limits, coil matrix validation.
5. **Preflight extraction:** `common_preflight` should become a workspace crate callable by executor.

---

## 13. Blockers Before M5B (Multi-Axis ODMR Acquisition)

1. All M4.2 blockers above.
2. **Error-path RF shutdown:** Fix M5A to guarantee `OUTP OFF` on any failure.
3. **SIGINT handler:** Emergency shutdown sequence.
4. **Ledger hygiene:** Post-run safe/unsafe updates.
5. **Lock lifetime:** Hold locks during execution or use executor-level resource management.
6. **Audit fidelity:** Fix misleading boolean flags.
7. **`odmr-mag` real bridge:** Connect mock models to `odmr-maynuo-m8812` driver.
8. **Multi-axis coordination:** Sequential → simultaneous axis control (if required).

---

## 14. Recommended Next Milestone

**Milestone: P6.2 — Error-Path Hardening + Preflight Crate Extraction**

Scope:
1. Fix M5A error-path RF shutdown (guarantee `OUTP OFF`)
2. Add SIGINT handler with emergency shutdown
3. Improve audit boolean accuracy
4. Extract `common_preflight` to workspace crate (`odmr-preflight` or merge into `odmr-device`)
5. Add post-run ledger update
6. Extend device locks to cover execution (or document the TOCTOU window)

This is **preparation work**, not new hardware behavior. It hardens the existing M5A tool before any recipe integration begins.

---

## Appendix: Honest Notes

| # | Issue | Severity | Action Taken |
|---|-------|----------|-------------|
| 1 | `rf_mag_oe_minimal_run` test compile error (`E0063` missing `ledger_path`) | Medium | **Fixed** in this audit |
| 2 | `visa_probe` link failure (`framework 'VISA' not found`) | Low | **Fixed** — `visa-sys` defaults to `framework=VISA` on macOS but R&S installs as `RsVisa.framework`. Added `.cargo/config.toml` with `LIB_VISA_NAME="framework=RsVisa"`. |
| 3 | AGENTS.md stale status ("Mag-M1 mock-only") | Low | **Fixed** in this audit |
| 4 | 5 clippy warnings in `rf_mag_oe_minimal_run/src/tests.rs` | Low | **Fixed** in this audit |
| 5 | `tauri-plugin-shell` initialized but unused | Very Low | Documented; optional cleanup |
| 6 | `serialport` unconditional in `odmr-maynuo-m8812` | Low | Documented; optional future feature-gate |
