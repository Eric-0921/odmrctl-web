// Prevents additional console window on Windows in release mode.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod experiment_plan;
mod panels;
mod station;
mod workbench_state;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tauri::Manager;

/// Returns static application metadata.
#[tauri::command]
fn app_metadata() -> serde_json::Value {
    serde_json::json!({
        "name": "ODMR Device Workbench",
        "version": "0.1.0",
        "phase": "M5C-A",
        "mode": "DEVICE WORKBENCH",
        "boundary_note": "Typed hardware access via Tauri commands only. No raw SCPI from frontend."
    })
}

// ---------------------------------------------------------------------------
// M4.0 analysis data types — must match frontend types/analysis.ts exactly
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpectrumPoint {
    run_id: String,
    step_id: String,
    repeat_index: i64,
    frequency_hz: f64,
    frequency_verified_hz: f64,
    b_x_mean_v: f64,
    b_x_mean_mv: f64,
    b_x_std_v: f64,
    b_y_mean_v: f64,
    b_y_mean_mv: f64,
    b_y_std_v: f64,
    frames_used: i64,
    frames_parse_failed: i64,
    step_passed: bool,
    quality_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrequencyGroup {
    frequency_hz: f64,
    point_count: i64,
    total_frames_used: i64,
    frames_parse_failed: i64,
    contributing_run_ids: Vec<String>,
    b_x_mean_v: f64,
    b_x_mean_mv: f64,
    b_x_std_v: f64,
    b_x_std_mv: f64,
    b_x_min_v: f64,
    b_x_min_mv: f64,
    b_x_max_v: f64,
    b_x_max_mv: f64,
    b_y_mean_v: f64,
    b_y_mean_mv: f64,
    b_y_std_v: f64,
    b_y_std_mv: f64,
    b_y_min_v: f64,
    b_y_min_mv: f64,
    b_y_max_v: f64,
    b_y_max_mv: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunOverlaySummaryData {
    schema_version: String,
    kind: String,
    frequency_count: i64,
    generated_at: String,
    frequencies: Vec<FrequencyGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnalysisSummaryData {
    schema_version: String,
    kind: String,
    source_run_ids: Vec<String>,
    frequency_count: i64,
    point_count: i64,
    frames_used: i64,
    frames_parse_failed: i64,
    parse_failure_rate: f64,
    all_runs_passed: bool,
    all_safe_states_confirmed: bool,
    no_csv: bool,
    no_magnetic: bool,
    quality_flags_passed: bool,
    odmr_dip_detected: bool,
    physical_odmr_response_required: bool,
    contrast_estimate_b_x_v: Option<f64>,
    contrast_estimate_b_x_mv: Option<f64>,
    contrast_estimate_b_y_v: Option<f64>,
    contrast_estimate_b_y_mv: Option<f64>,
    oe1022d_display_idn_by_run: HashMap<String, String>,
    generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QualityFlagsData {
    schema_version: String,
    kind: String,
    passed: bool,
    missing_artifact: bool,
    #[serde(default)]
    missing_artifact_details: HashMap<String, Vec<String>>,
    failed_run: bool,
    failed_run_ids: Vec<String>,
    parse_failures: bool,
    parse_failure_count: i64,
    audit_mismatch: bool,
    audit_mismatch_run_ids: Vec<String>,
    unsafe_final_state: bool,
    unsafe_final_state_run_ids: Vec<String>,
    csv_present: bool,
    #[serde(default)]
    csv_present_details: HashMap<String, Vec<String>>,
    magnetic_command_present: bool,
    #[serde(default)]
    magnetic_command_details: HashMap<String, i64>,
    frequency_grid_mismatch: bool,
    empty_signal_series: bool,
    generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportManifestFile {
    relative_path: String,
    sha256: String,
    size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportManifestData {
    schema_version: String,
    kind: String,
    source_run_ids: Vec<String>,
    generated_at: String,
    files: Vec<ExportManifestFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnalysisData {
    spectrum_points: Vec<SpectrumPoint>,
    run_overlay_summary: RunOverlaySummaryData,
    analysis_summary: AnalysisSummaryData,
    quality_flags: QualityFlagsData,
    export_manifest: Option<ExportManifestData>,
    warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// M4.0 Tauri commands
// ---------------------------------------------------------------------------

/// Read all M3.6 analysis artifacts from a directory.
///
/// Accepts either the parent analysis run directory (looks for `analysis/` subdir)
/// or the `analysis/` subdirectory directly.
///
/// Returns structured AnalysisData with all parsed artifacts.
/// Never writes to disk. Never touches hardware.
#[tauri::command]
fn read_analysis_directory(path: String) -> Result<AnalysisData, String> {
    let base = Path::new(&path);

    // Support both parent directory and analysis/ subdirectory
    let analysis_dir = if base.join("analysis").is_dir()
        && base.join("analysis").join("quality_flags.json").exists()
    {
        base.join("analysis")
    } else {
        base.to_path_buf()
    };

    let mut warnings: Vec<String> = Vec::new();

    // --- Required: quality_flags.json ---
    let qf_path = analysis_dir.join("quality_flags.json");
    if !qf_path.exists() {
        return Err(format!("Required file not found: {}", qf_path.display()));
    }
    let qf_text =
        fs::read_to_string(&qf_path).map_err(|e| format!("read {}: {}", qf_path.display(), e))?;
    let quality_flags: QualityFlagsData = serde_json::from_str(&qf_text)
        .map_err(|e| format!("parse {}: {}", qf_path.display(), e))?;

    // --- Required: odmr_like_analysis_summary.json ---
    let as_path = analysis_dir.join("odmr_like_analysis_summary.json");
    if !as_path.exists() {
        return Err(format!("Required file not found: {}", as_path.display()));
    }
    let as_text =
        fs::read_to_string(&as_path).map_err(|e| format!("read {}: {}", as_path.display(), e))?;
    let analysis_summary: AnalysisSummaryData = serde_json::from_str(&as_text)
        .map_err(|e| format!("parse {}: {}", as_path.display(), e))?;

    // --- Required: run_overlay_summary.json ---
    let ro_path = analysis_dir.join("run_overlay_summary.json");
    if !ro_path.exists() {
        return Err(format!("Required file not found: {}", ro_path.display()));
    }
    let ro_text =
        fs::read_to_string(&ro_path).map_err(|e| format!("read {}: {}", ro_path.display(), e))?;
    let run_overlay_summary: RunOverlaySummaryData = serde_json::from_str(&ro_text)
        .map_err(|e| format!("parse {}: {}", ro_path.display(), e))?;

    // --- Required: spectrum_points.jsonl ---
    let sp_path = analysis_dir.join("spectrum_points.jsonl");
    if !sp_path.exists() {
        return Err(format!("Required file not found: {}", sp_path.display()));
    }
    let sp_text =
        fs::read_to_string(&sp_path).map_err(|e| format!("read {}: {}", sp_path.display(), e))?;
    let spectrum_points: Vec<SpectrumPoint> = sp_text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str::<SpectrumPoint>(line)
                .map_err(|e| format!("parse {} line {}: {}", sp_path.display(), i + 1, e))
        })
        .collect::<Result<Vec<_>, String>>()?;

    // --- Optional: export_manifest.json ---
    let export_manifest: Option<ExportManifestData> = {
        let em_path = analysis_dir.join("export_manifest.json");
        if em_path.exists() {
            let em_text = fs::read_to_string(&em_path)
                .map_err(|e| format!("read {}: {}", em_path.display(), e))?;
            match serde_json::from_str::<ExportManifestData>(&em_text) {
                Ok(m) => Some(m),
                Err(e) => {
                    warnings.push(format!("Failed to parse {}: {}", em_path.display(), e));
                    None
                }
            }
        } else {
            None
        }
    };

    Ok(AnalysisData {
        spectrum_points,
        run_overlay_summary,
        analysis_summary,
        quality_flags,
        export_manifest,
        warnings,
    })
}

/// Open a native folder picker and return the selected path.
/// Returns None if the user cancelled.
#[tauri::command]
async fn pick_analysis_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let path = app.dialog().file().blocking_pick_folder();
    Ok(path.map(|p| p.to_string()))
}

// ---------------------------------------------------------------------------
// M4.1 recipe viewer commands — read-only, no hardware access
// ---------------------------------------------------------------------------

/// Read a recipe JSON file as raw text.
#[tauri::command]
fn read_recipe_file(path: String) -> Result<String, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read {}: {}", path, e))?;
    Ok(text)
}

/// Open a native file picker for recipe JSON files.
/// Returns None if the user cancelled.
#[tauri::command]
async fn pick_recipe_file(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let path = app
        .dialog()
        .file()
        .add_filter("Recipe JSON", &["json"])
        .blocking_pick_file();
    Ok(path.map(|p| p.to_string()))
}

// ---------------------------------------------------------------------------
// M5A combined run data types — must match frontend types/m5aRun.ts exactly
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SafeStateData {
    confirmed: bool,
    rf_output: Option<String>,
    modulation: Option<String>,
    fm: Option<String>,
    magnetic_output: Option<String>,
    magnetic_current_ma: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DevicePreflightReportData {
    device_id: String,
    kind: String,
    reachability: bool,
    identity_raw: Option<String>,
    identity_display: Option<String>,
    error_queue: Vec<String>,
    safe_state: Option<SafeStateData>,
    warnings: Vec<String>,
    commands_sent: Option<Vec<String>>,
    laser_on_sent: Option<bool>,
    nonzero_power_sent: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceLockStatusData {
    device_id: String,
    acquired: bool,
    lock_file: String,
    pid: Option<u32>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StationPreflightReportData {
    schema_version: String,
    generated_at: String,
    station_profile: String,
    all_devices_reachable: bool,
    all_identities_verified: bool,
    all_safe_states_confirmed: bool,
    operator_approved: bool,
    elapsed_ms: u64,
    devices: Vec<DevicePreflightReportData>,
    lock_status: Vec<DeviceLockStatusData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RfReportSectionData {
    requested_frequency_hz: u64,
    requested_power_dbm: f64,
    readback_frequency_hz: Option<f64>,
    readback_power_dbm: Option<f64>,
    rf_on_window_start_unix_ms: Option<u64>,
    rf_on_window_end_unix_ms: Option<u64>,
    rf_final_off: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MagReportSectionData {
    axis_id: String,
    expected_sn: String,
    observed_sn: String,
    zero_readback_current_ma: f64,
    zero_readback_std_ma: f64,
    commanded_recur_current_ma: f64,
    measured_recur_current_ma: f64,
    measured_recur_field_nt: f64,
    current_error_ma: f64,
    mag_final_output_off: bool,
    mag_final_current_zero: bool,
    mag_final_local_requested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OeReportSectionData {
    frames_requested: u64,
    frames_acquired: u64,
    raw_bin_bytes: u64,
    frame_size_bytes: u64,
    parse_failures: u64,
    timeout_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimelineReportSectionData {
    rf_on_before_oe_capture: bool,
    mag_hold_before_oe_capture: bool,
    oe_capture_completed_before_cleanup: bool,
    cleanup_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CombinedRunReportData {
    schema_version: String,
    run_id: String,
    passed: bool,
    rf: RfReportSectionData,
    magnetic: MagReportSectionData,
    oe: OeReportSectionData,
    timeline: TimelineReportSectionData,
    errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CombinedRunEventData {
    event_type: String,
    timestamp_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandAuditEntryData {
    seq: u64,
    timestamp_unix_ms: u64,
    device_id: String,
    command: String,
    command_class: String,
    allowed: bool,
    sent_to_transport: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    rejection_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transport_error: Option<String>,
    safety_relevant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrameIndexEntryData {
    frame_index: u64,
    length: u64,
    offset: u64,
    timestamp_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrameSummaryEntryData {
    elapsed_ms: u64,
    frame_index: u64,
    size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct M5aRunData {
    preflight: Option<StationPreflightReportData>,
    combined_run_report: Option<CombinedRunReportData>,
    events: Vec<CombinedRunEventData>,
    smb_audit: Vec<CommandAuditEntryData>,
    maynuo_audit: Vec<CommandAuditEntryData>,
    oe_audit: Vec<CommandAuditEntryData>,
    frame_index: Vec<FrameIndexEntryData>,
    frame_summary: Vec<FrameSummaryEntryData>,
    warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// M5A Tauri commands — read-only, no hardware access
// ---------------------------------------------------------------------------

/// Read all M5A combined run artifacts from a directory.
///
/// Looks for:
/// - preflight/station_preflight_report.json
/// - combined_run_report.json
/// - combined_events.jsonl
/// - smb_command_audit.jsonl
/// - maynuo_command_audit.jsonl
/// - oe_command_audit.jsonl
/// - frame_index.jsonl
/// - frame_summary.jsonl
///
/// Returns structured M5aRunData with all parsed artifacts.
/// Never writes to disk. Never touches hardware.
#[tauri::command]
fn read_m5a_run_directory(path: String) -> Result<M5aRunData, String> {
    let base = Path::new(&path);
    let mut warnings: Vec<String> = Vec::new();

    // --- Optional: preflight/station_preflight_report.json ---
    let preflight: Option<StationPreflightReportData> = {
        let p = base.join("preflight").join("station_preflight_report.json");
        if p.exists() {
            let text =
                fs::read_to_string(&p).map_err(|e| format!("read {}: {}", p.display(), e))?;
            match serde_json::from_str(&text) {
                Ok(v) => Some(v),
                Err(e) => {
                    warnings.push(format!("parse {}: {}", p.display(), e));
                    None
                }
            }
        } else {
            None
        }
    };

    // --- Optional: combined_run_report.json ---
    let combined_run_report: Option<CombinedRunReportData> = {
        let p = base.join("combined_run_report.json");
        if p.exists() {
            let text =
                fs::read_to_string(&p).map_err(|e| format!("read {}: {}", p.display(), e))?;
            match serde_json::from_str(&text) {
                Ok(v) => Some(v),
                Err(e) => {
                    warnings.push(format!("parse {}: {}", p.display(), e));
                    None
                }
            }
        } else {
            None
        }
    };

    // --- Helper: read JSONL ---
    fn read_jsonl<T: serde::de::DeserializeOwned>(
        path: &Path,
        warnings: &mut Vec<String>,
    ) -> Vec<T> {
        if !path.exists() {
            return Vec::new();
        }
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                warnings.push(format!("read {}: {}", path.display(), e));
                return Vec::new();
            }
        };
        text.lines()
            .filter(|l| !l.trim().is_empty())
            .enumerate()
            .filter_map(|(i, line)| match serde_json::from_str::<T>(line) {
                Ok(v) => Some(v),
                Err(e) => {
                    warnings.push(format!("parse {} line {}: {}", path.display(), i + 1, e));
                    None
                }
            })
            .collect()
    }

    let events = read_jsonl(&base.join("combined_events.jsonl"), &mut warnings);
    let smb_audit = read_jsonl(&base.join("smb_command_audit.jsonl"), &mut warnings);
    let maynuo_audit = read_jsonl(&base.join("maynuo_command_audit.jsonl"), &mut warnings);
    let oe_audit = read_jsonl(&base.join("oe_command_audit.jsonl"), &mut warnings);
    let frame_index = read_jsonl(&base.join("frame_index.jsonl"), &mut warnings);
    let frame_summary = read_jsonl(&base.join("frame_summary.jsonl"), &mut warnings);

    Ok(M5aRunData {
        preflight,
        combined_run_report,
        events,
        smb_audit,
        maynuo_audit,
        oe_audit,
        frame_index,
        frame_summary,
        warnings,
    })
}

/// Open a native folder picker for M5A run directories.
/// Returns None if the user cancelled.
#[tauri::command]
async fn pick_m5a_run_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let path = app.dialog().file().blocking_pick_folder();
    Ok(path.map(|p| p.to_string()))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            app_metadata,
            read_analysis_directory,
            pick_analysis_directory,
            read_recipe_file,
            pick_recipe_file,
            read_m5a_run_directory,
            pick_m5a_run_directory,
            station::load_station_profile,
            station::load_example_station_profile,
            station::run_station_preflight_cmd,
            station::release_all_locks,
            station::serial_list_ports,
            station::serial_identify_devices,
            station::discover_devices,
            station::auto_bind_identified_devices,
            station::auto_bind_discovered_devices,
            station::connect_bound_devices,
            station::connect_single_device,
            station::disconnect_single_device,
            station::get_workbench_state,
            experiment_plan::load_experiment_plan,
            experiment_plan::set_experiment_plan_draft,
            experiment_plan::get_experiment_plan_draft,
            experiment_plan::get_device_preset_drafts,
            experiment_plan::set_device_preset_draft,
            experiment_plan::get_selected_default_packages,
            experiment_plan::set_selected_default_package,
            experiment_plan::capture_current_setup_as_preset_draft,
            experiment_plan::capture_current_setup_as_plan_draft,
            experiment_plan::project_experiment_plan,
            experiment_plan::resolve_plan_with_current_zero,
            experiment_plan::get_experiment_plan_run_readiness,
            experiment_plan::get_experiment_plan_run_status,
            experiment_plan::start_experiment_plan_run,
            experiment_plan::stop_experiment_plan_run,
            panels::smb100a::smb100a_get_status,
            panels::smb100a::smb100a_set_frequency,
            panels::smb100a::smb100a_set_power,
            panels::smb100a::smb100a_set_output,
            panels::smb100a::smb100a_set_fm,
            panels::smb100a::smb100a_set_lf,
            panels::smb100a::smb100a_apply_safe_config,
            panels::oe1022d::oe1022d_get_status,
            panels::oe1022d::oe1022d_set_filter,
            panels::oe1022d::oe1022d_set_reference,
            panels::oe1022d::oe1022d_apply_default_config,
            panels::oe1022d::oe1022d_auto_phase,
            panels::magnetic::magnetic_get_status,
            panels::magnetic::magnetic_get_xyz_package_status,
            panels::magnetic::import_magnetic_para_xml,
            panels::magnetic::magnetic_init_axis,
            panels::magnetic::magnetic_set_zero_bias,
            panels::magnetic::magnetic_set_recur_current,
            panels::magnetic::magnetic_set_recur_mag,
            panels::magnetic::magnetic_toggle_output,
            panels::magnetic::magnetic_toggle_lock_zero,
            panels::magnetic::magnetic_safe_cleanup,
            panels::magnetic::magnetic_init_all,
            panels::magnetic::magnetic_measure_zero_all,
            panels::magnetic::magnetic_lock_zero_all,
            panels::magnetic::magnetic_apply_vector_field,
            panels::magnetic::magnetic_cleanup_all,
            panels::laser::laser_get_status,
            panels::laser::laser_set_power,
            panels::laser::laser_set_enabled,
            panels::laser::laser_emergency_off,
        ])
        .setup(|app| {
            app.manage(workbench_state::WorkbenchState::default());
            let _window = app.get_webview_window("main").unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
