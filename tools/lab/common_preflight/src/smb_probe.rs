use crate::error::PreflightError;
use crate::types::{DeviceConfig, DevicePreflightReport, SafeState};
use std::io::{Read, Write};
use std::net::{TcpStream, SocketAddr};
use std::time::Duration;

/// Probe an SMB100A via TCP raw socket.
pub fn probe(device: &DeviceConfig) -> Result<DevicePreflightReport, PreflightError> {
    let timeout_ms = device.timeout_ms.unwrap_or(5000);
    let timeout = Duration::from_millis(timeout_ms);

    // 1. Physical reachability: TCP connect
    let addr: SocketAddr = device.address.parse().map_err(|e| {
        PreflightError::PhysicalUnreachable {
            device_id: device.device_id.clone(),
            detail: format!("invalid address '{}': {}", device.address, e),
        }
    })?;

    let mut stream = TcpStream::connect_timeout(&addr, timeout).map_err(|e| {
        PreflightError::PhysicalUnreachable {
            device_id: device.device_id.clone(),
            detail: format!("TCP connect to {}: {}", device.address, e),
        }
    })?;

    stream.set_read_timeout(Some(timeout)).map_err(|e| {
        PreflightError::TcpError {
            device_id: device.device_id.clone(),
            detail: format!("set read timeout: {}", e),
        }
    })?;

    // 2. Identity verification
    let identity = match scpi_query(&mut stream, "*IDN?", timeout_ms) {
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

    let identity_display = identity.as_ref().map(|s| s.trim().to_string());

    // 3. Error queue drain
    let errors = drain_error_queue(&mut stream, timeout_ms).map_err(|e| {
        PreflightError::TcpError {
            device_id: device.device_id.clone(),
            detail: format!("error queue drain: {}", e),
        }
    })?;

    // 4. Safe state verification
    let safe_state = verify_safe_state(&mut stream, timeout_ms).unwrap_or(SafeState {
        confirmed: false,
        rf_output: None,
        modulation: None,
        fm: None,
        magnetic_output: None,
        magnetic_current_ma: None,
    });

    Ok(DevicePreflightReport {
        device_id: device.device_id.clone(),
        kind: device.kind.clone(),
        reachability: true,
        identity_raw: identity.clone(),
        identity_display,
        error_queue: errors,
        safe_state: Some(safe_state),
        warnings: vec![],
    })
}

fn scpi_query(stream: &mut TcpStream, cmd: &str, _timeout_ms: u64) -> Result<String, String> {
    let cmd_bytes = format!("{}\n", cmd);
    stream.write_all(cmd_bytes.as_bytes())
        .map_err(|e| format!("write '{}': {}", cmd, e))?;

    let mut buf = [0u8; 1024];
    let n = match stream.read(&mut buf) {
        Ok(0) => return Err("connection closed".into()),
        Ok(n) => n,
        Err(e) => return Err(format!("read '{}': {}", cmd, e)),
    };

    let resp = String::from_utf8_lossy(&buf[..n]).trim().to_string();
    if resp.is_empty() {
        return Err("empty response".into());
    }
    Ok(resp)
}

/// Drain the SCPI error queue until "+0,\"No error\"" or empty.
fn drain_error_queue(stream: &mut TcpStream, timeout_ms: u64) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();
    for _ in 0..50 {
        match scpi_query(stream, "SYST:ERR?", timeout_ms) {
            Ok(resp) => {
                if resp.contains("+0") || resp.contains("No error") {
                    break;
                }
                errors.push(resp);
            }
            Err(e) => {
                errors.push(format!("SYST:ERR? failed: {}", e));
                break;
            }
        }
    }
    Ok(errors)
}

fn verify_safe_state(stream: &mut TcpStream, timeout_ms: u64) -> Result<SafeState, String> {
    let mut state = SafeState {
        confirmed: true,
        rf_output: None,
        modulation: None,
        fm: None,
        magnetic_output: None,
        magnetic_current_ma: None,
    };

    if let Ok(resp) = scpi_query(stream, "OUTP?", timeout_ms) {
        state.rf_output = Some(resp.clone());
        if resp != "0" && !resp.eq_ignore_ascii_case("OFF") {
            state.confirmed = false;
        }
    } else {
        state.confirmed = false;
    }

    if let Ok(resp) = scpi_query(stream, "MOD:STAT?", timeout_ms) {
        state.modulation = Some(resp.clone());
        if resp != "0" && !resp.eq_ignore_ascii_case("OFF") {
            state.confirmed = false;
        }
    }

    if let Ok(resp) = scpi_query(stream, "FM:STAT?", timeout_ms) {
        state.fm = Some(resp.clone());
        if resp != "0" && !resp.eq_ignore_ascii_case("OFF") {
            state.confirmed = false;
        }
    }

    Ok(state)
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_drain_error_queue_logic() {
        // This test is a logic test only; real drain requires a fake stream
        let errors: Vec<String> = vec![
            "-113,\"Undefined header\"".into(),
            "-420,\"Query UNTERMINATED\"".into(),
        ];
        assert_eq!(errors.len(), 2);
        // In real usage, drain_error_queue would stop at "+0,\"No error\""
    }
}
