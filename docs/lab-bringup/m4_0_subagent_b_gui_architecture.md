# M4.0 Subagent B — GUI Architecture Review

## Current State

The desktop app at `apps/desktop/` is a Tauri v2 + React 18 + TypeScript app with:
- HashRouter (not BrowserRouter)
- 8 existing route pages in `src/routes/`
- 4 shared components (AppShell, SideNav, TopStatusBar, MockOnlyBanner)
- Single mock-data directory with static TypeScript objects
- One Tauri command: `app_metadata`
- No charting library
- CSS custom properties with inline `style={}` objects (no Tailwind, no CSS modules)

## Where to Add Route

**File:** `apps/desktop/src/App.tsx`
Add inside `<Routes>` block:
```tsx
<Route path="/analysis-viewer" element={<AnalysisViewerPage />} />
```

Path `/analysis-viewer` follows existing pattern (lowercase, hyphenated).

## Where to Add Navigation

**File:** `apps/desktop/src/components/SideNav.tsx`
Add to `navItems` array:
```tsx
{ path: "/analysis-viewer", label: "Analysis Viewer" }
```

This auto-renders as a `NavLink` with active state highlighting.

## Component Structure (AnalysisViewerPage)

Single page component at `src/routes/AnalysisViewerPage.tsx` — one file, following existing pattern (all route pages are single default-export function components).

Internal sections rendered as inline JSX, not separate component files:

1. **EmptyState** — shown when no directory selected
   - "Select Analysis Directory" heading
   - Folder picker button (calls `pick_analysis_directory` Tauri command)
   - Error display area

2. **Header** — directory path, quality grade badge, run count, timestamp

3. **QualityFlagsPanel** — 3-column card grid (9 sub-checks)
   - Each card: check name, pass/fail icon, color coding

4. **SpectrumPlot** — recharts `<LineChart>` component
   - X-axis: frequency (GHz), formatted from Hz
   - Y-axis: B-X mean (mV), B-Y mean (mV) as two `<Line>` series
   - Tooltip on hover

5. **RunOverlayTable** — HTML `<table>` with frequency, B-X stats, B-Y stats, frames

6. **SourceRunsTable** — HTML `<table>` with run_id, frames, audit, safe state

7. **AnalysisSummary** — card grid with frequency range, contrast, quality

8. **BoundaryBanner** — fixed warning banner at bottom

No new shared components needed. All sections are specific to this page.

## Tauri Commands Needed

Two new commands in `src-tauri/src/main.rs`:

### `read_analysis_directory(path: String) -> Result<AnalysisData, String>`
- Validates path exists
- Detects parent vs analysis subdirectory
- Reads all 5 required files
- Parses JSON/JSONL
- Returns structured `AnalysisData` with all fields
- Never writes files

### `pick_analysis_directory() -> Result<Option<String>, String>`
- Opens native folder picker dialog
- Returns selected path or None if cancelled
- Uses `tauri-plugin-dialog`

## Tauri Dependencies to Add

In `src-tauri/Cargo.toml`:
- `tauri-plugin-dialog` (for native folder picker)
- `serde_json` already available via `serde_json 1`

In `apps/desktop/src-tauri/capabilities/default.json`:
- Add `"dialog:default"` or `"dialog:allow-open"` permission

## Charting Library

**Choice: recharts** — `pnpm add recharts`
- Lightest React charting dependency
- Declarative SVG-based charts
- Works with CSS custom properties via `stroke` prop
- Good for simple 2-series line charts

## What to Reuse

- **CSS tokens:** `var(--color-*)`, `var(--space-*)`, `var(--font-size-*)` — all existing
- **MockOnlyBanner:** Adapt to show "M4.0 READ-ONLY VIEWER" variant
- **TopStatusBar:** No changes needed (shows "MOCK ONLY" from context)
- **AppShell:** No changes needed (wraps all pages)
- **SideNav:** Just add one entry

## What NOT to Add

- No new React contexts
- No new shared components (unless reused by future M4.1)
- No CSS framework (Tailwind, styled-components, etc.)
- No state management library (Redux, Zustand, etc.)
- No routing library changes
