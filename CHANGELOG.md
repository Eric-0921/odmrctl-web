# Changelog

## [0.2.0] GUI-M0 — Mock-only Tauri + React Viewer

### Added

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

- **GUI-M0-02: Bundled mock data layer**
  - `src/mock-data/` with static snapshots from example artifacts
  - Full-copy modules: recipe, dry-run plan (201 steps), safety report, run manifest
  - Parsed JSONL arrays: 407 events, 201 index entries
  - Raw artifact metadata only: filename + 3,216 bytes (no binary parsing)
  - Helper functions: `getRunSummary()`, `getRecipe()`, `getDryRunSteps()`, `getSafetyReport()`, `getEvents()`, `getIndexEntries()`, `getRawArtifactSummary()`
  - All 7 data-displaying routes updated to consume bundled mock data
  - No filesystem access at runtime

### Changed

- `docs/prd/08_gui_tauri_chart_prd_v0.2.md` → `docs/prd/08_gui_tauri_chart_prd_v0.3.md`

### Documents

- `docs/adr/ADR-006-gui-m0-mock-only-boundary.md` — ADR for GUI-M0 mock-only boundary
- `docs/GUI/GUI-M0-spec.md` — UI/UX wireframe spec
- `docs/prd/deep-research-report.md` — GUI-M0 specification research report

---

## [0.1.0] — Initial Repository Structure

- Rust workspace with 12 crates
- PRD v0.2 (13 documents)
- ADR (5 documents)
- JSON Schemas (10 documents)
- Example artifacts: recipe, resolved recipe, dry-run plan, safety report, mock run
