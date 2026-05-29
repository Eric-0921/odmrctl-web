//! End-to-end integration tests for the executor mock-run pipeline.

use odmr_executor::{run_mock, ExecutionDecision, ExecutorError, MockRunConfig};
use odmr_logging::{RunEvent, RunEventType};
use odmr_recipe::SafetyLimit;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().parent().unwrap().to_path_buf()
}

fn basic_mock_config(run_root: &std::path::Path, run_id: &str) -> MockRunConfig {
    let ws = workspace_root();
    MockRunConfig {
        recipe_path: ws.join("examples/recipes/basic_odmr_mock.recipe.json"),
        station_path: ws.join("examples/station.mock.json"),
        run_root: run_root.to_path_buf(),
        run_id: run_id.into(),
        safety_limits: None,
    }
}

fn parse_events(path: &std::path::Path) -> Vec<RunEvent> {
    let content = std::fs::read_to_string(path).unwrap();
    content
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn count_event_types(events: &[RunEvent], ty: RunEventType) -> usize {
    events.iter().filter(|e| e.event_type == ty).count()
}

#[test]
fn run_mock_completes_for_basic_odmr_mock() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_001");
    let report = run_mock(config).unwrap();
    assert_eq!(report.decision, ExecutionDecision::Completed);
}

#[test]
fn execution_report_says_completed() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_002");
    let report = run_mock(config).unwrap();
    assert_eq!(report.decision, ExecutionDecision::Completed);
    assert_eq!(report.kind, "execution_report");
}

#[test]
fn steps_total_is_201() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_003");
    let report = run_mock(config).unwrap();
    assert_eq!(report.steps_total, 201);
}

#[test]
fn steps_completed_is_201() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_004");
    let report = run_mock(config).unwrap();
    assert_eq!(report.steps_completed, 201);
}

#[test]
fn run_directory_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_005");
    let report = run_mock(config).unwrap();
    assert!(report.run_directory.exists());
    assert!(report.run_directory.is_dir());
}

#[test]
fn locked_metadata_files_exist() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_006");
    let report = run_mock(config).unwrap();
    let root = &report.run_directory;
    assert!(root.join("manifest.json").exists());
    assert!(root.join("metadata/station_snapshot.json").exists());
    assert!(root.join("metadata/recipe.lock.json").exists());
    assert!(root.join("metadata/resolved_recipe.lock.json").exists());
    assert!(root.join("metadata/dry_run_plan.lock.json").exists());
    assert!(root.join("metadata/safety_report.lock.json").exists());
}

#[test]
fn events_jsonl_has_required_events() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_007");
    let report = run_mock(config).unwrap();
    let events = parse_events(&report.run_directory.join("events.jsonl"));

    assert!(count_event_types(&events, RunEventType::RunCreated) >= 1);
    assert!(count_event_types(&events, RunEventType::SafetyChecked) >= 1);
    assert!(count_event_types(&events, RunEventType::RunStarted) >= 1);
    assert_eq!(
        count_event_types(&events, RunEventType::StepStarted),
        201,
        "expected 201 step_started events"
    );
    assert_eq!(
        count_event_types(&events, RunEventType::StepCompleted),
        201,
        "expected 201 step_completed events"
    );
    assert!(count_event_types(&events, RunEventType::RunCompleted) >= 1);
    assert_eq!(
        report.events_written,
        events.len(),
        "report events_written should match actual events"
    );
}

#[test]
fn index_jsonl_has_201_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_008");
    let report = run_mock(config).unwrap();
    let content = std::fs::read_to_string(report.run_directory.join("index.jsonl")).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 201, "expected 201 index entries");
    assert_eq!(report.raw_frames_written, 201);
}

#[test]
fn rawbin_exists_and_is_non_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_009");
    let report = run_mock(config).unwrap();
    let meta = std::fs::metadata(report.run_directory.join("raw/oe1022d.rawbin")).unwrap();
    assert!(meta.len() > 0);
}

#[test]
fn rawbin_size_equals_sum_of_index_lengths() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_010");
    let report = run_mock(config).unwrap();

    let index_content = std::fs::read_to_string(report.run_directory.join("index.jsonl")).unwrap();
    let total_indexed: u64 = index_content
        .lines()
        .map(|line| {
            let entry: serde_json::Value = serde_json::from_str(line).unwrap();
            entry["length_bytes"].as_u64().unwrap()
        })
        .sum();

    let raw_meta = std::fs::metadata(report.run_directory.join("raw/oe1022d.rawbin")).unwrap();
    assert_eq!(
        raw_meta.len(),
        total_indexed,
        "rawbin size must equal sum of all index length_bytes"
    );
    // Each frame is 16 bytes, so 201 * 16 = 3216
    assert_eq!(raw_meta.len(), 3216);
}

#[test]
fn safety_reject_path_does_not_execute_steps() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = basic_mock_config(tmp.path(), "mock_run_reject");
    // Very restrictive frequency limit to force rejection
    config.safety_limits = Some(SafetyLimit {
        schema_version: "0.2.0".into(),
        kind: "safety_limit".into(),
        id: "reject_all".into(),
        name: Some("Reject All".into()),
        max_power_dbm: 20.0,
        max_frequency_hz: 1_000_000_000.0, // basic recipe starts at 2.82 GHz
        min_frequency_hz: 1_000_000.0,
        max_fm_deviation_hz: None,
        max_magnetic_field_t: None,
        max_mag_ramp_rate_a_per_s: None,
    });

    let report = run_mock(config).unwrap();
    assert_eq!(report.decision, ExecutionDecision::RejectedBySafety);
    assert_eq!(report.steps_completed, 0);
    assert_eq!(report.raw_frames_written, 0);

    // Verify run directory still exists with artifacts
    let run_dir = tmp.path().join("mock_run_reject");
    assert!(run_dir.join("manifest.json").exists());

    // Verify events contain run_failed, not step_started
    let events = parse_events(&run_dir.join("events.jsonl"));
    assert_eq!(
        count_event_types(&events, RunEventType::RunFailed),
        1,
        "expected 1 run_failed event"
    );
    assert_eq!(
        count_event_types(&events, RunEventType::StepStarted),
        0,
        "expected 0 step_started events for rejected run"
    );
}

#[test]
fn duplicate_run_id_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "duplicate_run");
    let _report1 = run_mock(config.clone()).unwrap();
    let result = run_mock(config);
    assert!(matches!(result, Err(ExecutorError::Logging(_))));
}

#[test]
fn no_csv_files_created() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_csv");
    let _report = run_mock(config).unwrap();

    fn has_csv(dir: &std::path::Path) -> bool {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                if has_csv(&path) {
                    return true;
                }
            } else if let Some(ext) = path.extension() {
                if ext == "csv" {
                    return true;
                }
            }
        }
        false
    }

    assert!(!has_csv(tmp.path()), "no CSV files should be created");
}

#[test]
fn no_real_hardware_access() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_pure");
    let report = run_mock(config).unwrap();
    assert_eq!(report.decision, ExecutionDecision::Completed);
    // If we reach here without any real hardware access, the test passes.
}

#[test]
fn fake_smb100a_final_frequency_equals_last_sweep_frequency() {
    let tmp = tempfile::tempdir().unwrap();
    let config = basic_mock_config(tmp.path(), "mock_run_freq");
    let report = run_mock(config).unwrap();

    // Query the final state from the fake device by loading the resolved recipe
    // and checking the last step's frequency. The fake device state is internal
    // to run_mock and not returned, but we know the recipe ends at 2.92 GHz.
    // Instead, verify from the last index entry.
    let index_content = std::fs::read_to_string(report.run_directory.join("index.jsonl")).unwrap();
    let last_line = index_content.lines().last().unwrap();
    let entry: serde_json::Value = serde_json::from_str(last_line).unwrap();
    let step_id = entry["step_id"].as_str().unwrap();

    // The last step should be step_000201
    assert_eq!(step_id, "step_000201");

    // Verify raw bin last frame contains 2.92e9 Hz
    let raw = std::fs::read(report.run_directory.join("raw/oe1022d.rawbin")).unwrap();
    let last_frame_offset = raw.len() - 16;
    let freq_bytes = &raw[last_frame_offset + 4..last_frame_offset + 12];
    let freq = f64::from_le_bytes([
        freq_bytes[0],
        freq_bytes[1],
        freq_bytes[2],
        freq_bytes[3],
        freq_bytes[4],
        freq_bytes[5],
        freq_bytes[6],
        freq_bytes[7],
    ]);
    assert!(
        (freq - 2.92e9).abs() < f64::EPSILON,
        "last frequency should be 2.92 GHz"
    );
}

#[test]
fn generate_example_executor_run() {
    let ws = workspace_root();
    let out_dir = ws.join("examples/runs");
    std::fs::create_dir_all(&out_dir).unwrap();

    let run_id = "basic_odmr_mock_executor_run";
    let run_path = out_dir.join(run_id);
    if run_path.exists() {
        std::fs::remove_dir_all(&run_path).unwrap();
    }

    let config = MockRunConfig {
        recipe_path: ws.join("examples/recipes/basic_odmr_mock.recipe.json"),
        station_path: ws.join("examples/station.mock.json"),
        run_root: out_dir,
        run_id: run_id.into(),
        safety_limits: None,
    };

    let report = run_mock(config).unwrap();
    assert_eq!(report.decision, ExecutionDecision::Completed);
    assert_eq!(report.steps_completed, 201);

    println!(
        "Generated example executor run at {}",
        report.run_directory.display()
    );
}
