# GUI-M0 Spec: Mock-only Tauri + React Viewer

> 文件定位：GUI-M0 UI / UX / Wireframe Spec  
> 项目代号：odmrctl-web / ODMR Automation  
> 当前版本：v0.1  
> 状态：Draft for review  
> 关联文档：
>
> - `docs/prd/08_gui_tauri_chart_prd_v0.3.md`
> - `docs/adr/ADR-006-gui-m0-mock-only-boundary.md`
> - `docs/adr/ADR-001-tauri-ui.md`
> - `docs/adr/ADR-004-no-ai-live-hardware.md`

---

## 1. Scope

本文件定义 GUI-M0 的页面、布局、组件、状态、按钮、表格字段、文案和视觉约束。

GUI-M0 是：

```text
mock-only viewer
Tauri + React 前端骨架
工业风操作员界面原型
mock run artifact 浏览器
边界文档化界面
```

GUI-M0 不是：

```text
真实控制面板
设备连接工具
executor 前端
实时采集 UI
AI 控制台
数据写入工具
```

---

## 2. Global application shell

### 2.1 Overall layout

Desktop layout:

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ TopStatusBar                                                                 │
├────────────────┬─────────────────────────────────────────────────────────────┤
│ SideNav        │ MockOnlyBanner                                             │
│                ├─────────────────────────────────────────────────────────────┤
│                │ PageHeader                                                  │
│                │                                                             │
│                │ PageContent                                                 │
│                │                                                             │
└────────────────┴─────────────────────────────────────────────────────────────┘
```

Recommended dimensions:

```text
TopStatusBar height: 56px
SideNav width: 232px
MockOnlyBanner height: 40-48px
Main content max width: none, full desktop width
Main content padding: 24px
Card gap: 16px
Table density: compact
```

### 2.2 TopStatusBar

Fields, left to right:

| Area | Element | Example text |
|---|---|---|
| Product | App name | ODMR Automation |
| Phase | Phase badge | M1 mock complete / M2 bring-up pending |
| Mode | Mode badge | MOCK ONLY |
| Safety | Safety badge | Safety: Allow / Reject / Unknown |
| Backend | Backend state | Static mock data |
| Run | Current run | basic_odmr_mock_executor_run |

Behavior:

```text
Always visible.
No dropdowns required in M0.
No connection indicator should imply real hardware availability.
```

### 2.3 MockOnlyBanner

Required text:

```text
GUI-M0 MOCK VIEWER — No hardware access. No executor connection. Real controls disabled.
```

Placement:

```text
Directly under TopStatusBar, above every page content area.
```

Visual treatment:

```text
Light blue or pale cyan background
Left border or small badge reading MOCK ONLY
Text must not rely on color alone
```

### 2.4 SideNav

Required nav items:

```text
Dashboard
Devices
Recipe
Dry Run
Safety
Events
Raw Data Preview
About / Boundaries
```

Optional disabled future nav items:

```text
Live Chart          disabled: requires backend trace stream
Run Control         disabled: requires executor backend
Magnetic Planner    disabled: future module
Settings            disabled: future module
```

SideNav item states:

| State | Visual | Behavior |
|---|---|---|
| active | blue left bar + bold text | current route |
| default | neutral text | navigate |
| disabled | muted text + lock/label | no navigation unless future placeholder page exists |

---

## 3. Design tokens

### 3.1 Color tokens

Use internal tokens rather than hard-coded colors in components.

```css
--color-bg: #f5f7fa;
--color-surface: #ffffff;
--color-surface-alt: #eef3f8;
--color-border: #d6dee8;
--color-border-strong: #a9b8c7;

--color-text: #17212b;
--color-text-muted: #5f6f80;
--color-text-subtle: #7d8b99;

--color-primary: #005a9c;
--color-primary-strong: #003f73;
--color-primary-soft: #e5f1fb;

--color-accent: #0097a7;
--color-accent-soft: #e3f7f9;

--color-success: #1f7a3f;
--color-success-soft: #e6f4eb;

--color-warning: #9a6a00;
--color-warning-soft: #fff4d6;

--color-danger: #b3261e;
--color-danger-soft: #fdecea;

--color-disabled-bg: #eef1f4;
--color-disabled-text: #8b98a5;
```

### 3.2 Typography tokens

```css
--font-sans: Inter, "Segoe UI", system-ui, sans-serif;
--font-mono: "JetBrains Mono", "SFMono-Regular", Consolas, monospace;

--font-size-xs: 12px;
--font-size-sm: 13px;
--font-size-md: 14px;
--font-size-lg: 16px;
--font-size-xl: 20px;
--font-size-2xl: 24px;
```

### 3.3 Spacing tokens

```css
--space-1: 4px;
--space-2: 8px;
--space-3: 12px;
--space-4: 16px;
--space-5: 20px;
--space-6: 24px;
--space-8: 32px;
```

### 3.4 Shape / border tokens

```css
--radius-sm: 4px;
--radius-md: 8px;
--radius-lg: 12px;
--shadow-card: 0 1px 2px rgba(15, 23, 42, 0.06);
```

### 3.5 Density

Default density should be compact:

```text
Metric cards: compact
Tables: row height 36-44px
Buttons: height 32-36px
Page header: no large hero section
```

---

## 4. Component specs

### 4.1 StatusBadge

Variants:

| Variant | Text examples | Notes |
|---|---|---|
| mock | MOCK ONLY | always visible in M0 |
| allow | Safety: Allow | green + text |
| reject | Safety: Reject | red + text |
| warning | Warning | amber + text |
| unknown | Unknown | gray + text |
| disabled | Disabled | gray + reason |
| static | Static mock data | neutral/cyan |

Rules:

```text
Never rely on color only.
Every badge must include explicit text.
```

### 4.2 MetricCard

Fields:

```text
title
value
unit optional
status optional
helper text optional
```

Example:

```text
Title: Resolved Steps
Value: 201
Helper: from basic_odmr_mock.dry_run_plan.json
```

### 4.3 DeviceCard

Fields:

```text
device_name
role
required_by_recipe
mock_connection_state
last_known_state
capabilities_preview
controls
boundary_note
```

Controls must be disabled in M0.

### 4.4 DisabledControlButton

Props:

```text
label
reason
severity optional
```

Rendering:

```text
[disabled button label]
Reason: <reason>
```

Required examples:

```text
Start Run — Requires executor backend
Connect — M2 bring-up only
RF Output ON — Forbidden in GUI-M0
MOD ON — Forbidden in GUI-M0
Auto Phase — Forbidden in GUI-M0
```

### 4.5 DataTable

Required features for M0:

```text
header row
compact rows
horizontal scroll for long fields
monospace option for IDs / paths / hashes
empty state
```

Not required in M0:

```text
server pagination
column sorting
column resizing
filter builder
virtualization
```

Pagination is acceptable for Dry Run steps.

### 4.6 JsonPreview

Requirements:

```text
read-only
monospace
collapsible or fixed-height scroll area
long values wrap or scroll predictably
label source file
```

### 4.7 BoundaryNotice

Used on pages where user may expect real control.

Required text examples:

```text
No hardware access exists in GUI-M0.
This page displays static mock data only.
Future real integration must go through Rust backend, executor, and safety interlock.
```

---

## 5. Page specs

## 5.1 Dashboard page

### Purpose

Give a compact overview of the loaded mock run and current application boundary.

### Layout

```text
PageHeader
  title: Dashboard
  subtitle: Mock run overview and operator state summary.

MetricCardGrid, 3 columns desktop / 2 tablet / 1 narrow:
  System phase
  Current mode
  Safety decision
  Resolved steps
  Estimated duration
  Event count
  Artifact count
  Backend state

DisabledControlStrip:
  Start Run
  Pause Run
  Stop Run
  Emergency Stop

Two-column lower section:
  Recent Events Preview
  Artifact Inventory Preview
```

### Required cards

| Card | Value source | Example |
|---|---|---|
| System phase | static | M1 mock complete / M2 hardware bring-up pending |
| Current mode | static | GUI-M0 MOCK ONLY |
| Safety decision | safety report | Allow / Reject / Unknown |
| Resolved steps | dry-run plan | 201 |
| Estimated duration | dry-run plan | value or Unknown |
| Event count | events mock data | count or Unknown |
| Artifact count | manifest / static summary | count or Unknown |
| Backend state | static | Static mock data |

### Disabled controls

| Button | State | Reason |
|---|---|---|
| Start Run | disabled | Requires executor backend |
| Pause Run | disabled | Requires active executor run |
| Stop Run | disabled | Requires active executor run |
| Emergency Stop | disabled / visual-only | M0 has no hardware authority |

### Notes

Dashboard must not show any state that suggests real device readiness.

---

## 5.2 Devices page

### Purpose

Show fake device statuses and make M2-only hardware bring-up explicit.

### Layout

```text
PageHeader
BoundaryNotice
DeviceCardGrid, 2 columns desktop:
  SMB100A
  OE1022D
  Laser Controller
  Magnetic Axes
```

### Device cards

#### SMB100A card

Fields:

```text
Role: RF / microwave signal generator
Required by recipe: yes
Connection: unavailable in GUI-M0
Mock state: static snapshot only
Potential future parameters: RF freq, RF power, RF output, modulation, LF generator
```

Disabled controls:

```text
Connect SMB100A — M2 bring-up only
Probe LAN — No TCP socket probing in GUI-M0
RF Output ON — Forbidden in GUI-M0
MOD ON — Forbidden in GUI-M0
Configure RF — Backend device registry required
```

#### OE1022D card

Fields:

```text
Role: DSP lock-in amplifier / acquisition source
Required by recipe: yes
Connection: unavailable in GUI-M0
Mock state: static snapshot only
Important future channel: Ch-B X / Y / R / theta / ref freq
```

Disabled controls:

```text
Connect OE1022D — M2 bring-up only
Probe USB/RS232 — No USB/serial probing in GUI-M0
Read RALL? — Forbidden in GUI-M0
Auto Phase — Forbidden in GUI-M0
Configure Ch-B — Backend device registry required
```

#### Laser card

Fields:

```text
Role: laser controller placeholder
Required by recipe: unknown or yes if mock says so
Connection: unavailable in GUI-M0
```

Disabled controls:

```text
Connect Laser — M2 bring-up only
Emission ON — Forbidden in GUI-M0
Safe shutdown — No hardware authority in GUI-M0
```

#### Magnetic axes card

Fields:

```text
Role: X/Y/Z magnetic field or current controller placeholder
Required by recipe: unknown or yes if mock says so
Connection: unavailable in GUI-M0
```

Disabled controls:

```text
Connect axes — M2 bring-up only
Set B vector — Future magnetic planner
Ramp current — Forbidden in GUI-M0
Safe zero — No hardware authority in GUI-M0
```

---

## 5.3 Recipe page

### Purpose

Show recipe metadata and resolved recipe summary without editing or compiling.

### Layout

```text
PageHeader
Summary card row:
  Recipe name
  Recipe source
  Resolved steps
  Required devices
  Hashes
Main section:
  Left: Metadata table
  Right: Read-only JSON preview or key-value panel
Disabled controls row:
  Open Recipe
  Compile Recipe
  Ask AI to Draft
```

### Required fields

| Field | Requirement |
|---|---|
| recipe name | show if available, otherwise `basic_odmr_mock` |
| source | show static mock data source |
| resolved step count | must show 201 |
| required devices | show list if available |
| recipe hash | show if available, otherwise `unknown` |
| resolved hash | show if available, otherwise `unknown` |
| station/profile | show if available, otherwise `unknown` |

### Disabled controls

| Button | Reason |
|---|---|
| Open Recipe | Real file picker deferred |
| Compile Recipe | Compiler backend required |
| Ask AI to Draft | AI workflow deferred; cannot bypass review |

---

## 5.4 Dry Run page

### Purpose

Show the resolved execution plan in a way operators can inspect.

### Layout

```text
PageHeader
Metric row:
  Total steps
  Estimated duration
  Required devices
  Gated actions
Controls row:
  Pagination / page size only, if implemented
Main table:
  Dry-run steps
Optional:
  Details drawer or JSON preview for selected step
```

### Step table columns

| Column | Type | Notes |
|---|---|---|
| # | number | display index |
| step_id | monospace | stable ID if available |
| device | text | SMB100A / OE1022D / Laser / Mag / system |
| action | text | command intent, not executable |
| parameters | compact text | summarized JSON |
| duration | text | dwell / estimated duration if available |
| safety | badge/text | gated / safe / unknown |

### Required behavior

```text
Total count must visibly show 201.
If rendering all 201 rows is inconvenient, render paginated subset.
No row click may execute anything.
```

### Prohibited behavior

```text
No execute step
No send SCPI
No call executor
No write run event
```

---

## 5.5 Safety page

### Purpose

Make safety decision prominent and auditable.

### Layout

```text
PageHeader
Large decision panel
Severity summary cards
Findings table
Read-only JSON preview optional
BoundaryNotice
```

### Decision panel

Variants:

| Decision | Required text |
|---|---|
| allow | Safety decision: Allow |
| reject | Safety decision: Reject |
| unknown | Safety decision: Unknown |

Decision panel must also say:

```text
Displayed from existing mock safety report. Frontend does not compute safety.
```

### Findings table columns

| Column | Notes |
|---|---|
| severity | text + badge |
| code | monospace if stable |
| message | full readable message |
| step_id | monospace or `—` |
| device | text or `—` |

### Prohibited behavior

```text
No safety override
No edit limits
No recompute decision
No approve run
```

---

## 5.6 Events page

### Purpose

Render the mock executor run event log.

### Layout

```text
PageHeader
Summary row:
  event count
  source
  run id
Main event table
Optional raw JSONL preview
```

### Event table columns

| Column | Type |
|---|---|
| timestamp | text |
| event_type | monospace |
| step_id | monospace / `—` |
| level | badge/text |
| message | text |

### Required behavior

```text
Handle missing fields gracefully.
Show `—` for unknown fields.
Do not subscribe to live events in M0.
```

---

## 5.7 Raw Data Preview page

### Purpose

Show artifact inventory while explicitly not parsing raw binary.

### Layout

```text
PageHeader
BoundaryNotice
Artifact summary cards:
  rawbin file
  rawbin size
  index entries
  metadata files
Artifact table
No-binary-parsing notice
```

### Artifact table columns

| Column | Notes |
|---|---|
| path | monospace |
| type | manifest / events / index / rawbin / metadata |
| size | bytes if available or `unknown` |
| role | human-readable explanation |
| parsed by GUI-M0 | always No for raw binary |

### Required notice

```text
Raw binary parsing belongs to Rust backend / offline parser, not GUI-M0 frontend.
```

### Prohibited behavior

```text
No RALL? parser
No binary decode
No CSV export
No file writer
```

---

## 5.8 About / Boundaries page

### Purpose

Document boundaries inside the running app.

### Layout

```text
PageHeader
Boundary statement
Allowed in M0 list
Forbidden in M0 list
Future M1/M2 integration path
Related docs list
```

### Required text block

```text
This GUI is mock-only.
It does not connect to devices.
It does not call executor.
It does not send SCPI.
It does not read OE1022D RALL?.
It does not write experiment data.
Future M2 integration must go through backend APIs, executor, and safety interlock.
```

### Allowed list

```text
Display mock run summary
Display dry-run plan
Display safety report
Display events
Display artifact inventory
Display disabled future controls
```

### Forbidden list

```text
serial / USB / VISA / TCP socket access
SCPI sending
executor calls
hardware polling
raw data parsing
run data writing
AI live hardware control
```

---

## 6. Empty, loading, and error states

### 6.1 Empty state

Use when a static mock artifact is intentionally unavailable.

Example:

```text
No safety findings are available in this mock report.
```

### 6.2 Loading state

If data is imported synchronously, loading may not appear. If asynchronous loading is used:

```text
Loading static mock data…
```

Do not say:

```text
Connecting to backend…
Connecting to devices…
```

### 6.3 Error state

If mock data fails to load:

```text
Mock data unavailable. GUI-M0 remains safe because no hardware or executor connection exists.
```

Recommended fields:

```text
artifact name
error summary
fallback behavior
```

---

## 7. Operator text rules

Use precise operator-facing labels.

Prefer:

```text
Mock plan loaded
Safety decision: Allow
Start Run disabled: requires executor backend
Connect disabled: M2 hardware bring-up only
Raw binary not parsed in GUI-M0
```

Avoid:

```text
OK
Go
Run now
Connected
Live
Ready
```

Unless the state is truly connected/live/ready in a future phase.

---

## 8. Mock data shape recommendations

Use frontend types close to future backend DTOs:

```ts
type RunSummary = {
  runId: string;
  name: string;
  mode: "mock";
  stepCount: number;
  estimatedDurationMs?: number;
  eventCount?: number;
  artifactCount?: number;
};

type DeviceStatusSnapshot = {
  deviceId: string;
  displayName: string;
  role: string;
  requiredByRecipe: boolean | "unknown";
  connectionState: "mock-only" | "unavailable";
  lastKnownState?: string;
};

type SafetySummary = {
  decision: "allow" | "reject" | "unknown";
  findings: SafetyFinding[];
};

type SafetyFinding = {
  severity: "info" | "warning" | "error" | "critical" | "unknown";
  code?: string;
  message: string;
  stepId?: string;
  deviceId?: string;
};

type RunEventRecord = {
  timestamp?: string;
  eventType: string;
  stepId?: string;
  level?: string;
  message?: string;
};

type ArtifactRecord = {
  path: string;
  type: "manifest" | "events" | "index" | "rawbin" | "metadata" | "summary" | "other";
  sizeBytes?: number;
  role: string;
};
```

These types are advisory for M0 and can be adjusted to match actual artifacts.

---

## 9. Suggested frontend structure

```text
apps/desktop/src/
  main.tsx
  App.tsx
  routes/
    DashboardPage.tsx
    DevicesPage.tsx
    RecipePage.tsx
    DryRunPage.tsx
    SafetyPage.tsx
    EventsPage.tsx
    RawDataPreviewPage.tsx
    AboutBoundariesPage.tsx
  components/
    AppShell.tsx
    TopStatusBar.tsx
    SideNav.tsx
    MockOnlyBanner.tsx
    PageHeader.tsx
    StatusBadge.tsx
    MetricCard.tsx
    DeviceCard.tsx
    DisabledControlButton.tsx
    BoundaryNotice.tsx
    DataTable.tsx
    JsonPreview.tsx
    ArtifactList.tsx
    EmptyState.tsx
    ErrorState.tsx
  mock-data/
    runSummary.ts
    recipeSummary.ts
    dryRunPlan.ts
    safetyReport.ts
    events.ts
    artifacts.ts
  styles/
    tokens.css
    app.css
```

Do not over-engineer a full design system in M0.

---

## 10. Disabled controls catalog

| Location | Control | State | Reason |
|---|---|---|---|
| Dashboard | Start Run | disabled | Requires executor backend |
| Dashboard | Pause Run | disabled | Requires active executor run |
| Dashboard | Stop Run | disabled | Requires active executor run |
| Dashboard | Emergency Stop | disabled / visual-only | M0 has no hardware authority |
| Devices / SMB100A | Connect | disabled | M2 bring-up only |
| Devices / SMB100A | Probe LAN | disabled | No TCP socket probing in GUI-M0 |
| Devices / SMB100A | RF Output ON | disabled | Forbidden in GUI-M0 |
| Devices / SMB100A | MOD ON | disabled | Forbidden in GUI-M0 |
| Devices / OE1022D | Connect | disabled | M2 bring-up only |
| Devices / OE1022D | Probe USB/RS232 | disabled | No USB/serial probing in GUI-M0 |
| Devices / OE1022D | Read RALL? | disabled | Forbidden in GUI-M0 |
| Devices / OE1022D | Auto Phase | disabled | Forbidden in GUI-M0 |
| Recipe | Open Recipe | disabled | Real file picker deferred |
| Recipe | Compile Recipe | disabled | Compiler backend required |
| Recipe | Ask AI to Draft | disabled | AI workflow deferred; cannot bypass review |
| Dry Run | Execute Step | absent or disabled | Executor backend required |
| Safety | Override Safety | absent | Forbidden |
| Raw Data | Export CSV | disabled | Offline parser/exporter required |

---

## 11. Accessibility checklist

M0 implementation must satisfy:

```text
1. All buttons have visible text labels.
2. Disabled controls have text reasons.
3. Status is not conveyed by color alone.
4. Tables have column headers.
5. Focus states are visible.
6. Normal text uses sufficient contrast.
7. Long file paths and hashes are readable or copyable.
8. Page titles are unique and visible.
9. Error states explain that no hardware is affected.
10. Motion/animation is minimal.
```

---

## 12. Testing checklist

Manual review checklist:

```text
Open app locally.
Confirm top status bar appears.
Confirm mock-only banner appears on every page.
Navigate all required pages.
Confirm no page claims real hardware is connected.
Confirm all real controls are disabled.
Confirm Dashboard shows mock summary.
Confirm Recipe shows resolved step count 201.
Confirm Dry Run shows total count 201.
Confirm Safety shows decision.
Confirm Events renders event rows or clear empty state.
Confirm Raw Data Preview does not parse binary.
Confirm About / Boundaries states all restrictions.
Search code for serial / USB / VISA / socket / SCPI / executor imports.
```

Suggested code search terms:

```text
serial
serialport
usb
visa
socket
TcpStream
SCPI
OUTP
MOD:STAT
RALL
executor
rawbin write
csv
```

These strings may appear in documentation or disabled labels, but must not appear as executable hardware logic in GUI-M0.

---

## 13. Acceptance criteria

GUI-M0 UI implementation passes if:

```text
1. The app starts.
2. It renders the eight required pages.
3. It uses static mock data.
4. It visibly identifies itself as GUI-M0 MOCK VIEWER.
5. Dashboard summarizes the mock run.
6. Devices page contains fake SMB100A and OE1022D cards.
7. Recipe page shows mock recipe metadata and step count 201.
8. Dry Run page shows step count 201 and step rows or paginated subset.
9. Safety page shows decision and findings.
10. Events page renders events or a clear mock-data empty state.
11. Raw Data Preview shows artifact inventory only.
12. About / Boundaries page states no hardware / no executor / no SCPI.
13. All real-control buttons are disabled.
14. No hardware access code exists.
15. No executor call exists.
```

---

## 14. Non-acceptance examples

Reject implementation if any page contains:

```text
an enabled Connect button that probes ports
an enabled Start Run button that calls executor
an enabled RF Output ON button
an enabled MOD ON button
a Tauri command that opens TCP/serial/USB/VISA
frontend code that parses RALL? raw binary frames
code that writes rawbin / index.jsonl / events.jsonl / CSV
AI control hook for live hardware
```

---

## 15. Future UI notes

M1 may add:

```text
read-only backend artifact loader
mock replay timeline
downsampled chart preview
run directory browser for safe local artifacts
```

M2 may add:

```text
device status snapshots
controlled connect workflow
run request workflow
executor-owned run lifecycle
safety-gated controls
real event subscription
```

But even in M2:

```text
GUI sends user intent.
Executor owns run authority.
Safety owns allow/reject.
Device drivers own hardware access.
Logger owns raw-first data writing.
```

---

## 16. Summary

GUI-M0 should feel like a serious industrial experiment viewer, but it must be technically harmless.

The desired result is a polished skeleton that makes the future system understandable while making the current boundary impossible to miss:

```text
Mock data only.
No hardware.
No executor.
No SCPI.
No raw binary parsing.
No experiment data writing.
```
