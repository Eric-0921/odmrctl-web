# Sub-PRD 07: Data Logging & File Format

Version: v0.2  
Status: Draft for implementation planning  
Owner: ODMR Automation Project  
Related PRDs:

- `00_main_prd.md`
- `01_architecture_prd.md`
- `03_oe1022d_acquisition_prd.md`
- `04_recipe_json_schema_prd.md`
- `05_recipe_compiler_executor_prd.md`
- `06_timing_sync_averaging_prd.md`
- `10_safety_interlock_prd.md`
- `11_harness_mock_replay_prd.md`
- `12_agent_workflow_prd.md`

---

## 1. Purpose

This PRD defines the data logging model and file formats for the ODMR automation system.

The goal is to prevent future data confusion by making every recorded data point traceable to:

```text
experiment run
resolved recipe step
device state
hardware command history
timing window
raw acquisition frame
parsed sample
post-processed dataset
```

The system must support two different modes of data handling:

```text
real-time acquisition path:
  minimal, append-only, crash-tolerant, high-throughput

post-run analysis path:
  rich, columnar, queryable, exportable, ML-ready
```

The central principle is:

```text
Real-time acquisition must not write CSV.
Real-time acquisition writes raw binary + index + structured events only.
Parquet and CSV are produced after the run or by an explicit conversion job.
```

---

## 2. Scope

This module is responsible for:

```text
run directory layout
raw binary file format
raw frame indexing
structured event logging
station snapshot logging
resolved recipe snapshot logging
post-run parsed parquet generation
CSV export format
file naming convention
schema versioning
append-only write strategy
crash recovery rules
checksum / integrity checks
replay compatibility
traceability between recipe steps and raw samples
```

This module defines the persistent data contract between:

```text
Executor
OE1022D acquisition core
SMB100A command executor
magnetic planner / magnetic executor
laser executor
GUI status view
replay harness
post-run parser
notebook / ML analysis
```

---

## 3. Non-goals

This module does not implement:

```text
real-time chart rendering
ODMR fitting algorithms
machine learning models
database server deployment
cloud synchronization
multi-user permission system
long-term archive policy
GUI file browser UX
device command drivers
recipe compilation logic
magnetic field planning logic
```

This module also explicitly does not allow:

```text
CSV as real-time acquisition format
GUI thread writing raw files
frontend parsing raw binary frames
AI agent modifying raw data files
analysis notebook overwriting canonical raw data
lossy compression of canonical raw frames
```

---

## 4. Design Principles

### 4.1 Canonical Data Principle

The canonical data for an experiment run is:

```text
raw binary frames
index.jsonl
events.jsonl
station_snapshot.json
resolved_recipe.json
```

Everything else is derived:

```text
parsed.parquet
sweep_points.parquet
summary.json
export.csv
figures
model datasets
```

Derived files can be deleted and regenerated. Canonical files must be append-only and immutable after the run is sealed.

---

### 4.2 Traceability Principle

Every parsed sample must be traceable to:

```text
run_id
step_id
substep_id, if present
raw file name
raw byte offset
raw frame sequence
monotonic timestamp
wall timestamp, if available
device state snapshot or device state reference
```

A later notebook or ML pipeline must be able to answer:

```text
Which recipe step produced this point?
What SMB100A frequency and power were active?
What magnetic field vector was requested?
What coil currents were commanded?
What OE1022D sensitivity and time constant were active?
Which raw bytes produced this parsed value?
Which events happened nearby?
```

---

### 4.3 Real-time Minimalism Principle

The real-time acquisition path writes only:

```text
raw/*.rawbin
index.jsonl
events.jsonl
minimal run_state.json, optional
```

It must not perform:

```text
CSV formatting
Parquet encoding
fitting
large JSON serialization per sample
plot downsampling persistence
machine learning inference
```

---

### 4.4 Append-only Principle

During a running experiment:

```text
raw binary is append-only
index.jsonl is append-only
events.jsonl is append-only
```

No process may rewrite the middle of these files during acquisition.

If a crash occurs, recovery is done by:

```text
reading the last valid index entry
checking raw file byte offsets
truncating incomplete raw tail only if necessary
writing a recovery event
marking the run as recovered
```

---

### 4.5 Derived Data Principle

Post-run converters generate:

```text
parsed.parquet
sweep_points.parquet
export.csv
summary.json
```

These files must record:

```text
converter version
input raw file checksums
input index checksum
input events checksum
created_at timestamp
schema version
```

---

## 5. Runtime Model

### 5.1 Write Path

```text
OE1022D acquisition thread
  -> raw frame queue
  -> raw bin writer
  -> index writer

Executor thread
  -> events writer
  -> run state updates

Device command executors
  -> events writer

Post-run parser
  -> reads raw + index + events
  -> writes parsed.parquet
  -> writes conversion_report.json
```

### 5.2 Thread Ownership

| Component | May write raw bin | May write index | May write events | May write parquet | May write CSV |
|---|---:|---:|---:|---:|---:|
| OE1022D acquisition thread | No | No | No | No | No |
| Raw writer thread | Yes | Yes | No | No | No |
| Executor | No | No | Yes | No | No |
| GUI | No | No | No | No | No |
| AI agent | No | No | No | No | No |
| Post-run converter | No | No | Yes, conversion events only | Yes | Yes |
| Replay tool | Optional replay output only | Optional replay output only | Optional replay events only | Yes | Yes |

The acquisition thread may enqueue raw frames, but it must not perform filesystem writes directly if that increases latency or jitter.

---

## 6. Run Directory Layout

Each run is stored in one immutable run directory.

### 6.1 Directory Naming

Recommended format:

```text
runs/YYYY-MM-DD/YYYY-MM-DDTHH-MM-SSZ_<run_id>_<short_name>/
```

Example:

```text
runs/2026-05-28/2026-05-28T15-42-10Z_run_01JXYZ_odmr_bz_sweep/
```

Rules:

```text
run_id must be globally unique
folder name must be filesystem-safe
short_name must not contain spaces or non-portable characters
wall timestamp uses UTC for directory naming
local timezone may be recorded in metadata
```

---

### 6.2 Required Layout

```text
<run_dir>/
  README.md
  manifest.json
  station_snapshot.json
  resolved_recipe.json
  safety_report.json
  run_config.json

  raw/
    oe1022d_ch_b.rawbin
    oe1022d_ch_b.index.jsonl

  events/
    events.jsonl
    warnings.jsonl
    errors.jsonl

  parsed/
    parsed.parquet
    sweep_points.parquet
    device_states.parquet
    events.parquet
    conversion_report.json

  exports/
    export.csv
    summary.json

  logs/
    executor.log
    acquisition.log
    conversion.log

  checksums/
    sha256sums.txt
```

### 6.3 Minimal Valid Run

A minimal valid run must contain:

```text
manifest.json
station_snapshot.json
resolved_recipe.json
safety_report.json
raw/oe1022d_ch_b.rawbin
raw/oe1022d_ch_b.index.jsonl
events/events.jsonl
```

If the run fails before acquisition starts, it may contain no raw frames, but it must still contain:

```text
manifest.json
station_snapshot.json
resolved_recipe.json
safety_report.json
events/events.jsonl
```

---

## 7. File Roles

### 7.1 `manifest.json`

Role:

```text
Top-level run metadata and file inventory.
```

Required fields:

```json
{
  "schema_version": "odmr.run_manifest.v0.2",
  "run_id": "run_01JXYZ...",
  "run_name": "odmr_bz_sweep",
  "created_at_utc": "2026-05-28T07:42:10Z",
  "created_at_local": "2026-05-28T15:42:10+08:00",
  "timezone": "Asia/Shanghai",
  "software": {
    "app_version": "0.2.0",
    "git_commit": "unknown",
    "rust_core_version": "0.2.0",
    "schema_pack_version": "0.2.0"
  },
  "status": "running",
  "files": []
}
```

Valid `status` values:

```text
created
running
completed
failed
aborted
emergency_stopped
recovered
sealed
```

Rules:

```text
manifest may be updated during run only for status and file inventory
canonical file contents must not be rewritten through manifest updates
on run seal, manifest receives final checksums and byte sizes
```

---

### 7.2 `station_snapshot.json`

Role:

```text
Frozen snapshot of station and device registry at run start.
```

Must include:

```text
station id
device ids
device profiles
transport addresses
IDN query results
capabilities
calibration references
safety limits active at run start
```

Rules:

```text
must be copied into run directory before first dangerous command
must not be modified after acquisition starts
must include enough metadata to interpret old runs after profiles change
```

---

### 7.3 `resolved_recipe.json`

Role:

```text
Frozen recipe after all profile references, blocks, sweeps, defaults, dwell, settle, averaging, and safety annotations have been expanded.
```

Rules:

```text
Executor may only execute resolved_recipe.json
original recipe is optional, resolved_recipe is mandatory
resolved_recipe must include stable step_id values
step_id values must be used in index and event files
```

---

### 7.4 `safety_report.json`

Role:

```text
Frozen safety validation result from compiler / safety checker.
```

Required top-level fields:

```json
{
  "schema_version": "odmr.safety_report.v0.2",
  "result": "passed",
  "checked_at_utc": "2026-05-28T07:42:05Z",
  "limits_source": "station_snapshot.json",
  "violations": [],
  "warnings": []
}
```

Rules:

```text
Executor refuses to run if result != passed
AI cannot edit this file
manual override, if allowed by future policy, must create explicit events and never silently modify limits
```

---

## 8. Raw Binary Format

### 8.1 File Name

Recommended default:

```text
raw/oe1022d_ch_b.rawbin
```

If multiple raw streams exist:

```text
raw/oe1022d_ch_a.rawbin
raw/oe1022d_ch_b.rawbin
raw/aux_adc.rawbin
```

---

### 8.2 Format Goal

The raw binary format stores bytes received from the device with minimal modification.

It must support:

```text
fast append
byte offset indexing
frame replay
parser compatibility testing
partial tail recovery
format versioning
checksum validation
```

---

### 8.3 Raw File Header

All `.rawbin` files begin with a fixed header.

Proposed header layout:

| Offset | Size | Type | Field | Description |
|---:|---:|---|---|---|
| 0 | 8 | ASCII | magic | `ODMRRAW\0` |
| 8 | 2 | u16 LE | major_version | `0` for v0.x |
| 10 | 2 | u16 LE | minor_version | `2` |
| 12 | 4 | u32 LE | header_len | total header length in bytes |
| 16 | 16 | bytes | run_uuid | binary UUID or zero-filled if unavailable |
| 32 | 8 | u64 LE | created_monotonic_ns | monotonic timestamp at file creation |
| 40 | 8 | i64 LE | created_wall_unix_ns | wall clock unix ns |
| 48 | 4 | u32 LE | stream_id_len | length of stream id string |
| 52 | N | UTF-8 | stream_id | e.g. `oe1022d.ch_b.raw` |
| 52+N | M | UTF-8 JSON | header_json | optional padded JSON metadata |

The header must be padded to a multiple of 64 bytes.

The raw frame payload starts at `header_len`.

---

### 8.4 Raw Frame Storage Policy

Two modes are allowed.

#### Mode A: Exact Device Bytes

```text
rawbin stores exactly the byte sequence received from OE1022D
frame boundaries are defined only in index.jsonl
```

Advantages:

```text
closest to hardware
best for parser regression
minimal write overhead
```

Disadvantages:

```text
index is required for random access
raw file alone cannot detect frame boundaries
```

#### Mode B: Length-prefixed Raw Frames

```text
rawbin stores each frame as:
  u32_le frame_len
  raw device bytes
```

Advantages:

```text
raw file can be scanned without index
partial tail recovery is easier
```

Disadvantages:

```text
not exact device stream
slightly more write overhead
```

### v0.2 Decision

Use **Mode B: length-prefixed raw frames** for ODMR v0.2.

Reason:

```text
replay and crash recovery are more important than preserving a byte-perfect continuous serial stream
OE1022D RALL? is command/response frame-based, so length-prefixed frames match the acquisition model
```

Raw frame payload layout:

| Field | Type | Description |
|---|---|---|
| frame_len | u32 LE | raw device byte length |
| frame_crc32 | u32 LE | CRC32 of raw device bytes |
| raw_bytes | `[u8; frame_len]` | bytes returned by device |

`index.jsonl` records the byte offset of `frame_len`.

---

## 9. Index Format: `index.jsonl`

### 9.1 Role

`index.jsonl` maps raw binary frames to experiment context.

It is the main bridge between:

```text
raw bytes
parsed values
recipe steps
device states
timing windows
```

---

### 9.2 File Name

```text
raw/oe1022d_ch_b.index.jsonl
```

---

### 9.3 Record Rules

Each line is one JSON object.

Rules:

```text
each line must be valid UTF-8
each line must be a complete JSON object
no trailing commas
records are append-only
record order must match acquisition order
sequence numbers must be monotonic
```

---

### 9.4 Required Fields

Example record:

```json
{
  "schema_version": "odmr.raw_index.v0.2",
  "run_id": "run_01JXYZ",
  "stream_id": "oe1022d.ch_b.raw",
  "seq": 128,
  "raw_file": "raw/oe1022d_ch_b.rawbin",
  "raw_offset": 40960,
  "raw_record_len": 84,
  "raw_payload_len": 76,
  "raw_crc32": "0x91a2b3c4",
  "acq_request_mono_ns": 918273645000,
  "acq_response_mono_ns": 918273695000,
  "writer_mono_ns": 918273700000,
  "wall_time_utc": "2026-05-28T07:42:15.123456Z",
  "step_id": "step_000128",
  "substep_id": null,
  "sample_window_id": "win_000128",
  "device_state_ref": "state_000128",
  "parser_status": "unparsed"
}
```

### 9.5 Field Definitions

| Field | Required | Meaning |
|---|---:|---|
| `schema_version` | Yes | Index schema version |
| `run_id` | Yes | Unique run id |
| `stream_id` | Yes | Raw stream id |
| `seq` | Yes | Monotonic raw frame sequence number |
| `raw_file` | Yes | Relative path to rawbin file |
| `raw_offset` | Yes | Byte offset of length prefix |
| `raw_record_len` | Yes | `4 + 4 + raw_payload_len` for v0.2 |
| `raw_payload_len` | Yes | Raw device byte length |
| `raw_crc32` | Yes | CRC32 of raw device bytes |
| `acq_request_mono_ns` | Yes | Monotonic timestamp before command/read request |
| `acq_response_mono_ns` | Yes | Monotonic timestamp after response completed |
| `writer_mono_ns` | Yes | Monotonic timestamp after write queued or written |
| `wall_time_utc` | Yes | Best-effort wall timestamp |
| `step_id` | Yes | Resolved recipe step id active at acquisition |
| `substep_id` | No | Optional nested operation id |
| `sample_window_id` | No | Timing window id from PRD 06 |
| `device_state_ref` | No | Device state event or snapshot id |
| `parser_status` | Yes | `unparsed`, `parsed`, `parse_error`, `skipped` |

---

## 10. Event Format: `events.jsonl`

### 10.1 Role

`events.jsonl` stores structured events from the executor and device drivers.

Events must cover:

```text
run lifecycle
recipe step lifecycle
device command send / response
settle start / settle end
sample window start / sample window end
errors
warnings
safety rejections
manual stop
emergency stop
conversion start / end
```

---

### 10.2 File Name

```text
events/events.jsonl
```

Warnings and errors may also be duplicated into:

```text
events/warnings.jsonl
events/errors.jsonl
```

The canonical event stream remains `events.jsonl`.

---

### 10.3 Event Record

Example:

```json
{
  "schema_version": "odmr.run_event.v0.2",
  "run_id": "run_01JXYZ",
  "event_id": "evt_00000128",
  "event_type": "device.command.completed",
  "severity": "info",
  "mono_ns": 918273600000,
  "wall_time_utc": "2026-05-28T07:42:15.120000Z",
  "step_id": "step_000128",
  "device_id": "smb100a.main",
  "correlation_id": "cmd_000128",
  "payload": {
    "command": "FREQ 2.882GHz",
    "transport": "tcp_scpi",
    "duration_ms": 12.4,
    "result": "ok"
  }
}
```

---

### 10.4 Event Type Convention

Use dot-separated event names:

```text
run.created
run.started
run.completed
run.failed
run.aborted
run.emergency_stopped
run.sealed

recipe.step.started
recipe.step.completed
recipe.step.failed

safety.check.started
safety.check.passed
safety.check.failed
safety.limit.warning
safety.limit.violation

resource.lease.requested
resource.lease.granted
resource.lease.released
resource.lease.denied

device.connect.started
device.connect.completed
device.connect.failed
device.command.sent
device.command.completed
device.command.failed
device.query.sent
device.query.completed
device.query.failed
device.disconnect.started
device.disconnect.completed

acquisition.started
acquisition.frame.received
acquisition.frame.dropped
acquisition.frame.parse_error
acquisition.stopped

settle.started
settle.completed
sample_window.started
sample_window.completed
average.started
average.completed

conversion.started
conversion.completed
conversion.failed
```

---

### 10.5 Severity Levels

```text
trace
debug
info
warning
error
critical
```

Rules:

```text
safety violations must be warning/error/critical
emergency stop must be critical
recoverable communication timeout may be warning or error depending on retry result
normal command completion is info or debug
```

---

## 11. Device State Snapshot Model

### 11.1 Why Device State Is Needed

A parsed sample without device state is not scientifically useful.

Each sample or sample window must be associated with the active values of:

```text
SMB100A frequency
SMB100A power
SMB100A RF output state
SMB100A modulation state
SMB100A FM deviation
SMB100A LF generator frequency / shape / amplitude
magnetic field target
coil currents
laser power / state
OE1022D sensitivity
OE1022D time constant
OE1022D filter slope
OE1022D reference source
OE1022D channel and displayed quantity
```

---

### 11.2 v0.2 Decision

Device state is logged through events and periodically compacted into `parsed/device_states.parquet` after the run.

During real-time acquisition:

```text
events.jsonl records device command results and state-changing actions
index.jsonl stores device_state_ref when available
```

After post-run conversion:

```text
parsed.parquet contains denormalized key device state columns for analysis
parsed/device_states.parquet contains the full normalized state timeline
```

---

## 12. Parsed Parquet Format

### 12.1 Role

`parsed.parquet` is the primary analysis-ready table generated after the run.

It is used by:

```text
Python notebooks
Polars / DuckDB pipelines
ML dataset builders
ODMR fitting scripts
paper figure generation
```

---

### 12.2 File Name

```text
parsed/parsed.parquet
```

---

### 12.3 Required Column Groups

#### Identity Columns

```text
run_id: string
stream_id: string
seq: uint64
step_id: string
substep_id: string|null
sample_window_id: string|null
raw_file: string
raw_offset: uint64
```

#### Time Columns

```text
acq_request_mono_ns: uint64
acq_response_mono_ns: uint64
sample_mid_mono_ns: uint64
wall_time_utc: timestamp[ns, UTC]
step_elapsed_ms: float64
run_elapsed_ms: float64
```

#### OE1022D Parsed Columns

```text
oe_channel: string
x_v: float64
y_v: float64
r_v: float64
theta_deg: float64
freq_hz: float64
noise_vrms: float64|null
x_raw_text: string|null
y_raw_text: string|null
r_raw_text: string|null
theta_raw_text: string|null
parser_status: string
parser_error: string|null
```

#### OE1022D Setting Columns

```text
oe_sensitivity: string|null
oe_sensitivity_v: float64|null
oe_time_constant_s: float64|null
oe_filter_slope_db_oct: int32|null
oe_dynamic_reserve: string|null
oe_reference_source: string|null
oe_reference_frequency_hz: float64|null
oe_input_source: string|null
oe_input_coupling: string|null
oe_notch_filter: string|null
```

#### SMB100A Columns

```text
smb_frequency_hz: float64|null
smb_power_dbm: float64|null
smb_rf_output: bool|null
smb_mod_output: bool|null
smb_fm_state: bool|null
smb_fm_deviation_hz: float64|null
smb_lf_frequency_hz: float64|null
smb_lf_shape: string|null
smb_lf_voltage_v: float64|null
smb_sweep_state: string|null
```

#### Magnetic Field Columns

```text
b_target_x_t: float64|null
b_target_y_t: float64|null
b_target_z_t: float64|null
b_target_abs_t: float64|null
b_target_theta_deg: float64|null
b_target_phi_deg: float64|null
coil_x_current_a: float64|null
coil_y_current_a: float64|null
coil_z_current_a: float64|null
mag_settle_time_s: float64|null
```

#### Laser Columns

```text
laser_state: string|null
laser_power_mw: float64|null
laser_current_ma: float64|null
laser_warmup_s: float64|null
```

#### Timing and Averaging Columns

```text
dwell_s: float64|null
settle_s: float64|null
discard_s: float64|null
average_count: int32|null
average_index: int32|null
sample_window_start_mono_ns: uint64|null
sample_window_end_mono_ns: uint64|null
```

#### Quality Columns

```text
is_valid: bool
quality_flags: list<string>
warning_count_nearby: int32
error_count_nearby: int32
overload_flag: bool|null
dropped_frame_count_nearby: int32
```

---

### 12.4 Parquet Metadata

Each Parquet file must include file-level metadata:

```text
odmr.schema_version = odmr.parsed.v0.2
odmr.run_id = <run_id>
odmr.source_raw_sha256 = <sha256>
odmr.source_index_sha256 = <sha256>
odmr.source_events_sha256 = <sha256>
odmr.converter_version = <version>
odmr.created_at_utc = <timestamp>
```

---

## 13. Sweep Points Parquet

### 13.1 Role

`sweep_points.parquet` aggregates raw samples into one row per resolved recipe step or logical sweep point.

It is used for:

```text
ODMR line plotting
frequency sweep fitting
magnetic sweep fitting
ML training table
fast notebook loading
```

---

### 13.2 File Name

```text
parsed/sweep_points.parquet
```

---

### 13.3 Required Columns

```text
run_id
step_id
sweep_index
point_index
n_samples
n_valid_samples
x_mean_v
x_std_v
x_min_v
x_max_v
y_mean_v
y_std_v
r_mean_v
r_std_v
theta_mean_deg
freq_mean_hz
noise_mean_vrms
smb_frequency_hz
smb_power_dbm
b_target_x_t
b_target_y_t
b_target_z_t
b_target_abs_t
coil_x_current_a
coil_y_current_a
coil_z_current_a
laser_power_mw
oe_time_constant_s
oe_sensitivity_v
quality_flags
```

Rules:

```text
aggregation window must be derived from resolved_recipe and PRD 06 timing rules
front discard must not be included in mean/std unless explicitly requested
all aggregation decisions must be recorded in conversion_report.json
```

---

## 14. CSV Export Format

### 14.1 Role

`export.csv` is a compatibility export only.

It is intended for:

```text
quick inspection
Excel / Origin import
sharing with collaborators
legacy comparison
```

It is not canonical.

---

### 14.2 File Name

```text
exports/export.csv
```

---

### 14.3 Rules

```text
CSV must be generated from parsed.parquet or sweep_points.parquet
CSV must not be written during real-time acquisition
CSV must include header row
CSV must use UTF-8
CSV must use comma separator
missing values should be empty cells
units must be encoded in column names or export metadata
```

Recommended export modes:

```text
sample_level_csv
sweep_point_csv
summary_csv
```

Default:

```text
sweep_point_csv
```

---

### 14.4 Required CSV Header for Sweep Point Export

```text
run_id,step_id,sweep_index,point_index,n_samples,n_valid_samples,
smb_frequency_hz,smb_power_dbm,
b_target_x_t,b_target_y_t,b_target_z_t,b_target_abs_t,
coil_x_current_a,coil_y_current_a,coil_z_current_a,
laser_power_mw,
oe_time_constant_s,oe_sensitivity_v,
x_mean_v,x_std_v,x_min_v,x_max_v,
y_mean_v,y_std_v,
r_mean_v,r_std_v,
theta_mean_deg,
quality_flags
```

---

## 15. Summary File

### 15.1 File Name

```text
exports/summary.json
```

### 15.2 Role

`summary.json` gives a small machine-readable overview of the run.

Example:

```json
{
  "schema_version": "odmr.run_summary.v0.2",
  "run_id": "run_01JXYZ",
  "run_name": "odmr_bz_sweep",
  "status": "completed",
  "started_at_utc": "2026-05-28T07:42:10Z",
  "ended_at_utc": "2026-05-28T08:12:10Z",
  "duration_s": 1800.0,
  "raw_frame_count": 36000,
  "parsed_sample_count": 36000,
  "valid_sample_count": 35982,
  "dropped_frame_count": 0,
  "warning_count": 3,
  "error_count": 0,
  "emergency_stop": false,
  "files": {
    "parsed": "parsed/parsed.parquet",
    "sweep_points": "parsed/sweep_points.parquet",
    "csv": "exports/export.csv"
  }
}
```

---

## 16. Checksums and Integrity

### 16.1 Required Checksums

At run seal, generate:

```text
checksums/sha256sums.txt
```

Include at least:

```text
manifest.json
station_snapshot.json
resolved_recipe.json
safety_report.json
raw/*.rawbin
raw/*.index.jsonl
events/events.jsonl
parsed/*.parquet, if generated
exports/*.csv, if generated
```

### 16.2 Recovery from Interrupted Runs

If the process crashes:

```text
1. open manifest.json
2. inspect status
3. scan rawbin length-prefixed records
4. validate CRC32 per raw frame
5. compare valid raw records with index.jsonl
6. ignore incomplete index tail lines
7. truncate incomplete raw tail only when needed
8. write run.recovered event
9. set manifest status to recovered or failed
```

Recovery must never fabricate raw data.

---

## 17. File Locking and Atomicity

### 17.1 During Run

The run directory must contain a lock file:

```text
.run.lock
```

The lock file should include:

```json
{
  "run_id": "run_01JXYZ",
  "pid": 12345,
  "host": "lab-pc-01",
  "created_at_utc": "2026-05-28T07:42:10Z"
}
```

### 17.2 After Run

When the run is complete and sealed:

```text
remove .run.lock
write .run.sealed
```

`run.sealed` contains:

```json
{
  "run_id": "run_01JXYZ",
  "sealed_at_utc": "2026-05-28T08:12:30Z",
  "manifest_sha256": "..."
}
```

### 17.3 Atomic File Writes

For JSON snapshots:

```text
write to filename.tmp
fsync file
rename to final name
fsync parent directory where supported
```

For append-only files:

```text
append complete record
flush according to configured durability policy
never rewrite previous records
```

---

## 18. Durability Policy

### 18.1 Configurable Durability Levels

```text
fast:
  OS buffered writes, periodic flush

balanced:
  flush every N frames or every T seconds

safe:
  fsync critical files frequently
```

Default for v0.2:

```text
balanced
```

Recommended default values:

```text
flush raw/index every 1 s or every 256 frames
flush events every event for critical severity
fsync snapshots at creation
fsync at run seal
```

---

## 19. API / Interface

### 19.1 Rust Interfaces

Suggested crates / modules:

```text
odmr-data-core
  RunDirectory
  RunManifest
  RawBinWriter
  RawIndexWriter
  EventWriter
  RunSealer
  RunRecovery

odmr-data-convert
  RawReplayReader
  Oe1022dParserAdapter
  ParsedParquetWriter
  CsvExporter
```

### 19.2 Example Interface Sketch

```rust
pub struct RunDirectory {
    pub run_id: RunId,
    pub root: PathBuf,
}

pub trait RawFrameSink {
    fn append_frame(&mut self, ctx: FrameContext, bytes: &[u8]) -> Result<RawFrameRef>;
    fn flush(&mut self) -> Result<()>;
}

pub trait EventSink {
    fn emit(&mut self, event: RunEvent) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}

pub trait RunConverter {
    fn convert_to_parquet(&self, run_dir: &RunDirectory) -> Result<ConversionReport>;
    fn export_csv(&self, run_dir: &RunDirectory, mode: CsvExportMode) -> Result<PathBuf>;
}
```

---

## 20. Safety Rules

Data logging must enforce safety-relevant traceability.

Required rules:

```text
run cannot start without station_snapshot.json
run cannot start without resolved_recipe.json
run cannot start without safety_report.json result = passed
all safety rejections must be written to events.jsonl
all emergency stops must be written to events.jsonl when possible
manifest must mark emergency_stopped status when applicable
canonical raw/index/events must not be deleted by GUI
AI agent must not modify safety_report or canonical run data
```

If event writing fails during a safety-critical operation:

```text
executor must prefer safe shutdown over continuing an untraceable experiment
```

---

## 21. Error Handling

### 21.1 Disk Full

If disk is full:

```text
stop acquisition
emit acquisition.stopped if possible
emit run.failed if possible
execute safe shutdown
mark manifest failed if possible
show GUI critical error
```

### 21.2 Raw Write Failure

```text
stop acquisition immediately
release device leases
safe shutdown dangerous outputs
mark run failed
```

### 21.3 Index Write Failure

Index failure is critical because traceability is lost.

```text
stop acquisition immediately
mark run failed
raw frames after last valid index entry must not be treated as canonical parsed data
```

### 21.4 Event Write Failure

If event write failure is transient:

```text
retry with backoff
buffer small number of events in memory
```

If persistent:

```text
stop run
safe shutdown
mark run failed if possible
```

### 21.5 Parquet Conversion Failure

Parquet conversion failure does not invalidate raw data.

```text
write conversion.failed event
write conversion_report.json with error
keep raw/index/events untouched
allow retry
```

### 21.6 CSV Export Failure

CSV export failure does not invalidate raw or parsed data.

```text
write export failure to conversion_report.json
allow retry
```

---

## 22. Test Harness

### 22.1 Required Tests

```text
raw bin writer unit tests
raw index writer unit tests
events.jsonl writer unit tests
run directory creation tests
manifest schema tests
station snapshot copy tests
resolved recipe copy tests
safety report presence tests
mock RALL frame recording tests
raw replay tests
partial raw tail recovery tests
partial JSONL line recovery tests
parquet conversion tests
CSV export tests
checksum tests
```

### 22.2 Replay Tests

Replay test flow:

```text
1. load run directory
2. validate manifest
3. validate checksums if sealed
4. read rawbin + index
5. parse all indexed frames
6. regenerate parsed.parquet
7. compare parsed values with expected output
```

### 22.3 Crash Simulation Tests

Must simulate:

```text
crash after raw write before index write
crash after partial raw frame write
crash after partial index line write
crash after event write before manifest update
crash during parquet conversion
```

### 22.4 Long-run Test

Minimum v0.2 test:

```text
30 min continuous acquisition
no corrupted raw record
no invalid JSONL line except intentionally injected crash case
raw replay produces same parsed count as live parser
UI activity does not affect raw write throughput
```

---

## 23. Acceptance Criteria

### 23.1 Functional Acceptance

```text
system creates complete run directory before acquisition starts
rawbin and index are written during acquisition
index seq is monotonic
events.jsonl captures run, step, command, acquisition, error, and stop events
post-run converter generates parsed.parquet
CSV export is generated only after parsed data exists
run can be replayed from rawbin + index
```

### 23.2 Traceability Acceptance

For any row in `parsed.parquet`, the system can identify:

```text
raw file
raw byte offset
raw sequence number
recipe step id
active SMB100A settings
active OE1022D settings
active magnetic target / coil currents
active laser state
nearby warnings/errors
```

### 23.3 Performance Acceptance

```text
raw logging does not reduce target OE1022D acquisition rate
real-time path does not perform CSV or Parquet writes
append latency remains bounded under normal disk conditions
30 min continuous acquisition succeeds
```

### 23.4 Recovery Acceptance

```text
incomplete raw tail can be detected
partial JSONL line can be ignored or repaired safely
recovered run is marked as recovered
recovery never fabricates missing frames
post-run parser refuses unindexed raw tail by default
```

---

## 24. Agent Constraints

AI agents may:

```text
read PRDs
read schema definitions
propose data format changes
generate converter code
write tests
generate notebooks using parsed parquet
summarize run metadata
```

AI agents must not:

```text
modify canonical rawbin files
modify index.jsonl from completed runs
modify events.jsonl from completed runs
edit station_snapshot.json after run start
edit resolved_recipe.json after run start
edit safety_report.json
silently change schema versions
produce CSV as canonical data
remove warnings/errors from event logs
```

Any AI-generated migration must:

```text
write a new derived file
preserve original canonical files
write a migration_report.json
include old and new schema versions
include checksum of source files
```

---

## 25. Open Decisions

| ID | Question | Default v0.2 Decision |
|---|---|---|
| DLOG-001 | Raw format exact stream or length-prefixed frames? | Length-prefixed frames |
| DLOG-002 | CRC32 per frame or only SHA256 per file? | Both: CRC32 per frame, SHA256 per file at seal |
| DLOG-003 | Store full device state per sample or reconstruct from events? | Reconstruct from events, denormalize in parsed parquet |
| DLOG-004 | Default CSV export level? | Sweep point level |
| DLOG-005 | Compress rawbin? | No compression in v0.2 |
| DLOG-006 | Parquet row group size? | TBD after benchmark |
| DLOG-007 | Event schema fully OpenTelemetry-compatible? | No, only naming style is inspired; keep ODMR-specific schema |

---

## 26. Implementation Milestones

### Milestone 1: Minimal Run Directory

```text
RunDirectory builder
manifest.json
station_snapshot.json copy
resolved_recipe.json copy
safety_report.json copy
events writer
```

### Milestone 2: Raw Logging

```text
rawbin writer
index.jsonl writer
CRC32 per frame
mock RALL frame recording
basic replay reader
```

### Milestone 3: Conversion

```text
OE1022D raw parser adapter
parsed.parquet writer
sweep_points.parquet writer
conversion_report.json
CSV exporter
```

### Milestone 4: Recovery and Integrity

```text
partial raw recovery
partial JSONL recovery
run seal
sha256sums.txt
replay validation
```

### Milestone 5: Long-run Validation

```text
30 min mock acquisition
30 min real OE1022D acquisition
UI active during acquisition
replay equality check
benchmark report
```

---

## 27. External Format Notes

This PRD uses:

```text
JSON Lines / JSONL for append-only structured event and index logs
Apache Parquet for post-run columnar analytical datasets
CSV only as a compatibility export format
```

Rationale:

```text
JSONL supports one structured record per line and is suitable for logs and stream-like append patterns.
Parquet is a column-oriented format suitable for efficient analytical storage and retrieval.
CSV is widely compatible but too weak for canonical metadata, nested state, typing, and real-time high-rate acquisition.
```

---

## 28. Final v0.2 Rule Summary

```text
Canonical during run:
  rawbin + index.jsonl + events.jsonl + frozen snapshots

Derived after run:
  parsed.parquet + sweep_points.parquet + export.csv

Never:
  real-time CSV
  GUI writes raw data
  AI edits canonical run data
  parsed row without recipe/device traceability

Always:
  step_id traceability
  byte offset traceability
  event traceability
  safety report before execution
  replay from raw data
```
