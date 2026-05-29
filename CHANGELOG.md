# Changelog

## [0.2.0] GUI-M0 — Mock-only Tauri + React Viewer

### Added

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
