use crate::smb_safety::{
    classify_command_for_audit, is_safety_relevant, validate_smb_sweep_query,
    validate_smb_sweep_set,
};
use crate::timeline::utc_now_ms;
use crate::types::CommandAuditEntry;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub struct SmbTransport {
    stream: TcpStream,
    timeout_ms: u64,
}

impl SmbTransport {
    pub fn connect(host: &str, port: u16, timeout_ms: u64) -> Result<Self, String> {
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(&addr)
            .map_err(|e| format!("TCP connect to {} failed: {}", addr, e))?;
        stream
            .set_read_timeout(Some(Duration::from_millis(timeout_ms)))
            .map_err(|e| format!("set read timeout: {}", e))?;
        stream
            .set_write_timeout(Some(Duration::from_millis(timeout_ms)))
            .map_err(|e| format!("set write timeout: {}", e))?;
        Ok(Self { stream, timeout_ms })
    }

    pub fn query(&mut self, cmd: &str) -> Result<String, String> {
        let cmd_with_term = format!("{}\n", cmd);
        self.stream
            .write_all(cmd_with_term.as_bytes())
            .map_err(|e| format!("TCP write failed: {}", e))?;
        self.stream
            .flush()
            .map_err(|e| format!("TCP flush failed: {}", e))?;
        let mut reader = BufReader::new(&self.stream);
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => Err("TCP read returned empty".to_string()),
            Ok(_) => {
                line = line.trim().to_string();
                Ok(line)
            }
            Err(e) => Err(format!("TCP read failed: {}", e)),
        }
    }

    pub fn send_no_response(&mut self, cmd: &str) -> Result<(), String> {
        let cmd_with_term = format!("{}\n", cmd);
        self.stream
            .write_all(cmd_with_term.as_bytes())
            .map_err(|e| format!("TCP write failed: {}", e))?;
        self.stream
            .flush()
            .map_err(|e| format!("TCP flush failed: {}", e))?;
        Ok(())
    }

    pub fn drain_buffer(&mut self) {
        let _ = self
            .stream
            .set_read_timeout(Some(Duration::from_millis(50)));
        let mut buf = [0u8; 256];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        let _ = self
            .stream
            .set_read_timeout(Some(Duration::from_millis(self.timeout_ms)));
    }

    pub fn close(self) {
        drop(self.stream);
    }
}

pub fn do_smb_sweep_query(
    transport: &mut SmbTransport,
    audit: &mut Vec<CommandAuditEntry>,
    forbidden_attempted: &mut Vec<String>,
    delay_ms: u64,
    cmd: &str,
) -> Result<String, String> {
    let ts = utc_now_ms();
    if let Err(e) = validate_smb_sweep_query(cmd) {
        audit.push(CommandAuditEntry {
            timestamp_unix_ms: ts,
            device_id: "smb100a".into(),
            command: cmd.into(),
            command_class: classify_command_for_audit(cmd).into(),
            allowed: false,
            sent_to_transport: false,
            manual_approval_required: None,
            manual_approval_present: None,
            rejection_reason: Some(e.clone()),
            response_preview: None,
            transport_error: None,
            safety_relevant: Some(is_safety_relevant(cmd)),
        });
        forbidden_attempted.push(cmd.to_string());
        return Err(e);
    }
    let resp = transport.query(cmd)?;
    transport.drain_buffer();
    if delay_ms > 0 {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
    audit.push(CommandAuditEntry {
        timestamp_unix_ms: ts,
        device_id: "smb100a".into(),
        command: cmd.into(),
        command_class: "query".into(),
        allowed: true,
        sent_to_transport: true,
        manual_approval_required: None,
        manual_approval_present: None,
        rejection_reason: None,
        response_preview: Some(resp.clone()),
        transport_error: None,
        safety_relevant: Some(is_safety_relevant(cmd)),
    });
    Ok(resp)
}

pub fn do_smb_sweep_set(
    transport: &mut SmbTransport,
    audit: &mut Vec<CommandAuditEntry>,
    forbidden_attempted: &mut Vec<String>,
    delay_ms: u64,
    cmd: &str,
    requires_approval: bool,
    approval_present: bool,
) -> Result<(), String> {
    let ts = utc_now_ms();
    if let Err(e) = validate_smb_sweep_set(cmd) {
        audit.push(CommandAuditEntry {
            timestamp_unix_ms: ts,
            device_id: "smb100a".into(),
            command: cmd.into(),
            command_class: classify_command_for_audit(cmd).into(),
            allowed: false,
            sent_to_transport: false,
            manual_approval_required: Some(requires_approval),
            manual_approval_present: Some(approval_present),
            rejection_reason: Some(e.clone()),
            response_preview: None,
            transport_error: None,
            safety_relevant: Some(is_safety_relevant(cmd)),
        });
        forbidden_attempted.push(cmd.to_string());
        return Err(e);
    }

    if requires_approval && !approval_present {
        audit.push(CommandAuditEntry {
            timestamp_unix_ms: ts,
            device_id: "smb100a".into(),
            command: cmd.into(),
            command_class: "set".into(),
            allowed: false,
            sent_to_transport: false,
            manual_approval_required: Some(true),
            manual_approval_present: Some(false),
            rejection_reason: Some("Operator approval required but not present".into()),
            response_preview: None,
            transport_error: None,
            safety_relevant: Some(true),
        });
        forbidden_attempted.push(cmd.to_string());
        return Err("Operator approval required".into());
    }

    transport.send_no_response(cmd)?;
    transport.drain_buffer();
    if delay_ms > 0 {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
    audit.push(CommandAuditEntry {
        timestamp_unix_ms: ts,
        device_id: "smb100a".into(),
        command: cmd.into(),
        command_class: "set".into(),
        allowed: true,
        sent_to_transport: true,
        manual_approval_required: Some(requires_approval),
        manual_approval_present: Some(approval_present),
        rejection_reason: None,
        response_preview: None,
        transport_error: None,
        safety_relevant: Some(is_safety_relevant(cmd)),
    });
    Ok(())
}
