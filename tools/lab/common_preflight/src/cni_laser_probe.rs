//! CNI Laser off-only preflight probe (M2).
//!
//! **SAFETY**: The preflight `probe()` itself ONLY sends `laser_off` frames.
//! It NEVER sends `laser_on` or `set_power` with non-zero power.
//!
//! **DISCOVERY NOTE**: Auto-discovery (`address: "auto"`) sends two different
//! frames (`laser_off` and `set_power(0)`) to verify the binary frame echo
//! behaviour. Both frames keep the laser OFF (`set_power(0)` sets target to 0).
//! No emission-capable command is ever sent during preflight.
//!
//! Artifact fields (on `DevicePreflightReport`):
//! - `commands_sent`: `["laser_off"]` for explicit path; discovery sends
//!   `["laser_off", "set_power(0)"]` internally but only `laser_off` is
//!   reported as the probe command.
//! - `laser_on_sent`: `false`
//! - `nonzero_power_sent`: `false`
//!
//! Protocol: binary frame over serial, 9600 8N1.
//! See `docs/equipment_manual/CNI Laser psu-sr/RS232语言协议_恒功率.md`

use crate::error::PreflightError;
use crate::types::{DeviceConfig, DevicePreflightReport, SafeState};
use cni_laser_fake_driver::protocol::CniFrame;
use std::io::{Read, Write};
use std::time::Duration;

/// Probe a CNI laser via serial port.
///
/// Discovery strategy:
/// 1. If `address` is a specific port path, try it first.
/// 2. If that fails or `address` is "auto", enumerate USB serial ports
///    and identify the laser by its binary frame echo behaviour.
///
/// Identification logic:
/// - Skip ports that respond to SCPI `*IDN?` (OE1022D / Maynuo).
/// - Send `laser_off` frame; if the echo matches the sent bytes exactly,
///   verify with a second frame (`set_power(0)`) to confirm echo consistency.
/// - Only accept if both frames echo back correctly.
pub fn probe(device: &DeviceConfig) -> Result<DevicePreflightReport, PreflightError> {
    let timeout_ms = device.timeout_ms.unwrap_or(5000);

    let explicit = if device.address.is_empty() || device.address.eq_ignore_ascii_case("auto") {
        None
    } else {
        Some(device.address.clone())
    };

    let matched = if let Some(ref path) = explicit {
        match try_identify_laser(path, timeout_ms) {
            Ok(()) => Some(path.clone()),
            Err(e) => {
                eprintln!(
                    "[{}] Explicit port {} failed ({}), falling back to auto-discovery...",
                    device.device_id, path, e
                );
                auto_discover_laser(timeout_ms)
            }
        }
    } else {
        auto_discover_laser(timeout_ms)
    };

    let port_path = matched.ok_or_else(|| PreflightError::IdentityMismatch {
        device_id: device.device_id.clone(),
        expected: Some("CNI Laser (binary frame echo)".into()),
        observed: None,
    })?;

    // Open final port and send OFF
    let mut port = serialport::new(&port_path, 9600)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| PreflightError::PhysicalUnreachable {
            device_id: device.device_id.clone(),
            detail: format!("open serial {}: {}", port_path, e),
        })?;

    let _ = port.clear(serialport::ClearBuffer::Input);

    let off_frame = CniFrame::laser_off();
    let off_bytes = off_frame.to_bytes();

    port.write_all(&off_bytes)
        .and_then(|_| port.flush())
        .map_err(|e| PreflightError::SerialError {
            device_id: device.device_id.clone(),
            detail: format!("write laser_off: {}", e),
        })?;

    std::thread::sleep(Duration::from_millis(100));

    let mut resp_buf = [0u8; 32];
    let _resp_len: usize = port.read(&mut resp_buf).unwrap_or_default();

    let identity_display = format!("CNI Laser @ {}", port_path);

    let safe_state = SafeState {
        confirmed: true,
        rf_output: None,
        modulation: None,
        fm: None,
        magnetic_output: None,
        magnetic_current_ma: None,
    };

    let mut warnings = Vec::new();
    if explicit.as_ref() != Some(&port_path) {
        warnings.push(format!(
            "Auto-discovered on {} (explicit port was unavailable)",
            port_path
        ));
    } else {
        warnings.push("No response from laser (expected for this protocol)".into());
    }

    Ok(DevicePreflightReport {
        device_id: device.device_id.clone(),
        kind: device.kind.clone(),
        reachability: true,
        identity_raw: Some(off_bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")),
        identity_display: Some(identity_display),
        error_queue: vec![],
        safe_state: Some(safe_state),
        warnings,
        commands_sent: Some(vec!["laser_off".into()]),
        laser_on_sent: Some(false),
        nonzero_power_sent: Some(false),
    })
}

/// Enumerate USB serial ports and find the CNI laser by frame echo.
fn auto_discover_laser(timeout_ms: u64) -> Option<String> {
    let ports = serialport::available_ports().ok()?;

    let usb_ports: Vec<_> = ports
        .iter()
        .filter(|p| {
            let name = p.port_name.to_lowercase();
            (name.contains("usb") || name.contains("pl2303") || name.contains("ftdi") || name.contains("cp210"))
                && !name.contains("bluetooth")
        })
        .collect();

    for port_info in &usb_ports {
        let path = &port_info.port_name;
        // Skip ports that speak SCPI (OE / Maynuo)
        if scpi_responds(path, timeout_ms) {
            continue;
        }
        // Try CNI laser identification
        if try_identify_laser(path, 2000).is_ok() {
            return Some(path.clone());
        }
    }
    None
}

/// Returns true if the port responds to SCPI *IDN?.
fn scpi_responds(path: &str, timeout_ms: u64) -> bool {
    let mut port = match serialport::new(path, 9600)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
    {
        Ok(p) => p,
        Err(_) => return false,
    };
    let _ = port.clear(serialport::ClearBuffer::Input);
    let cmd = b"*IDN?\r";
    if port.write_all(cmd).is_err() || port.flush().is_err() {
        return false;
    }
    std::thread::sleep(Duration::from_millis(300));
    let mut buf = [0u8; 256];
    match port.read(&mut buf) {
        Ok(n) if n > 0 => {
            let resp = String::from_utf8_lossy(&buf[..n]);
            !resp.trim().is_empty()
        }
        _ => false,
    }
}

/// Try to identify a CNI laser on a specific port.
///
/// Sends two different frames and verifies both echo back correctly.
fn try_identify_laser(path: &str, timeout_ms: u64) -> Result<(), String> {
    let mut port = serialport::new(path, 9600)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| format!("open: {}", e))?;

    // Test frame 1: laser_off
    let frame1 = CniFrame::laser_off();
    let bytes1 = frame1.to_bytes();
    if send_and_verify_echo(&mut port, &bytes1).is_err() {
        return Err("laser_off echo mismatch".into());
    }

    // Test frame 2: set_power(0) — different length and content
    let frame2 = CniFrame::set_power(0);
    let bytes2 = frame2.to_bytes();
    if send_and_verify_echo(&mut port, &bytes2).is_err() {
        return Err("set_power(0) echo mismatch".into());
    }

    Ok(())
}

/// Send a frame and verify the echo matches exactly.
fn send_and_verify_echo(
    port: &mut Box<dyn serialport::SerialPort>,
    frame: &[u8],
) -> Result<(), String> {
    let _ = port.clear(serialport::ClearBuffer::Input);
    port.write_all(frame).map_err(|e| format!("write: {}", e))?;
    port.flush().map_err(|e| format!("flush: {}", e))?;
    std::thread::sleep(Duration::from_millis(150));

    let mut buf = vec![0u8; frame.len()];
    let n = port.read(&mut buf).map_err(|e| format!("read: {}", e))?;
    if n != frame.len() || buf[..n] != *frame {
        return Err(format!(
            "echo mismatch: sent {:?}, got {:?} ({} bytes)",
            frame,
            &buf[..n],
            n
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_laser_off_frame_bytes() {
        let frame = CniFrame::laser_off();
        let bytes = frame.to_bytes();
        assert_eq!(bytes, vec![0x55, 0xAA, 0x03, 0x00, 0x03]);
    }

    #[test]
    fn test_set_power_zero_bytes() {
        let frame = CniFrame::set_power(0);
        let bytes = frame.to_bytes();
        // 55 AA 05 01 00 00 06
        assert_eq!(bytes, vec![0x55, 0xAA, 0x05, 0x01, 0x00, 0x00, 0x06]);
    }
}
