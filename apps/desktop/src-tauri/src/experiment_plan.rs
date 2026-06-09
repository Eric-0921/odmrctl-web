//! Experiment plan bridge commands for the Device Workbench.
//!
//! Plan editing and run-launcher bridge commands.
//!
//! Loading/projecting/resolving commands do not send hardware commands and do
//! not mutate device output state. Run-launcher commands are explicit and keep
//! the hardware handoff visible through readiness/progress/artifacts.

use crate::workbench_state::{RuntimeZeroBaseline, WorkbenchState};
use odmr_config::load_station_config_str;
use odmr_executor::{
    run_hardware, HardwareLaserTarget, HardwareMagAxisTarget, HardwareOeAcquisition,
    HardwareProgress, HardwareRfSweep, HardwareRunConfig, HardwareRunStep, RunControl,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
pub struct ExperimentPlanSummary {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub station_ref: Option<String>,
    pub preset_refs: Value,
    pub field_point_count: usize,
    pub rf_point_count: usize,
    pub estimated_measurements: usize,
    pub require_zero_lock: bool,
    pub warnings: Vec<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedPlanPreview {
    pub kind: String,
    pub executable: bool,
    pub blocked_reasons: Vec<String>,
    pub zero_baseline: Option<RuntimeZeroBaseline>,
    pub magnetic_points: Vec<ResolvedMagneticPoint>,
    pub rf_point_count: usize,
    pub estimated_measurements: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedMagneticPoint {
    pub point_index: usize,
    pub b_target_nt: [f64; 3],
    pub computed_total_current_a: HashMap<String, f64>,
    pub recurrent_current_a: HashMap<String, f64>,
    pub zero_baseline_ref: Option<String>,
    pub coil_constant_source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExperimentPlanProjection {
    pub kind: String,
    pub panel_catalogs: Vec<DevicePanelCatalog>,
    pub default_packages: Vec<DeviceDefaultPackage>,
    pub step_rows: Vec<ExperimentStepProjection>,
    pub step_row_count: usize,
    pub preview_limit: usize,
    pub truncated: bool,
    pub magnetic_points: Vec<MagneticPointProjection>,
    pub smb100a_rf_points: Vec<Smb100aRfPointProjection>,
    pub laser_rows: Vec<LaserProjection>,
    pub oe1022d_rows: Vec<Oe1022dProjection>,
    pub combination_preview: Vec<CombinationPreviewRow>,
    pub estimated_measurements: usize,
    pub estimated_duration_s: Option<f64>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DevicePanelCatalog {
    pub device: String,
    pub panel_group: String,
    pub field_id: String,
    pub label_cn: String,
    pub unit: Option<String>,
    pub field_type: String,
    pub allowed_values: Vec<String>,
    pub display_values: Vec<CatalogDisplayValue>,
    pub unit_options: Vec<String>,
    pub default_unit: Option<String>,
    pub default_value: Option<Value>,
    pub default_value_si: Option<Value>,
    pub safe_value: Option<Value>,
    pub safe_value_si: Option<Value>,
    pub query_command: Option<String>,
    pub set_command: Option<String>,
    pub remote_code: Option<String>,
    pub ui_location: String,
    pub channel_scope: Option<String>,
    pub enabled_when: Option<String>,
    pub disabled_reason_cn: Option<String>,
    pub write_policy: String,
    pub json_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogDisplayValue {
    pub value: String,
    pub label_cn: String,
    pub status_color: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceDefaultPackage {
    pub device: String,
    pub package_id: String,
    pub label_cn: String,
    pub source: String,
    pub risk_level: String,
    pub values: Value,
    pub values_si: Value,
    pub note_cn: String,
    pub apply_target: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExperimentStepProjection {
    pub step_index: usize,
    pub step_id: String,
    pub group_id: Option<String>,
    pub bx_nt: f64,
    pub by_nt: f64,
    pub bz_nt: f64,
    pub rf_start_hz: Option<f64>,
    pub rf_stop_hz: Option<f64>,
    pub rf_step_hz: Option<f64>,
    pub smb100a_frequency_hz: Option<f64>,
    pub smb100a_power_dbm: Option<f64>,
    pub smb100a_fm_enabled: Option<bool>,
    pub smb100a_lf_frequency_hz: Option<f64>,
    pub smb100a_rf_sweep_summary: String,
    pub smb100a_sweep_output_start_v: Option<f64>,
    pub smb100a_sweep_output_stop_v: Option<f64>,
    pub laser_power_mw: Option<f64>,
    pub laser_enabled: Option<bool>,
    pub oe1022d_summary: String,
    pub oe_pre_start_ms: Option<f64>,
    pub oe_post_stop_ms: Option<f64>,
    pub oe_ch_a_time_constant_s: Option<f64>,
    pub oe_ch_a_filter_slope_db_oct: Option<f64>,
    pub oe_ch_a_dynamic_reserve: Option<String>,
    pub oe_ch_a_sensitivity: Option<String>,
    pub oe_ch_b_time_constant_s: Option<f64>,
    pub oe_ch_b_filter_slope_db_oct: Option<f64>,
    pub oe_ch_b_dynamic_reserve: Option<String>,
    pub oe_ch_b_sensitivity: Option<String>,
    pub dwell_ms: Option<f64>,
    pub estimated_duration_s: Option<f64>,
    pub executable: bool,
    pub blocked_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MagneticPointProjection {
    pub point_index: usize,
    pub group_id: Option<String>,
    pub bx_nt: f64,
    pub by_nt: f64,
    pub bz_nt: f64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Smb100aRfPointProjection {
    pub point_index: usize,
    pub frequency_hz: f64,
    pub power_dbm: Option<f64>,
    pub dwell_ms: Option<f64>,
    pub fm_enabled: Option<bool>,
    pub lf_frequency_hz: Option<f64>,
    pub modulation_on: Option<bool>,
    pub sweep_output_start_v: Option<f64>,
    pub sweep_output_stop_v: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LaserProjection {
    pub mode: String,
    pub power_mw: Option<f64>,
    pub enabled: Option<bool>,
    pub settle_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Oe1022dProjection {
    pub frames_per_point: Option<u64>,
    pub pre_discard_ms: Option<f64>,
    pub pre_start_ms: Option<f64>,
    pub post_stop_ms: Option<f64>,
    pub time_constant_s: Option<f64>,
    pub filter_slope_db_oct: Option<f64>,
    pub reference_source: Option<String>,
    pub acquisition: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct CombinationPreviewRow {
    pub row_index: usize,
    pub magnetic_point_index: usize,
    pub rf_point_index: usize,
    pub bx_nt: f64,
    pub by_nt: f64,
    pub bz_nt: f64,
    pub frequency_hz: f64,
    pub laser_mode: String,
    pub oe_frames_per_point: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExperimentRunReadiness {
    pub kind: String,
    pub ready_for_preview_execution: bool,
    pub ready_for_hardware_execution: bool,
    pub blocked_reasons: Vec<String>,
    pub hardware_blocked_reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub step_count: usize,
    pub rf_point_count: usize,
    pub estimated_measurements: usize,
    pub estimated_duration_s: Option<f64>,
    pub require_zero_lock: bool,
    pub zero_baseline_present: bool,
    pub connected_devices: Vec<String>,
    pub required_devices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentPlanRunStatus {
    pub kind: String,
    pub run_id: String,
    pub mode: String,
    pub state: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub run_directory: Option<String>,
    pub step_count: usize,
    pub rf_point_count: usize,
    pub estimated_measurements: usize,
    pub estimated_duration_s: Option<f64>,
    pub steps_completed: usize,
    pub current_step_index: Option<usize>,
    pub current_step_id: Option<String>,
    pub current_b_nt: Option<[f64; 3]>,
    pub current_phase: Option<String>,
    pub smb_sweep_running: bool,
    pub oe_frames_captured: u64,
    pub cleanup_state: Option<String>,
    pub recent_error: Option<String>,
    pub blocked_reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub artifact_paths: HashMap<String, String>,
}

fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}

fn str_field(json: &Value, key: &str, default_value: &str) -> String {
    json.get(key)
        .and_then(Value::as_str)
        .unwrap_or(default_value)
        .to_string()
}

fn axis_values(axis: Option<&Value>) -> Vec<f64> {
    let Some(axis) = axis else {
        return vec![0.0];
    };
    if let Some(v) = axis.get("value").and_then(Value::as_f64) {
        return vec![v];
    }
    if let Some(values) = axis.get("values").and_then(Value::as_array) {
        let out: Vec<f64> = values.iter().filter_map(Value::as_f64).collect();
        if !out.is_empty() {
            return out;
        }
    }
    vec![0.0]
}

fn magnetic_points_from_system_scan(plan: &Value) -> Vec<[f64; 3]> {
    let mut points = Vec::new();
    let sweeps = plan
        .get("sweeps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for sweep in sweeps {
        if sweep.get("device").and_then(Value::as_str) != Some("magnetic") {
            continue;
        }
        let Some(axes) = sweep.get("axes") else {
            continue;
        };
        let xs = axis_values(axes.get("bx_nt"));
        let ys = axis_values(axes.get("by_nt"));
        let zs = axis_values(axes.get("bz_nt"));
        for x in &xs {
            for y in &ys {
                for z in &zs {
                    points.push([*x, *y, *z]);
                }
            }
        }
    }

    if points.is_empty() {
        points.push([0.0, 0.0, 0.0]);
    }
    points
}

#[derive(Debug, Clone)]
struct FieldPointEntry {
    point: [f64; 3],
    group_id: Option<String>,
    source: String,
}

fn axis_index(axis: &str) -> Option<usize> {
    match axis {
        "x" | "bx" | "Bx" | "BX" => Some(0),
        "y" | "by" | "By" | "BY" => Some(1),
        "z" | "bz" | "Bz" | "BZ" => Some(2),
        _ => None,
    }
}

fn read_fixed_axis(fixed: Option<&Value>, axis: &str) -> f64 {
    let Some(fixed) = fixed else {
        return 0.0;
    };
    let lower_key = format!("b{axis}_nt");
    let upper_key = format!("B{axis}");
    fixed
        .get(axis)
        .or_else(|| fixed.get(lower_key.as_str()))
        .or_else(|| fixed.get(upper_key.as_str()))
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

fn axis_key(index: usize) -> &'static str {
    match index {
        0 => "x",
        1 => "y",
        _ => "z",
    }
}

fn range_values(range: &Value) -> Vec<f64> {
    let start = range
        .get("start")
        .or_else(|| range.get("start_nt"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let stop = range
        .get("stop")
        .or_else(|| range.get("stop_nt"))
        .and_then(Value::as_f64)
        .unwrap_or(start);
    let step = range
        .get("step")
        .or_else(|| range.get("step_nt"))
        .and_then(Value::as_f64)
        .unwrap_or(1.0)
        .abs();
    if step <= 0.0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut value = start;
    let ascending = stop >= start;
    loop {
        out.push(value);
        if ascending {
            value += step;
            if value > stop + f64::EPSILON {
                break;
            }
        } else {
            value -= step;
            if value < stop - f64::EPSILON {
                break;
            }
        }
    }
    out
}

fn axis_range_values(group: &Value, axis: usize) -> Vec<f64> {
    let key = axis_key(axis);
    let legacy_range = group.get("range_nt").unwrap_or(&Value::Null);
    let range = group
        .get("axis_ranges_nt")
        .and_then(|ranges| ranges.get(key))
        .or_else(|| {
            let lower_key = format!("b{key}_nt");
            group
                .get("axis_ranges_nt")
                .and_then(|ranges| ranges.get(lower_key.as_str()))
        })
        .unwrap_or(legacy_range);
    range_values(range)
}

fn grouped_scan_entries(field_space: &Value) -> Vec<FieldPointEntry> {
    let Some(groups) = field_space.get("groups").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut ordered: Vec<&Value> = groups.iter().collect();
    ordered.sort_by_key(|group| group.get("order").and_then(Value::as_i64).unwrap_or(0));

    let mut out = Vec::new();
    for group in ordered {
        if !group
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true)
        {
            continue;
        }
        let group_id = group
            .get("group_id")
            .and_then(Value::as_str)
            .unwrap_or("group")
            .to_string();
        let axes: Vec<usize> = group
            .get("axes")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .filter_map(axis_index)
                    .collect()
            })
            .unwrap_or_default();
        if axes.is_empty() {
            continue;
        }
        let fixed = group.get("fixed_axes_nt");
        let mut axis_values = [
            vec![read_fixed_axis(fixed, "x")],
            vec![read_fixed_axis(fixed, "y")],
            vec![read_fixed_axis(fixed, "z")],
        ];
        for axis in &axes {
            let values = axis_range_values(group, *axis);
            if values.is_empty() {
                continue;
            }
            axis_values[*axis] = values;
        }
        for bx in &axis_values[0] {
            for by in &axis_values[1] {
                for bz in &axis_values[2] {
                    out.push(FieldPointEntry {
                        point: [*bx, *by, *bz],
                        group_id: Some(group_id.clone()),
                        source: "field_space.groups".into(),
                    });
                }
            }
        }
    }
    out
}

fn magnetic_field_entries_from_plan(plan: &Value) -> Vec<FieldPointEntry> {
    let Some(field_space) = plan.get("field_space") else {
        return magnetic_points_from_system_scan(plan)
            .into_iter()
            .map(|point| FieldPointEntry {
                point,
                group_id: None,
                source: "system_scan".into(),
            })
            .collect();
    };

    if matches!(
        field_space.get("mode").and_then(Value::as_str),
        Some("grouped_grid_scan") | Some("grouped_path_scan")
    ) {
        let out = grouped_scan_entries(field_space);
        if !out.is_empty() {
            return out;
        }
    }

    if let Some(points) = field_space.get("points").and_then(Value::as_array) {
        let out: Vec<FieldPointEntry> = points
            .iter()
            .filter_map(|point| {
                let arr = point.as_array()?;
                Some(FieldPointEntry {
                    point: [
                        arr.first()?.as_f64()?,
                        arr.get(1)?.as_f64()?,
                        arr.get(2)?.as_f64()?,
                    ],
                    group_id: None,
                    source: "field_space.points".into(),
                })
            })
            .collect();
        if !out.is_empty() {
            return out;
        }
    }

    if let Some(subspaces) = field_space.get("subspaces").and_then(Value::as_array) {
        let mut out = Vec::new();
        for space in subspaces {
            match space.get("kind").and_then(Value::as_str) {
                Some("axis_line") => {
                    let axis = space.get("axis").and_then(Value::as_str).unwrap_or("x");
                    let start = space.get("start").and_then(Value::as_f64).unwrap_or(0.0);
                    let stop = space.get("stop").and_then(Value::as_f64).unwrap_or(start);
                    let step = space
                        .get("step")
                        .and_then(Value::as_f64)
                        .unwrap_or(1.0)
                        .abs();
                    let mut v = start;
                    while v <= stop + f64::EPSILON {
                        let mut point = [0.0, 0.0, 0.0];
                        match axis {
                            "x" => point[0] = v,
                            "y" => point[1] = v,
                            "z" => point[2] = v,
                            _ => {}
                        }
                        out.push(FieldPointEntry {
                            point,
                            group_id: space
                                .get("group_id")
                                .and_then(Value::as_str)
                                .map(ToString::to_string),
                            source: "field_space.subspaces".into(),
                        });
                        v += step;
                    }
                }
                Some("vector") => {
                    let point = [
                        space.get("bx_nt").and_then(Value::as_f64).unwrap_or(0.0),
                        space.get("by_nt").and_then(Value::as_f64).unwrap_or(0.0),
                        space.get("bz_nt").and_then(Value::as_f64).unwrap_or(0.0),
                    ];
                    out.push(FieldPointEntry {
                        point,
                        group_id: space
                            .get("group_id")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                        source: "field_space.subspaces".into(),
                    });
                }
                _ => {}
            }
        }
        if !out.is_empty() {
            return out;
        }
    }

    vec![FieldPointEntry {
        point: [0.0, 0.0, 0.0],
        group_id: None,
        source: "default".into(),
    }]
}

fn magnetic_points_from_field_space(plan: &Value) -> Vec<[f64; 3]> {
    magnetic_field_entries_from_plan(plan)
        .into_iter()
        .map(|entry| entry.point)
        .collect()
}

fn rf_point_count(plan: &Value) -> usize {
    if let Some(rf) = rf_template_from_plan(plan) {
        let count = rf_count_from_template(rf);
        if count > 1 {
            return count;
        }
    }

    plan.get("sweeps")
        .and_then(Value::as_array)
        .and_then(|sweeps| {
            sweeps.iter().find_map(|sweep| {
                let axis = sweep.get("axis").and_then(Value::as_str)?;
                if !axis.contains("frequency_hz") {
                    return None;
                }
                Some(sweep.get("values")?.as_array()?.len())
            })
        })
        .unwrap_or(1)
}

fn rf_template_from_plan(plan: &Value) -> Option<&Value> {
    plan.pointer("/spectrum_template/rf_sweep")
        .or_else(|| plan.pointer("/spectrum_template/rf"))
}

fn merge_json_values(base: &Value, override_value: Option<&Value>) -> Value {
    let Some(override_value) = override_value else {
        return base.clone();
    };
    match (base, override_value) {
        (Value::Object(base_obj), Value::Object(override_obj)) => {
            let mut out = base_obj.clone();
            for (key, value) in override_obj {
                let merged = out
                    .get(key)
                    .map(|base_child| merge_json_values(base_child, Some(value)))
                    .unwrap_or_else(|| value.clone());
                out.insert(key.clone(), merged);
            }
            Value::Object(out)
        }
        (_, value) => value.clone(),
    }
}

fn group_override<'a>(plan: &'a Value, group_id: Option<&str>, key: &str) -> Option<&'a Value> {
    let group_id = group_id?;
    plan.get("group_overrides")?.get(group_id)?.get(key)
}

fn rf_template_for_group(plan: &Value, group_id: Option<&str>) -> Value {
    merge_json_values(
        rf_template_from_plan(plan).unwrap_or(&Value::Null),
        group_override(plan, group_id, "rf_sweep").or_else(|| group_override(plan, group_id, "rf")),
    )
}

fn laser_template_for_group(plan: &Value, group_id: Option<&str>) -> Value {
    merge_json_values(
        plan.pointer("/spectrum_template/laser")
            .unwrap_or(&Value::Null),
        group_override(plan, group_id, "laser"),
    )
}

fn oe_template_for_group(plan: &Value, group_id: Option<&str>) -> Value {
    merge_json_values(
        plan.pointer("/spectrum_template/oe1022d_acquisition")
            .or_else(|| plan.pointer("/spectrum_template/oe1022d"))
            .unwrap_or(&Value::Null),
        group_override(plan, group_id, "oe1022d_acquisition")
            .or_else(|| group_override(plan, group_id, "oe1022d")),
    )
}

fn rf_count_from_template(rf: &Value) -> usize {
    let start = rf.get("start_hz").and_then(Value::as_f64);
    let stop = rf.get("stop_hz").and_then(Value::as_f64);
    let step = rf.get("step_hz").and_then(Value::as_f64);
    if let (Some(start), Some(stop), Some(step)) = (start, stop, step) {
        if step > 0.0 && stop >= start {
            return (((stop - start) / step).floor() as usize) + 1;
        }
    }
    1
}

fn rf_points_from_template(rf: &Value) -> Vec<Smb100aRfPointProjection> {
    let start = rf.get("start_hz").and_then(Value::as_f64);
    let stop = rf.get("stop_hz").and_then(Value::as_f64);
    let step = rf.get("step_hz").and_then(Value::as_f64);
    let power_dbm = rf.get("power_dbm").and_then(Value::as_f64);
    let dwell_ms = rf.get("dwell_ms").and_then(Value::as_f64);
    let fm_enabled = rf.get("fm_enabled").and_then(Value::as_bool);
    let lf_frequency_hz = rf.get("lf_frequency_hz").and_then(Value::as_f64);
    let modulation_on = rf.get("modulation_on").and_then(Value::as_bool);
    let sweep_output_start_v = rf.get("sweep_output_start_v").and_then(Value::as_f64);
    let sweep_output_stop_v = rf.get("sweep_output_stop_v").and_then(Value::as_f64);
    if let (Some(start), Some(stop), Some(step)) = (start, stop, step) {
        if step > 0.0 && stop >= start {
            let mut out = Vec::new();
            let mut frequency = start;
            while frequency <= stop + f64::EPSILON && out.len() < 200 {
                out.push(Smb100aRfPointProjection {
                    point_index: out.len(),
                    frequency_hz: frequency,
                    power_dbm,
                    dwell_ms,
                    fm_enabled,
                    lf_frequency_hz,
                    modulation_on,
                    sweep_output_start_v,
                    sweep_output_stop_v,
                });
                frequency += step;
            }
            if !out.is_empty() {
                return out;
            }
        }
    }
    vec![Smb100aRfPointProjection {
        point_index: 0,
        frequency_hz: 0.0,
        power_dbm,
        dwell_ms,
        fm_enabled,
        lf_frequency_hz,
        modulation_on,
        sweep_output_start_v,
        sweep_output_stop_v,
    }]
}

fn rf_points_from_plan(plan: &Value) -> Vec<Smb100aRfPointProjection> {
    if let Some(template) = plan.get("spectrum_template") {
        if let Some(rf) = rf_template_from_plan(plan) {
            let start = rf.get("start_hz").and_then(Value::as_f64);
            let stop = rf.get("stop_hz").and_then(Value::as_f64);
            let step = rf.get("step_hz").and_then(Value::as_f64);
            let power_dbm = rf
                .get("power_dbm")
                .and_then(Value::as_f64)
                .or_else(|| template.get("power_dbm").and_then(Value::as_f64));
            let dwell_ms = rf.get("dwell_ms").and_then(Value::as_f64);
            let fm_enabled = rf.get("fm_enabled").and_then(Value::as_bool);
            let lf_frequency_hz = rf.get("lf_frequency_hz").and_then(Value::as_f64);
            let modulation_on = rf.get("modulation_on").and_then(Value::as_bool);
            let sweep_output_start_v = rf.get("sweep_output_start_v").and_then(Value::as_f64);
            let sweep_output_stop_v = rf.get("sweep_output_stop_v").and_then(Value::as_f64);
            if let (Some(start), Some(stop), Some(step)) = (start, stop, step) {
                if step > 0.0 && stop >= start {
                    let mut out = Vec::new();
                    let mut frequency = start;
                    while frequency <= stop + f64::EPSILON && out.len() < 200 {
                        out.push(Smb100aRfPointProjection {
                            point_index: out.len(),
                            frequency_hz: frequency,
                            power_dbm,
                            dwell_ms,
                            fm_enabled,
                            lf_frequency_hz,
                            modulation_on,
                            sweep_output_start_v,
                            sweep_output_stop_v,
                        });
                        frequency += step;
                    }
                    return out;
                }
            }
        }
    }

    if let Some(values) = plan
        .get("sweeps")
        .and_then(Value::as_array)
        .and_then(|sweeps| {
            sweeps.iter().find_map(|sweep| {
                let axis = sweep.get("axis").and_then(Value::as_str)?;
                if !axis.contains("frequency_hz") {
                    return None;
                }
                sweep.get("values")?.as_array()
            })
        })
    {
        let out: Vec<Smb100aRfPointProjection> = values
            .iter()
            .take(200)
            .enumerate()
            .filter_map(|(idx, value)| {
                Some(Smb100aRfPointProjection {
                    point_index: idx,
                    frequency_hz: value.as_f64()?,
                    power_dbm: None,
                    dwell_ms: None,
                    fm_enabled: None,
                    lf_frequency_hz: None,
                    modulation_on: None,
                    sweep_output_start_v: None,
                    sweep_output_stop_v: None,
                })
            })
            .collect();
        if !out.is_empty() {
            return out;
        }
    }

    vec![Smb100aRfPointProjection {
        point_index: 0,
        frequency_hz: 0.0,
        power_dbm: None,
        dwell_ms: None,
        fm_enabled: None,
        lf_frequency_hz: None,
        modulation_on: None,
        sweep_output_start_v: None,
        sweep_output_stop_v: None,
    }]
}

fn laser_rows_from_plan(plan: &Value) -> Vec<LaserProjection> {
    let laser = plan
        .pointer("/spectrum_template/laser")
        .or_else(|| plan.pointer("/fixed_params/laser"));
    let Some(laser) = laser else {
        return vec![LaserProjection {
            mode: "fixed_power".into(),
            power_mw: None,
            enabled: None,
            settle_ms: None,
        }];
    };
    vec![LaserProjection {
        mode: laser
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("fixed_power")
            .to_string(),
        power_mw: laser
            .get("power_mw")
            .and_then(Value::as_f64)
            .or_else(|| laser.get("power_setpoint_mw").and_then(Value::as_f64)),
        enabled: laser.get("enabled").and_then(Value::as_bool),
        settle_ms: laser.get("settle_ms").and_then(Value::as_f64),
    }]
}

#[allow(clippy::too_many_arguments)]
fn catalog_field(
    device: &str,
    panel_group: &str,
    field_id: &str,
    label_cn: &str,
    unit: Option<&str>,
    field_type: &str,
    allowed_values: &[&str],
    safe_value: Option<Value>,
    query_command: Option<&str>,
    set_command: Option<&str>,
) -> DevicePanelCatalog {
    let safe_value_si = safe_value.clone();
    DevicePanelCatalog {
        device: device.to_string(),
        panel_group: panel_group.to_string(),
        field_id: field_id.to_string(),
        label_cn: label_cn.to_string(),
        unit: unit.map(ToString::to_string),
        field_type: field_type.to_string(),
        allowed_values: allowed_values.iter().map(|v| (*v).to_string()).collect(),
        display_values: allowed_values
            .iter()
            .map(|v| CatalogDisplayValue {
                value: (*v).to_string(),
                label_cn: (*v).to_string(),
                status_color: None,
            })
            .collect(),
        unit_options: unit.map(|u| vec![u.to_string()]).unwrap_or_default(),
        default_unit: unit.map(ToString::to_string),
        default_value: safe_value.clone(),
        default_value_si: safe_value.clone(),
        safe_value,
        safe_value_si,
        query_command: query_command.map(ToString::to_string),
        set_command: set_command.map(ToString::to_string),
        remote_code: set_command.or(query_command).map(ToString::to_string),
        ui_location: "experiment_plan".to_string(),
        channel_scope: None,
        enabled_when: None,
        disabled_reason_cn: None,
        write_policy: "draft_only".to_string(),
        json_path: format!("device_presets.{device}.{field_id}"),
    }
}

fn option(value: &str, label_cn: &str, status_color: Option<&str>) -> CatalogDisplayValue {
    CatalogDisplayValue {
        value: value.to_string(),
        label_cn: label_cn.to_string(),
        status_color: status_color.map(ToString::to_string),
    }
}

fn enrich_catalog_field(field: &mut DevicePanelCatalog) {
    field.remote_code = field
        .set_command
        .clone()
        .or_else(|| field.query_command.clone());

    if field.device == "smb100a" {
        field.ui_location = match field.field_id.as_str() {
            "frequency_hz"
            | "power_dbm"
            | "rf_sweep_start_hz"
            | "rf_sweep_stop_hz"
            | "rf_sweep_step_hz"
            | "rf_sweep_dwell_s"
            | "rf_sweep_output_start_v"
            | "rf_sweep_output_stop_v" => "experiment_step".to_string(),
            _ => "device_workbench".to_string(),
        };
    }
    if field.device == "oe1022d" {
        field.ui_location = match field.field_id.as_str() {
            "time_constant_s" | "filter_slope_db_oct" | "dynamic_reserve" | "sensitivity" => {
                "experiment_step"
            }
            _ => "device_workbench",
        }
        .to_string();
    }

    match field.field_id.as_str() {
        "frequency_hz" | "lf_frequency_hz" | "fm_deviation_hz" | "lf_sweep_start_hz"
        | "lf_sweep_stop_hz" | "lf_sweep_center_hz" | "lf_sweep_span_hz" | "rf_sweep_start_hz"
        | "rf_sweep_stop_hz" | "rf_sweep_step_hz" | "rf_sweep_span_hz" => {
            field.unit_options = vec!["Hz", "kHz", "MHz", "GHz"]
                .into_iter()
                .map(str::to_string)
                .collect();
            field.default_unit = Some("Hz".to_string());
        }
        "rf_sweep_dwell_s" | "time_constant_s" => {
            field.unit_options = vec!["s", "ms"].into_iter().map(str::to_string).collect();
        }
        "lf_voltage_v" | "rf_sweep_output_start_v" | "rf_sweep_output_stop_v" => {
            field.unit_options = vec!["V", "mV"].into_iter().map(str::to_string).collect();
        }
        _ => {}
    }

    if field.allowed_values.iter().any(|v| v == "ON" || v == "OFF") {
        field.display_values = vec![
            option("ON", "开启 (ON)", Some("on")),
            option("OFF", "关闭 (OFF)", Some("off")),
        ];
    }

    match field.field_id.as_str() {
        "fm_source" => {
            field.allowed_values = vec!["INTernal", "EXTernal", "INT,EXT"]
                .into_iter()
                .map(str::to_string)
                .collect();
            field.display_values = vec![
                option("INTernal", "内部 (INTernal)", None),
                option("EXTernal", "外部 (EXTernal)", None),
                option("INT,EXT", "内部+外部 (INT,EXT)", None),
            ];
        }
        "fm_mode" => {
            field.allowed_values = vec!["HDEViation", "NORMal", "LNOise"]
                .into_iter()
                .map(str::to_string)
                .collect();
            field.display_values = vec![
                option("HDEViation", "高偏差 (HDEViation)", None),
                option("NORMal", "正常 (NORMal)", None),
                option("LNOise", "低噪声 (LNOise)", None),
            ];
        }
        "lf_shape" => {
            field.allowed_values = vec!["SINE", "SQUare", "SAWtooth"]
                .into_iter()
                .map(str::to_string)
                .collect();
            field.display_values = vec![
                option("SINE", "正弦 (SINE)", None),
                option("SQUare", "方波 (SQUare)", None),
                option("SAWtooth", "锯齿 (SAWtooth)", None),
            ];
        }
        "rf_sweep_spacing" => {
            field.display_values = vec![
                option("LINear", "线性 (LINear)", None),
                option("LOGarithmic", "对数 (LOGarithmic)", None),
            ];
        }
        "rf_sweep_shape" => {
            field.display_values = vec![
                option("SAWtooth", "锯齿 (SAWtooth)", None),
                option("TRIangle", "三角 (TRIangle)", None),
            ];
        }
        "dynamic_reserve" => {
            field.display_values = vec![
                option("HIGH", "高储备 (HIGH)", None),
                option("NORMAL", "正常 (NORMAL)", Some("pass")),
                option("LOW", "低噪声 (LOW)", None),
            ];
        }
        "sensitivity" => {
            field.allowed_values = vec!["1 V/nA", "300 mV/nA", "100 mV/nA", "30 mV/nA", "10 mV/nA"]
                .into_iter()
                .map(str::to_string)
                .collect();
            field.display_values = field
                .allowed_values
                .iter()
                .map(|v| option(v, v, None))
                .collect();
        }
        "filter_slope_db_oct" => {
            field.field_type = "enum".to_string();
            field.allowed_values = vec!["6", "12", "18", "24"]
                .into_iter()
                .map(str::to_string)
                .collect();
            field.display_values = vec![
                option("6", "6 dB/oct", None),
                option("12", "12 dB/oct", Some("pass")),
                option("18", "18 dB/oct", None),
                option("24", "24 dB/oct", None),
            ];
        }
        "reference_source" => {
            field.display_values = vec![
                option("INT", "内部参考 (INT)", None),
                option("EXT", "外部参考 (EXT)", None),
                option("SWEEP", "内部扫频参考 (Internal Sweep)", None),
            ];
        }
        "sineout_start_voltage_vrms"
        | "sineout_stop_voltage_vrms"
        | "sineout_sweep_step"
        | "sineout_step_time_ms"
        | "sineout_run_mode"
        | "dc_output_voltage_v" => {
            field.enabled_when = Some("SineOut Mode != 固定幅值模式".to_string());
            field.disabled_reason_cn =
                Some("SineOut Mode = 固定幅值模式时，此扫幅/DC 参数不生效。".to_string());
        }
        "internal_frequency_hz"
        | "int_sweep_type"
        | "int_sweep_run_mode"
        | "int_sweep_start_hz"
        | "int_sweep_step"
        | "int_sweep_stop_hz"
        | "int_sweep_step_time_ms" => {
            field.enabled_when = Some("Reference Source = Internal Sweep".to_string());
            field.disabled_reason_cn =
                Some("当前参考源不是内部扫频参考时，此字段在旧 GUI 中灰显。".to_string());
        }
        "external_ref_trigger" => {
            field.enabled_when = Some("Reference Source = External".to_string());
            field.disabled_reason_cn =
                Some("参考源不是外部参考时，外部触发方式不适用。".to_string());
        }
        "auxout_vdc" => {
            field.enabled_when = Some("Channel output source = AUXOUT".to_string());
            field.disabled_reason_cn =
                Some("通道输出源不是 AUXOUT 时，AUXOUT 电压字段灰显。".to_string());
        }
        _ => {}
    }
}

fn device_panel_catalogs() -> Vec<DevicePanelCatalog> {
    let mut out = Vec::new();

    for (field_id, label, unit, ty, allowed, safe, query, set) in [
        (
            "frequency_hz",
            "频率",
            Some("Hz"),
            "number",
            &[][..],
            Some(serde_json::json!(1_000_000.0)),
            Some("FREQ?"),
            Some("FREQ"),
        ),
        (
            "power_dbm",
            "电平",
            Some("dBm"),
            "number",
            &[][..],
            Some(serde_json::json!(-30.0)),
            Some("POW?"),
            Some("POW"),
        ),
        (
            "rf_output",
            "RF 输出开关",
            None,
            "enum",
            &["ON", "OFF"][..],
            Some(serde_json::json!("OFF")),
            Some("OUTP?"),
            Some("OUTP"),
        ),
        (
            "mod_state",
            "调制总开关",
            None,
            "enum",
            &["ON", "OFF"][..],
            Some(serde_json::json!("OFF")),
            Some("MOD:STAT?"),
            Some("MOD:STAT"),
        ),
        (
            "alc_mode",
            "ALC 模式",
            None,
            "enum",
            &["AUTO", "ON", "OFF", "SAMPLE", "TABLE", "EXTERNAL"][..],
            Some(serde_json::json!("AUTO")),
            Some("POW:ALC?"),
            Some("POW:ALC"),
        ),
        (
            "lf_output",
            "LF 输出开关",
            None,
            "enum",
            &["ON", "OFF"][..],
            Some(serde_json::json!("OFF")),
            Some("LFO:STAT?"),
            Some("LFO:STAT"),
        ),
        (
            "lf_voltage_v",
            "LF 输出电压",
            Some("V"),
            "number",
            &[][..],
            Some(serde_json::json!(0.0)),
            Some("LFO:VOLT?"),
            Some("LFO:VOLT"),
        ),
        (
            "lf_frequency_hz",
            "LF 发生器频率",
            Some("Hz"),
            "number",
            &[][..],
            Some(serde_json::json!(500.0)),
            Some("LFO:FREQ?"),
            Some("LFO:FREQ"),
        ),
        (
            "lf_shape",
            "LF 波形",
            None,
            "enum",
            &["SINE", "SQUARE", "TRIANGLE", "RAMP"],
            Some(serde_json::json!("SINE")),
            Some("LFO:SHAP?"),
            Some("LFO:SHAP"),
        ),
        (
            "lf_impedance",
            "源阻抗",
            None,
            "enum",
            &["LOW", "50OHM"],
            Some(serde_json::json!("LOW")),
            Some("LFO:IMP?"),
            Some("LFO:IMP"),
        ),
        (
            "fm_enabled",
            "FM 调制开关",
            None,
            "enum",
            &["ON", "OFF"],
            Some(serde_json::json!("OFF")),
            Some("FM:STAT?"),
            Some("FM:STAT"),
        ),
        (
            "fm_source",
            "FM 调制源",
            None,
            "enum",
            &["INT", "EXT"],
            Some(serde_json::json!("INT")),
            Some("FM:SOUR?"),
            Some("FM:SOUR"),
        ),
        (
            "fm_mode",
            "FM 调制模式",
            None,
            "enum",
            &["NORM", "LOWN"],
            Some(serde_json::json!("NORM")),
            Some("FM:MODE?"),
            Some("FM:MODE"),
        ),
        (
            "fm_deviation_hz",
            "FM 偏差",
            Some("Hz"),
            "number",
            &[][..],
            Some(serde_json::json!(1_000.0)),
            Some("FM:DEV?"),
            Some("FM:DEV"),
        ),
        (
            "lf_sweep_enabled",
            "LF 扫频开关",
            None,
            "enum",
            &["ON", "OFF"],
            Some(serde_json::json!("OFF")),
            Some("LFO:SWE:STAT?"),
            Some("LFO:SWE:STAT"),
        ),
        (
            "lf_sweep_mode",
            "LF 扫频模式",
            None,
            "enum",
            &["AUTO", "MAN"],
            Some(serde_json::json!("AUTO")),
            Some("LFO:SWE:MODE?"),
            Some("LFO:SWE:MODE"),
        ),
        (
            "lf_sweep_start_hz",
            "LF 扫频起始频率",
            Some("Hz"),
            "number",
            &[][..],
            None,
            Some("LFO:SWE:STAR?"),
            Some("LFO:SWE:STAR"),
        ),
        (
            "lf_sweep_stop_hz",
            "LF 扫频截止频率",
            Some("Hz"),
            "number",
            &[][..],
            None,
            Some("LFO:SWE:STOP?"),
            Some("LFO:SWE:STOP"),
        ),
        (
            "lf_sweep_center_hz",
            "LF 扫频中心频率",
            Some("Hz"),
            "number",
            &[][..],
            None,
            Some("LFO:SWE:CENT?"),
            Some("LFO:SWE:CENT"),
        ),
        (
            "lf_sweep_span_hz",
            "LF 扫频宽度",
            Some("Hz"),
            "number",
            &[][..],
            None,
            Some("LFO:SWE:SPAN?"),
            Some("LFO:SWE:SPAN"),
        ),
        (
            "rf_sweep_start_hz",
            "RF 扫频起始频率",
            Some("Hz"),
            "number",
            &[][..],
            Some(serde_json::json!(2_800_000_000.0)),
            Some("FREQ:STAR?"),
            Some("FREQ:STAR"),
        ),
        (
            "rf_sweep_stop_hz",
            "RF 扫频截止频率",
            Some("Hz"),
            "number",
            &[][..],
            Some(serde_json::json!(2_900_000_000.0)),
            Some("FREQ:STOP?"),
            Some("FREQ:STOP"),
        ),
        (
            "rf_sweep_span_hz",
            "RF 扫频宽度",
            Some("Hz"),
            "number",
            &[][..],
            Some(serde_json::json!(80_000_000.0)),
            Some("FREQ:SPAN?"),
            Some("FREQ:SPAN"),
        ),
        (
            "rf_sweep_spacing",
            "RF 扫频步进间距模式",
            None,
            "enum",
            &["LINear", "LOGarithmic"],
            Some(serde_json::json!("LINear")),
            Some("SWE:SPAC?"),
            Some("SWE:SPAC"),
        ),
        (
            "rf_sweep_shape",
            "RF 扫频形状",
            None,
            "enum",
            &["SAWtooth", "TRIangle"],
            Some(serde_json::json!("SAWtooth")),
            Some("SWE:SHAP?"),
            Some("SWE:SHAP"),
        ),
        (
            "rf_sweep_step_hz",
            "RF 线性步进值",
            Some("Hz"),
            "number",
            &[][..],
            Some(serde_json::json!(500_000.0)),
            Some("SWE:STEP?"),
            Some("SWE:STEP"),
        ),
        (
            "rf_sweep_dwell_s",
            "RF 驻留时间",
            Some("s"),
            "number",
            &[][..],
            Some(serde_json::json!(0.5)),
            Some("SWE:DWEL?"),
            Some("SWE:DWEL"),
        ),
        (
            "rf_sweep_lf_output",
            "使用 LF 输出扫频电压",
            None,
            "enum",
            &["ON", "OFF"],
            Some(serde_json::json!("OFF")),
            Some("SWE:OUTP?"),
            Some("SWE:OUTP"),
        ),
        (
            "rf_sweep_output_start_v",
            "扫频起始输出电压",
            Some("V"),
            "number",
            &[][..],
            Some(serde_json::json!(0.0)),
            Some("SWE:OUTP:VOLT:STAR?"),
            Some("SWE:OUTP:VOLT:STAR"),
        ),
        (
            "rf_sweep_output_stop_v",
            "扫频截止输出电压",
            Some("V"),
            "number",
            &[][..],
            Some(serde_json::json!(3.0)),
            Some("SWE:OUTP:VOLT:STOP?"),
            Some("SWE:OUTP:VOLT:STOP"),
        ),
    ] {
        let group = if field_id.starts_with("lf_sweep") {
            "LF sweep"
        } else if field_id.starts_with("rf_sweep") {
            "RF frequency sweep"
        } else if field_id.starts_with("lf_") {
            "LF 输出"
        } else if field_id.starts_with("fm_") {
            "FM 设置"
        } else {
            "主设置"
        };
        out.push(catalog_field(
            "smb100a", group, field_id, label, unit, ty, allowed, safe, query, set,
        ));
    }

    for (field_id, group, label, unit, ty, allowed, query, set) in [
        (
            "input_source",
            "输入信号的软件配置",
            "Input Source",
            None,
            "enum",
            &["A", "B", "A-B"][..],
            Some("ISRCD?"),
            Some("ISRCD"),
        ),
        (
            "input_shield_grounding",
            "输入信号的软件配置",
            "Input Shield Grounding",
            None,
            "enum",
            &["FLOAT", "GROUND"],
            Some("IGNDD?"),
            Some("IGNDD"),
        ),
        (
            "input_coupling",
            "输入信号的软件配置",
            "Input Coupling",
            None,
            "enum",
            &["AC", "DC"],
            Some("ICPLD?"),
            Some("ICPLD"),
        ),
        (
            "input_notch_filter",
            "输入信号的软件配置",
            "Input Notch Filter",
            None,
            "enum",
            &["OFF", "LINE", "2LINE"],
            Some("ILIND?"),
            Some("ILIND"),
        ),
        (
            "dynamic_reserve",
            "动态储备和灵敏度配置",
            "Dynamic Reserve",
            None,
            "enum",
            &["HIGH", "NORMAL", "LOW"],
            Some("RMODD?"),
            Some("RMODD"),
        ),
        (
            "sensitivity",
            "动态储备和灵敏度配置",
            "Sensitivity",
            None,
            "enum",
            &[][..],
            Some("SENSD?"),
            Some("SENSD"),
        ),
        (
            "auto_sensitivity",
            "动态储备和灵敏度配置",
            "Auto Sensitivity",
            None,
            "action",
            &[][..],
            None,
            Some("AGAND"),
        ),
        (
            "auto_reserve",
            "动态储备和灵敏度配置",
            "Auto Reserve",
            None,
            "action",
            &[][..],
            None,
            Some("ARSVD"),
        ),
        (
            "time_constant_s",
            "滤波器配置",
            "Time Constant",
            Some("s"),
            "number",
            &[][..],
            Some("OFLTD?"),
            Some("OFLTD"),
        ),
        (
            "sync_filter",
            "滤波器配置",
            "Sync Filter",
            None,
            "enum",
            &["ON", "OFF"],
            Some("SYNCD?"),
            Some("SYNCD"),
        ),
        (
            "filter_slope_db_oct",
            "滤波器配置",
            "Filter Slope",
            Some("dB/oct"),
            "number",
            &[][..],
            Some("OFSLD?"),
            Some("OFSLD"),
        ),
        (
            "harmonic_1",
            "谐波测量配置",
            "Harmonic 1",
            None,
            "number",
            &[][..],
            Some("HARMD?"),
            Some("HARMD"),
        ),
        (
            "harmonic_2",
            "谐波测量配置",
            "Harmonic 2",
            None,
            "number",
            &[][..],
            Some("HARMD?"),
            Some("HARMD"),
        ),
        (
            "auto_scale",
            "谐波测量配置",
            "Auto Scale",
            None,
            "action",
            &[][..],
            None,
            Some("ASCLD"),
        ),
        (
            "reference_source",
            "参考信号配置",
            "Reference Source",
            None,
            "enum",
            &["INT", "EXT"],
            Some("FMODD?"),
            Some("FMODD"),
        ),
        (
            "external_ref_trigger",
            "参考信号配置",
            "External Ref Trigger",
            None,
            "enum",
            &["SINE", "TTL"],
            Some("RSLPD?"),
            Some("RSLPD"),
        ),
        (
            "internal_frequency_hz",
            "参考信号配置",
            "Int.Frequency",
            Some("Hz"),
            "number",
            &[][..],
            Some("FREQD?"),
            Some("FREQD"),
        ),
        (
            "phase_deg",
            "参考信号配置",
            "Phase",
            Some("deg"),
            "number",
            &[][..],
            Some("PHASD?"),
            Some("PHASD"),
        ),
        (
            "auto_phase",
            "参考信号配置",
            "Auto Phase",
            None,
            "action",
            &[][..],
            None,
            Some("APHSD"),
        ),
        (
            "int_sweep_type",
            "内部扫频参考配置",
            "Int.Sweep Type",
            None,
            "enum",
            &["LINEAR", "LOG"],
            Some("SWTPD?"),
            Some("SWTPD"),
        ),
        (
            "int_sweep_run_mode",
            "内部扫频参考配置",
            "Int.Sweep Run Mode",
            None,
            "enum",
            &["ONCE", "LOOP"],
            Some("SWRMD?"),
            Some("SWRMD"),
        ),
        (
            "int_sweep_start_hz",
            "内部扫频参考配置",
            "Int.Sweep Start",
            Some("Hz"),
            "number",
            &[][..],
            Some("SLLMD?"),
            Some("SLLMD"),
        ),
        (
            "int_sweep_step",
            "内部扫频参考配置",
            "Int.Sweep Step / Log Step",
            None,
            "number",
            &[][..],
            Some("SSLLD?"),
            Some("SSLLD"),
        ),
        (
            "int_sweep_stop_hz",
            "内部扫频参考配置",
            "Int.Sweep Stop",
            Some("Hz"),
            "number",
            &[][..],
            Some("SULMD?"),
            Some("SULMD"),
        ),
        (
            "int_sweep_step_time_ms",
            "内部扫频参考配置",
            "Linear Step Time",
            Some("ms"),
            "number",
            &[][..],
            Some("STLMD?"),
            Some("STLMD"),
        ),
        (
            "equation",
            "自定义公式配置",
            "Equation",
            None,
            "enum",
            &["E1", "E2", "E3", "E4"],
            Some("EQCDD?"),
            Some("EQCDD"),
        ),
        (
            "formula_a",
            "自定义公式配置",
            "A coefficient",
            None,
            "string",
            &[][..],
            Some("EQCDD?"),
            Some("EQCDD"),
        ),
        (
            "formula_b",
            "自定义公式配置",
            "B coefficient",
            None,
            "string",
            &[][..],
            Some("EQCDD?"),
            Some("EQCDD"),
        ),
        (
            "formula_c",
            "自定义公式配置",
            "C coefficient",
            None,
            "string",
            &[][..],
            Some("EQCDD?"),
            Some("EQCDD"),
        ),
        (
            "constant_c1",
            "自定义公式配置",
            "C1 constant",
            None,
            "number",
            &[][..],
            Some("EQCSD?"),
            Some("EQCSD"),
        ),
        (
            "constant_c2",
            "自定义公式配置",
            "C2 constant",
            None,
            "number",
            &[][..],
            Some("EQCSD?"),
            Some("EQCSD"),
        ),
        (
            "system_setting",
            "系统设置保存与读取",
            "System Setting",
            None,
            "enum",
            &["S1", "S2", "S3", "S4"],
            Some("SSETD?"),
            Some("SSETD"),
        ),
        (
            "save_setting",
            "系统设置保存与读取",
            "Save Setting",
            None,
            "action",
            &[][..],
            None,
            Some("SSETD"),
        ),
        (
            "recall_setting",
            "系统设置保存与读取",
            "Recall Setting",
            None,
            "action",
            &[][..],
            None,
            Some("RSETD"),
        ),
        (
            "sineout_mode",
            "Sine Output 正弦信号输出配置",
            "SineOut Mode",
            None,
            "enum",
            &["FIXED", "LINEAR", "LOG", "DC"],
            Some("SWVTD?"),
            Some("SWVTD"),
        ),
        (
            "sineout_voltage_vrms",
            "Sine Output 正弦信号输出配置",
            "Sine Out Voltage",
            Some("Vrms"),
            "number",
            &[][..],
            Some("SLVLD?"),
            Some("SLVLD"),
        ),
        (
            "sineout_start_voltage_vrms",
            "Sine Output 正弦信号输出配置",
            "SineOut Start Voltage",
            Some("Vrms"),
            "number",
            &[][..],
            Some("SVLLD?"),
            Some("SVLLD"),
        ),
        (
            "sineout_stop_voltage_vrms",
            "Sine Output 正弦信号输出配置",
            "SineOut Stop Voltage",
            Some("Vrms"),
            "number",
            &[][..],
            Some("SVULD?"),
            Some("SVULD"),
        ),
        (
            "sineout_sweep_step",
            "Sine Output 正弦信号输出配置",
            "SineOut Sweep Step",
            None,
            "number",
            &[][..],
            Some("SVSLD?"),
            Some("SVSLD"),
        ),
        (
            "sineout_run_mode",
            "Sine Output 正弦信号输出配置",
            "SineOut Run Mode",
            None,
            "enum",
            &["ONCE", "LOOP"],
            Some("SVRMD?"),
            Some("SVRMD"),
        ),
        (
            "sineout_step_time_ms",
            "Sine Output 正弦信号输出配置",
            "Step Time",
            Some("ms"),
            "number",
            &[][..],
            Some("SVTMD?"),
            Some("SVTMD"),
        ),
        (
            "dc_output_voltage_v",
            "Sine Output 正弦信号输出配置",
            "DC Output Voltage",
            Some("Vdc"),
            "number",
            &[][..],
            Some("SVDCD?"),
            Some("SVDCD"),
        ),
        (
            "channel_1",
            "Channel Out 通道输出配置",
            "Channel 1",
            None,
            "enum",
            &[][..],
            Some("CAUXD?"),
            Some("CAUXD"),
        ),
        (
            "channel_2",
            "Channel Out 通道输出配置",
            "Channel 2",
            None,
            "enum",
            &[][..],
            Some("CAUXD?"),
            Some("CAUXD"),
        ),
        (
            "offset_percent",
            "Channel Out 通道输出配置",
            "Offset",
            Some("%"),
            "number",
            &[][..],
            Some("FPOPD?"),
            Some("FPOPD"),
        ),
        (
            "expand",
            "Channel Out 通道输出配置",
            "Expand",
            None,
            "enum",
            &[][..],
            Some("OEXPD?"),
            Some("OEXPD"),
        ),
        (
            "speed",
            "Channel Out 通道输出配置",
            "Speed",
            None,
            "enum",
            &[][..],
            Some("SPEDD?"),
            Some("SPEDD"),
        ),
        (
            "auxout_vdc",
            "Channel Out 通道输出配置",
            "AUXOUT",
            Some("Vdc"),
            "number",
            &[][..],
            Some("CAUXD?"),
            Some("CAUXD"),
        ),
    ] {
        out.push(catalog_field(
            "oe1022d", group, field_id, label, unit, ty, allowed, None, query, set,
        ));
    }

    for field in &mut out {
        enrich_catalog_field(field);
    }

    out
}

fn device_default_packages() -> Vec<DeviceDefaultPackage> {
    vec![
        DeviceDefaultPackage {
            device: "smb100a".to_string(),
            package_id: "safe_defaults".to_string(),
            label_cn: "SMB100A 安全默认配置组".to_string(),
            source: "examples/面板基础配置-oe1022d/smb100a/*_checked_v2.json safe_value"
                .to_string(),
            risk_level: "safe".to_string(),
            values: serde_json::json!({
                "frequency_hz": 1_000_000.0,
                "power_dbm": -30.0,
                "rf_output": "OFF",
                "mod_state": "OFF",
                "alc_mode": "AUTO",
                "lf_output": "OFF",
                "lf_voltage_v": 0.0,
                "lf_frequency_hz": 500.0,
                "lf_shape": "SQU",
                "lf_impedance": "LOW",
                "fm_enabled": "OFF",
                "fm_source": "INT",
                "fm_mode": "NORM",
                "fm_deviation_hz": 3_500_000.0,
                "lf_sweep_enabled": "OFF",
                "lf_sweep_mode": "AUTO",
                "rf_sweep_step_hz": 500_000.0,
                "rf_sweep_dwell_s": 0.5,
                "rf_sweep_lf_output": "OFF"
            }),
            values_si: serde_json::json!({
                "frequency_hz": 1_000_000.0,
                "power_dbm": -30.0,
                "rf_output": "OFF",
                "mod_state": "OFF",
                "lf_frequency_hz": 500.0,
                "fm_deviation_hz": 3_500_000.0
            }),
            note_cn: "用于连接、预检和非采集阶段；RF 输出与调制默认关闭。".to_string(),
            apply_target: "device_preset_draft".to_string(),
        },
        DeviceDefaultPackage {
            device: "smb100a".to_string(),
            package_id: "legacy_panel_observed_defaults".to_string(),
            label_cn: "SMB100A 旧面板复现配置组".to_string(),
            source: "examples/面板基础配置-oe1022d/smb100a/*_checked_v2.json pdf_observed / safe sequence"
                .to_string(),
            risk_level: "warning".to_string(),
            values: serde_json::json!({
                "main": {
                    "frequency_hz": 2_856_500_000.0,
                    "power_dbm": 6.0,
                    "rf_output": "ON",
                    "mod_state": "ON",
                    "alc_mode": "AUTO"
                },
                "lf": {
                    "lf_output": "ON",
                    "lf_voltage_v": 0.137,
                    "lf_frequency_hz": 500.0,
                    "lf_shape": "SQU",
                    "lf_impedance": "LOW"
                },
                "fm": {
                    "fm_enabled": "ON",
                    "fm_source": "INT",
                    "fm_mode": "HDEV",
                    "fm_deviation_hz": 3_500_000.0
                }
            }),
            values_si: serde_json::json!({
                "rf_sweep": {
                    "start_hz": 2_830_000_000.0,
                    "stop_hz": 2_910_000_000.0,
                    "step_hz": 500_000.0,
                    "dwell_ms": 500.0,
                    "power_dbm": 6.0,
                    "spacing": "LINear",
                    "shape": "SAWtooth",
                    "sweep_output_start_v": 0.0,
                    "sweep_output_stop_v": 3.0
                },
                "workbench": {
                    "frequency_hz": 2_856_500_000.0,
                    "rf_output": "ON",
                    "mod_state": "ON",
                    "lf_frequency_hz": 500.0,
                    "lf_voltage_v": 0.137,
                    "fm_deviation_hz": 3_500_000.0,
                    "fm_mode": "HDEViation"
                }
            }),
            note_cn: "用于复现旧截图/旧面板状态；包含 RF ON，不应作为安全上电默认。".to_string(),
            apply_target: "device_preset_draft".to_string(),
        },
        DeviceDefaultPackage {
            device: "oe1022d".to_string(),
            package_id: "panel_current_defaults".to_string(),
            label_cn: "OE1022D 面板当前值配置组".to_string(),
            source: "docs/equipment_manual/oe1022d/校对后的oe1022d面板基础设置/*_checked_v2.json screenshot_current_values"
                .to_string(),
            risk_level: "pass".to_string(),
            values: serde_json::json!({
                "input_filter": {
                    "input_source": "单端电压信号",
                    "input_shield_grounding": "浮空",
                    "input_coupling": "交流耦合",
                    "input_notch_filter": "关闭所有陷波器",
                    "dynamic_reserve": "正常",
                    "sensitivity": "100 mV/nA",
                    "time_constant": "300 ms",
                    "filter_slope": "12 dB/oct",
                    "synchronous_filter": "关",
                    "harmonic_1": 1,
                    "harmonic_2": 1
                },
                "reference": {
                    "ch_a_reference_source": "内部参考",
                    "ch_a_internal_frequency_hz": 1.0,
                    "ch_a_phase_deg": 0.0,
                    "ch_b_reference_source": "外部参考",
                    "ch_b_internal_frequency_hz": 102_000.0,
                    "ch_b_phase_deg": 0.0
                },
                "formula_system": {
                    "equation_index": "E1",
                    "equation_expression": "E1 = R * R / R",
                    "c1_value": 0.0,
                    "c2_value": 0.0,
                    "system_setting": "默认设置"
                },
                "output": {
                    "sineout_mode": "固定幅值模式",
                    "sineout_voltage_vrms": 1.0,
                    "sineout_run_mode": "停止扫幅",
                    "dc_output_voltage_vdc": 0.0,
                    "ch1_source": "A-R",
                    "ch2_source": "A-R",
                    "offset_percent": 0,
                    "expand": 1,
                    "speed": "慢速",
                    "auxout_vdc": 0.0
                }
            }),
            values_si: serde_json::json!({
                "oe1022d_acquisition": {
                    "mode": "follow_rf_sweep",
                    "pre_start_ms": 50.0,
                    "post_stop_ms": 50.0,
                    "channels": {
                        "ch_a": {
                            "time_constant_s": 0.3,
                            "filter_slope_db_oct": 12,
                            "dynamic_reserve": "NORMAL",
                            "sensitivity": "100 mV/nA"
                        },
                        "ch_b": {
                            "time_constant_s": 0.3,
                            "filter_slope_db_oct": 12,
                            "dynamic_reserve": "NORMAL",
                            "sensitivity": "100 mV/nA"
                        }
                    }
                }
            }),
            note_cn: "来自旧 GUI 截图当前值，适合作为采集模板草稿；灰显字段仍需由后端校验是否可下发。".to_string(),
            apply_target: "device_preset_draft".to_string(),
        },
    ]
}

fn oe1022d_rows_from_plan(plan: &Value) -> Vec<Oe1022dProjection> {
    let oe = plan
        .pointer("/spectrum_template/oe1022d_acquisition")
        .or_else(|| plan.pointer("/spectrum_template/oe1022d"))
        .or_else(|| plan.pointer("/acquisition/oe1022d"))
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    vec![Oe1022dProjection {
        frames_per_point: oe.get("frames_per_point").and_then(Value::as_u64),
        pre_discard_ms: oe.get("pre_discard_ms").and_then(Value::as_f64),
        pre_start_ms: oe.get("pre_start_ms").and_then(Value::as_f64),
        post_stop_ms: oe.get("post_stop_ms").and_then(Value::as_f64),
        time_constant_s: oe.get("time_constant_s").and_then(Value::as_f64),
        filter_slope_db_oct: oe.get("filter_slope_db_oct").and_then(Value::as_f64),
        reference_source: oe
            .get("reference_source")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        acquisition: oe,
    }]
}

fn estimate_duration_s(plan: &Value, measurements: usize) -> Option<f64> {
    let dwell_ms = rf_template_from_plan(plan)?
        .get("dwell_ms")
        .and_then(Value::as_f64)?;
    Some(measurements as f64 * dwell_ms / 1000.0)
}

fn read_b_vector(step: &Value) -> [f64; 3] {
    if let Some(arr) = step.get("b_target_nt").and_then(Value::as_array) {
        return [
            arr.first().and_then(Value::as_f64).unwrap_or(0.0),
            arr.get(1).and_then(Value::as_f64).unwrap_or(0.0),
            arr.get(2).and_then(Value::as_f64).unwrap_or(0.0),
        ];
    }
    [
        step.get("bx_nt").and_then(Value::as_f64).unwrap_or(0.0),
        step.get("by_nt").and_then(Value::as_f64).unwrap_or(0.0),
        step.get("bz_nt").and_then(Value::as_f64).unwrap_or(0.0),
    ]
}

fn oe_channel_value<'a>(oe: &'a Value, channel: &str, key: &str) -> Option<&'a Value> {
    oe.pointer(&format!("/channels/{channel}/{key}"))
        .or_else(|| oe.pointer(&format!("/{channel}/{key}")))
}

fn oe_channel_f64(oe: &Value, channel: &str, key: &str) -> Option<f64> {
    oe_channel_value(oe, channel, key).and_then(Value::as_f64)
}

fn oe_channel_string(oe: &Value, channel: &str, key: &str) -> Option<String> {
    oe_channel_value(oe, channel, key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn rf_sweep_summary(rf: &Value) -> String {
    let start = rf
        .get("start_hz")
        .and_then(Value::as_f64)
        .map(|v| format!("{:.6} GHz", v / 1e9))
        .unwrap_or_else(|| "—".into());
    let stop = rf
        .get("stop_hz")
        .and_then(Value::as_f64)
        .map(|v| format!("{:.6} GHz", v / 1e9))
        .unwrap_or_else(|| "—".into());
    let step = rf
        .get("step_hz")
        .and_then(Value::as_f64)
        .map(|v| format!("{v} Hz"))
        .unwrap_or_else(|| "—".into());
    let dwell = rf
        .get("dwell_ms")
        .and_then(Value::as_f64)
        .map(|v| format!("{v} ms"))
        .unwrap_or_else(|| "—".into());
    format!("{start} → {stop}; step {step}; dwell {dwell}")
}

fn step_rows_from_manual(plan: &Value) -> Option<Vec<ExperimentStepProjection>> {
    let steps = plan
        .get("steps")
        .and_then(Value::as_array)
        .or_else(|| plan.get("manual_steps").and_then(Value::as_array))?;
    let rf_template = rf_template_from_plan(plan).unwrap_or(&Value::Null);
    let laser_template = plan
        .pointer("/spectrum_template/laser")
        .unwrap_or(&Value::Null);
    let oe_template = plan
        .pointer("/spectrum_template/oe1022d_acquisition")
        .or_else(|| plan.pointer("/spectrum_template/oe1022d"))
        .unwrap_or(&Value::Null);
    let rf_total_count = rf_point_count(plan);
    let rows: Vec<ExperimentStepProjection> = steps
        .iter()
        .take(200)
        .enumerate()
        .map(|(idx, step)| {
            let b = read_b_vector(step);
            let smb = step
                .get("rf_sweep")
                .or_else(|| step.get("smb100a"))
                .unwrap_or(rf_template);
            let laser = step.get("laser").unwrap_or(laser_template);
            let oe = step
                .get("oe1022d_acquisition")
                .or_else(|| step.get("oe1022d"))
                .unwrap_or(oe_template);
            let dwell_ms = smb
                .get("dwell_ms")
                .and_then(Value::as_f64)
                .or_else(|| step.get("dwell_ms").and_then(Value::as_f64));
            let pre_ms = oe.get("pre_start_ms").and_then(Value::as_f64);
            let post_ms = oe.get("post_stop_ms").and_then(Value::as_f64);
            let mut blocked_reasons = Vec::new();
            if b.iter().any(|v| !v.is_finite()) {
                blocked_reasons.push("磁场向量不是有限数字".into());
            }
            if smb
                .get("frequency_hz")
                .and_then(Value::as_f64)
                .map(|v| v <= 0.0)
                .unwrap_or(false)
            {
                blocked_reasons.push("SMB100A 频率必须大于 0".into());
            }
            ExperimentStepProjection {
                step_index: idx,
                step_id: step
                    .get("step_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("step_{idx:04}")),
                group_id: step
                    .get("group_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                bx_nt: b[0],
                by_nt: b[1],
                bz_nt: b[2],
                rf_start_hz: smb.get("start_hz").and_then(Value::as_f64),
                rf_stop_hz: smb.get("stop_hz").and_then(Value::as_f64),
                rf_step_hz: smb.get("step_hz").and_then(Value::as_f64),
                smb100a_frequency_hz: smb
                    .get("frequency_hz")
                    .and_then(Value::as_f64)
                    .or_else(|| smb.get("start_hz").and_then(Value::as_f64)),
                smb100a_power_dbm: smb.get("power_dbm").and_then(Value::as_f64),
                smb100a_fm_enabled: smb.get("fm_enabled").and_then(Value::as_bool),
                smb100a_lf_frequency_hz: smb.get("lf_frequency_hz").and_then(Value::as_f64),
                smb100a_rf_sweep_summary: smb
                    .get("rf_sweep_summary")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| rf_sweep_summary(smb)),
                smb100a_sweep_output_start_v: smb
                    .get("sweep_output_start_v")
                    .and_then(Value::as_f64),
                smb100a_sweep_output_stop_v: smb.get("sweep_output_stop_v").and_then(Value::as_f64),
                laser_power_mw: laser.get("power_mw").and_then(Value::as_f64),
                laser_enabled: laser.get("enabled").and_then(Value::as_bool),
                oe1022d_summary: oe
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or("manual acquisition package")
                    .to_string(),
                oe_pre_start_ms: oe.get("pre_start_ms").and_then(Value::as_f64),
                oe_post_stop_ms: oe.get("post_stop_ms").and_then(Value::as_f64),
                oe_ch_a_time_constant_s: oe_channel_f64(oe, "ch_a", "time_constant_s"),
                oe_ch_a_filter_slope_db_oct: oe_channel_f64(oe, "ch_a", "filter_slope_db_oct"),
                oe_ch_a_dynamic_reserve: oe_channel_string(oe, "ch_a", "dynamic_reserve"),
                oe_ch_a_sensitivity: oe_channel_string(oe, "ch_a", "sensitivity"),
                oe_ch_b_time_constant_s: oe_channel_f64(oe, "ch_b", "time_constant_s"),
                oe_ch_b_filter_slope_db_oct: oe_channel_f64(oe, "ch_b", "filter_slope_db_oct"),
                oe_ch_b_dynamic_reserve: oe_channel_string(oe, "ch_b", "dynamic_reserve"),
                oe_ch_b_sensitivity: oe_channel_string(oe, "ch_b", "sensitivity"),
                dwell_ms,
                estimated_duration_s: dwell_ms.map(|v| {
                    (rf_total_count as f64 * v + pre_ms.unwrap_or(0.0) + post_ms.unwrap_or(0.0))
                        / 1000.0
                }),
                executable: blocked_reasons.is_empty(),
                blocked_reasons,
            }
        })
        .collect();
    Some(rows)
}

fn step_rows_from_projection(
    plan: &Value,
    magnetic_points: &[FieldPointEntry],
) -> Vec<ExperimentStepProjection> {
    let mut rows = Vec::new();
    for entry in magnetic_points {
        if rows.len() >= 200 {
            return rows;
        }
        let group_id = entry.group_id.as_deref();
        let rf_template = rf_template_for_group(plan, group_id);
        let rf_points = rf_points_from_template(&rf_template);
        let rf_total_count = rf_count_from_template(&rf_template);
        let first_rf = rf_points.first();
        let laser_template = laser_template_for_group(plan, group_id);
        let oe_template = oe_template_for_group(plan, group_id);
        let dwell_ms = first_rf.and_then(|rf| rf.dwell_ms);
        let pre_ms = oe_template.get("pre_start_ms").and_then(Value::as_f64);
        let post_ms = oe_template.get("post_stop_ms").and_then(Value::as_f64);
        let tc_a = oe_channel_f64(&oe_template, "ch_a", "time_constant_s");
        let slope_a = oe_channel_f64(&oe_template, "ch_a", "filter_slope_db_oct");
        let tc_b = oe_channel_f64(&oe_template, "ch_b", "time_constant_s");
        let slope_b = oe_channel_f64(&oe_template, "ch_b", "filter_slope_db_oct");
        let oe_summary = format!(
            "follow_rf_sweep; pre={}ms post={}ms; Ch-A TC={}s slope={}dB/oct; Ch-B TC={}s slope={}dB/oct",
            pre_ms.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
            post_ms.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
            tc_a.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
            slope_a.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
            tc_b.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
            slope_b.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
        );
        let estimated_duration_s = dwell_ms.map(|dwell| {
            (rf_total_count as f64 * dwell + pre_ms.unwrap_or(0.0) + post_ms.unwrap_or(0.0))
                / 1000.0
        });
        rows.push(ExperimentStepProjection {
            step_index: rows.len(),
            step_id: format!("spectrum_{:04}", rows.len()),
            group_id: entry.group_id.clone(),
            bx_nt: entry.point[0],
            by_nt: entry.point[1],
            bz_nt: entry.point[2],
            rf_start_hz: rf_template.get("start_hz").and_then(Value::as_f64),
            rf_stop_hz: rf_template.get("stop_hz").and_then(Value::as_f64),
            rf_step_hz: rf_template.get("step_hz").and_then(Value::as_f64),
            smb100a_frequency_hz: first_rf.map(|rf| rf.frequency_hz).filter(|v| *v > 0.0),
            smb100a_power_dbm: first_rf.and_then(|rf| rf.power_dbm),
            smb100a_fm_enabled: first_rf.and_then(|rf| rf.fm_enabled),
            smb100a_lf_frequency_hz: first_rf.and_then(|rf| rf.lf_frequency_hz),
            smb100a_rf_sweep_summary: rf_sweep_summary(&rf_template),
            smb100a_sweep_output_start_v: first_rf.and_then(|rf| rf.sweep_output_start_v),
            smb100a_sweep_output_stop_v: first_rf.and_then(|rf| rf.sweep_output_stop_v),
            laser_power_mw: laser_template.get("power_mw").and_then(Value::as_f64),
            laser_enabled: laser_template.get("enabled").and_then(Value::as_bool),
            oe1022d_summary: oe_summary,
            oe_pre_start_ms: pre_ms,
            oe_post_stop_ms: post_ms,
            oe_ch_a_time_constant_s: tc_a,
            oe_ch_a_filter_slope_db_oct: slope_a,
            oe_ch_a_dynamic_reserve: oe_channel_string(&oe_template, "ch_a", "dynamic_reserve"),
            oe_ch_a_sensitivity: oe_channel_string(&oe_template, "ch_a", "sensitivity"),
            oe_ch_b_time_constant_s: tc_b,
            oe_ch_b_filter_slope_db_oct: slope_b,
            oe_ch_b_dynamic_reserve: oe_channel_string(&oe_template, "ch_b", "dynamic_reserve"),
            oe_ch_b_sensitivity: oe_channel_string(&oe_template, "ch_b", "sensitivity"),
            dwell_ms,
            estimated_duration_s,
            executable: true,
            blocked_reasons: Vec::new(),
        });
    }
    rows
}

fn project_plan(plan: &Value) -> ExperimentPlanProjection {
    const PREVIEW_LIMIT: usize = 200;
    let field_entries = magnetic_field_entries_from_plan(plan);
    let points: Vec<[f64; 3]> = field_entries.iter().map(|entry| entry.point).collect();
    let rf_points = rf_points_from_plan(plan);
    let laser_rows = laser_rows_from_plan(plan);
    let oe1022d_rows = oe1022d_rows_from_plan(plan);
    let rf_count = rf_point_count(plan);
    let estimated_measurements = points.len() * rf_count;
    let step_rows = step_rows_from_manual(plan)
        .unwrap_or_else(|| step_rows_from_projection(plan, &field_entries));
    let step_row_count = plan
        .get("manual_steps")
        .and_then(Value::as_array)
        .or_else(|| plan.get("steps").and_then(Value::as_array))
        .map(|steps| steps.len())
        .unwrap_or(field_entries.len());
    let truncated = step_row_count > PREVIEW_LIMIT;

    let magnetic_points: Vec<MagneticPointProjection> = points
        .iter()
        .zip(field_entries.iter())
        .take(PREVIEW_LIMIT)
        .enumerate()
        .map(|(idx, (point, entry))| MagneticPointProjection {
            point_index: idx,
            group_id: entry.group_id.clone(),
            bx_nt: point[0],
            by_nt: point[1],
            bz_nt: point[2],
            source: entry.source.clone(),
        })
        .collect();

    let mut combination_preview = Vec::new();
    for (mag_idx, point) in points.iter().enumerate() {
        for rf in &rf_points {
            if combination_preview.len() >= PREVIEW_LIMIT {
                break;
            }
            combination_preview.push(CombinationPreviewRow {
                row_index: combination_preview.len(),
                magnetic_point_index: mag_idx,
                rf_point_index: rf.point_index,
                bx_nt: point[0],
                by_nt: point[1],
                bz_nt: point[2],
                frequency_hz: rf.frequency_hz,
                laser_mode: laser_rows
                    .first()
                    .map(|row| row.mode.clone())
                    .unwrap_or_else(|| "fixed_power".into()),
                oe_frames_per_point: oe1022d_rows.first().and_then(|row| row.frames_per_point),
            });
        }
        if combination_preview.len() >= PREVIEW_LIMIT {
            break;
        }
    }

    let mut warnings = Vec::new();
    if estimated_measurements > combination_preview.len() {
        warnings.push(format!(
            "Combination preview is capped at {} rows; total measurements: {estimated_measurements}.",
            combination_preview.len()
        ));
    }
    if laser_rows.len() == 1 && laser_rows[0].mode != "fixed_power" {
        warnings.push(
            "Laser sweep is not implemented in v1; projection treats laser as one package row."
                .into(),
        );
    }

    ExperimentPlanProjection {
        kind: "experiment_plan_projection".into(),
        panel_catalogs: device_panel_catalogs(),
        default_packages: device_default_packages(),
        step_rows,
        step_row_count,
        preview_limit: PREVIEW_LIMIT,
        truncated,
        magnetic_points,
        smb100a_rf_points: rf_points,
        laser_rows,
        oe1022d_rows,
        combination_preview,
        estimated_measurements,
        estimated_duration_s: estimate_duration_s(plan, estimated_measurements),
        warnings,
    }
}

fn current_plan_from_state(state: &WorkbenchState) -> Result<Value, String> {
    let guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    guard
        .experiment_plan_draft
        .clone()
        .or_else(|| guard.experiment_plan.clone())
        .ok_or_else(|| "No experiment plan loaded.".to_string())
}

fn plan_requires_zero_lock(plan: &Value) -> bool {
    plan.pointer("/runtime_requirements/require_zero_lock")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn laser_enabled_in_projection(projection: &ExperimentPlanProjection) -> bool {
    projection
        .laser_rows
        .iter()
        .any(|row| row.enabled.unwrap_or(false))
        || projection
            .step_rows
            .iter()
            .any(|row| row.laser_enabled.unwrap_or(false))
}

fn experiment_run_readiness_for_plan(
    state: &WorkbenchState,
    plan: &Value,
) -> ExperimentRunReadiness {
    let projection = project_plan(plan);
    let require_zero_lock = plan_requires_zero_lock(plan);
    let mut required_devices = vec![
        "smb100a_main".to_string(),
        "oe1022d_main".to_string(),
        "maynuo.mag_x".to_string(),
        "maynuo.mag_y".to_string(),
        "maynuo.mag_z".to_string(),
    ];
    if laser_enabled_in_projection(&projection) {
        required_devices.push("cni_laser".to_string());
    }

    let mut connected_devices = Vec::new();
    let mut blocked_reasons = Vec::new();
    for device_id in &required_devices {
        if state.is_accessible(device_id) {
            connected_devices.push(device_id.clone());
        } else {
            blocked_reasons.push(format!("{device_id} is not connected or locked"));
        }
    }

    let zero_baseline_present = state
        .inner
        .lock()
        .ok()
        .and_then(|guard| guard.runtime_zero_baseline.clone())
        .is_some();
    if require_zero_lock && !zero_baseline_present {
        blocked_reasons.push("runtime zero baseline is missing".into());
    }
    if require_zero_lock {
        for device_id in ["maynuo.mag_x", "maynuo.mag_y", "maynuo.mag_z"] {
            if !state.mag_lock_zero(device_id) {
                blocked_reasons.push(format!("{device_id} zero is not locked"));
            }
        }
    }

    if projection.step_row_count == 0 {
        blocked_reasons.push("experiment plan has no spectrum steps".into());
    }
    if projection.estimated_measurements == 0 {
        blocked_reasons.push("experiment plan has no RF measurements".into());
    }

    let mut warnings = projection.warnings.clone();
    if projection.estimated_duration_s.unwrap_or(0.0) > 3600.0 {
        warnings.push(format!(
            "estimated run duration is {:.1} s; operator should confirm long-run conditions",
            projection.estimated_duration_s.unwrap_or(0.0)
        ));
    }
    if laser_enabled_in_projection(&projection) {
        warnings.push("laser is enabled in the experiment template; verify optical safety before hardware run".into());
    }

    let hardware_blocked_reasons = blocked_reasons.clone();

    ExperimentRunReadiness {
        kind: "experiment_plan_run_readiness".into(),
        ready_for_preview_execution: projection.step_row_count > 0
            && projection.estimated_measurements > 0,
        ready_for_hardware_execution: hardware_blocked_reasons.is_empty(),
        blocked_reasons,
        hardware_blocked_reasons,
        warnings,
        step_count: projection.step_row_count,
        rf_point_count: rf_point_count(plan),
        estimated_measurements: projection.estimated_measurements,
        estimated_duration_s: projection.estimated_duration_s,
        require_zero_lock,
        zero_baseline_present,
        connected_devices,
        required_devices,
    }
}

fn run_root_dir() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("current dir: {e}"))?;
    Ok(cwd.join("target").join("odmr-runs"))
}

fn base_run_status(
    run_id: String,
    mode: String,
    state: String,
    started_at: String,
    readiness: &ExperimentRunReadiness,
) -> ExperimentPlanRunStatus {
    ExperimentPlanRunStatus {
        kind: "experiment_plan_run_status".into(),
        run_id,
        mode,
        state,
        started_at,
        finished_at: None,
        run_directory: None,
        step_count: readiness.step_count,
        rf_point_count: readiness.rf_point_count,
        estimated_measurements: readiness.estimated_measurements,
        estimated_duration_s: readiness.estimated_duration_s,
        steps_completed: 0,
        current_step_index: None,
        current_step_id: None,
        current_b_nt: None,
        current_phase: None,
        smb_sweep_running: false,
        oe_frames_captured: 0,
        cleanup_state: None,
        recent_error: None,
        blocked_reasons: Vec::new(),
        warnings: readiness.warnings.clone(),
        artifact_paths: HashMap::new(),
    }
}

fn write_json_file(path: &PathBuf, value: &Value) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("create {}: {e}", path.display()))?;
    serde_json::to_writer_pretty(file, value)
        .map_err(|e| format!("write json {}: {e}", path.display()))
}

fn write_preview_run_artifacts(
    run_id: &str,
    plan: &Value,
    projection: &ExperimentPlanProjection,
    readiness: &ExperimentRunReadiness,
) -> Result<(String, HashMap<String, String>), String> {
    let run_dir = run_root_dir()?.join(run_id);
    if run_dir.exists() {
        return Err(format!(
            "run directory already exists: {}",
            run_dir.display()
        ));
    }
    fs::create_dir_all(run_dir.join("metadata"))
        .map_err(|e| format!("create metadata dir {}: {e}", run_dir.display()))?;

    let manifest = json!({
        "schema_version": "0.1.0",
        "kind": "experiment_plan_preview_run_manifest",
        "run_id": run_id,
        "created_at": now_rfc3339(),
        "mode": "preview",
        "step_count": readiness.step_count,
        "rf_point_count": readiness.rf_point_count,
        "estimated_measurements": readiness.estimated_measurements,
        "estimated_duration_s": readiness.estimated_duration_s,
        "artifact_paths": {
            "manifest": "manifest.json",
            "experiment_plan_lock": "metadata/experiment_plan.lock.json",
            "projection": "metadata/projection.json",
            "readiness": "metadata/readiness.json",
            "events": "events.jsonl"
        }
    });
    write_json_file(&run_dir.join("manifest.json"), &manifest)?;
    write_json_file(&run_dir.join("metadata/experiment_plan.lock.json"), plan)?;
    write_json_file(
        &run_dir.join("metadata/projection.json"),
        &serde_json::to_value(projection).map_err(|e| format!("serialize projection: {e}"))?,
    )?;
    write_json_file(
        &run_dir.join("metadata/readiness.json"),
        &serde_json::to_value(readiness).map_err(|e| format!("serialize readiness: {e}"))?,
    )?;

    let mut events = File::create(run_dir.join("events.jsonl"))
        .map_err(|e| format!("create events.jsonl: {e}"))?;
    let event = |event_id: usize, event_type: &str, message: String| {
        json!({
            "schema_version": "0.1.0",
            "kind": "experiment_plan_run_event",
            "run_id": run_id,
            "event_id": format!("evt_{event_id:06}"),
            "timestamp": now_rfc3339(),
            "level": "info",
            "event_type": event_type,
            "message": message
        })
    };
    let events_to_write = [
        event(1, "run_created", "Preview run directory created".into()),
        event(
            2,
            "plan_locked",
            format!(
                "Locked {} spectrum steps and {} total RF measurements",
                readiness.step_count, readiness.estimated_measurements
            ),
        ),
        event(
            3,
            "preview_execution_completed",
            "No hardware commands were sent; this verifies plan expansion and artifact writing only"
                .into(),
        ),
    ];
    for item in events_to_write {
        serde_json::to_writer(&mut events, &item)
            .map_err(|e| format!("write events.jsonl: {e}"))?;
        events
            .write_all(b"\n")
            .map_err(|e| format!("flush events.jsonl: {e}"))?;
    }

    let mut artifact_paths = HashMap::new();
    artifact_paths.insert("manifest".into(), "manifest.json".into());
    artifact_paths.insert(
        "experiment_plan_lock".into(),
        "metadata/experiment_plan.lock.json".into(),
    );
    artifact_paths.insert("projection".into(), "metadata/projection.json".into());
    artifact_paths.insert("readiness".into(), "metadata/readiness.json".into());
    artifact_paths.insert("events".into(), "events.jsonl".into());
    Ok((run_dir.to_string_lossy().to_string(), artifact_paths))
}

#[allow(dead_code)]
fn write_hardware_run_artifacts(
    _run_id: &str,
    _plan: &Value,
    _projection: &ExperimentPlanProjection,
    _readiness: &ExperimentRunReadiness,
    _operator_confirmed: bool,
) -> Result<(String, HashMap<String, String>, u64), String> {
    Err("obsolete placeholder writer: use odmr-executor::run_hardware".into())
}

fn default_experiment_plan_draft() -> Value {
    serde_json::json!({
        "schema_version": "0.1.0",
        "kind": "experiment_plan",
        "id": "default_device_package_draft",
        "source": "gui_default_projection",
        "field_space": {
            "mode": "explicit_points",
            "points": [[0.0, 0.0, 0.0]],
            "unit": "nT"
        },
        "spectrum_template": {
            "rf_sweep": {
                "start_hz": 2_800_000_000.0,
                "stop_hz": 2_900_000_000.0,
                "step_hz": 1_000_000.0,
                "dwell_ms": 300.0,
                "power_dbm": -30.0,
                "spacing": "LINear",
                "shape": "SAWtooth",
                "sweep_output_start_v": 0.0,
                "sweep_output_stop_v": 3.0
            },
            "laser": {
                "mode": "fixed_power",
                "enabled": false
            },
            "oe1022d_acquisition": {
                "mode": "follow_rf_sweep",
                "pre_start_ms": 50.0,
                "post_stop_ms": 50.0,
                "channels": {
                    "ch_a": {
                        "time_constant_s": 0.3,
                        "filter_slope_db_oct": 12.0,
                        "dynamic_reserve": "NORMAL",
                        "sensitivity": "100 mV/nA"
                    },
                    "ch_b": {
                        "time_constant_s": 0.3,
                        "filter_slope_db_oct": 12.0,
                        "dynamic_reserve": "NORMAL",
                        "sensitivity": "100 mV/nA"
                    }
                }
            }
        },
        "steps": [{
            "step_id": "spectrum_0000",
            "b_target_nt": [0.0, 0.0, 0.0]
        }],
        "runtime_requirements": {
            "require_zero_lock": true
        }
    })
}

fn summarize_plan(plan: Value) -> ExperimentPlanSummary {
    let points = magnetic_points_from_field_space(&plan);
    let rf_count = rf_point_count(&plan);
    let kind = str_field(&plan, "kind", "experiment_plan");
    let require_zero_lock = plan
        .pointer("/runtime_requirements/require_zero_lock")
        .and_then(Value::as_bool)
        .or_else(|| {
            plan.pointer("/fixed_params/magnetic/zero_lock_required")
                .and_then(Value::as_bool)
        })
        .unwrap_or(true);
    let mut warnings = Vec::new();
    if kind == "system_scan_recipe" {
        warnings.push(
            "Loaded legacy system_scan_recipe shape; preview uses its sweeps/fixed_params.".into(),
        );
    }

    ExperimentPlanSummary {
        schema_version: str_field(&plan, "schema_version", "0.1.0"),
        kind,
        id: str_field(&plan, "id", "unnamed_plan"),
        station_ref: plan
            .get("station_ref")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        preset_refs: plan.get("preset_refs").cloned().unwrap_or(Value::Null),
        field_point_count: points.len(),
        rf_point_count: rf_count,
        estimated_measurements: points.len() * rf_count,
        require_zero_lock,
        warnings,
        raw: plan,
    }
}

#[tauri::command]
pub fn load_experiment_plan(
    state: tauri::State<WorkbenchState>,
    path: String,
) -> Result<ExperimentPlanSummary, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    let plan: Value = serde_json::from_str(&text).map_err(|e| format!("parse {path}: {e}"))?;
    let summary = summarize_plan(plan.clone());
    let mut guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    guard.experiment_plan = Some(plan);
    guard.experiment_plan_draft = Some(summary.raw.clone());
    Ok(summary)
}

#[tauri::command]
pub fn set_experiment_plan_draft(
    state: tauri::State<WorkbenchState>,
    plan: Value,
) -> Result<ExperimentPlanSummary, String> {
    let summary = summarize_plan(plan.clone());
    let mut guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    guard.experiment_plan = Some(plan);
    guard.experiment_plan_draft = Some(summary.raw.clone());
    Ok(summary)
}

#[tauri::command]
pub fn project_experiment_plan(
    state: tauri::State<WorkbenchState>,
) -> Result<ExperimentPlanProjection, String> {
    let plan = {
        let guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard
            .experiment_plan_draft
            .clone()
            .or_else(|| guard.experiment_plan.clone())
            .unwrap_or_else(default_experiment_plan_draft)
    };
    Ok(project_plan(&plan))
}

#[tauri::command]
pub fn get_experiment_plan_draft(state: tauri::State<WorkbenchState>) -> Result<Value, String> {
    let plan = {
        let guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard
            .experiment_plan_draft
            .clone()
            .or_else(|| guard.experiment_plan.clone())
            .unwrap_or_else(default_experiment_plan_draft)
    };
    Ok(plan)
}

#[tauri::command]
pub fn get_experiment_plan_run_readiness(
    state: tauri::State<WorkbenchState>,
) -> Result<ExperimentRunReadiness, String> {
    let plan = current_plan_from_state(&state)?;
    Ok(experiment_run_readiness_for_plan(&state, &plan))
}

#[tauri::command]
pub fn get_experiment_plan_run_status(
    state: tauri::State<WorkbenchState>,
) -> Result<Option<ExperimentPlanRunStatus>, String> {
    let guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    guard
        .experiment_run_status
        .clone()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| format!("deserialize run status: {e}"))
}

fn station_config_from_profile(
    profile: &odmr_preflight::StationProfile,
    safety: &crate::panels::StationSafety,
) -> Result<odmr_config::StationConfig, String> {
    let mut station = load_station_config_str(
        &serde_json::to_string(profile).map_err(|e| format!("serialize station profile: {e}"))?,
    )
    .map_err(|e| format!("convert station profile: {e}"))?;
    station.safety.smb100a_max_power_dbm = safety.smb100a_max_power_dbm;
    station.safety.smb100a_min_freq_hz = safety.smb100a_min_freq_hz;
    station.safety.smb100a_max_freq_hz = safety.smb100a_max_freq_hz;
    station.safety.mag_max_current_a_per_axis = safety.mag_max_current_a_per_axis;
    station.safety.laser_max_power_mw = safety.laser_max_power_mw;
    Ok(station)
}

fn build_hardware_run_config(
    state: &WorkbenchState,
    plan: &Value,
    run_id: &str,
) -> Result<HardwareRunConfig, String> {
    let (profile, safety, zero_baseline) = {
        let guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        (
            guard.profile.clone().ok_or("No station profile loaded.")?,
            guard.safety.clone(),
            guard
                .runtime_zero_baseline
                .clone()
                .ok_or("Runtime zero baseline is missing.")?,
        )
    };
    let station = station_config_from_profile(&profile, &safety)?;
    let find_device_id = |device_type: &str| -> Result<String, String> {
        station
            .devices
            .iter()
            .find(|device| device.device_type == device_type)
            .map(|device| device.device_id.clone())
            .ok_or_else(|| format!("Missing device '{device_type}' in station profile"))
    };
    let smb_device_id = find_device_id("smb100a")?;
    let oe_device_id = find_device_id("oe1022d")?;
    let laser_device_id = station
        .devices
        .iter()
        .find(|device| device.device_type == "laser")
        .map(|device| device.device_id.clone());
    let field_entries = magnetic_field_entries_from_plan(plan);
    let mut steps = Vec::new();

    for (idx, entry) in field_entries.iter().enumerate() {
        let rf = rf_template_for_group(plan, entry.group_id.as_deref());
        let oe = oe_template_for_group(plan, entry.group_id.as_deref());
        let laser = laser_template_for_group(plan, entry.group_id.as_deref());

        let mut magnetic_axes = Vec::new();
        for (axis_idx, axis_name) in ["x", "y", "z"].iter().enumerate() {
            if let Some(base) = zero_baseline.axes.get(*axis_name) {
                let recur_a = (entry.point[axis_idx] / base.coil_constant_nt_per_ma) / 1000.0;
                magnetic_axes.push(HardwareMagAxisTarget {
                    device_id: base.device_id.clone(),
                    current_a: base.zero_mean_a + recur_a,
                });
            }
        }

        let laser_power_mw = laser
            .get("power_mw")
            .or_else(|| laser.get("power_setpoint_mw"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .max(0.0) as u16;
        let laser_enabled = laser.get("enabled").and_then(Value::as_bool).unwrap_or(false);

        steps.push(HardwareRunStep {
            step_id: format!("spectrum_{idx:04}"),
            step_index: idx,
            b_target_nt: entry.point,
            magnetic_axes,
            rf: HardwareRfSweep {
                device_id: smb_device_id.clone(),
                start_hz: rf.get("start_hz").and_then(Value::as_f64).unwrap_or(2.8e9),
                stop_hz: rf.get("stop_hz").and_then(Value::as_f64).unwrap_or(2.9e9),
                step_hz: rf.get("step_hz").and_then(Value::as_f64).unwrap_or(1.0e6),
                dwell_ms: rf.get("dwell_ms").and_then(Value::as_f64).unwrap_or(300.0) as u64,
                power_dbm: rf.get("power_dbm").and_then(Value::as_f64).unwrap_or(-30.0),
                spacing: rf
                    .get("spacing")
                    .and_then(Value::as_str)
                    .unwrap_or("LINear")
                    .to_string(),
                shape: rf
                    .get("shape")
                    .and_then(Value::as_str)
                    .unwrap_or("SAWtooth")
                    .to_string(),
                sweep_mode: "AUTO".into(),
                trigger_source: "SING".into(),
                sweep_output_start_v: rf.get("sweep_output_start_v").and_then(Value::as_f64),
                sweep_output_stop_v: rf.get("sweep_output_stop_v").and_then(Value::as_f64),
            },
            oe: HardwareOeAcquisition {
                device_id: oe_device_id.clone(),
                pre_start_ms: oe.get("pre_start_ms").and_then(Value::as_f64).unwrap_or(50.0)
                    as u64,
                post_stop_ms: oe
                    .get("post_stop_ms")
                    .and_then(Value::as_f64)
                    .unwrap_or(50.0) as u64,
                baud: 921_600,
                read_interval_ms: 48,
                timeout_ms: 5000,
            },
            laser: laser_device_id.clone().map(|device_id| HardwareLaserTarget {
                device_id,
                power_mw: laser_power_mw,
                enabled: laser_enabled,
            }),
        });
    }

    Ok(HardwareRunConfig {
        run_root: run_root_dir()?,
        run_id: run_id.to_string(),
        station_snapshot: json!({
            "profile": profile,
            "safety": safety,
            "runtime_zero_baseline": zero_baseline,
            "plan_id": plan.get("id").and_then(Value::as_str)
        }),
        station,
        steps,
    })
}

fn store_run_status(
    state: &Arc<Mutex<crate::workbench_state::WorkbenchStateInner>>,
    status: &ExperimentPlanRunStatus,
) -> Result<(), String> {
    let mut guard = state.lock().map_err(|e| format!("lock poison: {e}"))?;
    guard.experiment_run_status =
        Some(serde_json::to_value(status).map_err(|e| format!("serialize status: {e}"))?);
    Ok(())
}

fn apply_progress_update(
    state: &Arc<Mutex<crate::workbench_state::WorkbenchStateInner>>,
    progress: HardwareProgress,
) {
    let Ok(mut guard) = state.lock() else {
        return;
    };
    let Some(current) = guard.experiment_run_status.clone() else {
        return;
    };
    let Ok(mut status) = serde_json::from_value::<ExperimentPlanRunStatus>(current) else {
        return;
    };
    status.current_step_id = progress.step_id;
    status.current_step_index = progress.step_index;
    status.current_phase = Some(progress.phase);
    status.steps_completed = progress.steps_completed;
    status.oe_frames_captured = progress.oe_frames_captured;
    guard.experiment_run_status = serde_json::to_value(status).ok();
}

#[tauri::command]
pub fn start_experiment_plan_run(
    state: tauri::State<WorkbenchState>,
    mode: String,
    operator_confirmed: bool,
) -> Result<ExperimentPlanRunStatus, String> {
    let plan = current_plan_from_state(&state)?;
    let projection = project_plan(&plan);
    let readiness = experiment_run_readiness_for_plan(&state, &plan);
    let run_id = format!(
        "experiment_plan_{}_{}",
        mode.replace(|c: char| !c.is_ascii_alphanumeric(), "_"),
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    );
    let started_at = now_rfc3339();

    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.experiment_run_cancel_requested = false;
    }

    if mode == "hardware" {
        let mut blocked = readiness.hardware_blocked_reasons.clone();
        if !operator_confirmed {
            blocked.push("operator confirmation is required before hardware execution".into());
        }
        if !blocked.is_empty() {
            let mut status =
                base_run_status(run_id, mode, "blocked".into(), started_at, &readiness);
            status.finished_at = Some(now_rfc3339());
            status.blocked_reasons = blocked;
            let mut guard = state
                .inner
                .lock()
                .map_err(|e| format!("lock poison: {e}"))?;
            guard.experiment_run_status =
                Some(serde_json::to_value(&status).map_err(|e| format!("serialize status: {e}"))?);
            return Ok(status);
        }

        let mut running = base_run_status(
            run_id.clone(),
            mode.clone(),
            "running".into(),
            started_at.clone(),
            &readiness,
        );
        running.current_step_index = Some(0);
        running.current_step_id = Some("spectrum_0000".into());
        running.current_b_nt = projection
            .step_rows
            .first()
            .map(|row| [row.bx_nt, row.by_nt, row.bz_nt]);
        running.current_phase = Some("preflight/locks ok; launching hardware runtime".into());

        let shared_state = state.inner.clone();
        let run_config = build_hardware_run_config(state.inner(), &plan, &run_id)?;
        let control = RunControl::new();
        {
            let mut guard = state
                .inner
                .lock()
                .map_err(|e| format!("lock poison: {e}"))?;
            guard.experiment_run_control = Some(control.clone());
            guard.experiment_run_status =
                Some(serde_json::to_value(&running).map_err(|e| format!("serialize status: {e}"))?);
        }
        std::thread::spawn(move || {
            let result = run_hardware(run_config, &control, |progress| {
                apply_progress_update(&shared_state, progress);
            });

            let final_status = match result {
                Ok(report) => {
                    let mut status = base_run_status(
                        report.run_id.clone(),
                        "hardware".into(),
                        if report.stopped {
                            "stopped".into()
                        } else {
                            "completed".into()
                        },
                        started_at,
                        &readiness,
                    );
                    status.finished_at = Some(now_rfc3339());
                    status.run_directory = Some(report.run_directory.to_string_lossy().to_string());
                    status.steps_completed = report.steps_completed;
                    status.current_step_index = report.steps_completed.checked_sub(1);
                    status.current_step_id = report
                        .steps_completed
                        .checked_sub(1)
                        .map(|idx| format!("spectrum_{idx:04}"));
                    status.current_phase = Some("hardware runtime finished".into());
                    status.smb_sweep_running = false;
                    status.oe_frames_captured = report.oe_frames_captured;
                    status.cleanup_state = Some(report.cleanup_state);
                    status.artifact_paths = report.artifact_paths;
                    status
                }
                Err(error) => {
                    let mut status = base_run_status(
                        run_id.clone(),
                        "hardware".into(),
                        "failed_cleanup_incomplete".into(),
                        started_at,
                        &readiness,
                    );
                    status.finished_at = Some(now_rfc3339());
                    status.recent_error = Some(error.to_string());
                    status.cleanup_state = Some("cleanup_attempted_after_runtime_error".into());
                    status.blocked_reasons = vec![error.to_string()];
                    status
                }
            };

            if let Ok(mut guard) = shared_state.lock() {
                guard.experiment_run_control = None;
            }
            let _ = store_run_status(&shared_state, &final_status);
        });
        return Ok(running);
    }

    if mode != "preview" {
        return Err("mode must be 'preview' or 'hardware'".into());
    }
    if !readiness.ready_for_preview_execution {
        let mut status = base_run_status(run_id, mode, "blocked".into(), started_at, &readiness);
        status.finished_at = Some(now_rfc3339());
        status.blocked_reasons = readiness.blocked_reasons;
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.experiment_run_status =
            Some(serde_json::to_value(&status).map_err(|e| format!("serialize status: {e}"))?);
        return Ok(status);
    }

    let (run_directory, artifact_paths) =
        write_preview_run_artifacts(&run_id, &plan, &projection, &readiness)?;
    let mut status = base_run_status(run_id, mode, "completed".into(), started_at, &readiness);
    status.finished_at = Some(now_rfc3339());
    status.run_directory = Some(run_directory);
    status.steps_completed = readiness.step_count;
    status.cleanup_state = Some("no_hardware_preview".into());
    status.artifact_paths = artifact_paths;
    let mut guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    guard.experiment_run_status =
        Some(serde_json::to_value(&status).map_err(|e| format!("serialize status: {e}"))?);
    Ok(status)
}

#[tauri::command]
pub fn stop_experiment_plan_run(
    state: tauri::State<WorkbenchState>,
) -> Result<ExperimentPlanRunStatus, String> {
    let mut guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    guard.experiment_run_cancel_requested = true;
    if let Some(control) = &guard.experiment_run_control {
        control.request_stop();
    }

    let mut status = guard
        .experiment_run_status
        .clone()
        .map(serde_json::from_value::<ExperimentPlanRunStatus>)
        .transpose()
        .map_err(|e| format!("deserialize run status: {e}"))?
        .unwrap_or_else(|| ExperimentPlanRunStatus {
            kind: "experiment_plan_run_status".into(),
            run_id: "none".into(),
            mode: "preview".into(),
            state: "idle".into(),
            started_at: now_rfc3339(),
            finished_at: None,
            run_directory: None,
            step_count: 0,
            rf_point_count: 0,
            estimated_measurements: 0,
            estimated_duration_s: None,
            steps_completed: 0,
            current_step_index: None,
            current_step_id: None,
            current_b_nt: None,
            current_phase: None,
            smb_sweep_running: false,
            oe_frames_captured: 0,
            cleanup_state: None,
            recent_error: None,
            blocked_reasons: Vec::new(),
            warnings: Vec::new(),
            artifact_paths: HashMap::new(),
        });
    if status.state == "running" {
        status.current_phase = Some("stop requested; waiting for cooperative cleanup".into());
        status.smb_sweep_running = false;
    }
    guard.experiment_run_status =
        Some(serde_json::to_value(&status).map_err(|e| format!("serialize status: {e}"))?);
    Ok(status)
}

#[tauri::command]
pub fn export_experiment_plan_json(
    state: tauri::State<WorkbenchState>,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let plan = {
        let guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard
            .experiment_plan_draft
            .clone()
            .or_else(|| guard.experiment_plan.clone())
            .unwrap_or_else(default_experiment_plan_draft)
    };
    let default_name = format!(
        "odmr_experiment_plan_{}.json",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    );
    let Some(path) = app
        .dialog()
        .file()
        .set_file_name(&default_name)
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path_string = path.to_string();
    let local_path = PathBuf::from(&path_string);
    write_json_file(&local_path, &plan)?;
    Ok(Some(path_string))
}

#[tauri::command]
pub fn get_device_preset_drafts(
    state: tauri::State<WorkbenchState>,
) -> Result<HashMap<String, Value>, String> {
    let guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    Ok(guard.device_preset_drafts.clone())
}

#[tauri::command]
pub fn set_device_preset_draft(
    state: tauri::State<WorkbenchState>,
    device: String,
    draft: Value,
) -> Result<HashMap<String, Value>, String> {
    let mut guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    guard.device_preset_drafts.insert(device, draft);
    Ok(guard.device_preset_drafts.clone())
}

#[tauri::command]
pub fn get_selected_default_packages(
    state: tauri::State<WorkbenchState>,
) -> Result<HashMap<String, String>, String> {
    let guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    Ok(guard.selected_default_packages.clone())
}

#[tauri::command]
pub fn set_selected_default_package(
    state: tauri::State<WorkbenchState>,
    device: String,
    package_id: String,
) -> Result<HashMap<String, String>, String> {
    let mut guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    guard.selected_default_packages.insert(device, package_id);
    Ok(guard.selected_default_packages.clone())
}

#[tauri::command]
pub fn capture_current_setup_as_preset_draft(
    state: tauri::State<WorkbenchState>,
) -> Result<Value, String> {
    let guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    Ok(serde_json::json!({
        "schema_version": "0.1.0",
        "kind": "device_presets_draft",
        "source": "device_workbench_capture",
        "captured_at": chrono::Local::now().to_rfc3339(),
        "magnetic": {
            "coil_constants_nt_per_ma": guard.mag_coil_constant,
            "runtime_zero_baseline": guard.runtime_zero_baseline,
            "note": "Runtime zero baseline is included for review only; do not persist it as long-lived calibration."
        }
    }))
}

#[tauri::command]
pub fn capture_current_setup_as_plan_draft(
    state: tauri::State<WorkbenchState>,
) -> Result<Value, String> {
    let guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    Ok(serde_json::json!({
        "schema_version": "0.1.0",
        "kind": "experiment_plan",
        "id": format!("manual_capture_{}", chrono::Local::now().format("%Y%m%d_%H%M%S")),
        "preset_refs": {
            "smb100a": "manual_smb100a_current",
            "oe1022d": "manual_oe1022d_current",
            "magnetic": "manual_magnetic_xyz",
            "laser": "manual_laser_current"
        },
        "field_space": {
            "mode": "explicit_points",
            "unit": "nT",
            "points": [[0.0, 0.0, 0.0]]
        },
        "spectrum_template": {
            "rf_sweep": {
                "start_hz": 2800000000.0,
                "stop_hz": 2900000000.0,
                "step_hz": 1000000.0,
                "dwell_ms": 300.0,
                "power_dbm": -30.0,
                "spacing": "LINear",
                "shape": "SAWtooth",
                "sweep_output_start_v": 0.0,
                "sweep_output_stop_v": 3.0
            },
            "oe1022d_acquisition": {
                "mode": "follow_rf_sweep",
                "pre_start_ms": 50.0,
                "post_stop_ms": 50.0,
                "channels": {
                    "ch_a": {
                        "time_constant_s": 0.3,
                        "filter_slope_db_oct": 12.0,
                        "dynamic_reserve": "NORMAL",
                        "sensitivity": "100 mV/nA"
                    },
                    "ch_b": {
                        "time_constant_s": 0.3,
                        "filter_slope_db_oct": 12.0,
                        "dynamic_reserve": "NORMAL",
                        "sensitivity": "100 mV/nA"
                    }
                }
            },
            "laser": {
                "mode": "fixed_power",
                "enabled": false
            }
        },
        "steps": [{
            "step_id": "spectrum_0000",
            "b_target_nt": [0.0, 0.0, 0.0]
        }],
        "runtime_requirements": {
            "require_preflight": true,
            "require_zero_lock": true
        },
        "capture_context": {
            "runtime_zero_baseline_present": guard.runtime_zero_baseline.is_some()
        }
    }))
}

#[tauri::command]
pub fn resolve_plan_with_current_zero(
    state: tauri::State<WorkbenchState>,
) -> Result<ResolvedPlanPreview, String> {
    let (plan, zero_baseline, preflight_passed) = {
        let guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        (
            guard
                .experiment_plan_draft
                .clone()
                .or_else(|| guard.experiment_plan.clone())
                .ok_or("No experiment plan loaded.")?,
            guard.runtime_zero_baseline.clone(),
            guard
                .preflight_report
                .as_ref()
                .map(|r| {
                    r.all_devices_reachable
                        && r.all_identities_verified
                        && r.all_safe_states_confirmed
                })
                .unwrap_or(false)
                || !guard.single_device_connected.is_empty(),
        )
    };

    let points = magnetic_points_from_field_space(&plan);
    let rf_count = rf_point_count(&plan);
    let mut blocked_reasons = Vec::new();
    if zero_baseline.is_none() {
        blocked_reasons.push("runtime zero baseline is missing".into());
    }
    if !preflight_passed {
        blocked_reasons.push("preflight/connection state is not ready".into());
    }

    let magnetic_points = if let Some(zero) = &zero_baseline {
        points
            .iter()
            .take(200)
            .enumerate()
            .map(|(idx, point)| {
                let mut total = HashMap::new();
                let mut recur = HashMap::new();
                for (axis_idx, axis) in ["x", "y", "z"].iter().enumerate() {
                    if let Some(base) = zero.axes.get(*axis) {
                        let recur_a = (point[axis_idx] / base.coil_constant_nt_per_ma) / 1000.0;
                        recur.insert((*axis).to_string(), recur_a);
                        total.insert((*axis).to_string(), base.zero_mean_a + recur_a);
                    }
                }
                ResolvedMagneticPoint {
                    point_index: idx,
                    b_target_nt: *point,
                    computed_total_current_a: total,
                    recurrent_current_a: recur,
                    zero_baseline_ref: Some(zero.session_id.clone()),
                    coil_constant_source: "runtime_zero_baseline".into(),
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    Ok(ResolvedPlanPreview {
        kind: "resolved_plan_preview".into(),
        executable: blocked_reasons.is_empty(),
        blocked_reasons,
        zero_baseline,
        magnetic_points,
        rf_point_count: rf_count,
        estimated_measurements: points.len() * rf_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn points(field_space: Value) -> Vec<[f64; 3]> {
        grouped_scan_entries(&field_space)
            .into_iter()
            .map(|entry| entry.point)
            .collect()
    }

    fn load_example_plan(file_name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .join("examples/experiment_plans")
            .join(file_name);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        serde_json::from_str(&text).unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
    }

    #[test]
    fn grouped_grid_line_scans_one_axis_only() {
        let pts = points(json!({
            "mode": "grouped_grid_scan",
            "groups": [{
                "group_id": "bx_line",
                "axes": ["x"],
                "axis_ranges_nt": { "x": { "start": 0, "stop": 20, "step": 10 } },
                "fixed_axes_nt": { "y": 0, "z": 0 },
                "enabled": true
            }]
        }));
        assert_eq!(
            pts,
            vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [20.0, 0.0, 0.0]]
        );
    }

    #[test]
    fn grouped_grid_plane_uses_cartesian_product() {
        let pts = points(json!({
            "mode": "grouped_grid_scan",
            "groups": [{
                "group_id": "bx_by_plane",
                "axes": ["x", "y"],
                "axis_ranges_nt": {
                    "x": { "start": 0, "stop": 20, "step": 10 },
                    "y": { "start": 0, "stop": 20, "step": 10 }
                },
                "fixed_axes_nt": { "z": 0 },
                "enabled": true
            }]
        }));
        assert_eq!(pts.len(), 9);
        assert!(pts.contains(&[10.0, 0.0, 0.0]));
        assert!(pts.contains(&[0.0, 10.0, 0.0]));
        assert!(pts.contains(&[10.0, 10.0, 0.0]));
    }

    #[test]
    fn grouped_grid_volume_uses_bz_as_inner_loop() {
        let pts = points(json!({
            "mode": "grouped_grid_scan",
            "groups": [{
                "group_id": "volume",
                "axes": ["x", "y", "z"],
                "axis_ranges_nt": {
                    "x": { "start": 0, "stop": 20, "step": 10 },
                    "y": { "start": 0, "stop": 20, "step": 10 },
                    "z": { "start": 0, "stop": 20, "step": 10 }
                },
                "enabled": true
            }]
        }));
        assert_eq!(pts.len(), 27);
        assert_eq!(
            &pts[..4],
            &[
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 10.0],
                [0.0, 0.0, 20.0],
                [0.0, 10.0, 0.0]
            ]
        );
        assert!(pts.contains(&[10.0, 0.0, 0.0]));
        assert!(pts.contains(&[0.0, 10.0, 0.0]));
        assert!(pts.contains(&[0.0, 0.0, 10.0]));
        assert!(pts.contains(&[10.0, 10.0, 10.0]));
    }

    #[test]
    fn legacy_grouped_path_range_imports_as_grid() {
        let pts = points(json!({
            "mode": "grouped_path_scan",
            "groups": [{
                "group_id": "xy_legacy",
                "axes": ["x", "y"],
                "range_nt": { "start": 0, "stop": 20, "step": 10 },
                "fixed_axes_nt": { "z": 0 },
                "enabled": true
            }]
        }));
        assert_eq!(pts.len(), 9);
        assert!(pts.contains(&[10.0, 0.0, 0.0]));
        assert!(pts.contains(&[0.0, 10.0, 0.0]));
        assert!(pts.contains(&[10.0, 10.0, 0.0]));
    }

    #[test]
    fn importable_grid_examples_project_expected_step_counts() {
        let cases = [
            ("odmr_field_grid_1d.example.json", 9),
            ("odmr_field_grid_2d.example.json", 27),
            ("odmr_field_grid_3d.example.json", 27),
        ];
        for (file_name, expected_count) in cases {
            let plan = load_example_plan(file_name);
            let projection = project_plan(&plan);
            assert_eq!(projection.step_row_count, expected_count, "{file_name}");
        }
    }

    #[test]
    fn importable_grid_examples_are_not_diagonal_scans() {
        let two_d = load_example_plan("odmr_field_grid_2d.example.json");
        let two_d_points = points(two_d["field_space"].clone());
        assert!(two_d_points.contains(&[10.0, 0.0, 0.0]));
        assert!(two_d_points.contains(&[0.0, 10.0, 0.0]));
        assert!(two_d_points.contains(&[10.0, 10.0, 0.0]));

        let three_d = load_example_plan("odmr_field_grid_3d.example.json");
        let three_d_points = points(three_d["field_space"].clone());
        assert!(three_d_points.contains(&[10.0, 0.0, 0.0]));
        assert!(three_d_points.contains(&[0.0, 10.0, 0.0]));
        assert!(three_d_points.contains(&[0.0, 0.0, 10.0]));
        assert!(three_d_points.contains(&[10.0, 10.0, 10.0]));
    }

    #[test]
    fn run_readiness_allows_preview_but_blocks_real_missing_requirements() {
        let plan = load_example_plan("odmr_field_grid_1d.example.json");
        let state = WorkbenchState::default();
        let readiness = experiment_run_readiness_for_plan(&state, &plan);

        assert!(readiness.ready_for_preview_execution);
        assert!(!readiness.ready_for_hardware_execution);
        assert_eq!(readiness.step_count, 9);
        assert!(readiness
            .hardware_blocked_reasons
            .iter()
            .any(|reason| reason.contains("smb100a_main is not connected")));
        assert!(readiness
            .hardware_blocked_reasons
            .iter()
            .all(|reason| reason.contains("not connected") || reason.contains("zero")));
    }
}
