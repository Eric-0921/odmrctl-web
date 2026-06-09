//! OE1022D minimal device panel — M5C-A
//!
//! All commands require the device to be locked via preflight first.
//! Critical: clear(Input) before every serial command (already done in serial_query_ascii).

use super::{serial_open, serial_query_ascii, with_device_access};
use crate::workbench_state::WorkbenchState;
use odmr_oe1022d::commands::*;
use serde::{Deserialize, Serialize};

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
    pub input_source: Option<String>,
    pub input_grounding: Option<String>,
    pub input_coupling: Option<String>,
    pub input_notch: Option<String>,
    pub dynamic_reserve: Option<String>,
    pub sensitivity_index: Option<u8>,
    pub sync_filter_on: Option<bool>,
    pub input_overload: Option<bool>,
    pub gain_overload: Option<bool>,
    pub pll_locked: Option<bool>,
    pub last_readback_time: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Oe1022dChBApplyConfig {
    pub reference_source: String,
    pub external_ref_trigger: String,
    pub phase_deg: f64,
    pub input_source: String,
    pub input_grounding: String,
    pub input_coupling: String,
    pub input_notch: String,
    pub dynamic_reserve: String,
    pub sensitivity: String,
    pub time_constant_s: f64,
    pub slope_db_oct: u8,
    pub sync_filter_on: bool,
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

fn input_source_name(code: u8) -> &'static str {
    match code {
        0 => "Single-Ended Voltage",
        1 => "Differential Voltage",
        2 => "Current 1M",
        3 => "Current 100M",
        _ => "Unknown",
    }
}

fn input_grounding_name(code: u8) -> &'static str {
    match code {
        0 => "Float",
        1 => "Ground",
        _ => "Unknown",
    }
}

fn dynamic_reserve_name(code: u8) -> &'static str {
    match code {
        0 => "Low Noise",
        1 => "Normal",
        2 => "High Reserve",
        _ => "Unknown",
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
    let input_source = serial_query_ascii(port, &query_input_source(CH))
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .map(input_source_name)
        .map(String::from);
    let input_grounding = serial_query_ascii(port, &query_input_grounding(CH))
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .map(input_grounding_name)
        .map(String::from);
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
    let dynamic_reserve = serial_query_ascii(port, &query_dynamic_reserve(CH))
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .map(dynamic_reserve_name)
        .map(String::from);
    let sensitivity_index = serial_query_ascii(port, &query_sensitivity(CH))
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok());
    let sync_filter_on = serial_query_ascii(port, &query_sync_filter(CH))
        .ok()
        .and_then(|s| parse_bool_oe(&s));
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
        input_source,
        input_grounding,
        input_coupling: coupling,
        input_notch: notch,
        dynamic_reserve,
        sensitivity_index,
        sync_filter_on,
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

#[tauri::command]
pub fn oe1022d_apply_ch_b_config(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    config: Oe1022dChBApplyConfig,
) -> Result<Oe1022dStatus, String> {
    let port_path = with_device_access(&state, &device_id)?;
    let mut port = serial_open(&port_path, BAUD)?;
    apply_ch_b_config(&mut port, &config)?;
    read_full_status(&mut port)
}

/// Apply OE1022D default configuration: External TTL ref, 300ms TC, 12dB/oct, AC coupling, notch off.
#[tauri::command]
pub fn oe1022d_apply_default_config(
    state: tauri::State<WorkbenchState>,
    device_id: String,
) -> Result<Oe1022dStatus, String> {
    let port_path = with_device_access(&state, &device_id)?;
    let mut port = serial_open(&port_path, BAUD)?;
    let tc_idx = tc_seconds_to_index(0.3); // 300 ms
    let slope_idx = 1u8; // 12 dB/oct
    let _ = serial_query_ascii(&mut port, &set_reference_source(CH, 0)); // External
    let _ = serial_query_ascii(&mut port, &set_ref_slope(CH, 0)); // TTL rising
    let _ = serial_query_ascii(&mut port, &set_phase_deg(CH, 0.0));
    let _ = serial_query_ascii(&mut port, &set_input_source(CH, 0)); // Single-ended voltage
    let _ = serial_query_ascii(&mut port, &set_input_grounding(CH, 0)); // Float
    let _ = serial_query_ascii(&mut port, &set_time_constant(CH, tc_idx));
    let _ = serial_query_ascii(&mut port, &set_filter_slope(CH, slope_idx));
    let _ = serial_query_ascii(&mut port, &set_input_coupling(CH, 0)); // AC
    let _ = serial_query_ascii(&mut port, &set_line_notch_filter(CH, 0)); // Off
    let _ = serial_query_ascii(&mut port, &set_dynamic_reserve(CH, 1)); // Normal
    let _ = serial_query_ascii(&mut port, &set_sensitivity(CH, 24)); // 100 mV/nA
    let _ = serial_query_ascii(&mut port, &set_sync_filter(CH, 0)); // Off at 500 Hz default ref
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

fn apply_ch_b_config(
    port: &mut Box<dyn serialport::SerialPort>,
    config: &Oe1022dChBApplyConfig,
) -> Result<(), String> {
    let source_idx = normalize_reference_source(&config.reference_source)?;
    let ref_slope_idx = normalize_ref_slope(&config.external_ref_trigger)?;
    let input_source_idx = normalize_input_source(&config.input_source)?;
    let grounding_idx = normalize_input_grounding(&config.input_grounding)?;
    let coupling_idx = normalize_input_coupling(&config.input_coupling)?;
    let notch_idx = normalize_input_notch(&config.input_notch)?;
    let reserve_idx = normalize_dynamic_reserve(&config.dynamic_reserve)?;
    let sensitivity_idx = normalize_sensitivity(&config.sensitivity)?;
    let slope_idx = match config.slope_db_oct {
        6 => 0u8,
        12 => 1,
        18 => 2,
        24 => 3,
        _ => {
            return Err(format!(
                "Unsupported OE1022D filter slope: {} dB/oct",
                config.slope_db_oct
            ))
        }
    };

    let _ = serial_query_ascii(port, &set_input_source(CH, input_source_idx));
    let _ = serial_query_ascii(port, &set_input_grounding(CH, grounding_idx));
    let _ = serial_query_ascii(port, &set_input_coupling(CH, coupling_idx));
    let _ = serial_query_ascii(port, &set_line_notch_filter(CH, notch_idx));
    let _ = serial_query_ascii(port, &set_dynamic_reserve(CH, reserve_idx));
    let _ = serial_query_ascii(port, &set_sensitivity(CH, sensitivity_idx));
    let _ = serial_query_ascii(
        port,
        &set_time_constant(CH, tc_seconds_to_index(config.time_constant_s)),
    );
    let _ = serial_query_ascii(port, &set_filter_slope(CH, slope_idx));
    let _ = serial_query_ascii(port, &set_reference_source(CH, source_idx));
    if source_idx == 0 {
        let _ = serial_query_ascii(port, &set_ref_slope(CH, ref_slope_idx));
    }
    let _ = serial_query_ascii(port, &set_phase_deg(CH, config.phase_deg));
    let _ = serial_query_ascii(port, &set_sync_filter(CH, u8::from(config.sync_filter_on)));
    Ok(())
}

fn normalize_reference_source(input: &str) -> Result<u8, String> {
    if input.contains("外部") || input.eq_ignore_ascii_case("external") {
        Ok(0)
    } else if input.contains("内部扫频") || input.to_ascii_lowercase().contains("sweep") {
        Ok(2)
    } else if input.contains("内部") || input.eq_ignore_ascii_case("internal") {
        Ok(1)
    } else {
        Err(format!("Unsupported OE1022D reference source: {input}"))
    }
}

fn normalize_ref_slope(input: &str) -> Result<u8, String> {
    let lower = input.to_ascii_lowercase();
    if input.contains("TTL") || lower.contains("ttl") {
        Ok(0)
    } else if input.contains("过零") || lower.contains("sine") || lower.contains("zero") {
        Ok(1)
    } else {
        Err(format!("Unsupported OE1022D external ref trigger: {input}"))
    }
}

fn normalize_input_source(input: &str) -> Result<u8, String> {
    if input.contains("差分") {
        Ok(1)
    } else if input.contains("电流") {
        Ok(2)
    } else if input.contains("单端") || input.to_ascii_lowercase().contains("single") {
        Ok(0)
    } else {
        Err(format!("Unsupported OE1022D input source: {input}"))
    }
}

fn normalize_input_grounding(input: &str) -> Result<u8, String> {
    if input.contains("接地") || input.eq_ignore_ascii_case("ground") {
        Ok(1)
    } else if input.contains("浮空") || input.eq_ignore_ascii_case("float") {
        Ok(0)
    } else {
        Err(format!("Unsupported OE1022D input grounding: {input}"))
    }
}

fn normalize_input_coupling(input: &str) -> Result<u8, String> {
    if input.contains("直流") || input.eq_ignore_ascii_case("dc") {
        Ok(1)
    } else if input.contains("交流") || input.eq_ignore_ascii_case("ac") {
        Ok(0)
    } else {
        Err(format!("Unsupported OE1022D input coupling: {input}"))
    }
}

fn normalize_input_notch(input: &str) -> Result<u8, String> {
    if input.contains("50/100") {
        Ok(3)
    } else if input.contains("100") {
        Ok(2)
    } else if input.contains("50") {
        Ok(1)
    } else if input.contains("关闭") || input.eq_ignore_ascii_case("off") {
        Ok(0)
    } else {
        Err(format!("Unsupported OE1022D input notch: {input}"))
    }
}

fn normalize_dynamic_reserve(input: &str) -> Result<u8, String> {
    let lower = input.to_ascii_lowercase();
    if input.contains("高") || lower.contains("high") {
        Ok(2)
    } else if input.contains("低") || lower.contains("low") {
        Ok(0)
    } else if input.contains("正常") || lower.contains("normal") {
        Ok(1)
    } else {
        Err(format!("Unsupported OE1022D dynamic reserve: {input}"))
    }
}

fn normalize_sensitivity(input: &str) -> Result<u8, String> {
    match input.trim() {
        "1 mV/nA" => Ok(18),
        "2 mV/nA" => Ok(19),
        "3 mV/nA" => Ok(20),
        "5 mV/nA" => Ok(20),
        "10 mV/nA" => Ok(21),
        "20 mV/nA" => Ok(22),
        "30 mV/nA" => Ok(23),
        "50 mV/nA" => Ok(23),
        "100 mV/nA" => Ok(24),
        "200 mV/nA" => Ok(25),
        "300 mV/nA" => Ok(26),
        "500 mV/nA" => Ok(26),
        "1 V/uA" => Ok(27),
        "10 uV/nA" => Ok(18),
        "30 uV/nA" => Ok(18),
        "100 uV/nA" => Ok(18),
        "300 uV/nA" => Ok(18),
        other => Err(format!("Unsupported OE1022D sensitivity: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_reference_trigger_to_manual_indices() {
        assert_eq!(normalize_ref_slope("TTL 上升沿").unwrap(), 0);
        assert_eq!(normalize_ref_slope("过零检测").unwrap(), 1);
    }

    #[test]
    fn normalizes_notch_filter_to_manual_indices() {
        assert_eq!(normalize_input_notch("关闭所有陷波器").unwrap(), 0);
        assert_eq!(normalize_input_notch("50 Hz").unwrap(), 1);
        assert_eq!(normalize_input_notch("100 Hz").unwrap(), 2);
        assert_eq!(normalize_input_notch("50/100 Hz").unwrap(), 3);
    }

    #[test]
    fn normalizes_default_sensitivity_to_expected_index() {
        assert_eq!(normalize_sensitivity("100 mV/nA").unwrap(), 24);
    }
}
