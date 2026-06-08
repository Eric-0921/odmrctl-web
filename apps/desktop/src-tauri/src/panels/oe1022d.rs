//! OE1022D minimal device panel — M5C-A
//!
//! All commands require the device to be locked via preflight first.
//! Critical: clear(Input) before every serial command (already done in serial_query_ascii).

use super::{serial_open, serial_query_ascii, with_device_access};
use crate::workbench_state::WorkbenchState;
use odmr_oe1022d::commands::*;
use serde::Serialize;

const CH: u8 = 2; // Primary channel B for V1
const BAUD: u32 = 921_600;

#[derive(Debug, Clone, Serialize)]
pub struct Oe1022dStatus {
    pub connected: bool,
    pub reference_source: Option<String>,
    pub ref_slope: Option<String>,
    pub phase_deg: Option<f64>,
    pub time_constant_s: Option<f64>,
    pub filter_slope_db_oct: Option<u8>,
    pub input_coupling: Option<String>,
    pub input_notch: Option<String>,
    pub input_overload: Option<bool>,
    pub gain_overload: Option<bool>,
    pub pll_locked: Option<bool>,
    pub last_readback_time: String,
}

fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}

fn tc_index_to_seconds(index: u8) -> f64 {
    match index {
        0 => 10e-6,
        1 => 30e-6,
        2 => 100e-6,
        3 => 300e-6,
        4 => 1e-3,
        5 => 3e-3,
        6 => 10e-3,
        7 => 30e-3,
        8 => 100e-3,
        9 => 300e-3,
        10 => 1.0,
        11 => 3.0,
        12 => 10.0,
        13 => 30.0,
        14 => 100.0,
        15 => 300.0,
        16 => 1_000.0,
        17 => 3_000.0,
        18 => 10_000.0,
        19 => 30_000.0,
        _ => 1.0,
    }
}

fn tc_seconds_to_index(seconds: f64) -> u8 {
    let table: [(f64, u8); 20] = [
        (10e-6, 0),
        (30e-6, 1),
        (100e-6, 2),
        (300e-6, 3),
        (1e-3, 4),
        (3e-3, 5),
        (10e-3, 6),
        (30e-3, 7),
        (100e-3, 8),
        (300e-3, 9),
        (1.0, 10),
        (3.0, 11),
        (10.0, 12),
        (30.0, 13),
        (100.0, 14),
        (300.0, 15),
        (1_000.0, 16),
        (3_000.0, 17),
        (10_000.0, 18),
        (30_000.0, 19),
    ];
    let mut best = 10u8;
    let mut best_diff = (seconds - 1.0).abs();
    for (val, idx) in table {
        let diff = (seconds - val).abs();
        if diff < best_diff {
            best_diff = diff;
            best = idx;
        }
    }
    best
}

fn ref_source_name(code: u8) -> &'static str {
    match code {
        0 => "External",
        1 => "Internal",
        2 => "Internal Sweep",
        _ => "Unknown",
    }
}

fn ref_slope_name(code: u8) -> &'static str {
    match code {
        0 => "TTL Rising Edge",
        1 => "Sine Zero Crossing",
        _ => "Unknown",
    }
}

fn parse_bool_oe(s: &str) -> Option<bool> {
    match s.trim() {
        "1" | "YES" | "yes" => Some(true),
        "0" | "NO" | "no" => Some(false),
        _ => None,
    }
}

fn read_full_status(port: &mut Box<dyn serialport::SerialPort>) -> Result<Oe1022dStatus, String> {
    let ref_src = serial_query_ascii(port, &query_reference_source(CH))
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .map(ref_source_name)
        .map(String::from);
    let ref_sl = serial_query_ascii(port, &query_ref_slope(CH))
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .map(ref_slope_name)
        .map(String::from);
    let phase = serial_query_ascii(port, &query_phase(CH))
        .ok()
        .and_then(|s| s.trim().parse().ok());
    let tc_idx = serial_query_ascii(port, &query_time_constant(CH))
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok());
    let tc_s = tc_idx.map(tc_index_to_seconds);
    let slope = serial_query_ascii(port, &query_filter_slope(CH))
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok());
    let coupling = serial_query_ascii(port, &query_input_coupling(CH))
        .ok()
        .map(|s| {
            match s.trim() {
                "0" => "AC",
                "1" => "DC",
                _ => "Unknown",
            }
            .to_string()
        });
    let notch = serial_query_ascii(port, &query_line_notch_filter(CH))
        .ok()
        .map(|s| {
            match s.trim() {
                "0" => "Off",
                "1" => "50Hz",
                "2" => "100Hz",
                "3" => "Both",
                _ => "Unknown",
            }
            .to_string()
        });
    let in_ov = serial_query_ascii(port, &query_input_overload(CH))
        .ok()
        .and_then(|s| parse_bool_oe(&s));
    let gain_ov = serial_query_ascii(port, &query_gain_overload(CH))
        .ok()
        .and_then(|s| parse_bool_oe(&s));
    let pll = serial_query_ascii(port, &query_pll_locked(CH))
        .ok()
        .and_then(|s| parse_bool_oe(&s));

    Ok(Oe1022dStatus {
        connected: true,
        reference_source: ref_src,
        ref_slope: ref_sl,
        phase_deg: phase,
        time_constant_s: tc_s,
        filter_slope_db_oct: slope,
        input_coupling: coupling,
        input_notch: notch,
        input_overload: in_ov,
        gain_overload: gain_ov,
        pll_locked: pll,
        last_readback_time: now_rfc3339(),
    })
}

#[tauri::command]
pub fn oe1022d_get_status(
    state: tauri::State<WorkbenchState>,
    device_id: String,
) -> Result<Oe1022dStatus, String> {
    let port_path = with_device_access(&state, &device_id)?;
    let mut port = serial_open(&port_path, BAUD)?;
    read_full_status(&mut port)
}

#[tauri::command]
pub fn oe1022d_set_filter(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    time_constant_s: f64,
    slope_db_oct: u8,
) -> Result<Oe1022dStatus, String> {
    let port_path = with_device_access(&state, &device_id)?;
    let mut port = serial_open(&port_path, BAUD)?;
    let tc_idx = tc_seconds_to_index(time_constant_s);
    let slope_idx = match slope_db_oct {
        6 => 0u8,
        12 => 1,
        18 => 2,
        24 => 3,
        _ => 1,
    };
    let _ = serial_query_ascii(&mut port, &set_time_constant(CH, tc_idx));
    let _ = serial_query_ascii(&mut port, &set_filter_slope(CH, slope_idx));
    read_full_status(&mut port)
}

#[tauri::command]
pub fn oe1022d_set_reference(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    source: String,
    phase_deg: f64,
) -> Result<Oe1022dStatus, String> {
    let port_path = with_device_access(&state, &device_id)?;
    let mut port = serial_open(&port_path, BAUD)?;
    let source_idx = match source.as_str() {
        "External" => 0u8,
        "Internal" => 1,
        "Internal Sweep" => 2,
        _ => 0,
    };
    let _ = serial_query_ascii(&mut port, &set_reference_source(CH, source_idx));
    let _ = serial_query_ascii(&mut port, &set_phase_deg(CH, phase_deg));
    read_full_status(&mut port)
}

/// Apply OE1022D default configuration: External ref, 1s TC, 12dB/oct, AC coupling, Both notch.
#[tauri::command]
pub fn oe1022d_apply_default_config(
    state: tauri::State<WorkbenchState>,
    device_id: String,
) -> Result<Oe1022dStatus, String> {
    let port_path = with_device_access(&state, &device_id)?;
    let mut port = serial_open(&port_path, BAUD)?;
    let tc_idx = tc_seconds_to_index(1.0); // 1s
    let slope_idx = 1u8; // 12 dB/oct
    let _ = serial_query_ascii(&mut port, &set_reference_source(CH, 0)); // External
    let _ = serial_query_ascii(&mut port, &set_time_constant(CH, tc_idx));
    let _ = serial_query_ascii(&mut port, &set_filter_slope(CH, slope_idx));
    let _ = serial_query_ascii(&mut port, &set_input_coupling(CH, 0)); // AC
    let _ = serial_query_ascii(&mut port, &set_line_notch_filter(CH, 3)); // Both
    read_full_status(&mut port)
}

/// Auto phase adjustment (placeholder — no auto-phase SCPI command available in current driver).
#[tauri::command]
pub fn oe1022d_auto_phase(
    state: tauri::State<WorkbenchState>,
    device_id: String,
) -> Result<Oe1022dStatus, String> {
    let port_path = with_device_access(&state, &device_id)?;
    let mut port = serial_open(&port_path, BAUD)?;
    // OE1022D does not expose an auto-phase command in the documented protocol.
    // Manual phase adjustment via set_phase_deg is available instead.
    read_full_status(&mut port)
}
