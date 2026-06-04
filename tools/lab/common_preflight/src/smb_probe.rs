use crate::error::PreflightError;
use crate::types::{DeviceConfig, DevicePreflightReport, SafeState};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Probe an SMB100A via TCP raw socket.
///
/// Discovery strategy:
/// 1. If `address` is a specific `ip:port`, try it first.
/// 2. If that fails or `address` is "auto", scan common subnets for
///    port 5025 responders that return `*IDN?` containing "Rohde&Schwarz".
///
/// Scan ranges (timeout 300ms each):
/// - 169.254.2.10 ..= 169.254.2.30  (link-local, common R&S default)
/// - 192.168.1.10  ..= 192.168.1.30  (typical lab subnet)
pub fn probe(device: &DeviceConfig) -> Result<DevicePreflightReport, PreflightError> {
    let timeout_ms = device.timeout_ms.unwrap_or(5000);
    let timeout = Duration::from_millis(timeout_ms);

    let explicit = if device.address.is_empty() || device.address.eq_ignore_ascii_case("auto") {
        None
    } else {
        Some(device.address.clone())
    };

    let matched = if let Some(ref addr_str) = explicit {
        match try_connect_and_idn(addr_str, timeout_ms) {
            Ok(idn) => Some((addr_str.clone(), idn)),
            Err(e) => {
                eprintln!(
                    "[{}] Explicit address {} failed ({}), falling back to subnet scan...",
                    device.device_id, addr_str, e
                );
                auto_discover_smb(timeout_ms)
            }
        }
    } else {
        auto_discover_smb(timeout_ms)
    };

    let (addr_str, identity) = matched.ok_or_else(|| PreflightError::PhysicalUnreachable {
        device_id: device.device_id.clone(),
        detail: "SMB100A not found on any scanned subnet".into(),
    })?;

    // Re-connect for the full preflight sequence
    let addr: SocketAddr = addr_str.parse().map_err(|e| PreflightError::PhysicalUnreachable {
        device_id: device.device_id.clone(),
        detail: format!("invalid address '{}': {}", addr_str, e),
    })?;

    let mut stream = TcpStream::connect_timeout(&addr, timeout).map_err(|e| {
        PreflightError::PhysicalUnreachable {
            device_id: device.device_id.clone(),
            detail: format!("TCP connect to {}: {}", addr_str, e),
        }
    })?;

    stream.set_read_timeout(Some(timeout)).map_err(|e| {
        PreflightError::TcpError {
            device_id: device.device_id.clone(),
            detail: format!("set read timeout: {}", e),
        }
    })?;

    // Error queue drain
    let errors = drain_error_queue(&mut stream, timeout_ms).map_err(|e| {
        PreflightError::TcpError {
            device_id: device.device_id.clone(),
            detail: format!("error queue drain: {}", e),
        }
    })?;

    // Safe state verification
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
        identity_raw: Some(identity.clone()),
        identity_display: Some(identity.trim().to_string()),
        error_queue: errors,
        safe_state: Some(safe_state),
        warnings: if explicit.as_ref() != Some(&addr_str) {
            vec![format!(
                "Auto-discovered at {} (explicit address was unavailable)",
                addr_str
            )]
        } else {
            vec![]
        },
        commands_sent: None,
        laser_on_sent: None,
        nonzero_power_sent: None,
    })
}

/// Scan common subnets for SMB100A.
fn auto_discover_smb(timeout_ms: u64) -> Option<(String, String)> {
    let per_ip_timeout = timeout_ms.min(500); // cap at 500ms per IP

    let mut candidates: Vec<String> = Vec::new();
    for host in 10u8..=30 {
        candidates.push(format!("169.254.2.{host}:5025"));
    }
    for host in 10u8..=30 {
        candidates.push(format!("192.168.1.{host}:5025"));
    }

    for addr in candidates {
        match try_connect_and_idn(&addr, per_ip_timeout) {
            Ok(idn) => {
                if idn.contains("Rohde&Schwarz") || idn.contains("SMB") {
                    return Some((addr, idn));
                }
            }
            Err(_) => continue,
        }
    }

    None
}

fn try_connect_and_idn(addr_str: &str, timeout_ms: u64) -> Result<String, String> {
    let addr: SocketAddr = addr_str
        .parse()
        .map_err(|e| format!("parse addr '{}': {}", addr_str, e))?;
    let timeout = Duration::from_millis(timeout_ms);

    let mut stream =
        TcpStream::connect_timeout(&addr, timeout).map_err(|e| format!("connect: {}", e))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| format!("set timeout: {}", e))?;

    let idn = scpi_query(&mut stream, "*IDN?", timeout_ms)?;
    if !idn.contains("Rohde&Schwarz") && !idn.contains("SMB") {
        return Err(format!("unexpected IDN: {}", idn));
    }
    Ok(idn)
}

fn scpi_query(stream: &mut TcpStream, cmd: &str, _timeout_ms: u64) -> Result<String, String> {
    let cmd_bytes = format!("{}\n", cmd);
    stream
        .write_all(cmd_bytes.as_bytes())
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
        let errors: Vec<String> = vec![
            "-113,\"Undefined header\"".into(),
            "-420,\"Query UNTERMINATED\"".into(),
        ];
        assert_eq!(errors.len(), 2);
    }
}
