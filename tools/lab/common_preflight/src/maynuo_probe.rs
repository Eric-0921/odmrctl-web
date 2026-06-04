use crate::error::PreflightError;
use crate::types::{DeviceConfig, DevicePreflightReport, SafeState};
use std::io::{Read, Write};
use std::time::Duration;

/// Probe a Maynuo M8812 via serial enumeration + strict SN match.
///
/// Safe-state check sequence:
///   SYST:REM → OUTP? → MEAS:CURR? → SYST:LOC
///
/// The device is ALWAYS returned to local mode before probe() exits.
/// If output is ON or current > 1.0 mA, safe_state.confirmed = false.
pub fn probe(device: &DeviceConfig) -> Result<DevicePreflightReport, PreflightError> {
    let timeout_ms = device.timeout_ms.unwrap_or(5000);
    let expected_sn = device.expected_sn.as_deref();

    // 1. Enumerate serial ports and probe IDN
    let ports = serialport::available_ports().map_err(|e| {
        PreflightError::PhysicalUnreachable {
            device_id: device.device_id.clone(),
            detail: format!("enumerate serial ports: {}", e),
        }
    })?;

    // Filter: only probe USB serial ports, skip Bluetooth/audio
    let usb_ports: Vec<_> = ports.iter().filter(|p| {
        let name = p.port_name.to_lowercase();
        name.contains("usb") || name.contains("pl2303") || name.contains("ftdi") || name.contains("cp210")
    }).collect();

    let mut matched_port: Option<String> = None;
    let mut identity: Option<String> = None;

    for port_info in &usb_ports {
        let port_path = &port_info.port_name;
        // Use a shorter timeout for probing to avoid long waits on wrong devices
        match probe_port_idn(port_path, 2000) {
            Ok(idn) => {
                if let Some(expected) = expected_sn {
                    if sn_matches(&idn, expected) {
                        matched_port = Some(port_path.clone());
                        identity = Some(idn);
                        break;
                    }
                } else {
                    // No SN expectation: accept first responding Maynuo
                    if idn.contains("MAYNUO") || idn.contains("M8812") {
                        matched_port = Some(port_path.clone());
                        identity = Some(idn);
                        break;
                    }
                }
            }
            Err(_) => continue,
        }
    }

    let port_path = matched_port.ok_or_else(|| PreflightError::IdentityMismatch {
        device_id: device.device_id.clone(),
        expected: expected_sn.map(|s| s.to_string()),
        observed: None,
    })?;

    let identity_display = identity.as_ref().map(|s| s.trim().to_string());

    // 2. Open matched port and verify safe state
    let mut port = serialport::new(&port_path, 9600)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| PreflightError::SerialError {
            device_id: device.device_id.clone(),
            detail: format!("open {}: {}", port_path, e),
        })?;

    // Enter remote mode for safe-state query
    let _ = scpi_set(&mut port, "SYST:REM", timeout_ms);

    let output_on = scpi_query(&mut port, "OUTP?", timeout_ms)
        .map(|s| s.trim() == "1")
        .unwrap_or(true); // if query fails, assume unsafe

    let mut current_ma: Option<f64> = None;
    if let Ok(resp) = scpi_query(&mut port, "MEAS:CURR?", timeout_ms) {
        if let Ok(val) = resp.trim().parse::<f64>() {
            current_ma = Some(val * 1000.0);
        }
    }

    // ALWAYS return to local mode before exiting
    let _ = scpi_set(&mut port, "SYST:LOC", timeout_ms);

    let safe = !output_on && current_ma.map(|c| c.abs() < 1.0).unwrap_or(false);

    let safe_state = SafeState {
        confirmed: safe,
        rf_output: None,
        modulation: None,
        fm: None,
        magnetic_output: Some(if output_on { "1".into() } else { "0".into() }),
        magnetic_current_ma: current_ma,
    };

    let mut warnings = Vec::new();
    if output_on {
        warnings.push("Maynuo output is ON — unsafe".into());
    }
    if current_ma.map(|c| c.abs() >= 1.0).unwrap_or(false) {
        warnings.push(format!("Maynuo current {:.3} mA exceeds 1.0 mA tolerance", current_ma.unwrap()));
    }

    Ok(DevicePreflightReport {
        device_id: device.device_id.clone(),
        kind: device.kind.clone(),
        reachability: true,
        identity_raw: identity.clone(),
        identity_display,
        error_queue: vec![],
        safe_state: Some(safe_state),
        warnings,
        commands_sent: None,
        laser_on_sent: None,
        nonzero_power_sent: None,
    })
}

/// Strict SN matching: parse MAYNUO IDN and compare exact SN field.
///
/// IDN format: `MAYNUO,M8812,<SN>,<FW_VERSION>`
/// Malformed IDN (wrong field count or wrong manufacturer/model) is rejected.
fn sn_matches(idn: &str, expected_sn: &str) -> bool {
    let parts: Vec<&str> = idn.split(',').collect();
    if parts.len() < 4 {
        return false;
    }
    let manufacturer = parts[0].trim();
    let model = parts[1].trim();
    let actual_sn = parts[2].trim();
    manufacturer.eq_ignore_ascii_case("MAYNUO") && model == "M8812" && actual_sn == expected_sn
}

fn probe_port_idn(port_path: &str, timeout_ms: u64) -> Result<String, String> {
    let mut port = serialport::new(port_path, 9600)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| format!("open: {}", e))?;

    let _ = port.clear(serialport::ClearBuffer::Input);

    scpi_query(&mut port, "*IDN?", timeout_ms)
}

fn scpi_query(port: &mut Box<dyn serialport::SerialPort>, cmd: &str, _timeout_ms: u64) -> Result<String, String> {
    let cmd_bytes = format!("{}\r", cmd);
    port.write_all(cmd_bytes.as_bytes())
        .map_err(|e| format!("write '{}': {}", cmd, e))?;
    port.flush()
        .map_err(|e| format!("flush '{}': {}", cmd, e))?;

    std::thread::sleep(Duration::from_millis(300));

    let mut buf = vec![0u8; 256];
    let n = match port.read(&mut buf) {
        Ok(0) => return Err("port closed".into()),
        Ok(n) => n,
        Err(e) => return Err(format!("read '{}': {}", cmd, e)),
    };

    buf.truncate(n);
    let resp = String::from_utf8_lossy(&buf).trim().to_string();
    if resp.is_empty() {
        return Err("empty response".into());
    }
    Ok(resp)
}

fn scpi_set(port: &mut Box<dyn serialport::SerialPort>, cmd: &str, _timeout_ms: u64) -> Result<(), String> {
    let cmd_bytes = format!("{}\r", cmd);
    port.write_all(cmd_bytes.as_bytes())
        .map_err(|e| format!("write '{}': {}", cmd, e))?;
    port.flush()
        .map_err(|e| format!("flush '{}': {}", cmd, e))?;
    Ok(())
}

/// Unified safe cleanup for Maynuo M8812.
///
/// Sequence: SYST:REM → CURR 0 → OUTP 0 → wait 500ms → MEAS:CURR? → verify < 1.0mA → SYST:LOC
pub fn safe_zero_and_local(port: &mut Box<dyn serialport::SerialPort>) -> Result<f64, String> {
    scpi_set(port, "SYST:REM", 5000)?;
    scpi_set(port, "CURR 0.00000", 5000)?;
    scpi_set(port, "OUTP 0", 5000)?;
    std::thread::sleep(Duration::from_millis(500));

    let current_a = scpi_query(port, "MEAS:CURR?", 5000)
        .and_then(|s| s.trim().parse::<f64>().map_err(|e| format!("parse: {}", e)))?;
    let current_ma = current_a * 1000.0;

    if current_ma.abs() >= 1.0 {
        return Err(format!("current {} mA exceeds 1.0 mA tolerance", current_ma));
    }

    scpi_set(port, "SYST:LOC", 5000)?;
    Ok(current_ma)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sn_matches_exact() {
        let idn = "MAYNUO,M8812,080020960220402020,V2.7";
        assert!(sn_matches(idn, "080020960220402020"));
        assert!(!sn_matches(idn, "080020960220402022"));
    }

    #[test]
    fn test_sn_matches_rejects_malformed() {
        // Wrong field count
        let idn = "MAYNUO,M8812,080020960220402020";
        assert!(!sn_matches(idn, "080020960220402020"));

        // Wrong manufacturer
        let idn = "KEITHLEY,M8812,080020960220402020,V2.7";
        assert!(!sn_matches(idn, "080020960220402020"));

        // Wrong model
        let idn = "MAYNUO,M8813,080020960220402020,V2.7";
        assert!(!sn_matches(idn, "080020960220402020"));

        // Substring match in unstructured text must NOT succeed
        let idn = "MAYNUO M8812 SN=080020960220402020";
        assert!(!sn_matches(idn, "080020960220402020"));
    }
}
