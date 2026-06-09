# Changelog

## [Unreleased] — M5C-A Follow-up: Config / Replay / Laser Activation + Hardware Run Readiness

### Added

- **`odmr-laser` crate — CNI Laser Layer 1 Driver**
  - New workspace member: binary serial protocol for CNI laser PSU-SR
  - `LaserFrame`: `set_power()`, `laser_on()`, `laser_off()`, `to_bytes()`, `parse()`
  - `LaserClient`: `open()`, `set_power()`, `set_enabled()`, `emergency_off()`, `identity_or_echo_check()`
  - Protocol constants: `HEADER [0x55, 0xAA]`, `CMD_SET_POWER`, `CMD_OUTPUT`, max power 150mW (manual-derived)
  - Replaces `tools/lab/cni_laser_fake_driver`

- **`odmr-config` crate — Full Implementation (was placeholder)**
  - Canonical JSON configuration entrypoint for station/runtime loading
  - `AppConfig`, `StationConfig`, `StationDeviceConfig`, `DeviceIdentityConfig`
  - `StationSafetyConfig`, `StationCleanupPolicy`, `ArtifactPolicy`, `ReplayDefaults`, `FeatureFlags`
  - Device defaults: `Smb100aDefaults`, `Oe1022dDefaults`, `Oe1022dPllReferenceDefaults`, `PllReferenceContract`
  - `MaynuoM8812Defaults`, `CniLaserDefaults`, `EffectiveRuntimeDefaults`
  - Legacy station profile JSON compatibility loading with field normalization
  - Manual-derived defaults locked from 4 equipment manuals

- **`odmr-replay` crate — Full Implementation (was placeholder)**
  - Canonical `.rall` run artifact replay
  - `ReplaySession`, `ReplayMode` (OriginalTimestampPaced / ParseOnly / AsFastAsPossible)
  - `ReplaySource::CanonicalRunDirectory` with legacy rawbin compatibility
  - `open_replay_session()`, `replay_trace()`, `migrate_legacy_run_to_canonical()`
  - `MigrationReport` for legacy → canonical run directory conversion

- **Hardware Run Execution (`odmr-executor`)**
  - `HardwareRunConfig` / `HardwareRunStep` — structured hardware run parameters
  - `HardwareMagAxisTarget`, `HardwareRfSweep`, `HardwareOeAcquisition`, `HardwareLaserTarget`
  - `HardwareProgress` / `RunControl` — atomic stop flag + progress tracking
  - `run_hardware()` — real hardware execution entry point
  - `ExecutorError` extended with `Config` and `Runtime` variants

- **SMB100A Internal RF Sweep Command Set**
  - `set_freq_mode_sweep()`, `set_sweep_mode()`, `set_sweep_trigger_source()`
  - `set_sweep_spacing()`, `set_sweep_shape()`, `set_sweep_step_hz()`, `set_sweep_dwell_ms()`
  - `trigger_sweep_immediate()`, `execute_frequency_sweep()`, `query_sweep_running()`
  - `set_sweep_output_start_v()`, `set_sweep_output_stop_v()`, `set_sweep_lf_output()`
  - 13 golden tests + fake device state expansion

- **Magnetic Runtime Bridge Types (`odmr-mag`)**
  - `MagRuntimeCommand`: ApplyCurrent, QueryReadback, LockZero, CleanupAxis
  - `MagReadback`, `MagCleanupReport`

- **GUI Replay Commands**
  - `open_canonical_run_replay` — open canonical run directory as replay session
  - `replay_canonical_step` — step trace replay with 3 modes
  - `migrate_legacy_run_directory` — legacy rawbin → canonical migration
  - `export_experiment_plan_json` — experiment plan JSON export

- **Decision Documents**
  - `docs/decisions/config-compatibility-mapping.md` — canonical↔legacy field mapping
  - `docs/decisions/no-hardware-completion-checklist.md` — pure-software tasks without hardware

### Changed

- Workspace members: 16 → 17 crates (`odmr-laser` added)
- `odmr-preflight` types migrated to `odmr_config::StationConfig` / `DeviceTransportConfig`
- `apps/desktop/src-tauri/Cargo.toml` — added `odmr-config`, `odmr-executor`, `odmr-replay`, `odmr-laser`; removed `cni_laser_fake_driver`
- `examples/` — removed 10 LabVIEW-generated command JSON files (oe1022d ×4, smb100a ×6)
- `odmr-mag` doc comment: "mock-only" → "magnetic field planning, runtime bridge types"

---

## [0.5.0] — M5C-A: Device Workbench V1 + Live Server + Experiment Planning

### Added

- **M5C-A Device Workbench V1**
  - `apps/desktop/` transformed from GUI-M0 mock viewer to real device workbench
  - Station page: load station profile, run unified preflight, view preflight report
  - 4 minimal device panels with typed get/set + readback:
    - SMB100A: frequency, power, output state, modulation state
    - OE1022D: reference frequency, sensitivity, time constant, filter slope, input source
    - Magnetic: per-axis current, output state, zero-lock, sequential run
    - Laser: off-only preflight integration, power state readback
  - All set operations protected by safety gate; frontend still has zero direct hardware access

- **Experiment Planning**
  - `ExperimentPlanPage.tsx` — field grid scan recipe generation (1D / 2D / 3D)
  - `experiment_plan.rs` — Tauri command backend for plan creation, validation, and export
  - Supports ODMR field grid scans with RF sweep + magnetic axis sweep combinations
  - Example recipes: `odmr_field_grid_1d/2d/3d.example.json`
  - Import guide: `docs/experiment-plan/field_grid_scan_import_guide.md`

- **Experiment Plan Execution Launcher (preview)**
  - Preview UI for launching experiment plans from the workbench
  - Integration with workbench state and station preflight status

- **OE1022D Real-Time Chart — Phase 1 Sidecar**
  - `crates/odmr-live-server/` — new workspace crate
  - Actix-web HTTP server consuming `RallCollector` frames into ring buffer
  - Endpoints: `GET /api/trace` (snapshot), `GET /api/stats` (collector status)
  - CORS-enabled for browser-based chart frontend
  - 50 samples/frame at 1kHz-equivalent resolution; 2000-point ring capacity default

### Changed

- GUI boundary statement updated: no longer "mock-only", but "frontend禁止直接硬件访问" — all device interactions go through typed Tauri Command API
- `apps/desktop/src-tauri/Cargo.toml` — added `odmr-preflight`, `odmr-device`, panel modules

### Documents

- `docs/gui-screenshots/GUI-M0-Pages-Documentation.md` — merged per-page markdowns into single doc
- `docs/equipment_manual/oe1022d/` — 4 new remote command reference docs (input filter, output/formula, reference signal, UI-to-remote mapping)

---

## [0.4.0] — M5B: System Scan Recipe + Artifact Viewer

### Added

- **Recipe-M5B-A: System Scan Recipe Parser, Compiler, Safety**
  - `odmr-recipe`: `SystemScanRecipe`, `SystemSweepDefinition`, `SweepShape`, `AcquisitionPolicy`
  - `parse_system_scan_recipe()`, `validate_system_scan_recipe()` — 8 harness tests
  - `odmr-compiler`: `expand_system_scan_recipe()` → `ResolvedSystemScan`
    - Cartesian product of sweeps in `sweep_order`
    - setup + measure + cleanup phases with traceability
    - `build_system_scan_dry_run()` — 11 harness tests
  - `odmr-safety`: `build_system_scan_safety_report()` with 15 checks
    - Laser disabled, no safety limit override, magnetic current within limit, operator approval required
    - 5 harness tests
  - Examples: `m5b_rf_mag_oe_system_scan.{recipe,resolved,safety,dry_run}.json`

- **M5B-B-GUI-A: System Scan Artifact Viewer**
  - `SystemScanPage.tsx` — 7-tab read-only artifact viewer
    - Overview, Recipe, Station Safety, Device Profiles, Resolved Steps, Safety Report, Dry Run
  - `src/types/m5b.ts` — type definitions
  - `src/mock-data/m5b/` — 6 JSON artifacts bundled at build time

- **OE1022D Buffer Probe Toolkit**
  - `tools/lab/oe1022d_buffer_probe/` — RALL? benchmark, TRCAD? brute-force, collector test
  - Buffer sampling commands added to `odmr-oe1022d` command catalog (SRATD, SLEND, SSLED, etc.)

- **OE1022D 30-Minute RALL? Stability Test**
  - 35,713 frames captured over 30 minutes
  - 1.2% duplicate rate, 13.3ms average read time, zero parse errors, zero buffer overruns
  - Raw binary `.rall` preservation (ADR-005 raw-first落地)
  - Report: `docs/lab-bringup/oe1022d_rall_continuous_test_2026-06-06.md`

### Documents

- `docs/decisions/oe1022d-rall-continuous-benchmark.md`
- `docs/decisions/oe1022d-rall-stability-validation.md`
- `docs/decisions/oe1022d-rall-fast-read-correction.md`
- `docs/decisions/oe1022d-high-speed-buffer-acquisition.md`
- `docs/lab-bringup/oe1022d_buffer_1khz_validation_final_report.md`
- `docs/lab-bringup/oe1022d_buffer_1khz_validation_status_report.md`
- `docs/lab-bringup/oe1022d_rall_continuous_test_2026-06-06.md`

---

## [0.3.1] — M5A: RF + Mag + OE Combined Run + Preflight Crate

### Added

- **RF + Mag + OE Minimum Combined Run**
  - `tools/lab/rf_mag_oe_minimal_run/` — Mag-M5A milestone
  - Real hardware verified: SMB100A RF output + Maynuo M8812 magnetic coil + OE1022D lock-in acquisition
  - Step sequencer with preflight integration, safety limits, and emergency shutdown

- **`odmr-preflight` Crate (extracted from `common_preflight`)**
  - New workspace crate at `crates/odmr-preflight/`
  - `tools/lab/common_preflight/` reduced to thin CLI wrapper
  - Responsibilities: device discovery, identity verification, safe-state probing, cross-process device locks (`DeviceLock` via POSIX `flock`), StationLedger persistence, preflight report generation
  - `ProbeClass` enum: `IdentityOnly`, `QueryOnly`, `SafeStateProbe`, `SafeWriteProbe`, `OperatorApprovedProbe`
  - Public API: `run_station_preflight()`, `run_station_preflight_with_locks()`
  - Auto-discovery for all 4 device families (SMB100A, OE1022D, Maynuo, CNI Laser)

- **P6 Preflight Hardening**
  - SMB OUTP OFF sent in all error paths after RF ON step
  - Unix SIGINT handler with `ABORT_FLAG` atomic check after each step
  - Audit booleans truthful: `mag_final_output_off` / `mag_final_local_requested` only true when commands succeed
  - Post-run StationLedger update (safe/unsafe based on actual result)
  - Device locks held through full execution
  - 4 new fault-injection tests

- **OE1022D RALL? Performance Breakthrough**
  - Eliminated 800ms sleep bottleneck — single-frame read down to 12.0ms
  - Mechanical limit: 83.7 fps; effective dedup rate: 20.8 fps (~1040 pts/sec)
  - Producer-Consumer `RallCollector` (`collector.rs`)
    - Independent polling producer thread + bounded mpsc channel(8)
    - 48ms节拍, X[0] value dedup, 1ms fast-poll retry
    - Non-blocking Drop (does not join producer)

### Fixed

- M5A error paths: SMB OUTP OFF guarantee, truthful cleanup booleans, SIGINT abort
- `rf_mag_oe_minimal_run` fault-injection coverage (4 tests)

### Documents

- `docs/lab-bringup/p6_m5a_audit_b_device_connection_contract.md`
- `docs/lab-bringup/p6_2_error_path_hardening.md`
- `docs/lab-bringup/preflight_checklist_template.md`
- `docs/lab-bringup/device_connection_initialization_audit.md`
- `docs/lab-bringup/cni_laser_preflight_plan.md`

---

## [0.3.0] — M4: Magnetic Control Tools + GUI Analysis/Dry-Run Viewers

### Added

- **`odmr-mag` Crate — 3-Axis Magnetic Field Control**
  - Layer 1 driver-planning crate for Maynuo M8812 based magnetic coils
  - Responsibilities: coil matrix modeling, field-current conversion, magnetic safety policy (current limits, ramp rate limits, settle time), zero-lock workflow, sequential multi-axis run, mock state machine
  - Status: Mag-M5A real hardware verified

- **`odmr-maynuo-m8812` Crate — Serial Driver**
  - Layer 1 USB-to-serial driver for Maynuo M8812 programmable DC power supply
  - SCPI-like commands: `*IDN?`, `SYST:REM`, `CURR`, `OUTP`, `MEAS:CURR?`, `SYST:LOC`
  - Identity parsing with SN matching, event timeline, shutdown ordering

- **Magnetic Lab Bringup Toolchain**
  - `tools/lab/maynuo_m8812_identity_probe/` — Mag-M2A: identity-only probe with SN verification
  - `tools/lab/maynuo_m8812_zero_baseline/` — Mag-M2B: zero-mode output-on readback + lock-zero baseline
  - `tools/lab/maynuo_m8812_recur_microtest/` — Mag-M3: single-axis 10mA recurring microtest
  - `tools/lab/maynuo_m8812_sequential_axis_run/` — Mag-M4: sequential multi-axis low-current run

- **CNI Laser Tools**
  - `tools/lab/cni_laser_fake_driver/` — Laser-M1: fake driver + protocol frame definitions
  - `tools/lab/cni_laser_microtest/` — Laser-M3: low-power microtest
  - Off-only preflight integration in `common_preflight`

- **VISA A/B Benchmark**
  - `tools/lab/visa_probe/` — connection layer diagnostic for R&S VISA on macOS
  - Auto-set `LIB_VISA_NAME` for R&S VISA

- **GUI M4.0: Read-Only Analysis Viewer**
  - Analysis artifact viewer page (mock data, read-only)

- **GUI M4.1: Recipe Dry-Run Viewer**
  - Full dry-run plan visualization with 201 steps
  - Device/action parsing, parameter display, step grouping

### Changed

- Lab bringup tools: 26 total (up from 12 in M3)
- `odmr-oe1022d` command catalog completed with buffer sampling commands
- `odmr-smb100a` command catalog gaps filled (LFO:SIMP, FM:MODE parameter enums corrected)

### Documents

- `docs/lab-bringup/m3_6_odmr_like_analysis_artifacts.md`
- `docs/prd/` updates: CNI laser protocol, MAYNUO M8812 re-reverse-engineering, coil constants, device fingerprinting

---

## [0.2.2] — M3 Lab Bringup Phase 2: SMB100A Controlled RF/FM/MOD Micro-tests

### Added

- **M3.0-A: SMB100A preflight error queue clearance (query-only)**
  - `tools/lab/smb100a_preflight_clearance/` — Rust CLI tool
  - Verifies SMB100A in clean, safe, RF-OFF state before any RF output milestone
  - Hard-coded query allowlist (13 queries) + forbidden pattern rejection
  - Diagnostic `*CLS` mode with explicit operator approval gate
  - Safety: rejects if OUTP? ≠ 0 or MOD:STAT? ≠ 0; no set commands reach transport in normal mode

- **M3.0-B: SMB100A RF ON/OFF micro-test (fixed frequency, low power, no modulation)**
  - `tools/lab/smb100a_rf_microtest/` — Rust CLI tool
  - Controlled OUTP ON/OFF with full command audit and safety evidence
  - Operator approval gate for OUTP ON (`--operator-approves-rf-on`)
  - Preflight checks: OUTP=OFF, MOD:STAT=OFF, SYST:ERR clean
  - Emergency shutdown: OUTP OFF if failure after RF ON
  - Hard limits: power ≤ -10 dBm, duration ≤ 5 s

- **M3.1: SMB100A fixed-frequency FM/MOD ON/OFF micro-test**
  - `tools/lab/smb100a_fm_mod_microtest/` — Rust CLI tool
  - FREQ = 2.882 GHz, POW = -30 dBm, FM:DEV = 4 MHz verified
  - FM:SOUR INT → FM:STAT ON → MOD:STAT ON → OUTP ON → hold → OUTP OFF → MOD:STAT OFF → FM:STAT OFF
  - Internal LF generator parameter support (LFO:FREQ / LFO:SHAP / LFO:VOLT, LFO output kept OFF)
  - Operator approval gate for FM:STAT ON, MOD:STAT ON, OUTP ON (`--operator-approves-fm-mod-on`)
  - Full command audit JSONL, preflight check, forbidden command check, emergency shutdown evidence
  - `--leave-fm-config-enabled` flag for FM configuration persistence
  - 35 characterization tests covering safety gates, allowlists, forbidden patterns, serialization

- **M3.1.1: Modularize M3.1 tool (behavior-preserving refactor)**
  - main.rs reduced from ~2900 lines to 44 lines
  - 10 modules extracted: app, cli, types, timeline, artifacts, safety, transport, sequence, shutdown, tests
  - All CLI flags, defaults, command sequences, JSON schemas, and artifact paths preserved

### Fixed

- **M3.1 safety hardening** (8 fixes)
  - Reject SCPI semicolons (command chaining) in all validation functions
  - Validate `--lf-shape` against SMB100A manual §6.13.6 allowlist (SIN/SQU/TRI/SAW/ISAW)
  - Replace `?` with `unwrap_or_else` after state-changing commands so cleanup always runs
  - Expand emergency shutdown trigger to cover FM/MOD-enabled states, not only RF ON
  - Compare FREQ?/POW?/FM:DEV? readback against requested values after each set
  - Fix RF-ON timing math (was off by one `delay_ms`, ~50ms)
  - Add `drain_buffer()` before emergency shutdown verification queries
  - 40 total tests, all passing

### Changed

- Lab bringup tools: 12 total (up from 3 in M2): smb100a_preflight_clearance, smb100a_rf_microtest, smb100a_fm_mod_microtest, smb100a_safe_set, oe1022d_acquire, oe1022d_logged_acquire, oe1022d_rall_capture, oe1022d_run_audit, oe1022d_smb_fake_bridge, oe1022d_smb_query_bridge, executor_shadow_run, snapshot

### Documents

- `docs/lab-bringup/m3_0_preflight_error_queue_clearance_plan.md` — M3.0-A test plan
- `docs/lab-bringup/smb100a_safe_set_audit_2026-05-31.md` — safe set audit record
- `docs/lab-bringup/smb100a_command_verification.md` — command verification protocol
- `docs/lab-bringup/real_station_snapshot_2026-06-14.md` — real station snapshot

---

## [0.2.1] — M2 Lab Bringup Phase 1: Discovery, Verification, Snapshot

### Added

- **M2.0-A: macOS hardware discovery tools**
  - `tools/discover/` — Rust CLI for scanning lab instruments
    - LAN discovery: SMB100A via SCPI `*IDN?` over TCP/5025 (subnet sweep)
    - Serial discovery: OE1022D via `*IDN?` over USB CDC (port enumeration + baud probe)
    - Output: `station.lab.example.json` with detected device fingerprints
  - `scripts/lab/probe-*.sh` — bash wrapper scripts for quick discovery

- **M2.0-B: Human-in-the-loop command verification protocol**
  - `tools/manual_command_verify/` — verification protocol and JSONL receipt templates
  - Human approval gate before any write/change command reaches hardware
  - Immutable JSONL receipts: command, checksum, approver, timestamp, outcome

- **M2.1: Real-device read-only station snapshot**
  - `tools/lab/snapshot/` — Rust read-only snapshot tool
    - SMB100A: 21/21 read-only queries over TCP:5025 (freq, power, mod state, sweep config, etc.)
    - OE1022D: 10/11 read-only queries over serial @ 921600 baud (freq, phase, sensitivity, time constant, filter slope, etc.)
  - Double safety gate: hard-coded query whitelist + forbidden substring rejection (`OUTP ON`, `*RST`, `INIT`, etc.)
  - Safety gate tests: `tools/lab/snapshot/tests/safety_gate_test.rs` — 9 rejection cases
  - OE1022D serial transport notes documented in `tools/lab/snapshot/README.md`

### Changed

- `examples/station.lab.example.json` — OE1022D baud rate corrected from 115200 → 921600 (real device requirement)

### Documents

- PRD updates: CNI laser protocol, MAYNUO M8812 re-reverse-engineering results, measured coil constants, magnetic coil control workflow, device fingerprinting strategy

---

## [0.2.0] GUI-M0 — Mock-only Tauri + React Viewer

### Added

- **GUI-M0-06: Final audit, accessibility pass, README, and acceptance checklist**
  - Boundary audit: zero executable hardware-control logic; no prohibited npm/Rust dependencies
  - Button audit: all real-control buttons verified disabled with helper text and `cursor: not-allowed`
  - Accessibility: added `scope="col"` to all table `<th>` elements; added `aria-expanded` to Recipe JSON preview toggle
  - Root `README.md`: added GUI-M0 run instructions, known limitations, and mock-only boundary section
  - `docs/gui/GUI-M0-implementation-notes.md`: created — pages, components, mock data strategy, disabled controls, boundary checklist, M1/M2 roadmap

- **GUI-M0-05: Dry Run, Safety, Events, Raw Data, and About pages**
  - Dry Run: device/action parsed from `device_actions` instead of hardcoded; parameters show full sweep coordinate key=value
  - Safety: findings severity badge with color + text label (info/warning/error); empty state preserved
  - Events: level badge expanded to support error/danger/unknown levels gracefully
  - Raw Data: added Manifest metadata section (run ID, recipe hash, resolved recipe ID, safety report ID, created at)
  - About: boundary statement, Allowed/Forbidden lists, M1/M2 integration path — already complete

- **GUI-M0-04: Dashboard, Devices, and Recipe pages**
  - Dashboard: added "Run name" and "Required devices" summary cards; fixed 4 disabled button helper texts to match spec
  - Devices: added "Last known state: static bundled data" to all 4 device cards; adjusted disabled control labels to Connect/Probe/Configure/Output ON/MOD ON; added "Mock viewer only" helper label
  - Recipe: added "Schema version" card, "Main sweep parameters" panel (start/stop/step/order/points), expanded metadata table with Recipe ID and Schema version
  - Recipe: added read-only collapsible JSON preview panel (Show JSON / Hide JSON toggle)

- **GUI-M0-03: AppShell, navigation, design tokens, persistent mock-only state**
  - `MockModeContext` — React Context with localStorage persistence for mock-mode state
  - Banner collapse/expand toggle with persisted preference (`odmr-gui-m0-mode`)
  - Enhanced CSS design tokens: focus ring (`--focus-ring-*`), transitions (`--transition-*`), table density (`--table-density-*`)
  - Global `:focus-visible` styles for visible keyboard focus on all interactive elements
  - Global table base styles (`th`, `td`, `tr:hover`) using tokenized density
  - TopStatusBar refactored to consume `MockModeContext`; status items with `title` accessibility
  - MockOnlyBanner refactored to be dismissible/expandable via context
  - Scrollbar styling for webkit browsers
  - SideNav disabled items remain static (no routing needed)

- **GUI-M0-02: Bundled mock data layer**
  - `src/mock-data/` with static snapshots from example artifacts
  - Full-copy modules: recipe, dry-run plan (201 steps), safety report, run manifest
  - Parsed JSONL arrays: 407 events, 201 index entries
  - Raw artifact metadata only: filename + 3,216 bytes (no binary parsing)
  - Helper functions: `getRunSummary()`, `getRecipe()`, `getDryRunSteps()`, `getSafetyReport()`, `getEvents()`, `getIndexEntries()`, `getRawArtifactSummary()`
  - All 7 data-displaying routes updated to consume bundled mock data
  - No filesystem access at runtime

- **GUI-M0-01: Scaffold mock-only Tauri + React app**
  - Tauri 2.x + Vite + React + TypeScript skeleton under `apps/desktop/`
  - 8 routed pages: Dashboard, Devices, Recipe, Dry Run, Safety, Events, Raw Data Preview, About
  - Industrial blue-white-gray design with CSS tokens
  - Persistent `GUI-M0 MOCK VIEWER` banner on all pages
  - Side navigation with 8 active items + 4 disabled future items
  - Top status bar showing phase, mode, safety decision, backend state
  - All real-control buttons disabled with explanatory reasons
  - Minimal Rust backend with single static `app_metadata` command
  - No hardware dependencies in frontend or backend

### Changed

- `docs/prd/08_gui_tauri_chart_prd_v0.2.md` → `docs/prd/08_gui_tauri_chart_prd_v0.3.md`

### Documents

- `docs/adr/ADR-006-gui-m0-mock-only-boundary.md` — ADR for GUI-M0 mock-only boundary
- `docs/GUI/GUI-M0-spec.md` — UI/UX wireframe spec
- `docs/prd/deep-research-report.md` — GUI-M0 specification research report

---

## [0.1.5] — M1.5: Executor Mock-Run End-to-End

### Added

- `odmr-executor` crate with full executor implementation
  - Mock-run end-to-end execution loop
  - Step dispatch to fake device drivers
  - Event emission and index writing
- `examples/runs/basic_odmr_mock_executor_run/` — complete mock executor run artifacts
  - 407 events, 201 index entries, 3,216-byte raw binary
  - Locked metadata: recipe, resolved recipe, dry-run plan, safety report, station snapshot
  - `execution_report.schema.json`
- Integration test: `crates/odmr-executor/tests/run_mock_end_to_end.rs`

---

## [0.1.4] — M1.4: Run Directory Logging and Raw-First Artifact Writer

### Added

- `odmr-logging` crate with run directory management
  - Raw-first artifact writer (bin + index.jsonl + events.jsonl)
  - Metadata lock files for reproducibility
  - Run manifest generation
- `examples/runs/basic_odmr_mock_run/` — initial mock run artifacts
- `run_event.schema.json`, `run_manifest.schema.json` updates
- Integration test: `crates/odmr-logging/tests/generate_run_directory.rs`

---

## [0.1.3] — M1.3: Static Safety Interlock Engine

### Added

- `odmr-safety` crate with static safety interlock engine
  - Recipe-level safety limit checking
  - Action-by-action hazard analysis
  - Safety report generation (`allow` / `deny`)
- `examples/safety/` — safety report examples
- `safety_report.schema.json`
- Integration test: `crates/odmr-safety/tests/generate_safety_reports.rs`

---

## [0.1.2] — M1.2: Pure Recipe Compiler and Dry-Run Plan

### Added

- `odmr-compiler` crate with pure recipe compiler
  - Recipe → resolved recipe transformation
  - Dry-run plan generation (201 steps for basic ODMR)
  - Parameter validation against station schema
- `examples/resolved/` — resolved recipe and dry-run plan examples
- `dry_run_plan.schema.json`
- `odmr-recipe` type extensions
- Integration test: `crates/odmr-compiler/tests/generate_examples.rs`

---

## [0.1.1] — M1.1: Foundation Audit and Command Catalogs

### Added

- `odmr-oe1022d` command catalog — comprehensive SCPI-style command definitions
- `odmr-smb100a` command catalog — RF signal generator SCPI command definitions
- `odmr-recipe` validation module (`src/validation.rs`)
- `odmr-types` shared type definitions
- JSON Schemas: `recipe.schema.json`, `resolved_recipe.schema.json`, `run_event.schema.json`, `safety_limit.schema.json`, `station.schema.json`
- `scripts/check-schema-examples.sh` — schema validation script
- `scripts/check-frontend-hardware.sh` — frontend hardware dependency guard
- `reverse_application/AGENTS.md` — reverse engineering notes
- `docs/tasks/agent-tasks-completion-report.md` — task tracking

### Changed

- `odmr-oe1022d` fake device corrections and `lib.rs` API surface expansion
- `odmr-smb100a` command fixes and `lib.rs` API surface expansion
- `odmr-recipe` `Cargo.toml` and `lib.rs` extensions

---

## [0.1.0] — Bootstrap Workspace and Mechanical Checks

### Added

- Rust workspace with 12 crates (`odmr-compiler`, `odmr-config`, `odmr-device`, `odmr-executor`, `odmr-harness`, `odmr-logging`, `odmr-oe1022d`, `odmr-recipe`, `odmr-replay`, `odmr-safety`, `odmr-smb100a`, `odmr-types`)
- Each crate with `Cargo.toml`, `README.md`, and stub `src/lib.rs`
- `odmr-oe1022d` and `odmr-smb100a` with full fake device implementations
- `odmr-recipe` with types (`src/types.rs`) and validation (`src/validation.rs`)
- PRD v0.2 (13 documents: `00_main` through `12_agent_workflow`)
- ADR (5 documents: ADR-001 through ADR-005)
- Equipment manuals: OE1022D (9 docs), SMB100A (15 docs)
- Reverse application analysis: DMSkin / DataReader2 source tree
- Example artifacts: recipe, resolved recipe, command catalogs
- JSON Schemas (initial set)
- Mechanical check scripts: `check-consistency.sh`, `check-agents-md.sh`, `check-command-catalog.sh`, `check-docs-links.sh`, `check-prd-adr-index.sh`, `check-realtime-csv.sh`
- `.githooks/pre-commit` — consistency guard
- `.github/workflows/ci.yml` — CI pipeline
- `AGENTS.md`, `CONTRIBUTING.md`, `README.md`
