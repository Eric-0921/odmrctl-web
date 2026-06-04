//! SMB100A TCP transport bridge for Mag-M5A.
//! Copied and adapted from recipe_two_device_run/src/smb_bridge.rs.

use crate::types::CommandAuditEntry;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub struct SmbTransport {
    stream: TcpStream,
    timeout: Duration,
    query_delay: Duration,
}

impl SmbTransport {
    pub fn connect(
        host: &str,
        port: u16,
        timeout_ms: u64,
        query_delay_ms: u64,
    ) -> Result<Self, String> {
        let timeout = Duration::from_millis(timeout_ms);
        let stream = TcpStream::connect_timeout(
            &format!("{}:{}", host, port)
                .parse()
                .map_err(|e| format!("parse addr: {}", e))?,
            timeout,
        )
        .map_err(|e| format!("SMB connect {}:{}: {}", host, port, e))?;
        stream
            .set_nonblocking(false)
            .map_err(|e| format!("set blocking: {}", e))?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|e| format!("set read timeout: {}", e))?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|e| format!("set write timeout: {}", e))?;
        Ok(SmbTransport {
            stream,
            timeout,
            query_delay: Duration::from_millis(query_delay_ms),
        })
    }

    pub fn send_raw(&mut self, cmd: &str) -> Result<(), String> {
        write!(self.stream, "{}\r\n", cmd).map_err(|e| format!("write: {}", e))?;
        self.stream.flush().map_err(|e| format!("flush: {}", e))?;
        Ok(())
    }

    pub fn read_response(&mut self) -> Result<String, String> {
        let mut buf = [0u8; 1];
        let mut line = Vec::new();
        loop {
            let n = self
                .stream
                .read(&mut buf)
                .map_err(|e| format!("read: {}", e))?;
            if n == 0 {
                break;
            }
            if buf[0] == b'\n' {
                break;
            }
            line.push(buf[0]);
        }
        let s = String::from_utf8_lossy(&line).trim().to_string();
        Ok(s)
    }

    fn drain(&mut self) {
        let _ = self
            .stream
            .set_read_timeout(Some(Duration::from_millis(10)));
        let mut buf = [0u8; 256];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => continue,
            }
        }
        let _ = self.stream.set_read_timeout(Some(self.timeout));
    }

    pub fn query(
        &mut self,
        cmd: &str,
        audit: &mut Vec<CommandAuditEntry>,
        ts: u64,
    ) -> Result<String, String> {
        let allowed = validate_smb_command(cmd).is_ok();
        let safety = is_safety_relevant(cmd);

        if !allowed {
            let reason = validate_smb_command(cmd).unwrap_err();
            audit.push(CommandAuditEntry {
                seq: audit.len() as u64,
                timestamp_unix_ms: ts,
                device_id: "smb100a".into(),
                command: cmd.to_string(),
                command_class: if cmd.ends_with('?') { "query" } else { "set" }.into(),
                allowed: false,
                sent_to_transport: false,
                rejection_reason: Some(reason.clone()),
                response_preview: None,
                transport_error: None,
                safety_relevant: safety,
            });
            return Err(reason);
        }

        self.send_raw(cmd)?;
        std::thread::sleep(self.query_delay);
        let is_query = cmd.trim().ends_with('?');
        let response = if is_query {
            self.read_response()?
        } else {
            "ACK".into()
        };
        if is_query {
            self.drain();
        }

        audit.push(CommandAuditEntry {
            seq: audit.len() as u64,
            timestamp_unix_ms: ts,
            device_id: "smb100a".into(),
            command: cmd.to_string(),
            command_class: if cmd.ends_with('?') { "query" } else { "set" }.into(),
            allowed: true,
            sent_to_transport: true,
            rejection_reason: None,
            response_preview: Some(response.clone()),
            transport_error: None,
            safety_relevant: safety,
        });

        Ok(response)
    }
}

// ---------------------------------------------------------------------------
// Fake SMB transport for testing
// ---------------------------------------------------------------------------

pub struct FakeSmbTransport {
    pub idn: String,
    pub outp: bool,
    pub mod_stat: bool,
    pub freq_hz: u64,
    pub pow_dbm: f64,
    pub syst_err: String,
    /// If set, `query` returns Err when this exact command is received.
    pub fail_on: Option<String>,
}

impl FakeSmbTransport {
    pub fn new() -> Self {
        FakeSmbTransport {
            idn: "Rohde&Schwarz,SMB100A,123456,3.2.0".into(),
            outp: false,
            mod_stat: false,
            freq_hz: 0,
            pow_dbm: 0.0,
            syst_err: "0,No error".into(),
            fail_on: None,
        }
    }

    pub fn query(
        &mut self,
        cmd: &str,
        audit: &mut Vec<CommandAuditEntry>,
        ts: u64,
    ) -> Result<String, String> {
        if self.fail_on.as_deref() == Some(cmd.trim()) {
            audit.push(CommandAuditEntry {
                seq: audit.len() as u64,
                timestamp_unix_ms: ts,
                device_id: "smb100a".into(),
                command: cmd.to_string(),
                command_class: if cmd.ends_with('?') { "query" } else { "set" }.into(),
                allowed: true,
                sent_to_transport: false,
                rejection_reason: None,
                response_preview: None,
                transport_error: Some("injected failure".into()),
                safety_relevant: is_safety_relevant(cmd),
            });
            return Err(format!("injected failure for {}", cmd));
        }
        let response = match cmd.trim() {
            "*IDN?" => self.idn.clone(),
            "OUTP?" => (if self.outp { "1" } else { "0" }).into(),
            "MOD:STAT?" => (if self.mod_stat { "1" } else { "0" }).into(),
            "FREQ?" => self.freq_hz.to_string(),
            "POW?" => self.pow_dbm.to_string(),
            "SYST:ERR?" => self.syst_err.clone(),
            _ => {
                if cmd.starts_with("FREQ ") {
                    self.freq_hz = cmd[5..].trim().parse().unwrap_or(0);
                    "ACK".into()
                } else if cmd.starts_with("POW ") {
                    self.pow_dbm = cmd[4..].trim().parse().unwrap_or(0.0);
                    "ACK".into()
                } else if cmd == "OUTP ON" {
                    self.outp = true;
                    "ACK".into()
                } else if cmd == "OUTP OFF" {
                    self.outp = false;
                    "ACK".into()
                } else {
                    "ACK".into()
                }
            }
        };

        audit.push(CommandAuditEntry {
            seq: audit.len() as u64,
            timestamp_unix_ms: ts,
            device_id: "smb100a".into(),
            command: cmd.to_string(),
            command_class: if cmd.ends_with('?') { "query" } else { "set" }.into(),
            allowed: true,
            sent_to_transport: true,
            rejection_reason: None,
            response_preview: Some(response.clone()),
            transport_error: None,
            safety_relevant: is_safety_relevant(cmd),
        });

        Ok(response)
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

const SMB_QUERY_ALLOWLIST: &[&str] = &[
    "*IDN?",
    "OUTP?",
    "MOD:STAT?",
    "FREQ?",
    "POW?",
    "POW:ALC?",
    "FM:STAT?",
    "FM:SOUR?",
    "FM:DEV?",
    "SYST:ERR?",
];

const SMB_SET_ALLOWLIST: &[&str] = &[
    "FREQ ",
    "POW ",
    "POW:ALC ",
    "FM:SOUR ",
    "FM:DEV ",
    "FM:STAT ",
    "MOD:STAT ",
    "OUTP ON",
    "OUTP OFF",
];

pub(crate) fn validate_smb_command(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if trimmed.contains(';') {
        return Err(format!(
            "SMB command '{}' contains semicolon - rejected",
            trimmed
        ));
    }
    if trimmed.ends_with('?') {
        if !SMB_QUERY_ALLOWLIST.contains(&trimmed) {
            return Err(format!("SMB query '{}' not in allowlist", trimmed));
        }
    } else {
        let mut ok = false;
        for allowed in SMB_SET_ALLOWLIST {
            if trimmed.starts_with(allowed) {
                ok = true;
                break;
            }
        }
        if !ok {
            return Err(format!("SMB set '{}' not in allowlist", trimmed));
        }
    }
    Ok(())
}

fn is_safety_relevant(cmd: &str) -> bool {
    matches!(
        cmd.trim(),
        "OUTP?"
            | "MOD:STAT?"
            | "SYST:ERR?"
            | "OUTP ON"
            | "OUTP OFF"
            | "MOD:STAT ON"
            | "MOD:STAT OFF"
            | "FM:STAT ON"
            | "FM:STAT OFF"
    )
}
