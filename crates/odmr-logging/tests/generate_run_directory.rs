//! Integration test that generates a complete example run directory.
//!
//! Run with: cargo test -p odmr-logging --test generate_run_directory -- --nocapture

use odmr_logging::{
    create_run_directory, EventLevel, RawIndexEntry, RunArtifactPaths, RunEvent, RunEventType,
    RunManifest,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().parent().unwrap().to_path_buf()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[test]
fn generate_basic_odmr_mock_run_directory() {
    let ws = workspace_root();

    // Load existing example artifacts as generic JSON values.
    let resolved_recipe: serde_json::Value = {
        let path = ws.join("examples/resolved/basic_odmr_mock.resolved_recipe.json");
        let s = std::fs::read_to_string(&path).unwrap();
        serde_json::from_str(&s).unwrap()
    };

    let dry_run_plan: serde_json::Value = {
        let path = ws.join("examples/resolved/basic_odmr_mock.dry_run_plan.json");
        let s = std::fs::read_to_string(&path).unwrap();
        serde_json::from_str(&s).unwrap()
    };

    let safety_report: serde_json::Value = {
        let path = ws.join("examples/safety/basic_odmr_mock.safety_report.json");
        let s = std::fs::read_to_string(&path).unwrap();
        serde_json::from_str(&s).unwrap()
    };

    let station: serde_json::Value = {
        let path = ws.join("examples/station.mock.json");
        let s = std::fs::read_to_string(&path).unwrap();
        serde_json::from_str(&s).unwrap()
    };

    let recipe: serde_json::Value = {
        let path = ws.join("examples/recipes/basic_odmr_mock.recipe.json");
        let s = std::fs::read_to_string(&path).unwrap();
        serde_json::from_str(&s).unwrap()
    };

    // Extract IDs and hashes for the manifest.
    let resolved_recipe_id = resolved_recipe["id"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let recipe_hash = resolved_recipe["source_recipe_hash"]
        .as_str()
        .map(|s| s.to_string());
    let safety_report_id = safety_report["id"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    // Create run directory.
    let run_id = "basic_odmr_mock_run";
    let out_dir = ws.join("examples/runs");
    std::fs::create_dir_all(&out_dir).unwrap();

    // Remove previous example run directory if it exists.
    let run_path = out_dir.join(run_id);
    if run_path.exists() {
        std::fs::remove_dir_all(&run_path).unwrap();
    }

    let run = create_run_directory(&out_dir, run_id).unwrap();

    // Write metadata lock files.
    run.write_station_snapshot_json(&station).unwrap();
    run.write_recipe_lock_json(&recipe).unwrap();
    run.write_resolved_recipe_lock_json(&resolved_recipe)
        .unwrap();
    run.write_dry_run_plan_lock_json(&dry_run_plan).unwrap();
    run.write_safety_report_lock_json(&safety_report).unwrap();

    // Write manifest.
    let manifest = RunManifest {
        schema_version: "0.2.0".into(),
        kind: "run_manifest".into(),
        run_id: run_id.into(),
        created_at_unix_ms: now_ms(),
        artifact_paths: RunArtifactPaths::default(),
        recipe_hash,
        resolved_recipe_id: Some(resolved_recipe_id),
        safety_report_id: Some(safety_report_id),
    };
    run.write_manifest(&manifest).unwrap();

    // Write events.
    let mut event_writer = run.open_event_writer().unwrap();
    let events = vec![
        RunEvent {
            schema_version: "0.2.0".into(),
            kind: "run_event".into(),
            run_id: run_id.into(),
            event_id: "evt_000001".into(),
            timestamp_unix_ms: now_ms(),
            timestamp_monotonic_ns: None,
            level: EventLevel::Info,
            event_type: RunEventType::RunCreated,
            step_id: None,
            device_id: None,
            message: "Run directory created".into(),
            data: None,
        },
        RunEvent {
            schema_version: "0.2.0".into(),
            kind: "run_event".into(),
            run_id: run_id.into(),
            event_id: "evt_000002".into(),
            timestamp_unix_ms: now_ms(),
            timestamp_monotonic_ns: None,
            level: EventLevel::Info,
            event_type: RunEventType::ArtifactWritten,
            step_id: None,
            device_id: None,
            message: "Locked artifacts written".into(),
            data: Some(serde_json::json!({
                "artifacts": ["manifest.json", "metadata/resolved_recipe.lock.json"]
            })),
        },
        RunEvent {
            schema_version: "0.2.0".into(),
            kind: "run_event".into(),
            run_id: run_id.into(),
            event_id: "evt_000003".into(),
            timestamp_unix_ms: now_ms(),
            timestamp_monotonic_ns: None,
            level: EventLevel::Info,
            event_type: RunEventType::SafetyChecked,
            step_id: None,
            device_id: None,
            message: "Safety report loaded from pre-computed check".into(),
            data: None,
        },
    ];
    for event in &events {
        event_writer.write_event(event).unwrap();
    }

    // Write index entries.
    let mut index_writer = run.open_index_writer().unwrap();
    let index_entries = vec![
        RawIndexEntry {
            schema_version: "0.2.0".into(),
            kind: "raw_index_entry".into(),
            run_id: run_id.into(),
            stream_id: "oe1022d".into(),
            offset_bytes: 0,
            length_bytes: 8,
            timestamp_unix_ms: now_ms(),
            step_id: Some("step_000001".into()),
            sample_count: Some(2),
        },
        RawIndexEntry {
            schema_version: "0.2.0".into(),
            kind: "raw_index_entry".into(),
            run_id: run_id.into(),
            stream_id: "oe1022d".into(),
            offset_bytes: 8,
            length_bytes: 12,
            timestamp_unix_ms: now_ms(),
            step_id: Some("step_000002".into()),
            sample_count: Some(3),
        },
    ];
    for entry in &index_entries {
        index_writer.write_entry(entry).unwrap();
    }

    // Write mock raw bytes.
    let mut raw_writer = run.open_raw_bin_writer().unwrap();
    raw_writer
        .append_frame(b"\x01\x02\x03\x04\x05\x06\x07\x08")
        .unwrap();
    raw_writer
        .append_frame(b"\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15")
        .unwrap();

    // Assertions.
    assert!(run_path.join("manifest.json").exists());
    assert!(run_path.join("metadata/station_snapshot.json").exists());
    assert!(run_path.join("metadata/recipe.lock.json").exists());
    assert!(run_path.join("metadata/resolved_recipe.lock.json").exists());
    assert!(run_path.join("metadata/dry_run_plan.lock.json").exists());
    assert!(run_path.join("metadata/safety_report.lock.json").exists());
    assert!(run_path.join("events.jsonl").exists());
    assert!(run_path.join("index.jsonl").exists());
    assert!(run_path.join("raw/oe1022d.rawbin").exists());

    // Verify events.jsonl has 3 lines.
    let events_content = std::fs::read_to_string(run_path.join("events.jsonl")).unwrap();
    assert_eq!(events_content.lines().count(), 3);

    // Verify index.jsonl has 2 lines.
    let index_content = std::fs::read_to_string(run_path.join("index.jsonl")).unwrap();
    assert_eq!(index_content.lines().count(), 2);

    // Verify raw bin size.
    let raw_meta = std::fs::metadata(run_path.join("raw/oe1022d.rawbin")).unwrap();
    assert_eq!(raw_meta.len(), 20); // 8 + 12

    println!("Generated example run directory at {}", run_path.display());
}
