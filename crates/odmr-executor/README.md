# `odmr-executor`

Layer 3 recipe execution engine for the ODMR automation system.

## Responsibilities

- Load and compile recipes
- Run safety checks
- Create run directories via `odmr-logging`
- Execute resolved steps against fake (mock) or real devices
- Write events, index entries, and raw frames
- Produce execution reports

## Mock-Run API

```rust
use odmr_executor::{run_mock, MockRunConfig};

let config = MockRunConfig {
    recipe_path: "examples/recipes/basic_odmr_mock.recipe.json".into(),
    station_path: "examples/station.mock.json".into(),
    run_root: "./runs".into(),
    run_id: "run_20260528_001".into(),
    safety_limits: None,
};

let report = run_mock(config)?;
// report.decision == ExecutionDecision::Completed
// report.steps_completed == 201
```

## What this crate does NOT do

- Access real hardware in mock mode
- Write CSV or Parquet
- Run GUI code
- Parse real OE1022D RALL? frames (M1.5 uses mock frames)
