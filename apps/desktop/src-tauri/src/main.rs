// Prevents additional console window on Windows in release mode.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::fs;
use std::path::Path;

/// Returns static application metadata.
#[tauri::command]
fn app_metadata() -> serde_json::Value {
    serde_json::json!({
        "name": "ODMR GUI-M0 Mock Viewer",
        "version": "0.1.0",
        "phase": "M4.0 read-only analysis viewer",
        "mode": "READ ONLY",
        "boundary_note": "No hardware access. No executor connection. Real controls disabled."
    })
}

// ---------------------------------------------------------------------------
// M4.0 analysis data types — must match frontend types/analysis.ts exactly
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
struct RunOverlaySummaryData {
    schema_version: String,
    kind: String,
    frequency_count: i64,
    generated_at: String,
    frequencies: Vec<FrequencyGroup>,
}

#[derive(Debug, Clone, Serialize)]
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
    oe1022d_display_idn_by_run: serde_json::Value,
    generated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct QualityFlagsData {
    schema_version: String,
    kind: String,
    passed: bool,
    missing_artifact: bool,
    missing_artifact_details: serde_json::Value,
    failed_run: bool,
    failed_run_ids: Vec<String>,
    parse_failures: bool,
    parse_failure_count: i64,
    audit_mismatch: bool,
    audit_mismatch_run_ids: Vec<String>,
    unsafe_final_state: bool,
    unsafe_final_state_run_ids: Vec<String>,
    csv_present: bool,
    csv_present_details: serde_json::Value,
    magnetic_command_present: bool,
    magnetic_command_details: serde_json::Value,
    frequency_grid_mismatch: bool,
    empty_signal_series: bool,
    generated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportManifestFile {
    relative_path: String,
    sha256: String,
    size_bytes: i64,
}

#[derive(Debug, Clone, Serialize)]
struct ExportManifestData {
    schema_version: String,
    kind: String,
    source_run_ids: Vec<String>,
    generated_at: String,
    files: Vec<ExportManifestFile>,
}

#[derive(Debug, Clone, Serialize)]
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
        return Err(format!(
            "Required file not found: {}",
            qf_path.display()
        ));
    }
    let qf_text =
        fs::read_to_string(&qf_path).map_err(|e| format!("read {}: {}", qf_path.display(), e))?;
    let quality_flags: QualityFlagsData = serde_json::from_str(&qf_text)
        .map_err(|e| format!("parse {}: {}", qf_path.display(), e))?;

    // --- Required: odmr_like_analysis_summary.json ---
    let as_path = analysis_dir.join("odmr_like_analysis_summary.json");
    if !as_path.exists() {
        return Err(format!(
            "Required file not found: {}",
            as_path.display()
        ));
    }
    let as_text =
        fs::read_to_string(&as_path).map_err(|e| format!("read {}: {}", as_path.display(), e))?;
    let analysis_summary: AnalysisSummaryData = serde_json::from_str(&as_text)
        .map_err(|e| format!("parse {}: {}", as_path.display(), e))?;

    // --- Required: run_overlay_summary.json ---
    let ro_path = analysis_dir.join("run_overlay_summary.json");
    if !ro_path.exists() {
        return Err(format!(
            "Required file not found: {}",
            ro_path.display()
        ));
    }
    let ro_text =
        fs::read_to_string(&ro_path).map_err(|e| format!("read {}: {}", ro_path.display(), e))?;
    let run_overlay_summary: RunOverlaySummaryData = serde_json::from_str(&ro_text)
        .map_err(|e| format!("parse {}: {}", ro_path.display(), e))?;

    // --- Required: spectrum_points.jsonl ---
    let sp_path = analysis_dir.join("spectrum_points.jsonl");
    if !sp_path.exists() {
        return Err(format!(
            "Required file not found: {}",
            sp_path.display()
        ));
    }
    let sp_text =
        fs::read_to_string(&sp_path).map_err(|e| format!("read {}: {}", sp_path.display(), e))?;
    let spectrum_points: Vec<SpectrumPoint> = sp_text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str::<SpectrumPoint>(line).map_err(|e| {
                format!("parse {} line {}: {}", sp_path.display(), i + 1, e)
            })
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
                    warnings.push(format!(
                        "Failed to parse {}: {}",
                        em_path.display(),
                        e
                    ));
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
async fn pick_analysis_directory(
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let path = app
        .dialog()
        .file()
        .blocking_pick_folder();
    Ok(path.map(|p| p.to_string()))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            app_metadata,
            read_analysis_directory,
            pick_analysis_directory
        ])
        .setup(|app| {
            let _window = app.get_webview_window("main").unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
