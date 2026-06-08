//! SMB100A minimal device panel — M5C-A
//!
//! All commands require the device to be locked via preflight first.
//! Every set command checks SYST:ERR? after execution.

use super::{scpi_query, scpi_set, smb_connect, with_device_access};
use crate::workbench_state::WorkbenchState;
use odmr_smb100a::commands::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Smb100aStatus {
    pub connected: bool,
    pub frequency_hz: Option<f64>,
    pub power_dbm: Option<f64>,
    pub output_on: Option<bool>,
    pub modulation_on: Option<bool>,
    pub fm_enabled: Option<bool>,
    pub fm_deviation_hz: Option<f64>,
    pub lf_frequency_hz: Option<f64>,
    pub lf_voltage_v: Option<f64>,
    pub lf_output_on: Option<bool>,
    pub lf_shape: Option<String>,
    pub error_queue: Vec<String>,
    pub last_readback_time: String,
}

fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim() {
        "1" | "ON" | "on" => Some(true),
        "0" | "OFF" | "off" => Some(false),
        _ => None,
    }
}

/// Drain the SCPI error queue (up to 10 entries) and return them.
fn drain_error_queue(stream: &mut std::net::TcpStream) -> Vec<String> {
    let mut errors = Vec::new();
    for _ in 0..10 {
        match scpi_query(stream, "SYST:ERR?") {
            Ok(resp) if resp.trim() == "0" || resp.trim().starts_with('0') => break,
            Ok(resp) => errors.push(resp),
            Err(_) => break,
        }
    }
    errors
}

fn read_full_status(stream: &mut std::net::TcpStream) -> Result<Smb100aStatus, String> {
    let freq = scpi_query(stream, query_frequency())
        .ok()
        .and_then(|s| s.parse().ok());
    let power = scpi_query(stream, query_power())
        .ok()
        .and_then(|s| s.trim().replace("dBm", "").parse().ok());
    let output = scpi_query(stream, query_output())
        .ok()
        .and_then(|s| parse_bool(&s));
    let mod_on = scpi_query(stream, query_modulation_global())
        .ok()
        .and_then(|s| parse_bool(&s));
    let fm_on = scpi_query(stream, query_fm_state())
        .ok()
        .and_then(|s| parse_bool(&s));
    let fm_dev = scpi_query(stream, query_fm_deviation())
        .ok()
        .and_then(|s| s.trim().replace("Hz", "").parse().ok());
    let lf_freq = scpi_query(stream, query_lf_frequency())
        .ok()
        .and_then(|s| s.trim().replace("Hz", "").parse().ok());
    let lf_volt = scpi_query(stream, query_lf_voltage())
        .ok()
        .and_then(|s| s.trim().replace("V", "").parse().ok());
    let lf_out = scpi_query(stream, query_lf_output())
        .ok()
        .and_then(|s| parse_bool(&s));
    let lf_shp = scpi_query(stream, query_lf_shape()).ok();
    let errors = drain_error_queue(stream);

    Ok(Smb100aStatus {
        connected: true,
        frequency_hz: freq,
        power_dbm: power,
        output_on: output,
        modulation_on: mod_on,
        fm_enabled: fm_on,
        fm_deviation_hz: fm_dev,
        lf_frequency_hz: lf_freq,
        lf_voltage_v: lf_volt,
        lf_output_on: lf_out,
        lf_shape: lf_shp,
        error_queue: errors,
        last_readback_time: now_rfc3339(),
    })
}

#[tauri::command]
pub fn smb100a_get_status(
    state: tauri::State<WorkbenchState>,
    device_id: String,
) -> Result<Smb100aStatus, String> {
    let address = with_device_access(&state, &device_id)?;
    let mut stream = smb_connect(&address)?;
    let status = read_full_status(&mut stream)?;
    Ok(status)
}

#[tauri::command]
pub fn smb100a_set_frequency(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    hz: f64,
) -> Result<Smb100aStatus, String> {
    let safety = state.safety();
    if hz < safety.smb100a_min_freq_hz || hz > safety.smb100a_max_freq_hz {
        return Err(format!(
            "Frequency {hz} Hz outside safety limits [{}, {}] Hz",
            safety.smb100a_min_freq_hz, safety.smb100a_max_freq_hz
        ));
    }
    let address = with_device_access(&state, &device_id)?;
    let mut stream = smb_connect(&address)?;
    scpi_set(&mut stream, &set_frequency_hz(hz))?;
    read_full_status(&mut stream)
}

#[tauri::command]
pub fn smb100a_set_power(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    dbm: f64,
) -> Result<Smb100aStatus, String> {
    let safety = state.safety();
    if dbm > safety.smb100a_max_power_dbm {
        return Err(format!(
            "Power {dbm} dBm exceeds safety limit {} dBm",
            safety.smb100a_max_power_dbm
        ));
    }
    let address = with_device_access(&state, &device_id)?;
    let mut stream = smb_connect(&address)?;
    scpi_set(&mut stream, &set_power_dbm(dbm))?;
    read_full_status(&mut stream)
}

#[tauri::command]
pub fn smb100a_set_output(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    on: bool,
) -> Result<Smb100aStatus, String> {
    let safety = state.safety();
    let address = with_device_access(&state, &device_id)?;
    let mut stream = smb_connect(&address)?;

    if on {
        let freq = scpi_query(&mut stream, query_frequency())
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let power = scpi_query(&mut stream, query_power())
            .ok()
            .and_then(|s| s.trim().replace("dBm", "").parse::<f64>().ok())
            .unwrap_or(0.0);
        if freq < safety.smb100a_min_freq_hz || freq > safety.smb100a_max_freq_hz {
            return Err("RF output ON blocked: frequency outside safety limits".into());
        }
        if power > safety.smb100a_max_power_dbm {
            return Err("RF output ON blocked: power exceeds safety limit".into());
        }
    }

    scpi_set(&mut stream, set_output(on))?;
    read_full_status(&mut stream)
}

#[tauri::command]
pub fn smb100a_set_fm(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    enabled: bool,
    deviation_hz: f64,
) -> Result<Smb100aStatus, String> {
    let address = with_device_access(&state, &device_id)?;
    let mut stream = smb_connect(&address)?;
    scpi_set(&mut stream, set_fm_state(enabled))?;
    if enabled {
        scpi_set(&mut stream, &set_fm_deviation_hz(deviation_hz))?;
    }
    read_full_status(&mut stream)
}

#[tauri::command]
pub fn smb100a_set_lf(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    frequency_hz: f64,
    voltage_v: f64,
    output_on: bool,
) -> Result<Smb100aStatus, String> {
    let address = with_device_access(&state, &device_id)?;
    let mut stream = smb_connect(&address)?;
    scpi_set(&mut stream, &set_lf_frequency_hz(frequency_hz))?;
    scpi_set(&mut stream, &set_lf_voltage_v(voltage_v))?;
    scpi_set(&mut stream, set_lf_output(output_on))?;
    read_full_status(&mut stream)
}

/// Apply safe default configuration: RF OFF, MOD OFF, FM OFF, freq at min safe, power at max safe.
#[tauri::command]
pub fn smb100a_apply_safe_config(
    state: tauri::State<WorkbenchState>,
    device_id: String,
) -> Result<Smb100aStatus, String> {
    let safety = state.safety();
    let address = with_device_access(&state, &device_id)?;
    let mut stream = smb_connect(&address)?;
    scpi_set(&mut stream, &set_frequency_hz(safety.smb100a_min_freq_hz))?;
    scpi_set(&mut stream, &set_power_dbm(safety.smb100a_max_power_dbm))?;
    scpi_set(&mut stream, set_output(false))?;
    scpi_set(&mut stream, set_modulation_global(false))?;
    scpi_set(&mut stream, set_fm_state(false))?;
    read_full_status(&mut stream)
}
