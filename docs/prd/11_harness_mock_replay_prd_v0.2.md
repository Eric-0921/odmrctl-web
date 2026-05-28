# Sub-PRD 11: Harness / Mock / Replay Test

Version: v0.2  
Status: Draft for implementation planning  
Owner: ODMR Automation Project  
Target file: `docs/prd/11_harness_mock_replay_prd.md`  
Related PRDs:

- `00_main_prd.md`
- `01_architecture_prd.md`
- `02_device_registry_connection_prd.md`
- `03_oe1022d_acquisition_prd.md`
- `04_recipe_json_schema_prd.md`
- `05_recipe_compiler_executor_prd.md`
- `06_timing_sync_averaging_prd.md`
- `07_data_logging_file_format_prd.md`
- `08_gui_tauri_chart_prd.md`
- `09_magnetic_field_planner_prd.md`
- `10_safety_interlock_prd.md`
- `12_agent_workflow_prd.md`

External references:

- pytest monkeypatch / mock environment documentation: <https://docs.pytest.org/en/stable/how-to/monkeypatch.html>
- OpenTelemetry Semantic Conventions: <https://opentelemetry.io/docs/concepts/semantic-conventions/>

---

## 1. Purpose

This PRD defines the **Harness / Mock / Replay Test** system for the ODMR automation project.

The harness exists because multi-agent development without a shared executable test environment will drift. Different agents may implement device drivers, recipe compiler logic, safety checks, GUI code, and data logging assumptions differently. The harness provides a single standard for proving that a module works before it touches real instruments.

Core principle:

```text
The code must run without connected instruments.
When instruments are connected, the same interface must be able to run real serial / TCP / SCPI / acquisition paths.
```

The harness must support both:

```text
offline deterministic tests:
  fake OE1022D
  fake RALL frame
  fake SMB100A SCPI server
  fake laser serial
  fake magnetic axis
  fake clock
  fake filesystem

real hardware validation:
  real serial connection
  real TCP SCPI connection
  real command transmission
  real device response
  real acquisition replay comparison
```

The goal is not only unit testing. The goal is to make the whole experiment stack testable without unsafe hardware operations.

---

## 2. Scope

This module is responsible for:

```text
fake device implementations
mock transport interfaces
raw RALL frame generators
raw bin replay tools
recipe validation tests
dry-run tests
safety rejection tests
timestamp alignment tests
executor simulation tests
GUI mock backend tests
benchmark harness
real-device smoke test gates
CI test profiles
agent acceptance test fixtures
```

The module must provide a standard test surface for:

```text
Rust crates
Tauri backend commands
TypeScript GUI tests
Python recipe / AI generation scripts
post-run parsing tools
schema validation tools
```

---

## 3. Non-goals

This module does not:

```text
control real hardware during normal CI
replace real instrument acceptance testing
simulate full NV-center physics
produce final scientific conclusions
perform ML training
render final publication figures
make unsafe commands safe
bypass Safety & Interlock rules
```

The fake devices are behavioral test doubles, not high-fidelity physical simulators.

The harness should simulate enough behavior to test software correctness:

```text
command syntax
state transition
latency
jitter
timeout
overload
frame corruption
dropped frames
reconnection
safety rejection
log consistency
```

It should not attempt to fully model:

```text
laser-optical response
NV spin dynamics
microwave field distribution
coil heating
lock-in DSP internals
true physical magnetic hysteresis
```

---

## 4. Design Principles

### 4.1 Offline-first

Every core module must be runnable on a developer machine without attached instruments.

```text
cargo test
cargo bench
pnpm test
python -m pytest
```

must work in a no-hardware environment.

### 4.2 Real interface parity

Mocks must use the same interface shape as real devices.

```text
Real OE1022D serial transport  -> implements SerialTransport
Fake OE1022D serial transport  -> implements SerialTransport

Real SMB100A TCP SCPI transport -> implements ScpiTransport
Fake SMB100A SCPI server        -> speaks TCP + SCPI-like text protocol

Real magnetic controller        -> implements MagAxisTransport
Fake magnetic axis              -> implements MagAxisTransport
```

No production code should contain special logic such as:

```text
if mock_mode:
    skip safety
if test_mode:
    bypass parser
```

Test mode may replace transports, but it must not bypass validation logic.

### 4.3 Determinism first, noise optional

Default fake behavior must be deterministic.

```text
same seed
same recipe
same fake frame script
same expected events
same parsed samples
```

Noise, jitter, frame drops, and latency should be explicitly enabled by test scenario.

### 4.4 Fail visibly

Harness failures must produce inspectable artifacts:

```text
test_run/
  resolved_recipe.json
  safety_report.json
  events.jsonl
  fake_device_logs.jsonl
  raw/
    oe1022d.rawbin
    index.jsonl
  parsed/
    samples.parquet or samples.jsonl
  diffs/
    expected_vs_actual.json
```

### 4.5 Safety remains active

Mock mode must still enforce safety rules.

```text
unsafe recipe must fail in mock mode
unsafe recipe must fail in dry-run mode
unsafe recipe must fail before executor starts
```

---

## 5. Inputs

The harness consumes:

```text
station.json
device_profile.json
block.json
recipe.json
resolved_recipe.json
safety_report.json
fake_device_profile.json
fake_frame_script.json
raw bin replay files
index.jsonl
events.jsonl
expected_output snapshots
```

### 5.1 Fake device scenario file

A fake device scenario defines expected behavior.

Example:

```json
{
  "scenario_id": "oe1022d_chb_normal_500hz_v1",
  "seed": 12345,
  "devices": {
    "oe1022d_chb": {
      "type": "fake_oe1022d",
      "transport": "fake_serial",
      "frame_mode": "normal",
      "sample_rate_hz": 20,
      "x_baseline_v": 0.00042,
      "noise_rms_v": 0.00001,
      "latency_ms": 4,
      "jitter_ms": 1
    },
    "smb100a": {
      "type": "fake_smb100a_scpi",
      "transport": "tcp",
      "host": "127.0.0.1",
      "port": 5025,
      "idn": "Rohde&Schwarz,SMB100A,FAKE123,5.00.116"
    }
  }
}
```

---

## 6. Outputs

The harness produces:

```text
test result summary
structured test logs
fake device command logs
executor event logs
raw bin files
index.jsonl
parsed replay output
benchmark report
coverage report
snapshot diff report
real hardware smoke report
```

Required output files for full integration tests:

```text
test_artifacts/<test_id>/
  test_summary.json
  resolved_recipe.json
  safety_report.json
  events.jsonl
  fake_devices.jsonl
  raw/
    oe1022d.rawbin
    index.jsonl
  parsed/
    samples.jsonl
  diffs/
    event_diff.json
    sample_diff.json
  benchmark/
    timing_report.json
```

---

## 7. Runtime Model

### 7.1 Offline integration runtime

```text
Test Runner
  |
  |-- starts fake SMB100A SCPI server
  |-- starts fake OE1022D serial transport
  |-- starts fake laser serial transport
  |-- starts fake magnetic axis controller
  |-- installs fake clock if needed
  |
  |-- invokes schema validator
  |-- invokes compiler
  |-- invokes safety checker
  |-- invokes executor in mock transport mode
  |
  |-- records events.jsonl
  |-- records fake device logs
  |-- records raw bin
  |-- runs replay parser
  |-- compares expected vs actual
```

### 7.2 Real hardware gated runtime

```text
Real Hardware Smoke Test Runner
  |
  |-- requires explicit CLI flag
  |-- requires station profile
  |-- requires safety profile
  |-- requires operator confirmation
  |-- connects to real device
  |-- queries IDN / status
  |-- sends low-risk command
  |-- checks response
  |-- disconnects safely
  |-- writes smoke_report.json
```

Real hardware tests must never run by default in CI.

Required gate:

```text
--real-hardware
--station <station.json>
--operator-confirmed
```

---

## 8. Harness Components

### 8.1 Fake OE1022D

The fake OE1022D module must simulate:

```text
ASCII command parsing
IDN query
Ch-A / Ch-B channel selection
input / filter / sensitivity commands
reference commands
RALL? acquisition response
normal frame stream
noise frame stream
overload state
truncated frame
invalid frame checksum if applicable
delayed response
timeout
serial disconnect
reconnect
```

Minimum supported test commands:

```text
*IDN? or equivalent ID query
FMODD? i
PHASD? i
SENSD? i
TCOLD? i
RALL? i
```

The exact command set should follow OE1022D driver implementation, but fake coverage must include every command used by production executor.

### 8.2 Fake RALL frame generator

The fake RALL generator must support:

```text
constant X/Y/R/theta
sinusoidal drift
Gaussian-like deterministic pseudo-noise
step response after recipe step change
overload marker
frame truncation
frame duplication
out-of-order timestamp marker
missing frame
```

Modes:

```text
normal
noisy
slow_drift
overload
truncated_frame
dropped_frame
burst_jitter
parser_regression
```

The generator must produce both:

```text
raw bytes for parser tests
semantic expected values for validation
```

### 8.3 Fake SMB100A SCPI server

The fake SMB100A server must run as a TCP server and accept simple SCPI-like text commands.

Minimum supported commands:

```text
*IDN?
SYST:ERR?
FREQ?
FREQ <value>
POW?
POW <value>
OUTP?
OUTP ON|OFF
MOD:STAT?
MOD:STAT ON|OFF
FM:STAT?
FM:STAT ON|OFF
FM:DEV?
FM:DEV <value>
FM:SOUR?
FM:SOUR INT|EXT
LFO:STAT?
LFO:STAT ON|OFF
LFO:FREQ?
LFO:FREQ <value>
LFO:VOLT?
LFO:VOLT <value>
SWE:MODE?
SWE:MODE <mode>
```

The fake server must maintain internal state and return query responses from state.

Error injection:

```text
unknown command
invalid numeric value
out-of-range value
command timeout
connection drop
error queue non-empty
```

### 8.4 Fake laser serial

The fake laser module must simulate:

```text
connect / disconnect
query status
set power
enable output
disable output
warmup delay
interlock open
interlock close
fault state
```

It must reject values above fake station safety limit.

### 8.5 Fake magnetic axis

The fake magnetic module must simulate:

```text
x/y/z current setpoint
current readback
ramp rate
settle time
current limit
zero offset
coil matrix output preview
axis fault
overcurrent fault
thermal warning
```

It must be usable by Magnetic Planner tests and Executor tests.

### 8.6 Fake clock

The fake clock must support:

```text
monotonic timestamp
wall timestamp
time advance
jitter injection
step start time
command send time
command return time
dwell completion time
settle completion time
acquisition window start/end
```

Fake clock is required for deterministic timing tests.

### 8.7 Fake filesystem

The fake filesystem may be implemented either as a temp directory or an in-memory adapter.

It must test:

```text
run directory creation
raw bin append
index.jsonl append
events.jsonl append
partial write
crash recovery
read-only path failure
disk full simulation if practical
```

---

## 9. Test Categories

### 9.1 Schema validation tests

Tests:

```text
valid station.json passes
valid device_profile.json passes
valid block.json passes
valid recipe.json passes
invalid enum fails
missing required field fails
unknown field fails where additionalProperties=false
version mismatch fails
invalid device reference fails
```

### 9.2 Recipe compiler tests

Tests:

```text
profile reference resolution
block expansion
sweep expansion
nested sweep rejection or acceptance according to schema
estimated duration calculation
settle insertion
dwell insertion
average insertion
resolved_recipe determinism
stable step_id generation
```

### 9.3 Dry-run tests

Tests:

```text
dry-run produces command plan
dry-run does not touch hardware
dry-run includes dangerous commands visibly
dry-run includes estimated timing
dry-run includes safety report
dry-run includes resource lease plan
```

### 9.4 Safety rejection tests

Tests:

```text
laser power above limit rejected
SMB power above limit rejected
mag current above limit rejected
mag ramp rate above limit rejected
missing safety profile rejected
unknown device state rejected
dangerous command without explicit arm rejected
AI-generated recipe cannot override safety limit
```

### 9.5 Device mock tests

Tests:

```text
fake SMB100A accepts known SCPI and maintains state
fake SMB100A emits error queue for invalid commands
fake OE1022D returns valid RALL frames
fake laser follows warmup and interlock states
fake magnetic axis obeys current/ramp limits
```

### 9.6 Raw bin replay tests

Tests:

```text
raw bin can be replayed
index offsets match raw frame boundaries
parser output matches expected samples
replay output is deterministic
partial raw bin is detected
corrupt frame is reported
parser does not panic on invalid frame
```

### 9.7 Timestamp alignment tests

Tests:

```text
monotonic timestamps are strictly non-decreasing
wall timestamps exist for event audit
command send/return timestamps are recorded
settle window begins after command return or configured marker
acquisition window excludes front discard
average window matches resolved recipe
sample-to-step mapping is correct
```

### 9.8 GUI mock backend tests

Tests:

```text
GUI can connect to fake backend
GUI can show fake devices
GUI can load recipe
GUI can show dry-run
GUI can start mock run
GUI can stop mock run
GUI displays safety rejection
GUI does not parse raw binary frame
GUI chart receives downsampled series only
```

### 9.9 Benchmark tests

Benchmark minimums:

```text
OE1022D parser throughput
raw bin writer throughput
ring buffer push/pop throughput
UI downsampling throughput
mock executor step throughput
raw replay throughput
```

Benchmarks should produce machine-readable reports.

---

## 10. Real Hardware Smoke Tests

Real hardware tests are separate from normal CI.

### 10.1 Required protections

```text
must require explicit operator confirmation
must require station safety profile
must start with RF off / laser off / mag zero or safe hold
must query IDN before command
must send only low-risk command by default
must disconnect safely
must write real_hardware_smoke_report.json
```

### 10.2 SMB100A smoke test

Minimum test:

```text
connect TCP / VISA fallback if configured
query *IDN?
query OUTP?
set OUTP OFF
set MOD:STAT OFF
query SYST:ERR?
disconnect
```

Optional low-risk setting test:

```text
set frequency while RF OFF
query frequency
restore previous frequency if required
```

### 10.3 OE1022D smoke test

Minimum test:

```text
connect serial
query ID / status if supported
query Ch-B reference mode
query Ch-B sensitivity
read one RALL? frame with acquisition not armed for long run
parse frame
disconnect
```

### 10.4 Laser smoke test

Minimum test:

```text
connect serial
query status
ensure output disabled
query interlock
query configured power
safe disconnect
```

### 10.5 Magnetic axis smoke test

Minimum test:

```text
connect controller
query current readback
query limits
set zero or no-op setpoint if safe
verify readback
disconnect
```

---

## 11. Data Model

### 11.1 Test summary

```json
{
  "test_id": "mock_odmr_basic_sweep_001",
  "started_at": "2026-01-01T10:00:00Z",
  "mode": "mock",
  "scenario_id": "oe1022d_chb_normal_500hz_v1",
  "result": "passed",
  "artifacts_dir": "test_artifacts/mock_odmr_basic_sweep_001",
  "checks": [
    {"name": "schema_validation", "status": "passed"},
    {"name": "safety_check", "status": "passed"},
    {"name": "event_log_diff", "status": "passed"}
  ]
}
```

### 11.2 Fake device log event

```json
{
  "ts_monotonic_ns": 1234567890,
  "device_id": "smb100a_main",
  "transport": "fake_tcp_scpi",
  "direction": "rx",
  "payload": "FREQ 2.882GHz",
  "response": null,
  "latency_ms": 2,
  "scenario_step": "normal"
}
```

### 11.3 Replay expectation

```json
{
  "replay_id": "raw_replay_parser_regression_001",
  "input_raw_bin": "fixtures/raw/oe1022d_chb_normal.rawbin",
  "input_index": "fixtures/raw/index.jsonl",
  "expected_samples": "fixtures/expected/samples.jsonl",
  "tolerances": {
    "x_abs_error_v": 1e-12,
    "timestamp_error_ns": 0
  }
}
```

---

## 12. API / Interface

### 12.1 CLI commands

Required CLI commands:

```text
odmr-harness validate-recipe <recipe.json>
odmr-harness dry-run <recipe.json> --station <station.json>
odmr-harness run-mock <resolved_recipe.json> --scenario <fake_scenario.json>
odmr-harness replay-raw <run_dir>
odmr-harness diff-run <expected_dir> <actual_dir>
odmr-harness bench
odmr-harness smoke-real --station <station.json> --device <device_id> --operator-confirmed
```

### 12.2 Rust crate interface

Suggested crates:

```text
odmr-harness-core
odmr-fake-devices
odmr-replay
odmr-bench
```

Interfaces:

```rust
trait FakeDevice {
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn state_snapshot(&self) -> serde_json::Value;
}

trait ReplaySource {
    fn next_frame(&mut self) -> Result<Option<RawFrame>>;
}

trait ScenarioClock {
    fn now_monotonic_ns(&self) -> u64;
    fn now_wall(&self) -> chrono::DateTime<chrono::Utc>;
    fn advance_ms(&mut self, ms: u64);
}
```

### 12.3 GUI interface

GUI should be able to request:

```text
available fake scenarios
start mock run
stop mock run
view fake device status
view event stream
view downsampled chart data
view safety rejection result
```

GUI must not access fake raw binary frames directly.

---

## 13. Error Handling

Harness must simulate and validate handling for:

```text
serial timeout
TCP timeout
invalid SCPI response
OE1022D malformed frame
raw bin partial write
index mismatch
device disconnect
device reconnect
safety rejection
executor stop
epsilon timing drift
GUI event stream disconnect
```

Every injected error must be visible in:

```text
events.jsonl
fake_devices.jsonl
test_summary.json
```

Error names should be stable and machine-readable.

Example:

```text
ERR_FAKE_SERIAL_TIMEOUT
ERR_FAKE_SCPI_INVALID_COMMAND
ERR_RAW_REPLAY_INDEX_MISMATCH
ERR_SAFETY_REJECTED_SMB_POWER_LIMIT
ERR_TIMESTAMP_ALIGNMENT_FAILED
```

---

## 14. Test Matrix

Minimum test matrix:

| Layer | Mock required | Replay required | Real hardware required | CI default |
|---|---:|---:|---:|---:|
| Schema | Yes | No | No | Yes |
| Compiler | Yes | No | No | Yes |
| Safety | Yes | No | No | Yes |
| OE1022D parser | Yes | Yes | Optional | Yes |
| Raw bin writer | Yes | Yes | No | Yes |
| SMB100A SCPI driver | Yes | No | Optional gated | Yes for fake |
| Laser driver | Yes | No | Optional gated | Yes for fake |
| Magnetic driver | Yes | No | Optional gated | Yes for fake |
| Executor | Yes | Yes | Optional gated | Yes for fake |
| GUI | Yes | No | No | Yes |
| Full experiment | Yes | Yes | Manual gated | Yes for fake |

---

## 15. Acceptance Criteria

This module is acceptable when:

```text
all core code can run without physical instruments
fake OE1022D can generate valid and invalid RALL frames
fake SMB100A SCPI server supports all commands used by executor
fake laser and fake magnetic axis support safety-relevant states
recipe validation tests pass
compiler dry-run tests pass
safety rejection tests pass
raw bin replay tests pass
timestamp alignment tests pass
mock full-run produces deterministic event logs
GUI can run against fake backend
benchmark report is generated
real hardware smoke tests are gated and never run by default
```

Minimum v0.2 implementation target:

```text
one fake SMB100A SCPI server
one fake OE1022D RALL frame source
one raw replay tool
one mock ODMR recipe end-to-end test
one safety rejection test suite
one timestamp alignment test suite
```

---

## 16. Agent Constraints

AI / coding agents working on this module must follow:

```text
Do not modify safety limits to make tests pass.
Do not bypass schema validation.
Do not bypass compiler dry-run.
Do not make mock-only code paths in production executor.
Do not remove raw bin replay checks.
Do not make tests depend on real hardware unless explicitly marked gated.
Do not weaken acceptance criteria without updating this PRD.
```

Agent outputs must include:

```text
new or updated fake device code
test fixture files
expected output snapshots
unit tests
integration tests
README or usage notes
```

Each agent PR must answer:

```text
Which PRD requirement is implemented?
Which mock scenario proves it?
Which replay fixture proves it?
Which safety rejection case is covered?
Does it run without hardware?
Does it avoid touching GUI / recipe / safety outside its scope?
```

---

## 17. Open Questions

```text
Exact OE1022D RALL? binary frame layout must be finalized by implementation tests.
Whether fake serial should be pure in-process or pseudo-terminal based needs decision.
Whether GUI tests use Playwright, Vitest, or Tauri-specific test harness needs decision.
Benchmark thresholds should be calibrated after first Rust acquisition prototype.
Real hardware smoke tests require lab-specific station profiles.
```
