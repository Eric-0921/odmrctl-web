# GUI-M0 Implementation Notes

## Overview

GUI-M0 is a **mock-only Tauri + React + TypeScript viewer** for the ODMR Automation system. It displays static bundled mock experiment artifacts without any hardware access, executor connection, or data writing.

## Pages Implemented

| Page | File | Content |
|------|------|---------|
| Dashboard | `src/routes/DashboardPage.tsx` | Run summary cards (9 metrics), disabled control buttons |
| Devices | `src/routes/DevicesPage.tsx` | 4 fake device cards with disabled controls |
| Recipe | `src/routes/RecipePage.tsx` | Recipe metadata, sweep parameters, collapsible JSON preview |
| Dry Run | `src/routes/DryRunPage.tsx` | 201-step plan table (paginated to 50 rows) |
| Safety | `src/routes/SafetyPage.tsx` | Safety decision banner, findings table (empty state) |
| Events | `src/routes/EventsPage.tsx` | 407 event log table (paginated to 10 rows) |
| Raw Data Preview | `src/routes/RawDataPreviewPage.tsx` | Artifact inventory, manifest metadata |
| About / Boundaries | `src/routes/AboutBoundariesPage.tsx` | Boundary statement, Allowed/Forbidden lists |

## Components

| Component | File | Purpose |
|-----------|------|---------|
| AppShell | `src/components/AppShell.tsx` | Global layout: TopStatusBar + SideNav + MockOnlyBanner + content |
| TopStatusBar | `src/components/TopStatusBar.tsx` | Persistent status bar with mode/phase/safety badges |
| SideNav | `src/components/SideNav.tsx` | Left rail navigation (8 active + 4 disabled items) |
| MockOnlyBanner | `src/components/MockOnlyBanner.tsx` | Dismissible mock-only warning banner |

## Mock Data Strategy

All data is **bundled at build time** as static TypeScript `const` exports. No filesystem or network access at runtime.

| Module | Source | Records |
|--------|--------|---------|
| `recipe.ts` | `examples/recipes/basic_odmr_mock.recipe.json` | 1 recipe |
| `dryRunPlan.ts` | `examples/resolved/basic_odmr_mock.dry_run_plan.json` | 201 steps |
| `safetyReport.ts` | `examples/safety/basic_odmr_mock.safety_report.json` | 0 findings |
| `events.ts` | `examples/runs/.../events.jsonl` | 407 events |
| `indexEntries.ts` | `examples/runs/.../index.jsonl` | 201 entries |
| `runManifest.ts` | `examples/runs/.../manifest.json` | 1 manifest |
| `rawArtifactSummary.ts` | metadata only | 1 summary |
| `runSummary.ts` | computed | 1 summary |

Helper functions in `src/mock-data/helpers.ts` provide typed accessors.

## Disabled Controls

### Dashboard
- Start Run — requires executor backend
- Pause Run — no live run in GUI-M0
- Stop Run — no live run in GUI-M0
- Emergency Stop — no hardware authority in GUI-M0

### Devices
- Connect — M2 bring-up only
- Probe — Forbidden in GUI-M0
- Configure — Mock viewer only
- Output ON — Forbidden in GUI-M0
- MOD ON — Forbidden in GUI-M0
- Emission ON — Forbidden in GUI-M0
- Set B vector — Mock viewer only
- Ramp current — Forbidden in GUI-M0
- Safe zero — Forbidden in GUI-M0

### Recipe
- Open Recipe — Real file picker deferred
- Compile Recipe — Compiler backend required
- Ask AI to Draft — AI workflow deferred; cannot bypass review

## Accessibility

- Global `:focus-visible` outline on all interactive elements
- All status badges include text labels (not color alone)
- All disabled buttons have explanatory helper text
- Table headers use `scope="col"` for screen readers
- Toggle buttons use `aria-expanded` and `aria-label`
- Monospace font for IDs, hashes, paths

## Boundary Checklist

- [x] No serial / USB / VISA / TCP socket code
- [x] No SCPI sending
- [x] No executor calls
- [x] No hardware polling
- [x] No raw data parsing (rawbin metadata only)
- [x] No run data writing
- [x] No AI live hardware control
- [x] No live event subscription
- [x] All real-control buttons disabled

## Known Limitations

- Tables paginate to a fixed subset (50 steps, 10 events) rather than virtual scroll
- No charting / plotting (deferred to M1)
- No live backend connection (deferred to M2)
- No recipe editing or compilation (deferred to M2)

## Remaining Work for M1 / M2

### M1 (Read-only backend APIs)
- Mock run summary listing
- Safe static file loading via Tauri commands
- Mock replay timeline
- Downsampled chart preview

### M2 (Real hardware integration)
- `connect_device` command
- `get_device_status_snapshot` command
- `request_run_start` / `request_safe_shutdown` commands
- Real-time event stream
- Raw binary parsing in Rust backend
