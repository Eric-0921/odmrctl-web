//! Read-only real-device station snapshot library.
//!
//! **Safety invariant**: this library only sends pre-defined read-only queries.
//! There is no generic `send(cmd)` API. All outbound strings are validated
//! against hard-coded allow-lists before transmission. A secondary forbidden-
//! pattern gate provides defense in depth.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Hard-coded query allow-lists
// ---------------------------------------------------------------------------

const SMB100A_QUERIES: &[&str] = &[
    "*IDN?",
    "SYST:ERR?",
    "OUTP?",
    "MOD:STAT?",
    "FREQ:MODE?",
    "FREQ?",
    "POW?",
    "POW:ALC?",
    "FM:STAT?",
    "FM:SOUR?",
    "FM:DEV?",
    "LFO?",
    "LFO:FREQ?",
    "LFO:VOLT?",
    "LFO:SHAP?",
    "SWE:MODE?",
    "SWE:SPAC?",
    "SWE:FREQ:STEP?",
    "SWE:FREQ:DWEL?",
    "FREQ:STAR?",
    "FREQ:STOP?",
];

const OE1022D_QUERIES: &[&str] = &[
    "*IDN?",
    "FMODD? 2",
    "RSLPD? 2",
    "FREQD? 2",
    "PHASD? 2",
    "ISRCD? 2",
    "SENSD? 2",
    "OFLTD? 2",
    "OFSLD? 2",
    "HARMD? 2",
    "RALL?",
];

const FORBIDDEN_PATTERNS: &[&str] = &[
    "OUTP ON",
    "MOD:STAT ON",
    "FM:STAT ON",
    "FREQ:MODE SWE",
    "SWE:EXEC",
    "INIT",
    "RUN",
    "RST",
    "*RST",
    "SSETD",
    "RSETD",
];

// ---------------------------------------------------------------------------
// Safety validation
// ---------------------------------------------------------------------------

/// Validates that `cmd` is in the pre-defined allow-list and does not contain
/// any forbidden substring.
pub fn validate_query(cmd: &str, allow_list: &[&str]) -> Result<(), SnapshotError> {
    let trimmed = cmd.trim();
    if !allow_list.contains(&trimmed) {
        return Err(SnapshotError::NotInAllowList {
            cmd: trimmed.to_string(),
        });
    }
    for pat in FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return Err(SnapshotError::ForbiddenPattern {
                cmd: trimmed.to_string(),
                pattern: pat.to_string(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotRecord {
    pub device: String,
    pub transport: String,
    pub command: String,
    pub response: Option<String>,
    pub timestamp: String,
    pub duration_ms: u64,
    pub pass_fail: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SnapshotError {
    NotInAllowList { cmd: String },
    ForbiddenPattern { cmd: String, pattern: String },
    IoError(String),
    Timeout { cmd: String },
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::NotInAllowList { cmd } => {
                write!(f, "command '{}' is not in the allow-list", cmd)
            }
            SnapshotError::ForbiddenPattern { cmd, pattern } => {
                write!(
                    f,
                    "command '{}' contains forbidden pattern '{}'",
                    cmd, pattern
                )
            }
            SnapshotError::IoError(e) => write!(f, "io error: {}", e),
            SnapshotError::Timeout { cmd } => {
                write!(f, "timeout waiting for response to '{}'", cmd)
            }
        }
    }
}

impl std::error::Error for SnapshotError {}

// ---------------------------------------------------------------------------
// SMB100A snapshot
// ---------------------------------------------------------------------------

pub struct Smb100aSnapshot {
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
}

impl Smb100aSnapshot {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            timeout_ms: 2000,
        }
    }

    pub fn run(&self) -> Result<Vec<SnapshotRecord>, SnapshotError> {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect_timeout(
            &addr.parse::<std::net::SocketAddr>().map_err(|e| {
                SnapshotError::IoError(format!("parse address: {}", e))
            })?,
            Duration::from_millis(self.timeout_ms),
        )
        .map_err(|e| SnapshotError::IoError(format!("connect: {}", e)))?;

        stream
            .set_read_timeout(Some(Duration::from_millis(self.timeout_ms)))
            .map_err(|e| SnapshotError::IoError(format!("set timeout: {}", e)))?;

        let mut records = Vec::with_capacity(SMB100A_QUERIES.len());
        for cmd in SMB100A_QUERIES {
            validate_query(cmd, SMB100A_QUERIES)?;
            let record = tcp_query(&mut stream, cmd, &addr)?;
            records.push(record);
        }
        Ok(records)
    }
}

fn tcp_query(
    stream: &mut TcpStream,
    cmd: &str,
    addr: &str,
) -> Result<SnapshotRecord, SnapshotError> {
    let ts = utc_now();
    let start = Instant::now();

    let cmd_bytes = format!("{}\n", cmd.trim());
    stream
        .write_all(cmd_bytes.as_bytes())
        .map_err(|e| SnapshotError::IoError(format!("write: {}", e)))?;

    let mut buf = vec![0u8; 1024];
    let n = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(e) => {
            return Ok(SnapshotRecord {
                device: "smb100a.main".to_string(),
                transport: format!("tcp_scpi:{}", addr),
                command: cmd.to_string(),
                response: None,
                timestamp: ts,
                duration_ms: start.elapsed().as_millis() as u64,
                pass_fail: "timeout".to_string(),
                notes: format!("read error: {}", e),
            });
        }
    };

    buf.truncate(n);
    let response = String::from_utf8_lossy(&buf).trim().to_string();
    let has_response = !response.is_empty();

    Ok(SnapshotRecord {
        device: "smb100a.main".to_string(),
        transport: format!("tcp_scpi:{}", addr),
        command: cmd.to_string(),
        response: if has_response { Some(response) } else { None },
        timestamp: ts,
        duration_ms: start.elapsed().as_millis() as u64,
        pass_fail: if has_response {
            "pass".to_string()
        } else {
            "timeout".to_string()
        },
        notes: String::new(),
    })
}

// ---------------------------------------------------------------------------
// OE1022D snapshot
// ---------------------------------------------------------------------------

pub struct Oe1022dSnapshot {
    pub port: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
}

impl Oe1022dSnapshot {
    pub fn new(port: &str, baud_rate: u32) -> Self {
        Self {
            port: port.to_string(),
            baud_rate,
            timeout_ms: 2000,
        }
    }

    pub fn run(&self) -> Result<Vec<SnapshotRecord>, SnapshotError> {
        let builder = serialport::new(&self.port, self.baud_rate)
            .timeout(Duration::from_millis(self.timeout_ms));

        let mut port = builder
            .open()
            .map_err(|e| SnapshotError::IoError(format!("open serial: {}", e)))?;

        let mut records = Vec::with_capacity(OE1022D_QUERIES.len());
        for cmd in OE1022D_QUERIES {
            validate_query(cmd, OE1022D_QUERIES)?;
            let record = serial_query(&mut port, cmd, &self.port, self.baud_rate)?;
            records.push(record);
        }
        Ok(records)
    }
}

fn serial_query(
    port: &mut Box<dyn serialport::SerialPort>,
    cmd: &str,
    port_name: &str,
    baud: u32,
) -> Result<SnapshotRecord, SnapshotError> {
    let ts = utc_now();
    let start = Instant::now();

    let cmd_bytes = format!("{}\r", cmd.trim());
    port.write_all(cmd_bytes.as_bytes())
        .map_err(|e| SnapshotError::IoError(format!("write: {}", e)))?;
    port.flush()
        .map_err(|e| SnapshotError::IoError(format!("flush: {}", e)))?;

    std::thread::sleep(Duration::from_millis(50));

    let mut buf = vec![0u8; 2048];
    let n = match port.read(&mut buf) {
        Ok(n) => n,
        Err(_) => {
            return Ok(SnapshotRecord {
                device: "oe1022d.main".to_string(),
                transport: format!("serial:{}:{}", port_name, baud),
                command: cmd.to_string(),
                response: None,
                timestamp: ts,
                duration_ms: start.elapsed().as_millis() as u64,
                pass_fail: "timeout".to_string(),
                notes: "serial read timeout".to_string(),
            });
        }
    };

    buf.truncate(n);
    let response = String::from_utf8_lossy(&buf).trim().to_string();
    let has_response = !response.is_empty();

    Ok(SnapshotRecord {
        device: "oe1022d.main".to_string(),
        transport: format!("serial:{}:{}", port_name, baud),
        command: cmd.to_string(),
        response: if has_response { Some(response) } else { None },
        timestamp: ts,
        duration_ms: start.elapsed().as_millis() as u64,
        pass_fail: if has_response {
            "pass".to_string()
        } else {
            "timeout".to_string()
        },
        notes: String::new(),
    })
}

// ---------------------------------------------------------------------------
// Output formatters
// ---------------------------------------------------------------------------

pub fn records_to_jsonl(records: &[SnapshotRecord]) -> String {
    let mut lines = Vec::with_capacity(records.len());
    for r in records {
        let json = format!(
            "{{\"device\":\"{}\",\"transport\":\"{}\",\"command\":\"{}\",\"response\":{},\"timestamp\":\"{}\",\"duration_ms\":{},\"pass_fail\":\"{}\",\"notes\":\"{}\"}}",
            escape_json(&r.device),
            escape_json(&r.transport),
            escape_json(&r.command),
            match &r.response {
                Some(v) => format!("\"{}\"", escape_json(v)),
                None => "null".to_string(),
            },
            r.timestamp,
            r.duration_ms,
            r.pass_fail,
            escape_json(&r.notes)
        );
        lines.push(json);
    }
    lines.join("\n")
}

pub fn records_to_markdown(
    smb100a_records: &[SnapshotRecord],
    oe1022d_records: &[SnapshotRecord],
) -> String {
    let mut lines = Vec::new();
    lines.push("# Real-Device Read-Only Station Snapshot".to_string());
    lines.push("".to_string());
    lines.push("> **Safety Audit**: Only read-only queries were sent.".to_string());
    lines.push("> No state-changing commands were transmitted.".to_string());
    lines.push("> All commands validated against hard-coded allow-lists.".to_string());
    lines.push("".to_string());

    lines.push("## SMB100A".to_string());
    lines.push("".to_string());
    lines.push("| # | Command | Response | Duration (ms) | Status | Notes |".to_string());
    lines.push("|---|---------|----------|---------------|--------|-------|".to_string());
    for (i, r) in smb100a_records.iter().enumerate() {
        lines.push(format!(
            "| {} | `{}` | {} | {} | {} | {} |",
            i + 1,
            r.command,
            r.response.as_deref().unwrap_or("_(timeout)_"),
            r.duration_ms,
            r.pass_fail,
            r.notes
        ));
    }
    lines.push("".to_string());

    lines.push("## OE1022D".to_string());
    lines.push("".to_string());
    lines.push("| # | Command | Response | Duration (ms) | Status | Notes |".to_string());
    lines.push("|---|---------|----------|---------------|--------|-------|".to_string());
    for (i, r) in oe1022d_records.iter().enumerate() {
        lines.push(format!(
            "| {} | `{}` | {} | {} | {} | {} |",
            i + 1,
            r.command,
            r.response.as_deref().unwrap_or("_(timeout)_"),
            r.duration_ms,
            r.pass_fail,
            r.notes
        ));
    }
    lines.push("".to_string());

    lines.push("## Forbidden Command Audit".to_string());
    lines.push("".to_string());
    lines.push("The following patterns were explicitly blocked by the snapshot tool:".to_string());
    for pat in FORBIDDEN_PATTERNS {
        lines.push(format!("- `{}`", pat));
    }
    lines.push("".to_string());

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn utc_now() -> String {
    // Best-effort UTC timestamp without chrono dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}Z", now.as_secs(), now.subsec_millis())
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_query_accepts_allowlisted() {
        assert!(validate_query("*IDN?", SMB100A_QUERIES).is_ok());
        assert!(validate_query("FMODD? 2", OE1022D_QUERIES).is_ok());
    }

    #[test]
    fn validate_query_rejects_unknown() {
        assert!(validate_query("UNKNOWN?", SMB100A_QUERIES).is_err());
        assert!(validate_query("FREQ 2GHz", SMB100A_QUERIES).is_err());
    }

    #[test]
    fn validate_query_rejects_forbidden_patterns() {
        assert!(validate_query("OUTP ON", SMB100A_QUERIES).is_err());
        assert!(validate_query("MOD:STAT ON", SMB100A_QUERIES).is_err());
        assert!(validate_query("FM:STAT ON", SMB100A_QUERIES).is_err());
        assert!(validate_query("FREQ:MODE SWE", SMB100A_QUERIES).is_err());
        assert!(validate_query("SWE:EXEC", SMB100A_QUERIES).is_err());
        assert!(validate_query("*RST", OE1022D_QUERIES).is_err());
    }

    #[test]
    fn smb100a_query_list_is_readonly_only() {
        for cmd in SMB100A_QUERIES {
            assert!(
                cmd.ends_with('?'),
                "SMB100A query '{}' does not end with '?'; possible setter leak",
                cmd
            );
        }
    }

    #[test]
    fn oe1022d_query_list_is_readonly_only() {
        for cmd in OE1022D_QUERIES {
            assert!(
                cmd.contains('?'),
                "OE1022D query '{}' does not contain '?'; possible setter leak",
                cmd
            );
            // OE1022D queries end with channel number (e.g., "FMODD? 2"),
            // so we check they do NOT end with a digit-only pattern (which would indicate a setter).
            let last_char = cmd.chars().last().unwrap();
            assert!(
                last_char.is_ascii_digit() || last_char == '?',
                "OE1022D query '{}' has unexpected trailing character '{}'",
                cmd,
                last_char
            );
        }
    }

    #[test]
    fn jsonl_formatting() {
        let records = vec![SnapshotRecord {
            device: "smb100a.main".to_string(),
            transport: "tcp:169.254.2.20:5025".to_string(),
            command: "*IDN?".to_string(),
            response: Some("Rohde&Schwarz,SMB100A".to_string()),
            timestamp: "2026-05-30T12:00:00Z".to_string(),
            duration_ms: 12,
            pass_fail: "pass".to_string(),
            notes: "".to_string(),
        }];
        let jsonl = records_to_jsonl(&records);
        assert!(jsonl.contains("Rohde&Schwarz,SMB100A"));
        assert!(jsonl.contains("*IDN?"));
    }

    #[test]
    fn markdown_contains_audit() {
        let md = records_to_markdown(&[], &[]);
        assert!(md.contains("Forbidden Command Audit"));
        assert!(md.contains("OUTP ON"));
    }
}
