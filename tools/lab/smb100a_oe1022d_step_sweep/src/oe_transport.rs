use crate::smb_safety::{classify_oe_command_for_audit, validate_oe_command};
use crate::timeline::utc_now_ms;
use crate::types::CommandAuditEntry;
use odmr_oe1022d::RALL_FRAME_BYTES;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

pub struct OeSerialTransport {
    port: Box<dyn serialport::SerialPort>,
    #[allow(dead_code)]
    port_path: String,
    timeout_ms: u64,
}

impl OeSerialTransport {
    pub fn connect(port_path: &str, baud: u32, timeout_ms: u64) -> Result<Self, String> {
        let port = serialport::new(port_path, baud)
            .timeout(Duration::from_millis(timeout_ms))
            .open()
            .map_err(|e| format!("OE serial open {}: {}", port_path, e))?;
        Ok(Self {
            port,
            port_path: port_path.into(),
            timeout_ms,
        })
    }

    /// Send *IDN? and return the identity string.
    pub fn query_identity(
        &mut self,
        audit: &mut Vec<CommandAuditEntry>,
        forbidden_attempted: &mut Vec<String>,
    ) -> Result<String, String> {
        let cmd = "*IDN?";
        let ts = utc_now_ms();
        if let Err(e) = validate_oe_command(cmd) {
            audit.push(CommandAuditEntry {
                timestamp_unix_ms: ts,
                device_id: "oe1022d".into(),
                command: cmd.into(),
                command_class: classify_oe_command_for_audit(cmd).into(),
                allowed: false,
                sent_to_transport: false,
                manual_approval_required: None,
                manual_approval_present: None,
                rejection_reason: Some(e.clone()),
                response_preview: None,
                transport_error: None,
                safety_relevant: None,
            });
            forbidden_attempted.push(cmd.to_string());
            return Err(e);
        }

        self.port
            .clear(serialport::ClearBuffer::Input)
            .map_err(|e| format!("OE clear buffer: {}", e))?;
        let cmd_bytes = format!("{}\r", cmd);
        self.port
            .write_all(cmd_bytes.as_bytes())
            .map_err(|e| format!("OE write: {}", e))?;
        self.port.flush().map_err(|e| format!("OE flush: {}", e))?;

        std::thread::sleep(Duration::from_millis(200));

        let mut buf = vec![0u8; 256];
        let n = self
            .port
            .read(&mut buf)
            .map_err(|e| format!("OE read: {}", e))?;
        let resp = String::from_utf8_lossy(&buf[..n])
            .trim()
            .trim_matches('\0')
            .to_string();

        audit.push(CommandAuditEntry {
            timestamp_unix_ms: ts,
            device_id: "oe1022d".into(),
            command: cmd.into(),
            command_class: "oe_identity".into(),
            allowed: true,
            sent_to_transport: true,
            manual_approval_required: None,
            manual_approval_present: None,
            rejection_reason: None,
            response_preview: Some(resp.clone()),
            transport_error: None,
            safety_relevant: None,
        });
        Ok(resp)
    }

    /// Send RALL? and capture the full 12288-byte frame.
    pub fn capture_rall_frame(
        &mut self,
        audit: &mut Vec<CommandAuditEntry>,
        forbidden_attempted: &mut Vec<String>,
        frame_delay_ms: u64,
    ) -> Result<(Vec<u8>, u64), String> {
        let cmd = "RALL?";
        let ts = utc_now_ms();

        if let Err(e) = validate_oe_command(cmd) {
            audit.push(CommandAuditEntry {
                timestamp_unix_ms: ts,
                device_id: "oe1022d".into(),
                command: cmd.into(),
                command_class: classify_oe_command_for_audit(cmd).into(),
                allowed: false,
                sent_to_transport: false,
                manual_approval_required: None,
                manual_approval_present: None,
                rejection_reason: Some(e.clone()),
                response_preview: None,
                transport_error: None,
                safety_relevant: None,
            });
            forbidden_attempted.push(cmd.to_string());
            return Err(e);
        }

        self.port
            .clear(serialport::ClearBuffer::Input)
            .map_err(|e| format!("OE clear buffer: {}", e))?;
        let cmd_bytes = format!("{}\r", cmd);
        self.port
            .write_all(cmd_bytes.as_bytes())
            .map_err(|e| format!("OE write: {}", e))?;
        self.port.flush().map_err(|e| format!("OE flush: {}", e))?;

        std::thread::sleep(Duration::from_millis(frame_delay_ms));

        let deadline = Instant::now() + Duration::from_millis(self.timeout_ms);
        let mut frame_buf = Vec::new();
        let mut chunk = vec![0u8; 4096];
        loop {
            if Instant::now() > deadline {
                break;
            }
            match self.port.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    frame_buf.extend_from_slice(&chunk[..n]);
                    if frame_buf.len() >= RALL_FRAME_BYTES {
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(e) => {
                    return Err(format!("OE read error: {}", e));
                }
            }
        }

        let frame_len = frame_buf.len().min(RALL_FRAME_BYTES);
        let actual = if frame_buf.len() >= RALL_FRAME_BYTES {
            frame_buf[..RALL_FRAME_BYTES].to_vec()
        } else {
            frame_buf.clone()
        };

        let elapsed_ms = Instant::now()
            .duration_since(deadline - Duration::from_millis(self.timeout_ms))
            .as_millis() as u64;

        audit.push(CommandAuditEntry {
            timestamp_unix_ms: ts,
            device_id: "oe1022d".into(),
            command: cmd.into(),
            command_class: "oe_acquisition".into(),
            allowed: true,
            sent_to_transport: true,
            manual_approval_required: None,
            manual_approval_present: None,
            rejection_reason: None,
            response_preview: Some(format!("{} bytes", frame_len)),
            transport_error: None,
            safety_relevant: None,
        });
        Ok((actual, elapsed_ms))
    }

    pub fn close(self) {
        drop(self.port);
    }
}
