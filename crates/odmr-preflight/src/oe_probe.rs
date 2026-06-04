use crate::error::PreflightError;
use crate::types::{DeviceConfig, DevicePreflightReport, SafeState};
use std::io::{Read, Write};
use std::time::Duration;

const BAUD: u32 = 921600;

/// Probe an OE1022D via serial port.
///
/// Discovery strategy:
/// 1. If `address` is a specific port path, try it first.
/// 2. If that fails or `address` is "auto", enumerate all USB serial ports
///    and probe each with `*IDN?` at 921600 baud.
/// 3. Match identity containing "OE1022D" or "LIA-OE".
pub fn probe(device: &DeviceConfig) -> Result<DevicePreflightReport, PreflightError> {
    let timeout_ms = device.timeout_ms.unwrap_or(5000);

    // 1. Try explicit port if given and not "auto"
    let explicit = if device.address.is_empty() || device.address.eq_ignore_ascii_case("auto") {
        None
    } else {
        Some(device.address.clone())
    };

    let matched = if let Some(ref path) = explicit {
        match probe_port_idn(path, timeout_ms) {
            Ok(idn) => Some((path.clone(), idn)),
            Err(e) => {
                eprintln!(
                    "[{}] Explicit port {} failed ({}), falling back to auto-discovery...",
                    device.device_id, path, e
                );
                auto_discover_oe(timeout_ms)
            }
        }
    } else {
        auto_discover_oe(timeout_ms)
    };

    let (port_path, identity) = matched.ok_or_else(|| PreflightError::IdentityMismatch {
        device_id: device.device_id.clone(),
        expected: Some("OE1022D or LIA-OE".into()),
        observed: None,
    })?;

    // 2. Re-open matched port for the final report
    let port = serialport::new(&port_path, BAUD)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| PreflightError::PhysicalUnreachable {
            device_id: device.device_id.clone(),
            detail: format!("open serial {}: {}", port_path, e),
        })?;

    let _ = port.clear(serialport::ClearBuffer::Input);

    let identity_display = identity.replace('\x00', "").trim().to_string();
    let identity_raw = identity
        .bytes()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    // OE1022D has no error queue and no safe-state commands (read-only device)
    let safe_state = SafeState {
        confirmed: true,
        rf_output: None,
        modulation: None,
        fm: None,
        magnetic_output: None,
        magnetic_current_ma: None,
    };

    Ok(DevicePreflightReport {
        device_id: device.device_id.clone(),
        kind: device.kind.clone(),
        reachability: true,
        identity_raw: Some(identity_raw),
        identity_display: Some(identity_display),
        error_queue: vec![],
        safe_state: Some(safe_state),
        warnings: if explicit.is_some() && port_path != explicit.unwrap() {
            vec![format!(
                "Auto-discovered on {} (explicit port was unavailable)",
                port_path
            )]
        } else {
            vec![]
        },
        commands_sent: None,
        laser_on_sent: None,
        nonzero_power_sent: None,
    })
}

/// Enumerate USB serial ports and find the OE1022D by `*IDN?`.
fn auto_discover_oe(_timeout_ms: u64) -> Option<(String, String)> {
    let ports = serialport::available_ports().ok()?;

    let usb_ports: Vec<_> = ports
        .iter()
        .filter(|p| {
            let name = p.port_name.to_lowercase();
            name.contains("usb")
                || name.contains("pl2303")
                || name.contains("ftdi")
                || name.contains("cp210")
        })
        .collect();

    for port_info in &usb_ports {
        let port_path = &port_info.port_name;
        match probe_port_idn(port_path, 2000) {
            Ok(idn) => {
                if idn.contains("OE1022D") || idn.contains("LIA-OE") || idn.contains("SSI LIA") {
                    return Some((port_path.clone(), idn));
                }
            }
            Err(_) => continue,
        }
    }
    None
}

fn probe_port_idn(port_path: &str, timeout_ms: u64) -> Result<String, String> {
    let mut port = serialport::new(port_path, BAUD)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| format!("open: {}", e))?;

    let _ = port.clear(serialport::ClearBuffer::Input);

    scpi_query(&mut port, "*IDN?", timeout_ms)
}

fn scpi_query(
    port: &mut Box<dyn serialport::SerialPort>,
    cmd: &str,
    _timeout_ms: u64,
) -> Result<String, String> {
    let cmd_bytes = format!("{}\r", cmd);
    port.write_all(cmd_bytes.as_bytes())
        .map_err(|e| format!("write '{}': {}", cmd, e))?;
    port.flush()
        .map_err(|e| format!("flush '{}': {}", cmd, e))?;

    std::thread::sleep(Duration::from_millis(500));

    let mut buf = vec![0u8; 1024];
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

#[cfg(test)]
mod tests {

    #[test]
    fn test_identity_matching() {
        let samples = vec![
            "SSI LIA-OE1022D,SN:D6522078,Version:Ver6.3200831",
            "OE1022D",
            "LIA-OE1022D",
        ];
        for s in &samples {
            assert!(
                s.contains("OE1022D") || s.contains("LIA-OE") || s.contains("SSI LIA"),
                "should match OE identity: {}",
                s
            );
        }
    }
}
