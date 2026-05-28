# Sub-PRD 08: GUI / Tauri / Chart

Version: v0.2  
Status: Draft for implementation planning  
Owner: ODMR Automation Project  
Related PRDs:

- `00_main_prd.md`
- `01_architecture_prd.md`
- `02_device_registry_connection_prd.md`
- `03_oe1022d_acquisition_prd.md`
- `04_recipe_json_schema_prd.md`
- `05_recipe_compiler_executor_prd.md`
- `06_timing_sync_averaging_prd.md`
- `07_data_logging_file_format_prd.md`
- `09_magnetic_planner_prd.md`
- `10_safety_interlock_prd.md`
- `11_harness_mock_replay_prd.md`
- `12_agent_workflow_prd.md`

External references:

- Tauri official site: <https://tauri.app/>
- Tauri architecture documentation: <https://v2.tauri.app/concept/architecture/>
- uPlot official site: <https://leeoniya.github.io/uPlot/>
- uPlot GitHub repository: <https://github.com/leeoniya/uPlot>

---

## 1. Purpose

This PRD defines the GUI and real-time charting requirements for the ODMR automation system.

The GUI exists to make the experiment system usable, observable, and safe. It must not become the experiment engine.

The core design goal is:

```text
GUI is an operator console.
GUI is not a hardware driver.
GUI is not a real-time acquisition engine.
GUI is not a data analysis notebook.
GUI is not an AI agent runtime.
```

The GUI must prevent the new system from repeating the old pattern where the desktop interface, acquisition loop, device polling, data saving, and analysis logic become entangled in one process.

This module defines:

```text
Tauri application boundary
frontend page responsibilities
Rust backend command/event interface
real-time chart rendering rules
backend downsampling contract
run control UX
safety UX
mock/replay UI testing model
agent constraints for GUI development
```

The target user is a human experiment operator who needs to:

```text
connect devices
inspect station status
choose or load a recipe
view dry-run and safety results
start / pause / stop / emergency-stop a run
watch real-time signals
observe noise / drift / overload conditions
open magnetic planner
inspect run directory and logs
export derived data after the run
```

---

## 2. Scope

This module is responsible for:

```text
Tauri desktop shell
web frontend structure
operator workflow pages
backend command invocation boundaries
backend event subscription boundaries
real-time chart view
chart data decimation / viewport request protocol
run status dashboard
device status display
recipe selection and dry-run visualization
safety status visualization
emergency stop UI
run log viewer
basic run file browser
magnetic planner entry point
mock data UI mode
replay UI mode
```

This module defines the contract between:

```text
frontend webview
Tauri Rust command layer
Device Registry
Recipe Compiler
Recipe Executor
OE1022D acquisition core
Data Logging module
Safety Interlock module
Magnetic Planner module
Harness / Replay module
```

The frontend may display derived data, status data, and downsampled chart data. It may not request or parse canonical raw binary frames.

---

## 3. Non-goals

This module does not implement:

```text
hardware drivers
OE1022D RALL? acquisition loop
RALL? binary parser
SMB100A SCPI command engine
laser serial protocol
magnetic coil control protocol
recipe compilation algorithm
safety rule engine
raw binary writer
parquet / CSV converter
machine learning training
ODMR spectral fitting
complex publication plotting
AI recipe generation
multi-user permission system
cloud experiment dashboard
```

This module explicitly forbids:

```text
GUI thread reading serial / TCP / VISA directly
web frontend parsing binary raw frames
web frontend writing raw bin files
frontend generating device commands directly
frontend applying safety rules by itself
frontend converting raw bin to CSV
frontend performing long-running analysis
frontend using AI to modify active recipe while run is executing
frontend owning canonical experiment state
```

The GUI may request actions. The Rust backend decides whether the action is allowed.

---

## 4. Design Principles

### 4.1 Thin GUI Principle

The GUI must stay thin:

```text
render state
send operator intent
show dry-run
show safety status
show downsampled data
show logs
```

The GUI must not become the owner of:

```text
hardware state
run state
raw acquisition state
recipe expansion state
safety decision state
canonical data files
```

All authoritative states live in Rust backend modules.

---

### 4.2 Backend-owned State Principle

The frontend may cache UI state such as selected tab, current chart viewport, or collapsed panels. It must not cache authoritative experiment state.

Authoritative state examples:

```text
device connected / disconnected
current recipe id
compiled resolved_recipe id
safety pass / fail
run active / paused / stopping / failed / completed
current step id
resource lease owner
emergency stop active
raw writer status
acquisition thread status
```

These must be queried or subscribed from the backend.

---

### 4.3 Downsampled Chart Principle

The frontend receives only chart-ready data:

```text
downsampled samples
aggregated envelopes
rolling metrics
step-level summaries
```

The frontend never receives:

```text
RALL? raw binary frame
raw bin byte stream
full unbounded sample history
unparsed device response payload
```

The backend must prepare chart data according to viewport and display density.

---

### 4.4 Real-time Isolation Principle

UI refresh must not affect acquisition.

The system must tolerate:

```text
chart panel closed
chart panel open
chart zooming
frontend crash
webview reload
high CPU usage in browser rendering
operator opening logs
operator switching pages
```

None of these may block:

```text
OE1022D acquisition thread
raw bin writer
executor safety loop
emergency stop path
```

---

### 4.5 Safety-visible Principle

Dangerous run states must always be visible.

The GUI must provide:

```text
persistent emergency stop control
persistent run status indicator
persistent device connection status
persistent safety status indicator
clear disabled state for unsafe actions
explicit dry-run preview before real execution
visible dangerous commands before execution
```

The GUI must not hide safety-critical state inside secondary tabs.

---

## 5. Recommended Technology Stack

### 5.1 Desktop Shell

```text
Tauri v2
```

Reason:

```text
Rust backend integration
web frontend flexibility
small desktop app footprint
clear command/event boundary
works with React / Svelte / Vue / plain TypeScript
fits Rust acquisition core
```

The Tauri app is not selected because it automatically improves acquisition performance. It is selected because it enforces a cleaner boundary:

```text
frontend: web UI and chart rendering
backend: Rust command/event/process/hardware boundary
```

---

### 5.2 Frontend Framework

Default recommendation:

```text
React + TypeScript
```

Acceptable alternatives:

```text
Svelte + TypeScript
Vue + TypeScript
```

Decision rule:

```text
If team has stronger React experience, choose React.
If minimal runtime and simple state are prioritized, choose Svelte.
Do not choose based on trend; choose based on maintainability and agent coding reliability.
```

For v0.2 planning, this PRD assumes:

```text
React + TypeScript + Vite + Tauri
```

---

### 5.3 Chart Library

Default recommendation:

```text
uPlot
```

Reason:

```text
high-performance time series rendering
Canvas 2D based
small footprint
well-suited to line charts and dense time-series data
simple API compared with full dashboard frameworks
```

Allowed secondary chart libraries:

```text
lightweight SVG / Canvas chart for non-real-time summaries
plain HTML tables for status / dry-run
custom Canvas only if uPlot is insufficient
```

Not recommended for real-time dense data:

```text
ECharts as primary real-time trace plotter
Plotly as primary real-time trace plotter
D3-only custom SVG path for high-rate live traces
```

They may still be used for offline analysis pages later, but not for the first real-time acquisition chart.

---

## 6. GUI Responsibilities

The GUI is responsible for:

```text
device connection workflow
station status display
recipe file selection
recipe schema validation result display
dry-run preview display
safety report display
run start / pause / stop / emergency-stop control
real-time chart display
basic run event log display
acquisition health display
magnetic planner page entry
basic run directory browser
operator settings
mock / replay mode selection
```

The GUI is not responsible for:

```text
experiment execution logic
hardware acquisition
hardware command serialization
OE1022D RALL? parsing
SMB100A SCPI formatting
CSV conversion
raw data canonicalization
AI generation
complex data analysis
ML model training
safety limit calculation
resource lease enforcement
```

---

## 7. Page Model

### 7.1 Required Pages

The first implementation must include these pages:

```text
Dashboard
Devices
Recipes
Dry Run
Run Control
Live Charts
Run Logs
Data Runs
Magnetic Planner
Settings
```

The first version may combine some pages visually, but the conceptual boundary must stay clear.

---

### 7.2 Dashboard Page

Purpose:

```text
show the current station and run state at a glance
```

Required information:

```text
station id
connected devices
device health
active recipe name
current run id
current run phase
current step id
elapsed time
estimated remaining time
safety status
acquisition status
raw writer status
last warning
last error
emergency stop status
```

Required actions:

```text
open device page
open recipe page
open live chart
open logs
emergency stop
```

Dashboard must not contain complex configuration forms.

---

### 7.3 Devices Page

Purpose:

```text
connect and inspect devices
```

Device types:

```text
SMB100A
OE1022D
Laser
Mag X
Mag Y
Mag Z
```

Required UI elements:

```text
device card per device
transport type
port / address
connection status
IDN result
last heartbeat
capabilities summary
safe disconnect button
error display
mock/real mode label
```

Allowed actions:

```text
connect
disconnect
query IDN
refresh status
safe disconnect
```

Forbidden actions:

```text
send arbitrary SCPI from default UI
send arbitrary serial command from default UI
run acquisition loop from frontend
change safety limits from GUI without safety module support
```

A developer-only command console may exist later, but it must be behind a clearly marked debug mode and must never be part of normal experiment operation.

---

### 7.4 Recipes Page

Purpose:

```text
select and inspect recipe files
```

Required UI elements:

```text
recipe file picker
recent recipe list
schema validation result
recipe summary
referenced profiles
referenced blocks
estimated variable sweep dimensions
warnings
open in external editor button
```

Allowed actions:

```text
load recipe
validate recipe
open dry-run preview
copy recipe path
```

Forbidden actions:

```text
execute raw recipe directly
silently modify recipe
let AI modify active recipe during run
apply recipe without compiler result
```

The GUI may include a JSON editor later. In v0.2, the safer default is to use an external editor and have the GUI validate and preview.

---

### 7.5 Dry Run Page

Purpose:

```text
show exactly what will happen before hardware is touched
```

Required information:

```text
resolved_recipe id
compiled step count
estimated duration
device resource lease plan
per-step device commands summary
settle / dwell / sample windows
safety report
dangerous commands list
warnings
blocking errors
```

Required actions:

```text
approve run
reject run
export dry-run report
open safety report
open resolved_recipe.json
```

The Run button must remain disabled until:

```text
recipe schema passed
compiler succeeded
safety report passed
all required devices available or mock mode enabled
operator explicitly reviewed dry-run
```

---

### 7.6 Run Control Page

Purpose:

```text
control an active experiment run
```

Required information:

```text
run id
run directory
current phase
current step index / total steps
current block id
current device states
current timing state
current sample window
last device command
last event
warnings
errors
```

Required actions:

```text
start
pause if supported
resume if supported
request graceful stop
emergency stop
open live chart
open event log
```

Button semantics:

```text
Start: backend begins executor on approved resolved_recipe
Pause: backend reaches safe pause point, not immediate thread freeze
Stop: graceful stop; safe shutdown sequence runs
Emergency Stop: immediate safety path; dangerous outputs disabled
```

The GUI must clearly distinguish:

```text
stop
pause
emergency stop
```

---

### 7.7 Live Charts Page

Purpose:

```text
observe real-time acquisition and noise behavior without disturbing acquisition
```

Required chart panels:

```text
Ch-B X vs time
Ch-B Y vs time
Ch-B R vs time
Reference frequency vs time
Noise / rolling RMS vs time
current step marker overlay
run event marker overlay
```

Optional chart panels:

```text
ODMR sweep curve: frequency vs X
B field vs X
coil current vs X
rolling mean / rolling std
histogram of recent X samples
PSD after post-run analysis only
```

The first implementation must prioritize:

```text
Ch-B X live trace
current step markers
overload / warning markers
rolling RMS / drift indicator
```

---

### 7.8 Run Logs Page

Purpose:

```text
inspect structured run events without opening raw files
```

Required features:

```text
events.jsonl tail view
filter by level
filter by device
filter by step_id
filter by event_type
copy event JSON
open run directory
```

Log viewer must not parse raw bin.

---

### 7.9 Data Runs Page

Purpose:

```text
browse completed run directories and derived files
```

Required features:

```text
run directory list
run metadata summary
open resolved_recipe.json
open station_snapshot.json
open safety_report.json
show parsed parquet availability
show export csv availability
trigger post-run parse if backend supports it
trigger CSV export if backend supports it
```

Forbidden:

```text
frontend converting raw bin directly
frontend editing canonical raw files
frontend deleting raw files by default
```

Deletion must require explicit operator confirmation and should be disabled in early versions.

---

### 7.10 Magnetic Planner Page

Purpose:

```text
provide entry point to magnetic field planning module
```

This page belongs conceptually to Sub-PRD 09. In GUI PRD, it only defines integration requirements.

Required features:

```text
open magnetic planner
load coil matrix summary
preview B vector / coil current summary
show current limits and ramp limits
export magnetic block JSON
send block to recipe workflow
```

Forbidden in GUI PRD:

```text
magnetic inverse model implementation
complex 3D simulation engine
coil safety rule implementation
```

---

### 7.11 Settings Page

Purpose:

```text
operator-level application settings
```

Allowed settings:

```text
theme
chart refresh rate target
chart max displayed points
default run directory root
mock mode default
log verbosity display
external editor path
```

Forbidden settings:

```text
laser max power
SMB max power
mag max current
mag ramp limit
hard safety limits
raw file format version
schema bypass
```

Safety settings are not general GUI preferences. They belong to safety configuration and must be guarded by Sub-PRD 10 rules.

---

## 8. Runtime Model

### 8.1 Process Model

The Tauri application consists of:

```text
Tauri Rust process
  - command handlers
  - event emitters
  - shared backend state handles
  - access to Rust core modules

Webview frontend
  - React/Svelte/Vue UI
  - chart rendering
  - user interaction
  - local UI state only
```

The frontend and backend communicate through:

```text
Tauri commands for request/response actions
Tauri events for status updates and streaming chart data
```

The frontend must not open hardware ports.

---

### 8.2 Thread / Task Boundary

The GUI must interact only with safe async interfaces.

Expected backend threads / tasks:

```text
main Tauri runtime
device registry task
executor task
OE1022D acquisition thread
parser thread
raw writer thread
chart decimation task
event broadcaster task
```

Frontend actions must not block these real-time tasks.

---

### 8.3 UI Refresh Model

Recommended UI update frequencies:

```text
device status:       1-2 Hz
run progress:        2-5 Hz
live chart redraw:   10-30 Hz
log tail:            1-5 Hz or event-driven
safety status:       event-driven + visible persistent state
```

Do not poll high-frequency raw samples from frontend.

---

## 9. Chart Model

### 9.1 Chart Library

The default chart engine is:

```text
uPlot
```

Initial chart scope:

```text
time-series line chart
step markers
warning markers
multi-series overlay
viewport zoom / pan
fixed-point display cap
```

---

### 9.2 Chart Refresh Target

Target:

```text
10-30 Hz visual refresh
```

Rationale:

```text
The chart must feel real-time to the operator.
The chart does not need to redraw at acquisition sample rate.
Noise and drift must be visible through correct aggregation, not raw overdraw.
```

Hard rule:

```text
Increasing chart refresh rate must never increase hardware acquisition risk.
```

---

### 9.3 Displayed Point Limit

Default max visible points per chart:

```text
1000-5000 points
```

Default v0.2 setting:

```text
3000 points per visible trace
```

If the backend has more data than visible pixels, it must return a downsampled view.

---

### 9.4 Downsampling Strategy

The backend should support multiple downsampling modes:

```text
latest_n
min_max_envelope
lttb
step_bucket_mean
step_bucket_median
```

Default for noise-sensitive time traces:

```text
min_max_envelope
```

Reason:

```text
simple averaging can hide spikes and small noise envelope changes
min/max envelope preserves local extrema visible to the operator
```

Default for smooth overview:

```text
lttb or bucket mean
```

The frontend may request a display mode, but the backend decides whether it is supported.

---

### 9.5 Chart Data Contract

Chart data emitted to frontend must be structured as chart-ready arrays.

Example conceptual structure:

```json
{
  "chart_id": "live_chb_x",
  "run_id": "run_2026_...",
  "step_id": "step_000123",
  "series": [
    {
      "name": "ch_b_x",
      "unit": "V",
      "x": [1000000, 1001000, 1002000],
      "y": [0.0000012, 0.0000014, 0.0000011],
      "x_unit": "monotonic_ns"
    }
  ],
  "markers": [
    {
      "type": "step_start",
      "timestamp_monotonic_ns": 1000000,
      "step_id": "step_000123"
    }
  ],
  "downsample": {
    "mode": "min_max_envelope",
    "source_points": 120000,
    "display_points": 3000
  }
}
```

The exact Rust structs are defined by implementation, but the contract must preserve:

```text
run_id
step_id
time base
display sample count
source sample count
downsample mode
unit metadata
marker metadata
```

---

### 9.6 Time Base

All real-time chart x-axis values must use:

```text
monotonic timestamp for alignment
```

The GUI may display wall time labels as secondary labels, but alignment must use monotonic time.

This aligns with Sub-PRD 06.

---

### 9.7 Event Markers

Live charts must support markers for:

```text
step_start
step_end
sample_window_start
sample_window_end
device_command_sent
device_command_returned
warning
error
overload
operator_stop_requested
emergency_stop
```

Markers should be visually distinct, but the first implementation can use simple vertical lines and tooltip labels.

---

### 9.8 Noise Visibility Requirements

The chart must help the operator observe subtle noise changes.

Required derived displays:

```text
rolling mean
rolling RMS
rolling peak-to-peak
min/max envelope
step-level mean and std
```

Optional later displays:

```text
Allan deviation
PSD
histogram
spectrogram
```

Important rule:

```text
Noise visibility must be achieved by correct backend metrics and aggregation, not by trying to draw every raw sample in the frontend.
```

---

## 10. Inputs

The GUI receives inputs from:

```text
station config files
profile files
block files
recipe files
resolved_recipe files
safety_report files
run directories
backend state snapshots
backend event streams
backend chart streams
mock/replay streams
operator actions
```

Specific input files:

```text
station.json
*.profile.json
*.block.json
*.recipe.json
resolved_recipe.json
safety_report.json
events.jsonl
station_snapshot.json
parsed parquet metadata
```

The GUI must not treat these files as authoritative if backend state says otherwise. The backend mediates file loading and validation.

---

## 11. Outputs

The GUI may produce:

```text
operator intent commands
selected recipe path
selected profile path
chart viewport request
chart display preferences
run approval action
stop / emergency stop action
export request
magnetic block export request
```

The GUI must not directly produce:

```text
raw bin
index.jsonl
events.jsonl
resolved_recipe.json
safety_report.json
parsed.parquet
export.csv
hardware command logs
```

Those outputs are produced by backend modules.

---

## 12. API / Interface

### 12.1 Command Naming Rule

Tauri command names should be explicit and operator-intent based.

Good:

```text
load_recipe(path)
validate_recipe(recipe_id)
compile_recipe(recipe_id)
get_dry_run_report(resolved_recipe_id)
approve_run(resolved_recipe_id)
start_run(approved_run_id)
request_stop(run_id)
emergency_stop(reason)
get_device_status()
connect_device(device_id)
disconnect_device(device_id)
request_chart_view(chart_id, viewport)
```

Bad:

```text
send_scpi(command)
read_serial(port)
write_raw_bin(bytes)
parse_rall(bytes)
set_laser_power_direct(value)
set_mag_current_direct(value)
```

---

### 12.2 Required Backend Commands

Minimum command set:

```text
app_get_status()
station_get_snapshot()
devices_list()
devices_connect(device_id)
devices_disconnect(device_id)
devices_query_idn(device_id)
recipe_load(path)
recipe_validate(recipe_id)
recipe_compile(recipe_id)
dryrun_get_report(resolved_recipe_id)
run_approve(resolved_recipe_id)
run_start(approved_run_id)
run_request_stop(run_id)
run_emergency_stop(reason)
run_get_status(run_id)
chart_get_view(chart_id, viewport)
logs_tail(run_id, filter)
runs_list()
runs_get_summary(run_id)
```

---

### 12.3 Required Backend Events

Minimum event set:

```text
app_status_changed
device_status_changed
recipe_validation_changed
dryrun_report_ready
safety_status_changed
run_status_changed
run_step_changed
acquisition_status_changed
chart_frame_ready
run_event_appended
warning_raised
error_raised
emergency_stop_activated
```

Events should be structured and versioned.

---

### 12.4 Frontend Command Safety

Every command that can lead to hardware action must be backend-validated.

Examples:

```text
run_start -> requires approved resolved_recipe
connect_device -> requires registered device profile
emergency_stop -> allowed from any frontend state
chart_get_view -> read-only
```

---

## 13. Safety Rules

### 13.1 Emergency Stop Visibility

Emergency stop must be visible from all main pages.

Recommended placement:

```text
persistent top bar or side bar
```

The UI must distinguish:

```text
red emergency stop button
normal stop button
graceful pause button
```

---

### 13.2 Safety Status Display

The GUI must display:

```text
safety pass / warning / fail
failed rule id
failed device id
failed parameter
operator-readable reason
affected run action
```

The GUI must not allow Start if safety failed.

---

### 13.3 Dangerous Command Preview

Dry-run page must show dangerous operations before real execution:

```text
turn RF output ON
turn MOD ON
set microwave power
set laser power
set magnetic current
change magnetic ramp
start sweep
```

The operator must see these actions before approving the run.

---

### 13.4 Safety Limit Editing

Safety limits must not be editable as ordinary GUI settings.

Forbidden ordinary GUI edits:

```text
laser max power
SMB max power
mag max current
mag ramp rate
OE1022D overload policy
emergency shutdown sequence
```

Any future safety editing UI must belong to Safety Interlock PRD and require explicit review workflow.

---

### 13.5 Manual Override

Manual override must be explicitly designed. It is not part of v0.2 normal GUI.

If added later, it must:

```text
require debug/developer mode
show red warning state
log every command
enforce safety limits
require resource lease
never bypass emergency stop
```

---

## 14. Error Handling

### 14.1 Backend Unavailable

If backend state cannot be reached:

```text
show backend unavailable state
disable Start / Connect / Run actions
keep Emergency Stop button visible if command path still exists
show last known state as stale
```

---

### 14.2 Device Disconnect

If a device disconnects:

```text
show device degraded state
show affected run capability
show executor state
show recovery option if backend supports it
```

The GUI must not attempt direct reconnect unless user requests it or backend has automatic policy.

---

### 14.3 Chart Backpressure

If chart data arrives faster than frontend can render:

```text
drop old chart frames
keep latest chart frame
show chart frame drop indicator
never request raw stream to catch up
```

Chart backpressure must not propagate to acquisition.

---

### 14.4 Frontend Reload

If webview reloads during a run:

```text
frontend reconnects to backend state
current run status rehydrates from backend
chart resumes from latest backend viewport data
raw acquisition continues uninterrupted
```

---

### 14.5 Log File Too Large

The GUI must tail and filter logs through backend APIs. It must not load the entire `events.jsonl` into memory for long runs.

---

## 15. Test Harness

The GUI must run without physical devices.

Required modes:

```text
mock station mode
fake device registry mode
fake run executor mode
fake live chart stream mode
raw replay mode
error injection mode
```

Mock UI must cover:

```text
all devices connected
some devices disconnected
recipe validation failed
safety failed
run active
run completed
run stopped
emergency stop active
OE1022D overload warning
chart frame drop warning
backend unavailable state
```

---

## 16. GUI Testing Requirements

### 16.1 Unit Tests

Required frontend tests:

```text
status rendering
button disabled/enabled conditions
dry-run report rendering
safety fail rendering
chart settings reducer
log filter reducer
```

---

### 16.2 Integration Tests

Required integration tests:

```text
load recipe -> validate -> compile -> approve -> start mock run
mock run -> live chart update -> event log update
safety fail -> Start disabled
emergency stop -> backend command emitted
backend unavailable -> dangerous actions disabled
```

---

### 16.3 Performance Tests

Required chart tests:

```text
render 1000 points at 30 Hz
render 3000 points at 30 Hz
render 5000 points at 10 Hz
zoom / pan under active updates
chart frame drop under artificial frontend delay
```

Required rule:

```text
chart performance tests must not rely on physical devices
```

---

## 17. Acceptance Criteria

The GUI module is accepted when:

```text
1. GUI can run in full mock mode without hardware.
2. GUI can connect to backend Device Registry and display device status.
3. GUI can load a recipe path and show validation result.
4. GUI can display dry-run and safety report.
5. GUI disables Start when schema, compiler, safety, or device requirements fail.
6. GUI can start a mock run through backend executor API.
7. GUI can display live Ch-B X chart using backend downsampled data.
8. GUI chart refresh works at 10-30 Hz without blocking backend acquisition.
9. GUI displays only 1000-5000 chart points by default.
10. Frontend never receives or parses RALL? raw binary frames.
11. Frontend never writes raw bin / index / events files directly.
12. Emergency stop is visible from all primary pages.
13. Frontend reload during mock run does not lose backend run state.
14. Run log viewer tails events through backend API.
15. GUI supports replay mode using existing run data.
```

Minimum performance acceptance:

```text
30 min mock live stream
chart updates stable at configured rate
no unbounded frontend memory growth
no acquisition drop caused by chart rendering
UI interaction remains responsive during active charting
```

---

## 18. Agent Constraints

### 18.1 GUI Agent Scope

A GUI coding agent may modify:

```text
frontend components
frontend routing
frontend state management
Tauri command client wrappers
chart rendering components
mock UI fixtures
UI tests
CSS / layout / theming
```

A GUI coding agent may add backend command wrappers only if the backend API already exists or is explicitly requested by Architecture PRD.

---

### 18.2 GUI Agent Forbidden Areas

A GUI coding agent must not modify:

```text
OE1022D acquisition core
RALL? parser
raw bin writer
SMB100A driver
laser driver
magnetic coil driver
safety rule engine
recipe compiler logic
executor hardware command logic
raw data schema
safety limit defaults
```

The agent must not add:

```text
frontend serial port access
frontend TCP socket hardware access
frontend SCPI console for normal users
frontend raw binary parser
frontend CSV real-time writer
frontend AI auto-run feature
```

---

### 18.3 GUI Agent Review Checklist

Before accepting GUI changes, review:

```text
Does the frontend still avoid direct hardware access?
Does the frontend still avoid raw binary parsing?
Does every dangerous action call backend validation?
Is emergency stop still visible?
Are disabled states correct for safety failure?
Does chart rendering use downsampled backend data?
Can UI run in mock mode?
Are long lists/logs virtualized or paginated?
Does reload recover from backend state?
Does this change touch forbidden modules?
```

---

## 19. Implementation Milestones

### 19.1 Milestone 1: Static Shell

Deliver:

```text
Tauri app bootstraps
frontend routing
Dashboard skeleton
Devices skeleton
Recipes skeleton
Run Control skeleton
Live Charts skeleton
Settings skeleton
persistent emergency stop placeholder
```

No hardware required.

---

### 19.2 Milestone 2: Mock Backend Integration

Deliver:

```text
mock device status
mock recipe validation result
mock dry-run report
mock run status stream
mock chart stream
mock events tail
```

Acceptance:

```text
operator workflow can be demonstrated end-to-end without devices
```

---

### 19.3 Milestone 3: Real Backend Integration

Deliver:

```text
Device Registry status binding
Recipe Compiler binding
Safety Report binding
Executor run status binding
OE1022D chart stream binding
Run Log binding
```

Acceptance:

```text
GUI controls real backend but still does not access hardware directly
```

---

### 19.4 Milestone 4: Chart Performance

Deliver:

```text
uPlot live Ch-B X chart
backend min/max envelope downsample
step markers
warning markers
viewport requests
chart performance test report
```

Acceptance:

```text
stable 10-30 Hz chart refresh
1000-5000 visible points
no frontend raw frame access
```

---

### 19.5 Milestone 5: Run Replay UI

Deliver:

```text
load completed run
show run summary
replay parsed samples to chart
show event markers
compare live-style view with historical run
```

Acceptance:

```text
replay works without devices and without modifying canonical run data
```

---

## 20. Open Questions

```text
1. React or Svelte final decision?
2. Should the first chart stream use push events or pull viewport requests?
3. Should chart downsampling be done in oe1022d-core or a separate chart-data crate?
4. Should the GUI include a JSON editor in v0.2, or only external editor + validation?
5. Should the Run Logs page support full-text search in v0.2 or only filters?
6. How much of Magnetic Planner should be embedded in v0.2?
7. Should debug command console exist at all in v0.2?
```

Default answer for v0.2:

```text
Keep GUI minimal.
Prefer external editor.
No debug command console in normal app.
Use backend-prepared chart data.
Implement mock mode before real hardware mode.
```

---

## 21. Summary

The GUI must serve the experiment, not own the experiment.

The first implementation should be deliberately narrow:

```text
Tauri + Web frontend
Rust backend APIs
uPlot live chart
10-30 Hz visual refresh
1000-5000 visible points
backend downsampling
no frontend raw frames
no frontend hardware access
persistent emergency stop
mock-first workflow
```

The GUI is accepted only if it improves operator control and visibility while preserving the real-time and safety boundaries defined by the architecture PRD.
