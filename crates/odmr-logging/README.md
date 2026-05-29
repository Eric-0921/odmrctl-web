# `odmr-logging`

Layer 3 run artifact and raw-first persistence layer for the ODMR automation system.

## Responsibilities

- Create run directories with a stable layout
- Write metadata lock files (station snapshot, recipe, resolved recipe, dry-run plan, safety report)
- Write event logs as JSONL (`events.jsonl`)
- Write raw data index as JSONL (`index.jsonl`)
- Provide an append-only raw binary writer (`raw/oe1022d.rawbin`)

## What this crate does NOT do

- Execute recipes
- Access hardware or fake devices
- Write CSV or Parquet
- Run GUI code

## Run Directory Layout

```
<root>/<run_id>/
  manifest.json
  metadata/
    station_snapshot.json
    recipe.lock.json
    resolved_recipe.lock.json
    dry_run_plan.lock.json
    safety_report.lock.json
  events.jsonl
  index.jsonl
  raw/
    oe1022d.rawbin
```

## Public API

```rust
use odmr_logging::{create_run_directory, RunDirectory, RunManifest, RunEvent, RawIndexEntry};

let run = create_run_directory(Path::new("./runs"), "run_20260528_001")?;
run.write_manifest(&manifest)?;
run.write_resolved_recipe_lock_json(&resolved_recipe)?;

let mut events = run.open_event_writer()?;
events.write_event(&RunEvent { ... })?;

let mut raw = run.open_raw_bin_writer()?;
let entry = raw.append_frame(b"\x01\x02\x03\x04")?;
```
