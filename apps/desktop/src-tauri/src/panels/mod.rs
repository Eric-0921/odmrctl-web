//! Device Panel shared helpers — M5C-A
//!
//! Provides transport primitives (TCP, serial), safety-limit caching,
//! and device-address resolution for all panel commands.

pub mod laser;
pub mod magnetic;
pub mod oe1022d;
pub mod smb100a;

use crate::workbench_state::WorkbenchState;
use odmr_config::load_station_config;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Safety limits
// ---------------------------------------------------------------------------

/// Parsed safety limits from station.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationSafety {
    #[serde(default = "default_smb_max_power")]
    pub smb100a_max_power_dbm: f64,
    #[serde(default = "default_smb_min_freq")]
    pub smb100a_min_freq_hz: f64,
    #[serde(default = "default_smb_max_freq")]
    pub smb100a_max_freq_hz: f64,
    #[serde(default = "default_mag_max_current")]
    #[allow(dead_code)]
    pub mag_max_current_a_per_axis: f64,
    #[serde(default = "default_laser_max_power")]
    pub laser_max_power_mw: u16,
    #[serde(default = "default_laser_enabled")]
    #[allow(dead_code)]
    pub laser_default_enabled: bool,
}

fn default_smb_max_power() -> f64 {
    -10.0
}
fn default_smb_min_freq() -> f64 {
    2_800_000_000.0
}
fn default_smb_max_freq() -> f64 {
    2_950_000_000.0
}
fn default_mag_max_current() -> f64 {
    0.1
}
fn default_laser_max_power() -> u16 {
    100
}
fn default_laser_enabled() -> bool {
    false
}

impl Default for StationSafety {
    fn default() -> Self {
        Self {
            smb100a_max_power_dbm: default_smb_max_power(),
            smb100a_min_freq_hz: default_smb_min_freq(),
            smb100a_max_freq_hz: default_smb_max_freq(),
            mag_max_current_a_per_axis: default_mag_max_current(),
            laser_max_power_mw: default_laser_max_power(),
            laser_default_enabled: default_laser_enabled(),
        }
    }
}

/// Load safety limits from a station.json file.
pub fn load_station_safety(path: &str) -> Result<StationSafety, String> {
    let config = load_station_config(path).map_err(|e| format!("load {path}: {e}"))?;
    Ok(StationSafety {
        smb100a_max_power_dbm: config.safety.smb100a_max_power_dbm,
        smb100a_min_freq_hz: config.safety.smb100a_min_freq_hz,
        smb100a_max_freq_hz: config.safety.smb100a_max_freq_hz,
        mag_max_current_a_per_axis: config.safety.mag_max_current_a_per_axis,
        laser_max_power_mw: config.safety.laser_max_power_mw,
        laser_default_enabled: false,
    })
}

// ---------------------------------------------------------------------------
// Lock guard — every panel command must check the device is locked first.
// ---------------------------------------------------------------------------

/// Verify device accessibility and return its address.
///
/// A device is accessible if:
/// - It was locked via batch preflight (station.json), OR
/// - It was connected via single-device connect (per-card address input).
pub fn with_device_access(state: &WorkbenchState, device_id: &str) -> Result<String, String> {
    if !state.is_accessible(device_id) {
        return Err(format!(
            "Device '{}' is not connected. Enter address and click Connect on the Devices page.",
            device_id
        ));
    }
    let address = state.device_address(device_id).ok_or_else(|| {
        format!(
            "Device '{}' has no address. Enter address and click Connect.",
            device_id
        )
    })?;
    Ok(address)
}

// ---------------------------------------------------------------------------
// TCP transport (SMB100A)
// ---------------------------------------------------------------------------

/// Open a TCP connection to an SMB100A with a 5-second timeout.
pub fn smb_connect(address: &str) -> Result<TcpStream, String> {
    let stream =
        TcpStream::connect(address).map_err(|e| format!("TCP connect to {address} failed: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_millis(5000)))
        .map_err(|e| format!("set read timeout: {e}"))?;
    stream
        .set_write_timeout(Some(Duration::from_millis(5000)))
        .map_err(|e| format!("set write timeout: {e}"))?;
    Ok(stream)
}

/// Send an SCPI command and read back a single line response.
pub fn scpi_query(stream: &mut TcpStream, cmd: &str) -> Result<String, String> {
    let cmd_bytes = format!("{}\n", cmd.trim());
    stream
        .write_all(cmd_bytes.as_bytes())
        .map_err(|e| format!("TCP write '{cmd}': {e}"))?;
    stream
        .flush()
        .map_err(|e| format!("TCP flush '{cmd}': {e}"))?;

    let mut reader = BufReader::new(
        stream
            .try_clone()
            .map_err(|e| format!("clone stream: {e}"))?,
    );
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => Err(format!("TCP read '{cmd}' returned empty")),
        Ok(_) => {
            line = line.trim().to_string();
            Ok(line)
        }
        Err(e) => Err(format!("TCP read '{cmd}' failed: {e}")),
    }
}

/// Send an SCPI set command (no response expected).
pub fn scpi_set(stream: &mut TcpStream, cmd: &str) -> Result<(), String> {
    let cmd_bytes = format!("{}\n", cmd.trim());
    stream
        .write_all(cmd_bytes.as_bytes())
        .map_err(|e| format!("TCP write '{cmd}': {e}"))?;
    stream
        .flush()
        .map_err(|e| format!("TCP flush '{cmd}': {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Serial transport (OE1022D, Laser)
// ---------------------------------------------------------------------------

/// Open a serial port with the given baud rate and a 2-second timeout.
pub fn serial_open(port_path: &str, baud: u32) -> Result<Box<dyn serialport::SerialPort>, String> {
    serialport::new(port_path, baud)
        .timeout(Duration::from_millis(2000))
        .open()
        .map_err(|e| format!("open serial {port_path} @ {baud}: {e}"))
}

/// Send an ASCII command over serial and read back the response line.
/// **CRITICAL**: OE1022D requires `clear(Input)` before every command.
pub fn serial_query_ascii(
    port: &mut Box<dyn serialport::SerialPort>,
    cmd: &str,
) -> Result<String, String> {
    let _ = port.clear(serialport::ClearBuffer::Input);
    let cmd_bytes = format!("{}\r", cmd.trim());
    port.write_all(cmd_bytes.as_bytes())
        .map_err(|e| format!("serial write '{cmd}': {e}"))?;
    port.flush()
        .map_err(|e| format!("serial flush '{cmd}': {e}"))?;

    let mut buf = [0u8; 1024];
    let n = port
        .read(&mut buf)
        .map_err(|e| format!("serial read '{cmd}': {e}"))?;
    let resp = String::from_utf8_lossy(&buf[..n]).trim().to_string();
    Ok(resp)
}
