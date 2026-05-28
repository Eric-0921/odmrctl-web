# Sub-PRD 12: Agent Workflow

Version: v0.2  
Status: Draft for implementation planning  
Owner: ODMR Automation Project  
Target file: `docs/prd/12_agent_workflow_prd.md`  
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
- `11_harness_mock_replay_prd.md`

External references:

- Model Context Protocol introduction: <https://modelcontextprotocol.io/docs/getting-started/intro>
- Model Context Protocol specification index: <https://modelcontextprotocol.io/specification>
- OpenTelemetry Semantic Conventions: <https://opentelemetry.io/docs/concepts/semantic-conventions/>

---

## 1. Purpose

This PRD defines how AI agents and coding agents are allowed to participate in the ODMR automation project.

The system has two distinct meanings of “agent”:

```text
Development-time agent:
  helps write PRDs, schemas, code, tests, docs, reviews, and examples

Runtime experiment agent:
  proposes JSON recipe / block / analysis suggestions
```

This PRD allows agents to help with development and planning, but it strictly prevents agents from directly controlling physical instruments.

Core principle:

```text
AI agent may generate JSON.
AI agent may review JSON.
AI agent may generate code.
AI agent may generate tests.
AI agent must not directly control hardware.
AI agent must not bypass safety.
AI agent must not override human-maintained station limits.
```

The project goal is not to build an autonomous lab robot in v0.2. The goal is to build a reliable automation system where AI assists with structured experiment planning, code generation, and review under strict boundaries.

---

## 2. Scope

This module is responsible for defining:

```text
agent roles
agent input/output contracts
agent forbidden areas
context package format
review checklist
handoff protocol
PRD-driven development workflow
recipe generation workflow
code generation workflow
test generation workflow
safety review workflow
multi-agent conflict resolution
agent audit logs
```

This module applies to:

```text
ChatGPT / coding assistant sessions
local coding agents
repo automation agents
recipe generation agents
schema review agents
PRD writing agents
test generation agents
future MCP-like tool adapters if added
```

---

## 3. Non-goals

This module does not define:

```text
LLM provider selection
model prompt internals for every task
production autonomous closed-loop experiment control
online hardware control by AI
multi-agent negotiation framework
cloud agent deployment
agent marketplace
untrusted third-party tool execution
```

This module also does not allow:

```text
agent directly opening laser output
agent directly turning on SMB100A RF output
agent directly setting magnetic current
agent directly reading or writing instrument serial ports
agent modifying safety profiles to pass validation
agent silently changing PRD scope
```

---

## 4. Design Principles

### 4.1 PRD-first development

Every implementation task must name the PRD section it implements.

```text
No PRD reference = no merge.
No tests = no merge.
No safety review for dangerous modules = no merge.
```

### 4.2 Context-limited agents

An agent should receive only the context needed for its task.

Example:

```text
OE1022D Core Agent receives:
  03 OE1022D Acquisition PRD
  06 Timing / Sync / Averaging PRD
  07 Data Logging PRD
  11 Harness / Mock / Replay PRD
  OE1022D command reference snippets
  fake RALL frame fixture

OE1022D Core Agent should not receive:
  GUI implementation details unless needed
  magnetic planner internals
  unrelated AI prompt files
```

### 4.3 Output must be reviewable

Agent output must be concrete and reviewable:

```text
code diff
schema diff
PRD diff
test fixture
test report
review checklist
```

No agent should return vague “it should work” conclusions.

### 4.4 No hidden hardware authority

Agents must not have direct access to:

```text
serial ports
TCP instrument addresses
VISA resources
lab secrets
operator credentials
station safety profile write permission
hardware enable commands
```

If a future tool protocol is added for agent integration, it must expose read-only project context by default. Write or execution capabilities must be allowlisted, logged, and human-approved.

### 4.5 Safety is not negotiable

Agent-generated recipe, code, or tests must respect:

```text
10_safety_interlock_prd
station safety limits
device profile limits
schema validation
compiler dry-run
harness safety rejection tests
```

---

## 5. Agent Types

### 5.1 PRD Agent

Responsible for:

```text
writing and refining PRDs
keeping PRD structure consistent
identifying missing scope / non-goals / interfaces
aligning sub-PRDs with main PRD
```

Inputs:

```text
existing PRDs
module notes
user requirements
manual excerpts
architecture decisions
```

Outputs:

```text
updated PRD markdown
open questions
ADR suggestions
acceptance criteria
```

Forbidden:

```text
writing production code without request
changing technical decisions silently
weakening safety rules
inventing unsupported hardware capabilities
```

### 5.2 Schema Agent

Responsible for:

```text
station.schema.json
device_profile.schema.json
block.schema.json
recipe.schema.json
resolved_recipe.schema.json
event.schema.json
schema examples
schema validation tests
```

Inputs:

```text
04_recipe_json_schema_prd
05_recipe_compiler_executor_prd
10_safety_interlock_prd
example recipe requirements
```

Outputs:

```text
JSON Schema files
valid examples
invalid examples
schema test cases
schema changelog
```

Forbidden:

```text
adding fields not described in PRD without marking as proposal
allowing recipe to override safety limits
making schema too loose to pass invalid data
changing executor behavior
```

### 5.3 OE1022D Core Agent

Responsible for:

```text
Rust OE1022D acquisition core
RALL? request loop
raw frame parser
ring buffer
raw bin recorder
parser tests
replay tests
benchmark report
```

Inputs:

```text
03_oe1022d_acquisition_prd
06_timing_sync_averaging_prd
07_data_logging_file_format_prd
11_harness_mock_replay_prd
OE1022D command mapping files
fake RALL frame fixtures
```

Outputs:

```text
oe1022d-core crate
parser module
recorder module
fake frame tests
raw replay tool
benchmark output
```

Forbidden:

```text
modifying GUI
modifying recipe schema
controlling SMB100A
controlling laser
controlling magnetic axes
weakening safety checks
```

### 5.4 SMB100A Driver Agent

Responsible for:

```text
SMB100A SCPI transport
Rust TCP socket SCPI implementation
Python RsInstrument fallback notes if needed
SCPI command mapping
fake SMB100A SCPI server tests
```

Inputs:

```text
02_device_registry_connection_prd
05_recipe_compiler_executor_prd
10_safety_interlock_prd
11_harness_mock_replay_prd
SMB100A command JSON files
SMB100A operating manual excerpts
```

Outputs:

```text
SCPI driver module
command builder
query parser
error queue handling
fake SCPI integration tests
safe output-off procedure
```

Forbidden:

```text
turning RF output on in tests without explicit gated safety flow
bypassing safety power limits
changing OE1022D acquisition core
changing GUI charting logic
```

### 5.5 Magnetic Planner Agent

Responsible for:

```text
B-vector expression
Cartesian / spherical conversion
coil matrix calculation
current preview
ramp limit checks
magnetic block JSON generation
```

Inputs:

```text
09_magnetic_field_planner_prd
10_safety_interlock_prd
04_recipe_json_schema_prd
coil matrix calibration notes
```

Outputs:

```text
magnetic planner logic
magnetic block examples
safety preview tests
conversion tests
```

Forbidden:

```text
directly setting coil current on hardware
changing station current limits
modifying OE1022D parser
modifying SMB100A driver
```

### 5.6 Safety Agent

Responsible for:

```text
safety rule review
safety_report schema checks
dangerous command visibility
safety rejection test cases
safe shutdown review
```

Inputs:

```text
10_safety_interlock_prd
02_device_registry_connection_prd
05_recipe_compiler_executor_prd
11_harness_mock_replay_prd
all device profile schemas
```

Outputs:

```text
safety rule tests
safety rejection examples
review comments
safe shutdown checklist
```

Forbidden:

```text
raising limits without human maintainer approval
removing rejection cases
allowing AI-generated safety overrides
approving unknown hardware states
```

### 5.7 Data Logging Agent

Responsible for:

```text
run directory structure
raw bin metadata
index.jsonl
events.jsonl
parquet conversion
CSV export
traceability validation
```

Inputs:

```text
07_data_logging_file_format_prd
03_oe1022d_acquisition_prd
05_recipe_compiler_executor_prd
06_timing_sync_averaging_prd
11_harness_mock_replay_prd
```

Outputs:

```text
file format structs
writer tests
replay tests
parquet converter
export validator
```

Forbidden:

```text
using CSV as real-time format
dropping step_id or device state linkage
changing acquisition thread behavior without OE1022D PRD review
```

### 5.8 GUI Agent

Responsible for:

```text
Tauri frontend pages
device status display
recipe selection UI
dry-run display
run control
real-time chart rendering
safety warnings
mock backend UI tests
```

Inputs:

```text
08_gui_tauri_chart_prd
02_device_registry_connection_prd
05_recipe_compiler_executor_prd
07_data_logging_file_format_prd
10_safety_interlock_prd
11_harness_mock_replay_prd
```

Outputs:

```text
frontend components
Tauri command bindings
mock UI tests
chart performance test notes
```

Forbidden:

```text
GUI directly accessing hardware
GUI parsing raw RALL binary frames
GUI writing real-time CSV
GUI implementing experiment logic
GUI bypassing executor
```

### 5.9 Harness Agent

Responsible for:

```text
fake devices
mock transports
raw replay tools
diff tests
benchmark harness
CI test matrix
```

Inputs:

```text
11_harness_mock_replay_prd
all module PRDs
existing test fixtures
```

Outputs:

```text
fake OE1022D
fake SMB100A
fake laser
fake magnetic axis
replay fixtures
integration test runner
```

Forbidden:

```text
bypassing production validation logic
changing safety limits to make tests pass
requiring hardware in default CI
```

### 5.10 Documentation / Review Agent

Responsible for:

```text
README updates
developer guide
operator guide
ADR draft
review summary
known limitations
```

Inputs:

```text
PRDs
code diffs
test reports
manual excerpts
open issues
```

Outputs:

```text
docs updates
ADR files
review checklists
release notes
```

Forbidden:

```text
claiming untested features are complete
removing limitations
rewriting technical decisions without approval
```

---

## 6. Standard Agent Contract

Every agent task must define:

```text
scope
inputs
outputs
forbidden areas
test requirements
review checklist
```

Template:

```text
Agent Task: <name>

Scope:
  - ...

Inputs:
  - PRD sections
  - schemas
  - examples
  - manual snippets

Outputs:
  - files to create/update
  - tests to add
  - docs to update

Forbidden:
  - modules not allowed to modify
  - safety boundaries not allowed to change

Required tests:
  - unit tests
  - integration tests
  - mock/replay tests

Review checklist:
  - PRD section implemented
  - safety unaffected or reviewed
  - no hardware access in agent-generated code path
  - tests pass without hardware
  - artifacts reproducible
```

---

## 7. Agent Workflow

### 7.1 Standard development flow

```text
1. Select issue / module
2. Build context package
3. Assign agent role
4. Agent produces implementation plan
5. Human or lead agent reviews plan
6. Agent writes code / schema / docs
7. Agent adds tests
8. Run harness tests
9. Run lint / format
10. Generate review checklist
11. Human review
12. Merge only after acceptance criteria pass
```

### 7.2 Context package

A context package is a bounded set of files given to an agent.

Required fields:

```json
{
  "task_id": "oe1022d_parser_frame_v1",
  "agent_role": "OE1022D Core Agent",
  "prd_sections": [
    "03_oe1022d_acquisition_prd.md#raw-frame-parser",
    "11_harness_mock_replay_prd.md#fake-rall-frame-generator"
  ],
  "input_files": [
    "fixtures/fake_rall/normal_frame.bin",
    "schemas/event.schema.json"
  ],
  "allowed_paths": [
    "crates/oe1022d-core/**",
    "tests/fixtures/oe1022d/**"
  ],
  "forbidden_paths": [
    "src-tauri/gui/**",
    "schemas/recipe.schema.json",
    "docs/prd/10_safety_interlock_prd.md"
  ],
  "required_tests": [
    "cargo test -p oe1022d-core",
    "odmr-harness replay-raw tests/fixtures/oe1022d/normal_run"
  ]
}
```

### 7.3 Handoff packet

When one agent hands work to another, it must produce:

```text
summary of completed work
files changed
PRD sections touched
tests added
tests not run
known issues
next recommended agent
```

Example:

```text
Handoff: OE1022D Core Agent -> Data Logging Agent

Completed:
  - RawFrame parser implemented
  - ParsedSample struct added
  - fake normal frame fixture added

Files changed:
  - crates/oe1022d-core/src/parser.rs
  - tests/fixtures/oe1022d/normal_frame.bin

Tests:
  - cargo test -p oe1022d-core passed

Open issues:
  - raw bin index writer still needs offset validation

Next:
  - Data Logging Agent should connect ParsedSample to index.jsonl writer
```

---

## 8. Runtime Recipe Agent Boundary

A runtime recipe agent may:

```text
generate recipe.json
generate block.json
explain parameter choices
suggest scan ranges
suggest averaging strategy
suggest metadata tags
run schema validation if tool is available
run dry-run if tool is available
summarize safety_report
```

A runtime recipe agent must not:

```text
send serial commands
send SCPI commands
open RF output
open laser output
set magnetic current
modify station safety limit
ignore safety_report failed
execute unresolved recipe
execute recipe without human arming
```

Runtime recipe flow:

```text
AI proposes recipe.json
  ↓
schema validation
  ↓
compiler dry-run
  ↓
safety_report
  ↓
human review
  ↓
executor arm
  ↓
hardware run by deterministic executor only
```

---

## 9. Tool and Permission Model

### 9.1 Default permissions

By default, agents may access:

```text
PRD documents
schema files
example recipes
mock fixtures
unit test files
local source code under allowed paths
```

By default, agents may not access:

```text
real serial ports
VISA resources
TCP instrument addresses
hardware credentials
station safety limit write path
operator secrets
production run directory write path
```

### 9.2 Future MCP-like integration

If the project later exposes tools to an AI agent using a protocol such as MCP, the integration must be limited to:

```text
read_project_doc
read_schema
validate_recipe
compile_recipe_dry_run
run_mock_harness
summarize_test_result
```

Forbidden tools:

```text
send_serial_command
send_scpi_command
set_laser_output
set_magnetic_current
arm_executor
start_real_run
edit_safety_limits
```

All tool calls must be logged.

```text
tool_call_id
tool_name
input_hash
output_hash
agent_id
timestamp
result
```

---

## 10. Review Checklist

Every agent-generated change must be reviewed against this checklist.

### 10.1 General checklist

```text
Does the change reference a PRD section?
Does it stay within allowed scope?
Does it avoid unrelated refactors?
Does it add or update tests?
Does it run without hardware?
Does it preserve traceability?
Does it update docs or examples if behavior changes?
```

### 10.2 Safety checklist

```text
Does it affect laser output?
Does it affect SMB100A RF output?
Does it affect magnetic current?
Does it affect executor arming?
Does it affect safety_report generation?
Does it affect station/device/profile limits?
Does it introduce a way around dry-run?
```

If any answer is yes, Safety Agent review is required.

### 10.3 Data checklist

```text
Does it preserve run_id?
Does it preserve step_id?
Does it preserve timestamps?
Does it preserve raw-to-parsed traceability?
Does it avoid real-time CSV writes?
Does replay still work?
```

### 10.4 GUI checklist

```text
Does GUI remain an operator console only?
Does GUI avoid direct hardware access?
Does GUI receive downsampled data only?
Does GUI display safety rejection clearly?
Does GUI not mutate resolved_recipe after compile?
```

---

## 11. Agent Event Logging

Agent actions that affect repository or experiment planning should be logged when practical.

Suggested event types:

```text
agent.task.started
agent.task.completed
agent.file.modified
agent.test.executed
agent.review.requested
agent.review.completed
agent.recipe.generated
agent.recipe.validation_failed
agent.safety_review.required
agent.handoff.created
```

Example:

```json
{
  "event_type": "agent.task.completed",
  "agent_role": "Schema Agent",
  "task_id": "recipe_schema_v1",
  "files_changed": [
    "schemas/recipe.schema.json",
    "examples/odmr_basic_sweep.recipe.json"
  ],
  "tests_run": [
    "python -m pytest tests/schema"
  ],
  "result": "passed"
}
```

Event names should remain stable and follow a namespace-like convention.

---

## 12. Conflict Resolution

When agents disagree or produce conflicting changes:

```text
PRD wins over generated code.
Safety PRD wins over recipe convenience.
Architecture PRD wins over local module preference.
Harness tests win over untested assumptions.
Human maintainer wins over agent recommendation.
```

Conflict resolution order:

```text
1. Safety & Interlock PRD
2. Architecture PRD
3. Main PRD
4. Module Sub-PRD
5. ADR
6. Schema
7. Code
8. Agent output
```

If conflict remains unresolved, create an ADR proposal rather than silently choosing.

---

## 13. Acceptance Criteria

This module is acceptable when:

```text
each major agent role has scope / input / output / forbidden / tests / review checklist
a standard agent task template exists
a context package format exists
a handoff packet format exists
runtime recipe agent boundary is explicit
tool permissions are explicit
hardware access is forbidden by default
safety review triggers are explicit
agent event names are defined
multi-agent conflict resolution is defined
```

Minimum v0.2 deliverables:

```text
docs/prd/12_agent_workflow_prd.md
agent_task_template.md
agent_review_checklist.md
example_context_package.json
example_handoff_packet.md
```

---

## 14. Example Agent Task

```text
Agent Task: OE1022D RALL Parser v0.1

Role:
  OE1022D Core Agent

Scope:
  Implement parser for one known fake RALL frame layout.
  Add deterministic parser tests.

Inputs:
  - docs/prd/03_oe1022d_acquisition_prd.md
  - docs/prd/11_harness_mock_replay_prd.md
  - tests/fixtures/oe1022d/rall_normal_frame.bin

Outputs:
  - crates/oe1022d-core/src/parser.rs
  - crates/oe1022d-core/tests/parser_tests.rs
  - tests/fixtures/oe1022d/expected_normal_frame.json

Forbidden:
  - Do not modify GUI.
  - Do not modify recipe schema.
  - Do not touch SMB100A driver.
  - Do not touch real serial port.

Required tests:
  - cargo test -p oe1022d-core parser

Review checklist:
  - Parser handles valid frame.
  - Parser rejects malformed frame.
  - Parser returns structured error, not panic.
  - No hardware required.
  - No safety boundary changed.
```

---

## 15. Open Questions

```text
Whether to store agent logs inside repo artifacts or outside project history needs decision.
Whether future tool integration uses MCP, direct CLI, or custom local bridge needs decision.
How strict allowed_paths enforcement should be in local coding workflow needs decision.
Whether agent-generated recipes should include provenance metadata by default needs decision.
Whether every agent run should produce a signed summary is postponed.
```
