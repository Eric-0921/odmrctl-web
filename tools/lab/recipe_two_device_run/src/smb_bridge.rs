//! SMB100A TCP transport bridge for M3.4 real mode.

use crate::types::M3_4CommandAuditEntry;
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
        audit: &mut Vec<M3_4CommandAuditEntry>,
        ts: u64,
    ) -> Result<String, String> {
        let allowed = validate_smb_command(cmd).is_ok();
        let safety = is_safety_relevant(cmd);

        if !allowed {
            let reason = validate_smb_command(cmd).unwrap_err();
            audit.push(M3_4CommandAuditEntry {
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

        audit.push(M3_4CommandAuditEntry {
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

    #[allow(dead_code)]
    pub fn query_without_audit(&mut self, cmd: &str) -> Result<String, String> {
        self.send_raw(cmd)?;
        std::thread::sleep(self.query_delay);
        let response = self.read_response()?;
        self.drain();
        Ok(response)
    }
}

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
    "LFO?",
    "LFO:FREQ?",
    "LFO:VOLT?",
    "LFO:SHAP?",
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
    "LFO:FREQ ",
    "LFO:SHAP ",
    "LFO:VOLT ",
];

// Forbidden patterns now in crate::constants::SMB_FORBIDDEN_PATTERNS

fn validate_smb_command(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if trimmed.contains(';') {
        return Err(format!(
            "SMB command '{}' contains semicolon - rejected",
            trimmed
        ));
    }
    let upper = trimmed.to_ascii_uppercase();
    for pat in crate::constants::SMB_FORBIDDEN_PATTERNS {
        if upper.contains(pat) {
            return Err(format!(
                "SMB command '{}' matches forbidden pattern '{}'",
                trimmed, pat
            ));
        }
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
