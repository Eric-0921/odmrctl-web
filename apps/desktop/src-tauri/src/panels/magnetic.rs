//! Magnetic (Maynuo M8812) device panel — M5C-A
//!
//! Replicates the original GUI control logic:
//!   - zero_bias:   user-set zero-field offset current (A)
//!   - recur_current: reproduction current added on top of zero_bias when lock-zero=ON
//!   - recur_mag:   reproduction magnetic field (nT), converted via coil_constant
//!   - lock_zero:   when ON, zero_bias is frozen and recur_current is added to output
//!   - total output = zero_bias + (lock_zero ? recur_current : 0)
//!
//! Connection init: SYST:REM → VOLT 75 → CURR 0 → OUTP 0
//! Cleanup:         CURR 0 → OUTP 0 → wait 500ms → MEAS:CURR? → verify → SYST:LOC

use super::with_device_access;
use crate::workbench_state::{RuntimeZeroAxisBaseline, RuntimeZeroBaseline, WorkbenchState};
use odmr_maynuo_m8812::{MaynuoM8812Transport, MaynuoSerialPortConfig};
use odmr_types::DeviceId;
use serde::Serialize;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

const POWER_MAX_CURR_A: f64 = 2.0; // M8812 0-2A hardware range.
const AXES: [(&str, &str); 3] = [
    ("x", "maynuo.mag_x"),
    ("y", "maynuo.mag_y"),
    ("z", "maynuo.mag_z"),
];
const DEFAULT_CALIBRATION_SOURCE: &str = "reverse_application/reverse_output/para.xml";

#[derive(Debug, Clone, Serialize)]
pub struct MagneticStatus {
    pub connected: bool,
    pub device_id: String,
    pub output_on: bool,
    pub zero_bias_a: f64,
    pub recur_current_a: f64,
    pub recur_mag_nt: f64,
    pub lock_zero: bool,
    pub total_command_a: f64,
    pub measured_current_a: Option<f64>,
    pub coil_constant_nt_per_ma: f64,
    pub identity: Option<String>,
    pub error_queue: Option<String>,
    pub last_readback_time: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MagneticVectorApplyResult {
    pub b_target_nt: [f64; 3],
    pub runtime_zero_baseline: Option<RuntimeZeroBaseline>,
    pub axes: Vec<MagneticStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MagneticAxisPackageStatus {
    pub axis: String,
    pub device_id: String,
    pub address: String,
    pub expected_sn: String,
    pub observed_idn: Option<String>,
    pub connected: bool,
    pub sn_match: Option<bool>,
    pub coil_constant_nt_per_ma: f64,
    pub zero_bias_a: f64,
    pub runtime_zero_mean_a: Option<f64>,
    pub runtime_zero_std_a: Option<f64>,
    pub lock_zero: bool,
    pub recur_mag_nt: f64,
    pub recur_current_a: f64,
    pub total_command_a: f64,
    pub measured_total_current_a: Option<f64>,
    pub reconstructed_recur_mag_nt: Option<f64>,
    pub output_on: bool,
    pub max_current_a: f64,
    pub blocked_reasons: Vec<String>,
    pub last_readback_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MagneticXyzPackageStatus {
    pub package_id: String,
    pub calibration_source: String,
    pub target_b_nt: [f64; 3],
    pub runtime_zero_baseline: Option<RuntimeZeroBaseline>,
    pub axes: Vec<MagneticAxisPackageStatus>,
    pub ready_to_apply: bool,
    pub blocked_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportedMagneticParaXml {
    pub kind: String,
    pub source_path: String,
    pub calibration_source: String,
    pub coil_constants_nt_per_ma: HashMap<String, f64>,
    pub note: String,
}

fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}

fn stats(samples: &[f64]) -> (f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let var = samples
        .iter()
        .map(|v| {
            let d = *v - mean;
            d * d
        })
        .sum::<f64>()
        / samples.len() as f64;
    (mean, var.sqrt())
}

fn open_maynuo(state: &WorkbenchState, device_id: &str) -> Result<MaynuoM8812Transport, String> {
    let port_path = with_device_access(state, device_id)?;
    let config = MaynuoSerialPortConfig::default();
    MaynuoM8812Transport::open(DeviceId::new(device_id), &port_path, config)
        .map_err(|e| format!("open Maynuo {device_id} @ {port_path}: {e}"))
}

fn expected_sn(axis: &str) -> &'static str {
    match axis {
        "x" => "2020",
        "y" => "2022",
        "z" => "2003",
        _ => "",
    }
}

fn current_axis_status_without_io(
    state: &WorkbenchState,
    axis: &str,
    device_id: &str,
    target_b_nt: f64,
    observed_idn: Option<String>,
) -> MagneticAxisPackageStatus {
    let connected = state.is_accessible(device_id);
    let address = state
        .device_address(device_id)
        .unwrap_or_else(|| "auto".into());
    let coil = state.mag_coil_constant(device_id);
    let zero_bias = state.mag_zero_bias(device_id);
    let recur_current = state.mag_recur_current(device_id);
    let lock_zero = state.mag_lock_zero(device_id);
    let total = state.mag_total_current(device_id);
    let output_on = state.mag_output_on(device_id);
    let baseline = state
        .inner
        .lock()
        .ok()
        .and_then(|g| g.runtime_zero_baseline.clone())
        .and_then(|b| b.axes.get(axis).cloned());
    let sn = expected_sn(axis).to_string();
    let sn_match = observed_idn.as_ref().map(|idn| idn.contains(&sn));
    let mut blocked_reasons = Vec::new();
    if !connected {
        blocked_reasons.push("axis is not connected or locked".into());
    }
    if sn_match == Some(false) {
        blocked_reasons.push("observed IDN does not match expected SN".into());
    }
    if baseline.is_none() {
        blocked_reasons.push("runtime zero baseline is missing".into());
    }
    if !lock_zero {
        blocked_reasons.push("zero is not locked".into());
    }
    if !(0.0..=POWER_MAX_CURR_A).contains(&total) {
        blocked_reasons.push(format!(
            "total current {total:.6} A outside [0, {POWER_MAX_CURR_A}]"
        ));
    }

    MagneticAxisPackageStatus {
        axis: axis.to_string(),
        device_id: device_id.to_string(),
        address,
        expected_sn: sn,
        observed_idn,
        connected,
        sn_match,
        coil_constant_nt_per_ma: coil,
        zero_bias_a: zero_bias,
        runtime_zero_mean_a: baseline.as_ref().map(|b| b.zero_mean_a),
        runtime_zero_std_a: baseline.as_ref().map(|b| b.zero_std_a),
        lock_zero,
        recur_mag_nt: if recur_current == 0.0 {
            target_b_nt
        } else {
            recur_current * 1000.0 * coil
        },
        recur_current_a: recur_current,
        total_command_a: total,
        measured_total_current_a: None,
        reconstructed_recur_mag_nt: None,
        output_on,
        max_current_a: POWER_MAX_CURR_A,
        blocked_reasons,
        last_readback_time: None,
    }
}

fn parse_xml_attr(text: &str, tag: &str, attr: &str) -> Option<f64> {
    let start = text.find(&format!("<{tag}"))?;
    let rest = &text[start..];
    let end = rest.find('>')?;
    let tag_text = &rest[..end];
    let needle = format!("{attr}=\"");
    let attr_start = tag_text.find(&needle)? + needle.len();
    let attr_rest = &tag_text[attr_start..];
    let attr_end = attr_rest.find('"')?;
    attr_rest[..attr_end].parse::<f64>().ok()
}

/// Read current axis state from workbench + hardware.
fn build_status(
    device_id: &str,
    maynuo: &mut MaynuoM8812Transport,
    state: &WorkbenchState,
) -> Result<MagneticStatus, String> {
    let measured = maynuo.query_meas_current().ok();
    let idn = maynuo.query_idn().ok();
    let err = maynuo.query_error().ok();

    let zero_bias = state.mag_zero_bias(device_id);
    let recur_current = state.mag_recur_current(device_id);
    let lock_zero = state.mag_lock_zero(device_id);
    let coil = state.mag_coil_constant(device_id);
    let total = if lock_zero {
        zero_bias + recur_current
    } else {
        zero_bias
    };
    let recur_mag = recur_current * 1000.0 * coil; // A→mA then × nT/mA

    Ok(MagneticStatus {
        connected: true,
        device_id: device_id.to_string(),
        output_on: state.mag_output_on(device_id),
        zero_bias_a: zero_bias,
        recur_current_a: recur_current,
        recur_mag_nt: recur_mag,
        lock_zero,
        total_command_a: total,
        measured_current_a: measured,
        coil_constant_nt_per_ma: coil,
        identity: idn,
        error_queue: err,
        last_readback_time: now_rfc3339(),
    })
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Initialize a magnetic axis: REM → VOLT 75 → CURR 0 → OUTP 0.
/// Mirrors original GUI connection init.
#[tauri::command]
pub fn magnetic_init_axis(
    state: tauri::State<WorkbenchState>,
    device_id: String,
) -> Result<MagneticStatus, String> {
    let mut maynuo = open_maynuo(&state, &device_id)?;
    maynuo
        .send_set_remote()
        .map_err(|e| format!("SYST:REM: {e}"))?;
    thread::sleep(Duration::from_millis(50));
    maynuo
        .send_set_voltage(75)
        .map_err(|e| format!("VOLT 75: {e}"))?;
    thread::sleep(Duration::from_millis(50));
    maynuo
        .send_set_current(0.0)
        .map_err(|e| format!("CURR 0: {e}"))?;
    thread::sleep(Duration::from_millis(50));
    maynuo
        .send_set_output(false)
        .map_err(|e| format!("OUTP 0: {e}"))?;
    thread::sleep(Duration::from_millis(50));

    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.mag_output_on.insert(device_id.clone(), false);
    }

    let mut status = build_status(&device_id, &mut maynuo, &state)?;
    status.output_on = false;
    Ok(status)
}

/// Get full status for an axis.
#[tauri::command]
pub fn magnetic_get_status(
    state: tauri::State<WorkbenchState>,
    device_id: String,
) -> Result<MagneticStatus, String> {
    let mut maynuo = open_maynuo(&state, &device_id)?;
    build_status(&device_id, &mut maynuo, &state)
}

/// Get the operator-facing three-axis magnetic package status.
///
/// This is mostly a state projection for GUI display. It only probes connected
/// axes for readback; disconnected axes are still included with cached state.
#[tauri::command]
pub fn magnetic_get_xyz_package_status(
    state: tauri::State<WorkbenchState>,
    bx_nt: Option<f64>,
    by_nt: Option<f64>,
    bz_nt: Option<f64>,
) -> Result<MagneticXyzPackageStatus, String> {
    let target_b = [
        bx_nt.unwrap_or(0.0),
        by_nt.unwrap_or(0.0),
        bz_nt.unwrap_or(0.0),
    ];
    let runtime_zero_baseline = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?
        .runtime_zero_baseline
        .clone();
    let mut axes = Vec::new();

    for (idx, (axis, device_id)) in AXES.iter().enumerate() {
        let mut observed_idn = None;
        let mut measured = None;
        let mut last_readback = None;
        let mut output_on = false;
        if state.is_accessible(device_id) {
            if let Ok(mut maynuo) = open_maynuo(&state, device_id) {
                if let Ok(status) = build_status(device_id, &mut maynuo, &state) {
                    observed_idn = status.identity.clone();
                    measured = status.measured_current_a;
                    last_readback = Some(status.last_readback_time);
                    output_on = status.output_on;
                }
            }
        }

        let mut axis_status =
            current_axis_status_without_io(&state, axis, device_id, target_b[idx], observed_idn);
        axis_status.measured_total_current_a = measured;
        axis_status.last_readback_time = last_readback;
        axis_status.output_on = output_on;
        axis_status.reconstructed_recur_mag_nt = measured
            .map(|m| (m - axis_status.zero_bias_a) * 1000.0 * axis_status.coil_constant_nt_per_ma);
        axes.push(axis_status);
    }

    let mut blocked_reasons = Vec::new();
    for axis in &axes {
        for reason in &axis.blocked_reasons {
            let message = format!("{}: {}", axis.axis.to_uppercase(), reason);
            if !blocked_reasons.contains(&message) {
                blocked_reasons.push(message);
            }
        }
    }
    let ready_to_apply = blocked_reasons.is_empty();

    Ok(MagneticXyzPackageStatus {
        package_id: "maynuo_m8812_lab_xyz".into(),
        calibration_source: DEFAULT_CALIBRATION_SOURCE.into(),
        target_b_nt: target_b,
        runtime_zero_baseline,
        axes,
        ready_to_apply,
        blocked_reasons,
    })
}

/// Import coil constants from the legacy SimplePowerController para.xml.
///
/// This updates only the in-memory magnetic calibration draft for the current
/// workbench session; station/profile JSON remains unchanged.
#[tauri::command]
pub fn import_magnetic_para_xml(
    state: tauri::State<WorkbenchState>,
    path: String,
) -> Result<ImportedMagneticParaXml, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    let x = parse_xml_attr(&text, "CoilConstant", "X")
        .ok_or_else(|| "CoilConstant X not found in para.xml".to_string())?;
    let y = parse_xml_attr(&text, "CoilConstant", "Y")
        .ok_or_else(|| "CoilConstant Y not found in para.xml".to_string())?;
    let z = parse_xml_attr(&text, "CoilConstant", "Z")
        .ok_or_else(|| "CoilConstant Z not found in para.xml".to_string())?;

    let mut coil_constants = HashMap::new();
    coil_constants.insert("maynuo.mag_x".to_string(), x);
    coil_constants.insert("maynuo.mag_y".to_string(), y);
    coil_constants.insert("maynuo.mag_z".to_string(), z);

    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.mag_coil_constant = coil_constants.clone();
    }

    Ok(ImportedMagneticParaXml {
        kind: "magnetic_para_xml_import_draft".into(),
        source_path: path,
        calibration_source: DEFAULT_CALIBRATION_SOURCE.into(),
        coil_constants_nt_per_ma: coil_constants,
        note: "Imported into current workbench session only; save/export explicitly before using as a long-lived profile.".into(),
    })
}

/// Set the zero-field bias current (A).  This is the base current output when not locked.
/// If output is already ON, the new value is sent immediately.
#[tauri::command]
pub fn magnetic_set_zero_bias(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    bias_a: f64,
    output_on: bool,
) -> Result<MagneticStatus, String> {
    if !(0.0..=POWER_MAX_CURR_A).contains(&bias_a) {
        return Err(format!(
            "Zero bias {bias_a} A out of range [0, {POWER_MAX_CURR_A}]"
        ));
    }

    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.mag_zero_bias.insert(device_id.clone(), bias_a);
        guard.mag_output_on.insert(device_id.clone(), output_on);
    }

    let mut maynuo = open_maynuo(&state, &device_id)?;
    maynuo
        .send_set_remote()
        .map_err(|e| format!("SYST:REM: {e}"))?;

    if output_on {
        let total = state.mag_total_current(&device_id);
        if total > POWER_MAX_CURR_A {
            return Err(format!(
                "Total current {total} A exceeds max {POWER_MAX_CURR_A} A"
            ));
        }
        maynuo
            .send_set_current(total)
            .map_err(|e| format!("CURR {total}: {e}"))?;
        maynuo
            .send_set_output(true)
            .map_err(|e| format!("OUTP 1: {e}"))?;
    } else {
        maynuo
            .send_set_current(bias_a)
            .map_err(|e| format!("CURR {bias_a}: {e}"))?;
    }

    thread::sleep(Duration::from_millis(200));
    let mut status = build_status(&device_id, &mut maynuo, &state)?;
    status.output_on = output_on;
    Ok(status)
}

/// Set the recurrent (reproduction) current (A).
/// Only has hardware effect when lock_zero=ON and output=ON.
#[tauri::command]
pub fn magnetic_set_recur_current(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    recur_a: f64,
    output_on: bool,
) -> Result<MagneticStatus, String> {
    if !(0.0..=POWER_MAX_CURR_A).contains(&recur_a) {
        return Err(format!(
            "Recur current {recur_a} A out of range [0, {POWER_MAX_CURR_A}]"
        ));
    }

    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.mag_recur_current.insert(device_id.clone(), recur_a);
        guard.mag_output_on.insert(device_id.clone(), output_on);
    }

    let mut maynuo = open_maynuo(&state, &device_id)?;
    maynuo
        .send_set_remote()
        .map_err(|e| format!("SYST:REM: {e}"))?;

    if output_on && state.mag_lock_zero(&device_id) {
        let total = state.mag_total_current(&device_id);
        if total > POWER_MAX_CURR_A {
            // Auto-limit like original GUI
            let limited_recur = POWER_MAX_CURR_A - state.mag_zero_bias(&device_id);
            let mut guard = state
                .inner
                .lock()
                .map_err(|e| format!("lock poison: {e}"))?;
            guard
                .mag_recur_current
                .insert(device_id.clone(), limited_recur.max(0.0));
            maynuo
                .send_set_current(POWER_MAX_CURR_A)
                .map_err(|e| format!("CURR {POWER_MAX_CURR_A}: {e}"))?;
        } else {
            maynuo
                .send_set_current(total)
                .map_err(|e| format!("CURR {total}: {e}"))?;
        }
        maynuo
            .send_set_output(true)
            .map_err(|e| format!("OUTP 1: {e}"))?;
    }

    thread::sleep(Duration::from_millis(200));
    let mut status = build_status(&device_id, &mut maynuo, &state)?;
    status.output_on = output_on;
    Ok(status)
}

/// Set recurrent magnetic field (nT).  Converts to current via coil_constant.
#[tauri::command]
pub fn magnetic_set_recur_mag(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    mag_nt: f64,
    output_on: bool,
) -> Result<MagneticStatus, String> {
    let coil = state.mag_coil_constant(&device_id);
    let recur_ma = mag_nt / coil; // nT / (nT/mA) = mA
    let recur_a = recur_ma / 1000.0;
    magnetic_set_recur_current(state, device_id, recur_a, output_on)
}

/// Toggle output ON/OFF.  When turning ON, sends the appropriate total current.
#[tauri::command]
pub fn magnetic_toggle_output(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    on: bool,
) -> Result<MagneticStatus, String> {
    let mut maynuo = open_maynuo(&state, &device_id)?;
    maynuo
        .send_set_remote()
        .map_err(|e| format!("SYST:REM: {e}"))?;

    if on {
        let total = state.mag_total_current(&device_id);
        if total > POWER_MAX_CURR_A {
            return Err(format!(
                "Total current {total} A exceeds max {POWER_MAX_CURR_A} A"
            ));
        }
        maynuo
            .send_set_current(total)
            .map_err(|e| format!("CURR {total}: {e}"))?;
        maynuo
            .send_set_output(true)
            .map_err(|e| format!("OUTP 1: {e}"))?;
    } else {
        maynuo
            .send_set_current(0.0)
            .map_err(|e| format!("CURR 0: {e}"))?;
        maynuo
            .send_set_output(false)
            .map_err(|e| format!("OUTP 0: {e}"))?;
    }

    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.mag_output_on.insert(device_id.clone(), on);
    }

    thread::sleep(Duration::from_millis(200));
    let mut status = build_status(&device_id, &mut maynuo, &state)?;
    status.output_on = on;
    Ok(status)
}

/// Toggle lock-zero ON/OFF.
/// ON  → freeze zero_bias, enable recur_current, send total = zero_bias + recur
/// OFF → unlock zero_bias, send total = zero_bias
#[tauri::command]
pub fn magnetic_toggle_lock_zero(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    lock: bool,
    output_on: bool,
) -> Result<MagneticStatus, String> {
    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.mag_lock_zero.insert(device_id.clone(), lock);
        guard.mag_output_on.insert(device_id.clone(), output_on);
    }

    let mut maynuo = open_maynuo(&state, &device_id)?;
    maynuo
        .send_set_remote()
        .map_err(|e| format!("SYST:REM: {e}"))?;

    if output_on {
        let total = state.mag_total_current(&device_id);
        if total > POWER_MAX_CURR_A {
            return Err(format!(
                "Total current {total} A exceeds max {POWER_MAX_CURR_A} A"
            ));
        }
        maynuo
            .send_set_current(total)
            .map_err(|e| format!("CURR {total}: {e}"))?;
        maynuo
            .send_set_output(true)
            .map_err(|e| format!("OUTP 1: {e}"))?;
    } else {
        let bias = state.mag_zero_bias(&device_id);
        maynuo
            .send_set_current(bias)
            .map_err(|e| format!("CURR {bias}: {e}"))?;
    }

    thread::sleep(Duration::from_millis(200));
    let mut status = build_status(&device_id, &mut maynuo, &state)?;
    status.output_on = output_on;
    Ok(status)
}

/// Safe cleanup: zero current, turn off output, wait, verify, return to local.
/// Also clears all per-axis state (zero_bias, recur_current, lock_zero).
#[tauri::command]
pub fn magnetic_safe_cleanup(
    state: tauri::State<WorkbenchState>,
    device_id: String,
) -> Result<MagneticStatus, String> {
    let mut maynuo = open_maynuo(&state, &device_id)?;
    maynuo
        .send_set_remote()
        .map_err(|e| format!("SYST:REM: {e}"))?;
    maynuo
        .send_set_current(0.0)
        .map_err(|e| format!("CURR 0: {e}"))?;
    maynuo
        .send_set_output(false)
        .map_err(|e| format!("OUTP 0: {e}"))?;

    thread::sleep(Duration::from_millis(500));

    let measured = maynuo.query_meas_current().ok();
    maynuo
        .send_set_local()
        .map_err(|e| format!("SYST:LOC: {e}"))?;

    // Clear axis state
    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.mag_zero_bias.remove(&device_id);
        guard.mag_recur_current.remove(&device_id);
        guard.mag_lock_zero.remove(&device_id);
        guard.mag_output_on.insert(device_id.clone(), false);
    }

    Ok(MagneticStatus {
        connected: true,
        device_id: device_id.clone(),
        output_on: false,
        zero_bias_a: 0.0,
        recur_current_a: 0.0,
        recur_mag_nt: 0.0,
        lock_zero: false,
        total_command_a: 0.0,
        measured_current_a: measured,
        coil_constant_nt_per_ma: state.mag_coil_constant(&device_id),
        identity: None,
        error_queue: None,
        last_readback_time: now_rfc3339(),
    })
}

// ---------------------------------------------------------------------------
// Three-axis aggregate commands
// ---------------------------------------------------------------------------

/// Initialize all magnetic axes using the verified safe init sequence.
#[tauri::command]
pub fn magnetic_init_all(
    state: tauri::State<WorkbenchState>,
) -> Result<Vec<MagneticStatus>, String> {
    let mut out = Vec::new();
    for (_, device_id) in AXES {
        out.push(magnetic_init_axis(state.clone(), device_id.to_string())?);
    }
    Ok(out)
}

/// Measure zero current on all axes and store a runtime zero-baseline artifact.
///
/// Sequence per axis:
/// SYST:REM → VOLT 75 → CURR 0 → OUTP 1 → wait → MEAS:CURR? × N.
#[tauri::command]
pub fn magnetic_measure_zero_all(
    state: tauri::State<WorkbenchState>,
    samples_per_axis: Option<u8>,
) -> Result<RuntimeZeroBaseline, String> {
    let count = samples_per_axis.unwrap_or(5).clamp(1, 20);
    let mut axes = HashMap::new();

    for (axis, device_id) in AXES {
        let mut maynuo = open_maynuo(&state, device_id)?;
        maynuo
            .send_set_remote()
            .map_err(|e| format!("{device_id} SYST:REM: {e}"))?;
        thread::sleep(Duration::from_millis(50));
        maynuo
            .send_set_voltage(75)
            .map_err(|e| format!("{device_id} VOLT 75: {e}"))?;
        thread::sleep(Duration::from_millis(50));
        maynuo
            .send_set_current(0.0)
            .map_err(|e| format!("{device_id} CURR 0: {e}"))?;
        thread::sleep(Duration::from_millis(50));
        maynuo
            .send_set_output(true)
            .map_err(|e| format!("{device_id} OUTP 1: {e}"))?;
        thread::sleep(Duration::from_millis(2000));

        let mut samples = Vec::new();
        for _ in 0..count {
            let sample = maynuo
                .query_meas_current()
                .map_err(|e| format!("{device_id} MEAS:CURR?: {e}"))?;
            samples.push(sample);
            thread::sleep(Duration::from_millis(100));
        }
        let identity = maynuo.query_idn().ok();
        let (mean, std) = stats(&samples);
        let coil = state.mag_coil_constant(device_id);

        axes.insert(
            axis.to_string(),
            RuntimeZeroAxisBaseline {
                device_id: device_id.to_string(),
                axis: axis.to_string(),
                identity,
                zero_samples_a: samples,
                zero_mean_a: mean,
                zero_std_a: std,
                coil_constant_nt_per_ma: coil,
            },
        );
    }

    let baseline = RuntimeZeroBaseline {
        schema_version: "0.1.0".into(),
        kind: "runtime_zero_baseline".into(),
        session_id: format!("zero_{}", chrono::Local::now().format("%Y%m%d_%H%M%S")),
        locked_at: now_rfc3339(),
        axes,
    };

    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.runtime_zero_baseline = Some(baseline.clone());
        for (_, device_id) in AXES {
            guard.mag_lock_zero.insert(device_id.to_string(), false);
            guard.mag_recur_current.insert(device_id.to_string(), 0.0);
            guard.mag_output_on.insert(device_id.to_string(), true);
        }
    }

    Ok(baseline)
}

/// Lock the previously measured runtime zero baseline for all axes.
#[tauri::command]
pub fn magnetic_lock_zero_all(
    state: tauri::State<WorkbenchState>,
) -> Result<RuntimeZeroBaseline, String> {
    let baseline = {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        let baseline = guard
            .runtime_zero_baseline
            .clone()
            .ok_or("No runtime zero baseline measured. Run Measure Zero All first.")?;
        for (_, device_id) in AXES {
            guard.mag_lock_zero.insert(device_id.to_string(), true);
        }
        baseline
    };
    Ok(baseline)
}

/// Apply a target magnetic field vector using the locked runtime zero baseline.
#[tauri::command]
pub fn magnetic_apply_vector_field(
    state: tauri::State<WorkbenchState>,
    bx_nt: f64,
    by_nt: f64,
    bz_nt: f64,
) -> Result<MagneticVectorApplyResult, String> {
    let b_target = [bx_nt, by_nt, bz_nt];
    let baseline = {
        let guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard
            .runtime_zero_baseline
            .clone()
            .ok_or("No runtime zero baseline measured. Run Measure Zero All first.")?
    };

    let mut statuses = Vec::new();
    for (axis_index, (axis, device_id)) in AXES.iter().enumerate() {
        if !state.mag_lock_zero(device_id) {
            return Err(format!("{device_id} is not zero-locked."));
        }

        let axis_baseline = baseline
            .axes
            .get(*axis)
            .ok_or_else(|| format!("Runtime zero baseline missing axis {axis}"))?;
        let coil = state.mag_coil_constant(device_id);
        let recur_ma = b_target[axis_index] / coil;
        let recur_a = recur_ma / 1000.0;
        let total = axis_baseline.zero_mean_a + recur_a;

        if !(0.0..=POWER_MAX_CURR_A).contains(&total) {
            return Err(format!(
                "{device_id} total current {total:.6} A is outside [0, {POWER_MAX_CURR_A}]"
            ));
        }

        {
            let mut guard = state
                .inner
                .lock()
                .map_err(|e| format!("lock poison: {e}"))?;
            guard
                .mag_zero_bias
                .insert(device_id.to_string(), axis_baseline.zero_mean_a);
            guard
                .mag_recur_current
                .insert(device_id.to_string(), recur_a);
            guard.mag_lock_zero.insert(device_id.to_string(), true);
            guard.mag_output_on.insert(device_id.to_string(), true);
        }

        let mut maynuo = open_maynuo(&state, device_id)?;
        maynuo
            .send_set_remote()
            .map_err(|e| format!("{device_id} SYST:REM: {e}"))?;
        maynuo
            .send_set_current(total)
            .map_err(|e| format!("{device_id} CURR {total}: {e}"))?;
        maynuo
            .send_set_output(true)
            .map_err(|e| format!("{device_id} OUTP 1: {e}"))?;
        thread::sleep(Duration::from_millis(500));
        let mut status = build_status(device_id, &mut maynuo, &state)?;
        status.output_on = true;
        statuses.push(status);
    }

    Ok(MagneticVectorApplyResult {
        b_target_nt: b_target,
        runtime_zero_baseline: Some(baseline),
        axes: statuses,
    })
}

/// Safe cleanup all three magnetic axes and clear runtime zero state.
#[tauri::command]
pub fn magnetic_cleanup_all(
    state: tauri::State<WorkbenchState>,
) -> Result<Vec<MagneticStatus>, String> {
    let mut statuses = Vec::new();
    for (_, device_id) in AXES {
        statuses.push(magnetic_safe_cleanup(state.clone(), device_id.to_string())?);
    }
    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.runtime_zero_baseline = None;
    }
    Ok(statuses)
}
