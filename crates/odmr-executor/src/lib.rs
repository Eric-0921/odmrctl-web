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
    Config(odmr_config::ConfigError),
    Recipe(odmr_recipe::RecipeError),
    Compile(odmr_compiler::CompileError),
    Logging(odmr_logging::LoggingError),
    Device(odmr_device::DeviceError),
    UnsupportedAction { device_id: String, action: String },
    MissingParam { action: String, param: String },
    Runtime(String),
}

impl std::fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutorError::Io(e) => write!(f, "io error: {e}"),
            ExecutorError::Json(e) => write!(f, "json error: {e}"),
            ExecutorError::Config(e) => write!(f, "config error: {e}"),
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
            ExecutorError::Runtime(msg) => write!(f, "runtime error: {msg}"),
        }
    }
}

impl std::error::Error for ExecutorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecutorError::Io(e) => Some(e),
            ExecutorError::Json(e) => Some(e),
            ExecutorError::Config(e) => Some(e),
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

impl From<odmr_config::ConfigError> for ExecutorError {
    fn from(e: odmr_config::ConfigError) -> Self {
        ExecutorError::Config(e)
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
// Hardware-mode execution
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareRunConfig {
    pub run_root: PathBuf,
    pub run_id: String,
    pub station: odmr_config::StationConfig,
    pub station_snapshot: serde_json::Value,
    pub steps: Vec<HardwareRunStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareRunStep {
    pub step_id: String,
    pub step_index: usize,
    pub b_target_nt: [f64; 3],
    pub magnetic_axes: Vec<HardwareMagAxisTarget>,
    pub rf: HardwareRfSweep,
    pub oe: HardwareOeAcquisition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub laser: Option<HardwareLaserTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareMagAxisTarget {
    pub device_id: String,
    pub current_a: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareRfSweep {
    pub device_id: String,
    pub start_hz: f64,
    pub stop_hz: f64,
    pub step_hz: f64,
    pub dwell_ms: u64,
    pub power_dbm: f64,
    pub spacing: String,
    pub shape: String,
    #[serde(default = "default_sweep_mode")]
    pub sweep_mode: String,
    #[serde(default = "default_trigger_source")]
    pub trigger_source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sweep_output_start_v: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sweep_output_stop_v: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareOeAcquisition {
    pub device_id: String,
    pub pre_start_ms: u64,
    pub post_stop_ms: u64,
    #[serde(default = "default_oe_baud")]
    pub baud: u32,
    #[serde(default = "default_oe_read_interval_ms")]
    pub read_interval_ms: u64,
    #[serde(default = "default_oe_timeout_ms")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareLaserTarget {
    pub device_id: String,
    pub power_mw: u16,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareProgress {
    pub run_id: String,
    pub step_id: Option<String>,
    pub step_index: Option<usize>,
    pub phase: String,
    pub steps_completed: usize,
    pub oe_frames_captured: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareExecutionReport {
    pub run_id: String,
    pub run_directory: PathBuf,
    pub steps_total: usize,
    pub steps_completed: usize,
    pub oe_frames_captured: u64,
    pub stopped: bool,
    pub cleanup_state: String,
    pub artifact_paths: std::collections::HashMap<String, String>,
}

#[derive(Clone, Default)]
pub struct RunControl {
    stop_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl RunControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_stop(&self) {
        self.stop_requested
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_stop_requested(&self) -> bool {
        self.stop_requested
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

pub fn run_hardware<F>(
    config: HardwareRunConfig,
    control: &RunControl,
    mut on_progress: F,
) -> Result<HardwareExecutionReport, ExecutorError>
where
    F: FnMut(HardwareProgress),
{
    odmr_config::validate_station_config(&config.station)?;
    if config.steps.is_empty() {
        return Err(ExecutorError::Runtime(
            "hardware run requires at least one step".into(),
        ));
    }

    let run_dir = config.run_root.join(&config.run_id);
    if run_dir.exists() {
        return Err(ExecutorError::Runtime(format!(
            "run directory already exists: {}",
            run_dir.display()
        )));
    }

    std::fs::create_dir_all(run_dir.join("metadata"))?;
    std::fs::create_dir_all(run_dir.join("raw"))?;
    std::fs::create_dir_all(run_dir.join("summary"))?;

    std::fs::write(
        run_dir.join("metadata").join("station_snapshot.json"),
        serde_json::to_vec_pretty(&config.station_snapshot)?,
    )?;
    std::fs::write(
        run_dir.join("metadata").join("hardware_run_config.json"),
        serde_json::to_vec_pretty(&config)?,
    )?;

    let mut events = std::fs::File::create(run_dir.join("events.jsonl"))?;
    let mut index = std::fs::File::create(run_dir.join("index.jsonl"))?;
    let mut event_counter = 0usize;
    let mut total_frames = 0u64;
    let mut completed_steps = 0usize;

    let smb_device = find_device(&config.station, &config.steps[0].rf.device_id)?;
    let smb_address = tcp_address(&smb_device.transport)?;
    let mut smb = std::net::TcpStream::connect(&smb_address)?;
    smb.set_read_timeout(Some(std::time::Duration::from_millis(5000)))?;
    smb.set_write_timeout(Some(std::time::Duration::from_millis(5000)))?;

    let mut mag_axes = open_mag_axes(&config.station, &config.steps)?;
    let mut laser_client = open_optional_laser(&config.station, &config.steps)?;

    write_hardware_event(
        &mut events,
        &config.run_id,
        &mut event_counter,
        "info",
        "hardware_run_started",
        None,
        "hardware execution started",
    )?;

    for step in &config.steps {
        if control.is_stop_requested() {
            break;
        }

        on_progress(HardwareProgress {
            run_id: config.run_id.clone(),
            step_id: Some(step.step_id.clone()),
            step_index: Some(step.step_index),
            phase: "step_started".into(),
            steps_completed: completed_steps,
            oe_frames_captured: total_frames,
        });
        write_hardware_event(
            &mut events,
            &config.run_id,
            &mut event_counter,
            "info",
            "step_started",
            Some(&step.step_id),
            &format!("hardware step {} started", step.step_id),
        )?;

        apply_mag_step(&mut mag_axes, &step.magnetic_axes)?;
        apply_optional_laser(laser_client.as_mut(), step.laser.as_ref())?;

        let oe_device = find_device(&config.station, &step.oe.device_id)?;
        let oe_port = serial_port_path(&oe_device.transport)?;
        let (mut collector, rx) =
            odmr_oe1022d::RallCollector::start(odmr_oe1022d::CollectorConfig {
                port_path: oe_port,
                baud: step.oe.baud,
                read_interval_ms: step.oe.read_interval_ms,
                timeout_ms: step.oe.timeout_ms,
            })
            .map_err(ExecutorError::Runtime)?;

        std::thread::sleep(std::time::Duration::from_millis(step.oe.pre_start_ms));
        configure_smb_sweep(&mut smb, &step.rf)?;
        scpi_set(&mut smb, odmr_smb100a::commands::set_output(true))?;
        scpi_set(&mut smb, odmr_smb100a::commands::trigger_sweep_immediate())?;
        scpi_set(&mut smb, odmr_smb100a::commands::execute_frequency_sweep())?;
        wait_for_smb_sweep(&mut smb, control, estimate_sweep_timeout_ms(&step.rf))?;
        std::thread::sleep(std::time::Duration::from_millis(step.oe.post_start_ms()));
        collector.signal_stop();
        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut step_bytes = Vec::new();
        let mut captured_this_step = 0u64;
        for frame in rx.try_iter() {
            step_bytes.extend_from_slice(&frame.raw);
            captured_this_step += 1;
        }
        total_frames += captured_this_step;
        std::fs::write(
            run_dir.join("raw").join(format!("{}.rall", step.step_id)),
            &step_bytes,
        )?;

        let index_entry = serde_json::json!({
            "schema_version": "0.1.0",
            "kind": "spectrum_raw_index",
            "run_id": config.run_id,
            "step_id": step.step_id,
            "step_index": step.step_index,
            "b_target_nt": step.b_target_nt,
            "raw_path": format!("raw/{}.rall", step.step_id),
            "oe_frames_captured": captured_this_step,
            "rf_point_count": rf_point_count_from_sweep(&step.rf),
            "timestamp_unix_ms": now_ms()
        });
        serde_json::to_writer(&mut index, &index_entry)?;
        std::io::Write::write_all(&mut index, b"\n")?;

        completed_steps += 1;
        on_progress(HardwareProgress {
            run_id: config.run_id.clone(),
            step_id: Some(step.step_id.clone()),
            step_index: Some(step.step_index),
            phase: "step_completed".into(),
            steps_completed: completed_steps,
            oe_frames_captured: total_frames,
        });
        write_hardware_event(
            &mut events,
            &config.run_id,
            &mut event_counter,
            "info",
            "step_completed",
            Some(&step.step_id),
            &format!("captured {} OE frames", captured_this_step),
        )?;
    }

    let cleanup_state = cleanup_hardware(&mut smb, laser_client.as_mut(), &mut mag_axes)?;
    write_hardware_event(
        &mut events,
        &config.run_id,
        &mut event_counter,
        "info",
        "cleanup_completed",
        None,
        &cleanup_state,
    )?;

    std::fs::write(
        run_dir.join("summary").join("run_summary.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": "0.1.0",
            "kind": "experiment_plan_hardware_run_summary",
            "run_id": config.run_id,
            "steps_total": config.steps.len(),
            "steps_completed": completed_steps,
            "oe_frames_captured": total_frames,
            "cleanup_state": cleanup_state,
            "stopped": control.is_stop_requested()
        }))?,
    )?;

    let mut artifact_paths = std::collections::HashMap::new();
    artifact_paths.insert("events".into(), "events.jsonl".into());
    artifact_paths.insert("index".into(), "index.jsonl".into());
    artifact_paths.insert(
        "station_snapshot".into(),
        "metadata/station_snapshot.json".into(),
    );
    artifact_paths.insert(
        "hardware_run_config".into(),
        "metadata/hardware_run_config.json".into(),
    );
    artifact_paths.insert("summary".into(), "summary/run_summary.json".into());

    Ok(HardwareExecutionReport {
        run_id: config.run_id,
        run_directory: run_dir,
        steps_total: config.steps.len(),
        steps_completed: completed_steps,
        oe_frames_captured: total_frames,
        stopped: control.is_stop_requested(),
        cleanup_state,
        artifact_paths,
    })
}

impl HardwareOeAcquisition {
    fn post_start_ms(&self) -> u64 {
        self.post_stop_ms
    }
}

fn default_sweep_mode() -> String {
    "AUTO".into()
}

fn default_trigger_source() -> String {
    "SING".into()
}

fn default_oe_baud() -> u32 {
    921_600
}

fn default_oe_read_interval_ms() -> u64 {
    48
}

fn default_oe_timeout_ms() -> u64 {
    5000
}

fn find_device<'a>(
    station: &'a odmr_config::StationConfig,
    device_id: &str,
) -> Result<&'a odmr_config::StationDeviceConfig, ExecutorError> {
    station
        .devices
        .iter()
        .find(|device| device.device_id == device_id)
        .ok_or_else(|| ExecutorError::Runtime(format!("missing device in station: {device_id}")))
}

fn tcp_address(transport: &odmr_config::DeviceTransportConfig) -> Result<String, ExecutorError> {
    match transport {
        odmr_config::DeviceTransportConfig::TcpScpi { host, port, .. } => {
            Ok(format!("{host}:{port}"))
        }
        _ => Err(ExecutorError::Runtime("expected TCP SCPI transport".into())),
    }
}

fn serial_port_path(
    transport: &odmr_config::DeviceTransportConfig,
) -> Result<String, ExecutorError> {
    match transport {
        odmr_config::DeviceTransportConfig::Serial { port, .. } => Ok(port.clone()),
        _ => Err(ExecutorError::Runtime("expected serial transport".into())),
    }
}

fn scpi_set(stream: &mut std::net::TcpStream, command: &str) -> Result<(), ExecutorError> {
    use std::io::Write;
    stream.write_all(format!("{}\n", command.trim()).as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn scpi_query(stream: &mut std::net::TcpStream, command: &str) -> Result<String, ExecutorError> {
    use std::io::{BufRead, Write};
    stream.write_all(format!("{}\n", command.trim()).as_bytes())?;
    stream.flush()?;
    let mut reader = std::io::BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn configure_smb_sweep(
    smb: &mut std::net::TcpStream,
    rf: &HardwareRfSweep,
) -> Result<(), ExecutorError> {
    for command in [
        odmr_smb100a::commands::set_output(false),
        odmr_smb100a::commands::set_modulation_global(false),
        odmr_smb100a::commands::set_fm_state(false),
        odmr_smb100a::commands::set_freq_mode_sweep(),
    ] {
        scpi_set(smb, command)?;
    }
    scpi_set(smb, &odmr_smb100a::commands::set_power_dbm(rf.power_dbm))?;
    scpi_set(smb, &odmr_smb100a::commands::set_freq_start_hz(rf.start_hz))?;
    scpi_set(smb, &odmr_smb100a::commands::set_freq_stop_hz(rf.stop_hz))?;
    scpi_set(smb, &odmr_smb100a::commands::set_sweep_step_hz(rf.step_hz))?;
    scpi_set(
        smb,
        &odmr_smb100a::commands::set_sweep_dwell_ms(rf.dwell_ms),
    )?;
    scpi_set(smb, &odmr_smb100a::commands::set_sweep_spacing(&rf.spacing))?;
    scpi_set(smb, &odmr_smb100a::commands::set_sweep_shape(&rf.shape))?;
    scpi_set(smb, &odmr_smb100a::commands::set_sweep_mode(&rf.sweep_mode))?;
    scpi_set(
        smb,
        &odmr_smb100a::commands::set_sweep_trigger_source(&rf.trigger_source),
    )?;
    if let Some(v) = rf.sweep_output_start_v {
        scpi_set(smb, &odmr_smb100a::commands::set_sweep_output_start_v(v))?;
    }
    if let Some(v) = rf.sweep_output_stop_v {
        scpi_set(smb, &odmr_smb100a::commands::set_sweep_output_stop_v(v))?;
    }
    Ok(())
}

fn wait_for_smb_sweep(
    smb: &mut std::net::TcpStream,
    control: &RunControl,
    timeout_ms: u64,
) -> Result<(), ExecutorError> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        if control.is_stop_requested() {
            return Ok(());
        }
        if std::time::Instant::now() > deadline {
            return Err(ExecutorError::Runtime(
                "SMB100A sweep did not finish before timeout".into(),
            ));
        }
        let running = scpi_query(smb, odmr_smb100a::commands::query_sweep_running())?;
        if running == "0" {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn estimate_sweep_timeout_ms(rf: &HardwareRfSweep) -> u64 {
    let points = rf_point_count_from_sweep(rf) as u64;
    points.saturating_mul(rf.dwell_ms).saturating_add(3000)
}

fn rf_point_count_from_sweep(rf: &HardwareRfSweep) -> usize {
    if rf.step_hz <= 0.0 || rf.stop_hz < rf.start_hz {
        return 1;
    }
    (((rf.stop_hz - rf.start_hz) / rf.step_hz).floor() as usize) + 1
}

fn open_mag_axes(
    station: &odmr_config::StationConfig,
    steps: &[HardwareRunStep],
) -> Result<std::collections::HashMap<String, odmr_maynuo_m8812::MaynuoM8812Transport>, ExecutorError>
{
    let mut ids = std::collections::BTreeSet::new();
    for step in steps {
        for axis in &step.magnetic_axes {
            ids.insert(axis.device_id.clone());
        }
    }

    let mut out = std::collections::HashMap::new();
    for device_id in ids {
        let device = find_device(station, &device_id)?;
        let port = serial_port_path(&device.transport)?;
        let mut transport = odmr_maynuo_m8812::MaynuoM8812Transport::open(
            odmr_types::DeviceId::new(device_id.clone()),
            &port,
            odmr_maynuo_m8812::MaynuoSerialPortConfig::default(),
        )
        .map_err(|e| ExecutorError::Runtime(format!("open Maynuo {device_id}: {e}")))?;
        transport
            .send_set_remote()
            .map_err(|e| ExecutorError::Runtime(format!("Maynuo SYST:REM {device_id}: {e}")))?;
        transport
            .send_set_voltage(75)
            .map_err(|e| ExecutorError::Runtime(format!("Maynuo VOLT 75 {device_id}: {e}")))?;
        out.insert(device_id, transport);
    }
    Ok(out)
}

fn apply_mag_step(
    axes: &mut std::collections::HashMap<String, odmr_maynuo_m8812::MaynuoM8812Transport>,
    targets: &[HardwareMagAxisTarget],
) -> Result<(), ExecutorError> {
    for target in targets {
        let Some(axis) = axes.get_mut(&target.device_id) else {
            return Err(ExecutorError::Runtime(format!(
                "mag axis transport missing: {}",
                target.device_id
            )));
        };
        axis.send_set_current(target.current_a).map_err(|e| {
            ExecutorError::Runtime(format!("Maynuo CURR {}: {e}", target.device_id))
        })?;
        axis.send_set_output(true).map_err(|e| {
            ExecutorError::Runtime(format!("Maynuo OUTP 1 {}: {e}", target.device_id))
        })?;
    }
    Ok(())
}

fn open_optional_laser(
    station: &odmr_config::StationConfig,
    steps: &[HardwareRunStep],
) -> Result<Option<odmr_laser::LaserClient>, ExecutorError> {
    let laser_id = steps
        .iter()
        .find_map(|step| step.laser.as_ref().map(|laser| laser.device_id.clone()));
    let Some(laser_id) = laser_id else {
        return Ok(None);
    };
    let device = find_device(station, &laser_id)?;
    let port = serial_port_path(&device.transport)?;
    let client = odmr_laser::LaserClient::open(
        odmr_types::DeviceId::new(laser_id),
        port,
        odmr_laser::LaserSerialConfig::default(),
    )
    .map_err(|e| ExecutorError::Runtime(format!("open laser: {e}")))?;
    Ok(Some(client))
}

fn apply_optional_laser(
    laser: Option<&mut odmr_laser::LaserClient>,
    target: Option<&HardwareLaserTarget>,
) -> Result<(), ExecutorError> {
    let (Some(laser), Some(target)) = (laser, target) else {
        return Ok(());
    };
    laser
        .set_power(target.power_mw)
        .map_err(|e| ExecutorError::Runtime(format!("laser set_power: {e}")))?;
    laser
        .set_enabled(target.enabled)
        .map_err(|e| ExecutorError::Runtime(format!("laser set_enabled: {e}")))?;
    Ok(())
}

fn cleanup_hardware(
    smb: &mut std::net::TcpStream,
    laser: Option<&mut odmr_laser::LaserClient>,
    axes: &mut std::collections::HashMap<String, odmr_maynuo_m8812::MaynuoM8812Transport>,
) -> Result<String, ExecutorError> {
    for command in [
        odmr_smb100a::commands::set_output(false),
        odmr_smb100a::commands::set_modulation_global(false),
        odmr_smb100a::commands::set_fm_state(false),
    ] {
        let _ = scpi_set(smb, command);
    }
    if let Some(laser) = laser {
        let _ = laser.emergency_off();
    }
    for (device_id, axis) in axes.iter_mut() {
        axis.send_set_current(0.0).map_err(|e| {
            ExecutorError::Runtime(format!("Maynuo cleanup CURR 0 {device_id}: {e}"))
        })?;
        axis.send_set_output(false).map_err(|e| {
            ExecutorError::Runtime(format!("Maynuo cleanup OUTP 0 {device_id}: {e}"))
        })?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = axis.query_meas_current();
        let _ = axis.send_set_local();
    }
    Ok("rf_off_laser_off_magnetic_cleanup_completed".into())
}

fn write_hardware_event(
    writer: &mut std::fs::File,
    run_id: &str,
    event_counter: &mut usize,
    level: &str,
    event_type: &str,
    step_id: Option<&str>,
    message: &str,
) -> Result<(), ExecutorError> {
    use std::io::Write;
    *event_counter += 1;
    serde_json::to_writer(
        &mut *writer,
        &serde_json::json!({
            "schema_version": "0.1.0",
            "kind": "experiment_plan_run_event",
            "run_id": run_id,
            "event_id": format!("evt_{:06}", *event_counter),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "timestamp_unix_ms": now_ms(),
            "level": level,
            "event_type": event_type,
            "step_id": step_id,
            "message": message
        }),
    )?;
    writer.write_all(b"\n")?;
    Ok(())
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
