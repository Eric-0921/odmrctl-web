//! odmr-executor — Layer 3 recipe execution engine.
//!
//! Mock-run implementation: drives fake devices through resolved recipe steps,
//! writes events/index/rawbin, and produces execution reports.
//!
//! No hardware access in mock mode. No CSV. No GUI.

use odmr_device::FakeDevice;
use odmr_logging::{
    create_run_directory, EventLevel, RunArtifactPaths, RunEvent, RunEventType, RunManifest,
};
use odmr_oe1022d::FakeOe1022d;
use odmr_recipe::{DeviceAction, SafetyLimit};
use odmr_safety::{check_resolved_recipe, SafetyDecision};
use odmr_smb100a::FakeSmb100a;
use odmr_types::DeviceId;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during executor mock-run.
#[derive(Debug)]
pub enum ExecutorError {
    Io(io::Error),
    Json(serde_json::Error),
    Recipe(odmr_recipe::RecipeError),
    Compile(odmr_compiler::CompileError),
    Logging(odmr_logging::LoggingError),
    Device(odmr_device::DeviceError),
    UnsupportedAction { device_id: String, action: String },
    MissingParam { action: String, param: String },
}

impl std::fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutorError::Io(e) => write!(f, "io error: {e}"),
            ExecutorError::Json(e) => write!(f, "json error: {e}"),
            ExecutorError::Recipe(e) => write!(f, "recipe error: {e}"),
            ExecutorError::Compile(e) => write!(f, "compile error: {e}"),
            ExecutorError::Logging(e) => write!(f, "logging error: {e}"),
            ExecutorError::Device(e) => write!(f, "device error: {e}"),
            ExecutorError::UnsupportedAction { device_id, action } => {
                write!(f, "unsupported action '{action}' for device '{device_id}'")
            }
            ExecutorError::MissingParam { action, param } => {
                write!(f, "missing param '{param}' for action '{action}'")
            }
        }
    }
}

impl std::error::Error for ExecutorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecutorError::Io(e) => Some(e),
            ExecutorError::Json(e) => Some(e),
            ExecutorError::Recipe(e) => Some(e),
            ExecutorError::Compile(e) => Some(e),
            ExecutorError::Logging(e) => Some(e),
            ExecutorError::Device(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for ExecutorError {
    fn from(e: io::Error) -> Self {
        ExecutorError::Io(e)
    }
}

impl From<serde_json::Error> for ExecutorError {
    fn from(e: serde_json::Error) -> Self {
        ExecutorError::Json(e)
    }
}

impl From<odmr_recipe::RecipeError> for ExecutorError {
    fn from(e: odmr_recipe::RecipeError) -> Self {
        ExecutorError::Recipe(e)
    }
}

impl From<odmr_compiler::CompileError> for ExecutorError {
    fn from(e: odmr_compiler::CompileError) -> Self {
        ExecutorError::Compile(e)
    }
}

impl From<odmr_logging::LoggingError> for ExecutorError {
    fn from(e: odmr_logging::LoggingError) -> Self {
        ExecutorError::Logging(e)
    }
}

impl From<odmr_device::DeviceError> for ExecutorError {
    fn from(e: odmr_device::DeviceError) -> Self {
        ExecutorError::Device(e)
    }
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Configuration for a mock-run execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MockRunConfig {
    pub recipe_path: PathBuf,
    pub station_path: PathBuf,
    pub run_root: PathBuf,
    pub run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_limits: Option<SafetyLimit>,
}

/// Overall outcome of a mock run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionDecision {
    Completed,
    RejectedBySafety,
    Failed,
}

/// Final report produced after a mock run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub schema_version: String,
    pub kind: String,
    pub run_id: String,
    pub decision: ExecutionDecision,
    pub resolved_recipe_id: String,
    pub safety_report_id: String,
    pub steps_total: usize,
    pub steps_completed: usize,
    pub events_written: usize,
    pub raw_frames_written: usize,
    pub run_directory: PathBuf,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run a complete mock execution pipeline.
///
/// 1. Load and compile recipe
/// 2. Run safety check
/// 3. Create run directory and write locked artifacts
/// 4. Iterate resolved steps against fake devices
/// 5. Write events, index entries, and raw frames
/// 6. Return execution report
pub fn run_mock(config: MockRunConfig) -> Result<ExecutionReport, ExecutorError> {
    // 1. Load recipe
    let recipe = odmr_recipe::load_recipe(&config.recipe_path)?;

    // 2. Compile
    let resolved = odmr_compiler::compile_recipe(&recipe)?;

    // 3. Dry-run plan
    let dry_run = odmr_compiler::build_dry_run_plan(&resolved);

    // 4. Safety check
    let limits = config.safety_limits.unwrap_or_else(mock_safety_limits);
    let safety_report = check_resolved_recipe(&resolved, &limits);

    // 5. Create run directory
    let run = create_run_directory(&config.run_root, &config.run_id)?;

    // 6. Load station (or empty object if missing)
    let station = load_station_value(&config.station_path);

    // 7. Write lock artifacts
    run.write_station_snapshot_json(&station)?;
    run.write_recipe_lock_json(&recipe)?;
    run.write_resolved_recipe_lock_json(&resolved)?;
    run.write_dry_run_plan_lock_json(&dry_run)?;
    run.write_safety_report_lock_json(&safety_report)?;

    // 8. Manifest
    let manifest = RunManifest {
        schema_version: "0.2.0".into(),
        kind: "run_manifest".into(),
        run_id: config.run_id.clone(),
        created_at_unix_ms: now_ms(),
        artifact_paths: RunArtifactPaths::default(),
        recipe_hash: Some(resolved.source_recipe_hash.clone()),
        resolved_recipe_id: Some(resolved.header.id.clone()),
        safety_report_id: Some(odmr_safety::safety_report_id(&safety_report)),
    };
    run.write_manifest(&manifest)?;

    // 9. Open writers
    let mut event_writer = run.open_event_writer()?;
    let mut index_writer = run.open_index_writer()?;
    let mut raw_writer = run.open_raw_bin_writer()?;

    let mut events_written = 0usize;

    // 10. run_created
    event_writer.write_event(&new_run_event(
        &config.run_id,
        &mut events_written,
        EventLevel::Info,
        RunEventType::RunCreated,
        None,
        None,
        "Run directory created",
    ))?;

    // 11. artifact_written
    event_writer.write_event(&new_run_event(
        &config.run_id,
        &mut events_written,
        EventLevel::Info,
        RunEventType::ArtifactWritten,
        None,
        None,
        "Locked artifacts written",
    ))?;

    // 12. safety_checked
    let safety_level = match safety_report.decision {
        SafetyDecision::Allow => EventLevel::Info,
        SafetyDecision::AllowWithWarnings => EventLevel::Warning,
        SafetyDecision::Reject => EventLevel::Error,
    };
    event_writer.write_event(&new_run_event(
        &config.run_id,
        &mut events_written,
        safety_level,
        RunEventType::SafetyChecked,
        None,
        None,
        &format!(
            "Safety decision: {:?} ({} findings)",
            safety_report.decision,
            safety_report.findings.len()
        ),
    ))?;

    // 13. Safety gate
    if safety_report.decision == SafetyDecision::Reject {
        event_writer.write_event(&new_run_event(
            &config.run_id,
            &mut events_written,
            EventLevel::Error,
            RunEventType::RunFailed,
            None,
            None,
            "Run rejected by safety interlock",
        ))?;

        return Ok(ExecutionReport {
            schema_version: "0.2.0".into(),
            kind: "execution_report".into(),
            run_id: config.run_id,
            decision: ExecutionDecision::RejectedBySafety,
            resolved_recipe_id: resolved.header.id,
            safety_report_id: odmr_safety::safety_report_id(&safety_report),
            steps_total: resolved.steps.len(),
            steps_completed: 0,
            events_written,
            raw_frames_written: 0,
            run_directory: run.run_directory_path(),
        });
    }

    // 14. run_started
    event_writer.write_event(&new_run_event(
        &config.run_id,
        &mut events_written,
        EventLevel::Info,
        RunEventType::RunStarted,
        None,
        None,
        &format!("Mock run started ({} steps)", resolved.steps.len()),
    ))?;

    // 15. Create fake devices
    let mut smb = FakeSmb100a::new(DeviceId::new("smb100a_01"));
    let mut _oe = FakeOe1022d::new(DeviceId::new("oe1022d_01"));

    let mut steps_completed = 0usize;
    let mut raw_frames_written = 0usize;

    // 16. Execute steps
    for (i, step) in resolved.steps.iter().enumerate() {
        // step_started
        event_writer.write_event(&new_run_event(
            &config.run_id,
            &mut events_written,
            EventLevel::Info,
            RunEventType::StepStarted,
            Some(&step.step_id),
            None,
            &format!("Step {} started", step.step_id),
        ))?;

        // Execute device actions
        for action in &step.device_actions {
            let cmd = translate_action_to_command(action)?;
            smb.send_command(&cmd)?;
        }

        // Generate mock raw frame
        let freq = step
            .sweep_coordinates
            .as_ref()
            .and_then(|c| c.get("smb100a.rf.frequency_hz"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let frame = generate_mock_raw_frame(i as u32, freq);
        let entry = raw_writer.append_frame(&frame)?;
        let mut index_entry = entry;
        index_entry.run_id = config.run_id.clone();
        index_entry.timestamp_unix_ms = now_ms();
        index_entry.step_id = Some(step.step_id.clone());
        index_writer.write_entry(&index_entry)?;
        raw_frames_written += 1;

        // step_completed
        event_writer.write_event(&new_run_event(
            &config.run_id,
            &mut events_written,
            EventLevel::Info,
            RunEventType::StepCompleted,
            Some(&step.step_id),
            None,
            &format!("Step {} completed", step.step_id),
        ))?;
        steps_completed += 1;
    }

    // 17. run_completed
    event_writer.write_event(&new_run_event(
        &config.run_id,
        &mut events_written,
        EventLevel::Info,
        RunEventType::RunCompleted,
        None,
        None,
        &format!(
            "Mock run completed ({} steps, {} frames)",
            steps_completed, raw_frames_written
        ),
    ))?;

    Ok(ExecutionReport {
        schema_version: "0.2.0".into(),
        kind: "execution_report".into(),
        run_id: config.run_id,
        decision: ExecutionDecision::Completed,
        resolved_recipe_id: resolved.header.id,
        safety_report_id: odmr_safety::safety_report_id(&safety_report),
        steps_total: resolved.steps.len(),
        steps_completed,
        events_written,
        raw_frames_written,
        run_directory: run.run_directory_path(),
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn load_station_value(path: &Path) -> serde_json::Value {
    if path.exists() {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    }
}

fn mock_safety_limits() -> SafetyLimit {
    SafetyLimit {
        schema_version: "0.2.0".into(),
        kind: "safety_limit".into(),
        id: "safety_nv_station_default".into(),
        name: Some("NV Station Default".into()),
        max_power_dbm: 20.0,
        max_frequency_hz: 3_000_000_000.0,
        min_frequency_hz: 1_000_000.0,
        max_fm_deviation_hz: Some(10_000_000.0),
        max_magnetic_field_t: Some(0.01),
        max_mag_ramp_rate_a_per_s: Some(0.1),
    }
}

/// Build a RunEvent with an auto-incremented event_id.
fn new_run_event(
    run_id: &str,
    counter: &mut usize,
    level: EventLevel,
    event_type: RunEventType,
    step_id: Option<&str>,
    device_id: Option<&str>,
    message: &str,
) -> RunEvent {
    *counter += 1;
    RunEvent {
        schema_version: "0.2.0".into(),
        kind: "run_event".into(),
        run_id: run_id.into(),
        event_id: format!("evt_{:06}", *counter),
        timestamp_unix_ms: now_ms(),
        timestamp_monotonic_ns: None,
        level,
        event_type,
        step_id: step_id.map(|s| s.into()),
        device_id: device_id.map(|s| s.into()),
        message: message.into(),
        data: None,
    }
}

/// Translate a generic DeviceAction into a device-specific command string.
fn translate_action_to_command(action: &DeviceAction) -> Result<String, ExecutorError> {
    let params = action
        .params
        .as_ref()
        .ok_or_else(|| ExecutorError::MissingParam {
            action: action.action.clone(),
            param: "(any)".into(),
        })?;

    match action.action.as_str() {
        "set_rf_frequency" => {
            let freq = params
                .get("frequency_hz")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| ExecutorError::MissingParam {
                    action: action.action.clone(),
                    param: "frequency_hz".into(),
                })?;
            Ok(odmr_smb100a::commands::set_frequency_hz(freq))
        }
        "set_rf_power" => {
            let power = params
                .get("power_dbm")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| ExecutorError::MissingParam {
                    action: action.action.clone(),
                    param: "power_dbm".into(),
                })?;
            Ok(odmr_smb100a::commands::set_power_dbm(power))
        }
        "set_rf_output_enabled" => {
            let enabled = params
                .get("enabled")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| ExecutorError::MissingParam {
                    action: action.action.clone(),
                    param: "enabled".into(),
                })?;
            Ok(odmr_smb100a::commands::set_output(enabled).to_string())
        }
        "set_fm_deviation" => {
            let dev = params
                .get("fm_deviation_hz")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| ExecutorError::MissingParam {
                    action: action.action.clone(),
                    param: "fm_deviation_hz".into(),
                })?;
            Ok(odmr_smb100a::commands::set_fm_deviation_hz(dev))
        }
        "set_fm_state" => {
            let enabled = params
                .get("enabled")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| ExecutorError::MissingParam {
                    action: action.action.clone(),
                    param: "enabled".into(),
                })?;
            Ok(odmr_smb100a::commands::set_fm_state(enabled).to_string())
        }
        _ => Err(ExecutorError::UnsupportedAction {
            device_id: action.device_id.clone(),
            action: action.action.clone(),
        }),
    }
}

/// Generate a deterministic 16-byte mock raw frame.
///
/// Format (little-endian):
///   offset 0..4  : step_index (u32)
///   offset 4..12 : frequency_hz (f64)
///   offset 12..16: mock_x_value (f32)
///
/// This is **mock data** and does not match real OE1022D RALL? output.
fn generate_mock_raw_frame(step_index: u32, frequency_hz: f64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(16);
    buf.extend_from_slice(&step_index.to_le_bytes());
    buf.extend_from_slice(&frequency_hz.to_le_bytes());
    let mock_x = (frequency_hz / 1e9) as f32;
    buf.extend_from_slice(&mock_x.to_le_bytes());
    buf
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_action_set_rf_frequency() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({ "frequency_hz": 2.882e9 })),
        };
        let cmd = translate_action_to_command(&action).unwrap();
        assert_eq!(cmd, "FREQ 2882000000");
    }

    #[test]
    fn translate_action_set_rf_power() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_power".into(),
            params: Some(serde_json::json!({ "power_dbm": -15.0 })),
        };
        let cmd = translate_action_to_command(&action).unwrap();
        assert_eq!(cmd, "POW -15dBm");
    }

    #[test]
    fn translate_action_set_output() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_output_enabled".into(),
            params: Some(serde_json::json!({ "enabled": true })),
        };
        let cmd = translate_action_to_command(&action).unwrap();
        assert_eq!(cmd, "OUTP ON");
    }

    #[test]
    fn translate_action_unknown_returns_error() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_unknown_thing".into(),
            params: Some(serde_json::json!({})),
        };
        let err = translate_action_to_command(&action).unwrap_err();
        assert!(matches!(
            err,
            ExecutorError::UnsupportedAction { device_id, action }
            if device_id == "smb100a_01" && action == "set_unknown_thing"
        ));
    }

    #[test]
    fn translate_action_missing_param_returns_error() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({})),
        };
        let err = translate_action_to_command(&action).unwrap_err();
        assert!(matches!(
            err,
            ExecutorError::MissingParam { action, param }
            if action == "set_rf_frequency" && param == "frequency_hz"
        ));
    }

    #[test]
    fn generate_mock_raw_frame_size() {
        let frame = generate_mock_raw_frame(42, 2.87e9);
        assert_eq!(frame.len(), 16);
    }

    #[test]
    fn generate_mock_raw_frame_contents() {
        let frame = generate_mock_raw_frame(7, 2.5e9);
        let idx = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
        assert_eq!(idx, 7);
        let freq = f64::from_le_bytes([
            frame[4], frame[5], frame[6], frame[7], frame[8], frame[9], frame[10], frame[11],
        ]);
        assert!((freq - 2.5e9).abs() < f64::EPSILON);
        let x = f32::from_le_bytes([frame[12], frame[13], frame[14], frame[15]]);
        assert!((x - 2.5).abs() < f32::EPSILON);
    }

    #[test]
    fn mock_safety_limits_are_deterministic() {
        let a = mock_safety_limits();
        let b = mock_safety_limits();
        assert_eq!(a.max_frequency_hz, b.max_frequency_hz);
        assert_eq!(a.max_power_dbm, b.max_power_dbm);
    }

    #[test]
    fn executor_error_from_device_error() {
        let de = odmr_device::DeviceError::UnknownCommand("FOO".into());
        let ee: ExecutorError = de.into();
        assert!(ee.to_string().contains("FOO"));
    }
}
