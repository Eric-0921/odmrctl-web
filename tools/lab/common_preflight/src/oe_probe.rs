use crate::error::PreflightError;
use crate::types::{DeviceConfig, DevicePreflightReport, SafeState};
use std::io::{Read, Write};
use std::time::Duration;

/// Probe an OE1022D via serial port.
pub fn probe(device: &DeviceConfig) -> Result<DevicePreflightReport, PreflightError> {
    let timeout_ms = device.timeout_ms.unwrap_or(5000);
    let baud = 921600; // OE1022D fixed baud

    // Parse address as "port:baud" or just "port"
    let port_path = device.address.clone();

    // 1. Physical reachability: open serial port
    let mut port = serialport::new(&port_path, baud)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| PreflightError::PhysicalUnreachable {
            device_id: device.device_id.clone(),
            detail: format!("open serial {}: {}", port_path, e),
        })?;

    // 2. Clear input buffer before query
    let _ = port.clear(serialport::ClearBuffer::Input);

    // 3. Identity verification
    let identity = match scpi_query(&mut port, "*IDN?", timeout_ms) {
        Ok(resp) => Some(resp),
        Err(e) => {
            return Ok(DevicePreflightReport {
                device_id: device.device_id.clone(),
                kind: device.kind.clone(),
                reachability: true,
                identity_raw: None,
                identity_display: None,
                error_queue: vec![],
                safe_state: None,
                warnings: vec![format!("Identity query failed: {}", e)],
            });
        }
    };

    let identity_display = identity.as_ref().map(|s| {
        s.replace('\x00', "").trim().to_string()
    });
    let identity_raw = identity.as_ref().map(|s| {
        s.bytes().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")
    });

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
        identity_raw,
        identity_display,
        error_queue: vec![],
        safe_state: Some(safe_state),
        warnings: vec![],
    })
}

fn scpi_query(port: &mut Box<dyn serialport::SerialPort>, cmd: &str, _timeout_ms: u64) -> Result<String, String> {
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
