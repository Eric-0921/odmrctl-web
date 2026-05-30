//! Read-only hardware discovery library for ODMR lab bring-up.
//!
//! **Safety invariant**: this library only sends queries. No setter commands
//! that change hardware state are permitted. All outbound SCPI / ASCII strings
//! are validated against a hard-coded allow-list before transmission.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Safe-command allow-lists
// ---------------------------------------------------------------------------

const SMB100A_SAFE_QUERIES: &[&str] = &["*IDN?", "SYST:ERR?", "OUTP?", "MOD:STAT?"];
const OE1022D_SAFE_QUERIES: &[&str] = &["*IDN?"];

/// Returns `true` if `cmd` is a permitted read-only query for the given device.
pub fn is_safe_query(device: &str, cmd: &str) -> bool {
    let trimmed = cmd.trim();
    match device {
        "smb100a" => SMB100A_SAFE_QUERIES.contains(&trimmed),
        "oe1022d" => OE1022D_SAFE_QUERIES.contains(&trimmed),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Discovery result types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialProbeResult {
    pub port_name: String,
    pub baud_rate: u32,
    pub command_sent: String,
    pub response: Option<String>,
    pub matched_kind: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpProbeResult {
    pub address: String,
    pub command_sent: String,
    pub response: Option<String>,
    pub matched_kind: Option<String>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Serial discovery
// ---------------------------------------------------------------------------

/// Probe a single serial port with a safe query.
///
/// # Safety
/// `command` is rejected unless it is in the `oe1022d` safe-query list.
pub fn probe_serial_port(
    port_name: &str,
    baud_rate: u32,
    command: &str,
    timeout_ms: u64,
) -> SerialProbeResult {
    if !is_safe_query("oe1022d", command) {
        return SerialProbeResult {
            port_name: port_name.to_string(),
            baud_rate,
            command_sent: command.to_string(),
            response: None,
            matched_kind: None,
            error: Some(format!("Command '{}' rejected by safety allow-list", command)),
        };
    }

    let builder = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(timeout_ms));

    let mut port = match builder.open() {
        Ok(p) => p,
        Err(e) => {
            return SerialProbeResult {
                port_name: port_name.to_string(),
                baud_rate,
                command_sent: command.to_string(),
                response: None,
                matched_kind: None,
                error: Some(format!("Open failed: {}", e)),
            };
        }
    };

    let cmd_bytes = format!("{}\r", command.trim());
    if let Err(e) = port.write_all(cmd_bytes.as_bytes()) {
        return SerialProbeResult {
            port_name: port_name.to_string(),
            baud_rate,
            command_sent: command.to_string(),
            response: None,
            matched_kind: None,
            error: Some(format!("Write failed: {}", e)),
        };
    }
    if let Err(e) = port.flush() {
        return SerialProbeResult {
            port_name: port_name.to_string(),
            baud_rate,
            command_sent: command.to_string(),
            response: None,
            matched_kind: None,
            error: Some(format!("Flush failed: {}", e)),
        };
    }

    // Give the device a moment before reading
    std::thread::sleep(Duration::from_millis(50));

    let mut buf = vec![0u8; 1024];
    let n = match port.read(&mut buf) {
        Ok(n) => n,
        Err(e) => {
            return SerialProbeResult {
                port_name: port_name.to_string(),
                baud_rate,
                command_sent: command.to_string(),
                response: None,
                matched_kind: None,
                error: Some(format!("Read failed: {}", e)),
            };
        }
    };
    buf.truncate(n);
    let response = String::from_utf8_lossy(&buf).trim().to_string();

    let matched_kind = classify_serial_response(&response);

    SerialProbeResult {
        port_name: port_name.to_string(),
        baud_rate,
        command_sent: command.to_string(),
        response: if response.is_empty() { None } else { Some(response) },
        matched_kind,
        error: None,
    }
}

fn classify_serial_response(response: &str) -> Option<String> {
    let upper = response.to_uppercase();
    if upper.contains("ROHDE") && upper.contains("SMB100A") {
        Some("smb100a".to_string())
    } else if upper.contains("OE1022D") || upper.contains("LIA-OE1022D") {
        Some("oe1022d".to_string())
    } else if upper.contains("MAYNUO") && upper.contains("M8812") {
        Some("mag_axis".to_string())
    } else {
        None
    }
}

/// Enumerate all serial ports and probe candidates.
pub fn discover_serial_ports() -> Vec<SerialProbeResult> {
    let ports = match serialport::available_ports() {
        Ok(p) => p,
        Err(e) => {
            return vec![SerialProbeResult {
                port_name: "enumerate".to_string(),
                baud_rate: 0,
                command_sent: String::new(),
                response: None,
                matched_kind: None,
                error: Some(format!("Failed to enumerate serial ports: {}", e)),
            }];
        }
    };

    let mut results = Vec::new();
    for port_info in ports {
        let port_name = port_info.port_name;
        // Skip Bluetooth and internal modems on macOS to reduce noise
        if port_name.contains("Bluetooth") || port_name.contains("MALS") || port_name.contains("SAMA") {
            continue;
        }
        // Try OE1022D baud rate first
        let r = probe_serial_port(&port_name, 115200, "*IDN?", 2000);
        results.push(r);
    }
    results
}

// ---------------------------------------------------------------------------
// TCP / LAN discovery
// ---------------------------------------------------------------------------

/// Probe a single TCP address with a safe SCPI query.
///
/// # Safety
/// `command` is rejected unless it is in the `smb100a` safe-query list.
pub fn probe_tcp_address<A: ToSocketAddrs + std::fmt::Debug>(
    addr: A,
    command: &str,
    timeout_ms: u64,
) -> TcpProbeResult {
    let address_str = format!("{:?}", addr); // best-effort display

    if !is_safe_query("smb100a", command) {
        return TcpProbeResult {
            address: address_str,
            command_sent: command.to_string(),
            response: None,
            matched_kind: None,
            error: Some(format!("Command '{}' rejected by safety allow-list", command)),
        };
    }

    let stream = match TcpStream::connect_timeout(
        &addr.to_socket_addrs().unwrap().next().unwrap(),
        Duration::from_millis(timeout_ms),
    ) {
        Ok(s) => s,
        Err(e) => {
            return TcpProbeResult {
                address: address_str,
                command_sent: command.to_string(),
                response: None,
                matched_kind: None,
                error: Some(format!("Connect failed: {}", e)),
            };
        }
    };

    let mut stream = stream;
    if let Err(e) = stream.set_read_timeout(Some(Duration::from_millis(timeout_ms))) {
        return TcpProbeResult {
            address: address_str,
            command_sent: command.to_string(),
            response: None,
            matched_kind: None,
            error: Some(format!("Set timeout failed: {}", e)),
        };
    }

    let cmd_bytes = format!("{}\n", command.trim());
    if let Err(e) = stream.write_all(cmd_bytes.as_bytes()) {
        return TcpProbeResult {
            address: address_str,
            command_sent: command.to_string(),
            response: None,
            matched_kind: None,
            error: Some(format!("Write failed: {}", e)),
        };
    }

    let mut buf = vec![0u8; 1024];
    let n = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(e) => {
            return TcpProbeResult {
                address: address_str,
                command_sent: command.to_string(),
                response: None,
                matched_kind: None,
                error: Some(format!("Read failed: {}", e)),
            };
        }
    };
    buf.truncate(n);
    let response = String::from_utf8_lossy(&buf).trim().to_string();

    let matched_kind = classify_tcp_response(&response);

    TcpProbeResult {
        address: address_str,
        command_sent: command.to_string(),
        response: if response.is_empty() { None } else { Some(response) },
        matched_kind,
        error: None,
    }
}

fn classify_tcp_response(response: &str) -> Option<String> {
    let upper = response.to_uppercase();
    if upper.contains("ROHDE") && upper.contains("SMB100A") {
        Some("smb100a".to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Report generation
// ---------------------------------------------------------------------------

pub fn generate_report(serial_results: &[SerialProbeResult], tcp_results: &[TcpProbeResult]) -> String {
    let mut lines = Vec::new();
    lines.push("# Lab Hardware Discovery Report".to_string());
    lines.push("".to_string());
    lines.push("> **Warning**: This report was generated by a read-only discovery tool.".to_string());
    lines.push("> No state-changing commands were sent to any device.".to_string());
    lines.push("".to_string());

    lines.push("## Serial Port Discovery".to_string());
    lines.push("".to_string());
    lines.push("| Port | Baud | Command | Response | Matched | Error |".to_string());
    lines.push("|------|------|---------|----------|---------|-------|".to_string());
    for r in serial_results {
        lines.push(format!(
            "| {} | {} | `{}` | {} | {} | {} |",
            r.port_name,
            r.baud_rate,
            r.command_sent,
            r.response.as_deref().unwrap_or("_none_"),
            r.matched_kind.as_deref().unwrap_or("—"),
            r.error.as_deref().unwrap_or("—")
        ));
    }
    lines.push("".to_string());

    lines.push("## TCP / LAN Discovery".to_string());
    lines.push("".to_string());
    lines.push("| Address | Command | Response | Matched | Error |".to_string());
    lines.push("|---------|---------|----------|---------|-------|".to_string());
    for r in tcp_results {
        lines.push(format!(
            "| {} | `{}` | {} | {} | {} |",
            r.address,
            r.command_sent,
            r.response.as_deref().unwrap_or("_none_"),
            r.matched_kind.as_deref().unwrap_or("—"),
            r.error.as_deref().unwrap_or("—")
        ));
    }
    lines.push("".to_string());

    lines.push("## Safe-Command Audit".to_string());
    lines.push("".to_string());
    lines.push("### SMB100A permitted queries".to_string());
    for cmd in SMB100A_SAFE_QUERIES {
        lines.push(format!("- `{}`", cmd));
    }
    lines.push("".to_string());
    lines.push("### OE1022D permitted queries".to_string());
    for cmd in OE1022D_SAFE_QUERIES {
        lines.push(format!("- `{}`", cmd));
    }
    lines.push("".to_string());
    lines.push("### Forbidden commands (never sent)".to_string());
    lines.push("- `OUTP ON`".to_string());
    lines.push("- `MOD:STAT ON`".to_string());
    lines.push("- `FM:STAT ON`".to_string());
    lines.push("- `FREQ:MODE SWE`".to_string());
    lines.push("- `SWE:EXEC`".to_string());
    lines.push("".to_string());

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_query_allow_list_smb100a() {
        assert!(is_safe_query("smb100a", "*IDN?"));
        assert!(is_safe_query("smb100a", "SYST:ERR?"));
        assert!(is_safe_query("smb100a", "OUTP?"));
        assert!(is_safe_query("smb100a", "MOD:STAT?"));
        assert!(!is_safe_query("smb100a", "OUTP ON"));
        assert!(!is_safe_query("smb100a", "MOD:STAT ON"));
        assert!(!is_safe_query("smb100a", "FM:STAT ON"));
        assert!(!is_safe_query("smb100a", "FREQ:MODE SWE"));
        assert!(!is_safe_query("smb100a", "SWE:EXEC"));
    }

    #[test]
    fn safe_query_allow_list_oe1022d() {
        assert!(is_safe_query("oe1022d", "*IDN?"));
        assert!(!is_safe_query("oe1022d", "RALL?")); // not in safe list for discovery
        assert!(!is_safe_query("oe1022d", "OUTP ON"));
    }

    #[test]
    fn classify_serial_oe1022d() {
        assert_eq!(
            classify_serial_response("SSI LIA-OE1022D,SN:D6522078,Version:Ver6.3200831"),
            Some("oe1022d".to_string())
        );
    }

    #[test]
    fn classify_serial_mag_axis() {
        assert_eq!(
            classify_serial_response("MAYNUO,M8812,080020960220402020,V2.7"),
            Some("mag_axis".to_string())
        );
    }

    #[test]
    fn classify_tcp_smb100a() {
        assert_eq!(
            classify_tcp_response("Rohde&Schwarz,SMB100A,1406.6000k02/101623,3.1.19.15-3.20.390.24"),
            Some("smb100a".to_string())
        );
    }

    #[test]
    fn generate_report_contains_audit() {
        let report = generate_report(&[], &[]);
        assert!(report.contains("Safe-Command Audit"));
        assert!(report.contains("Forbidden commands"));
        assert!(report.contains("OUTP ON"));
    }
}
