//! OE1022D serial transport bridge for Mag-M5A.
//! Copied and adapted from recipe_two_device_run/src/oe_bridge.rs.

use crate::types::CommandAuditEntry;
use odmr_oe1022d::RALL_FRAME_BYTES;

pub struct OeTransport {
    port: Box<dyn serialport::SerialPort>,
}

impl OeTransport {
    pub fn connect(port: &str, baud: u32, timeout_ms: u64) -> Result<Self, String> {
        let sp = serialport::new(port, baud)
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .open()
            .map_err(|e| format!("OE connect {}: {}", port, e))?;
        Ok(OeTransport { port: sp })
    }

    pub fn query_identity(
        &mut self,
        audit: &mut Vec<CommandAuditEntry>,
        ts: u64,
    ) -> Result<String, String> {
        let cmd = "*IDN?";
        self.port
            .write_all(b"*IDN?\r")
            .map_err(|e| format!("OE write: {}", e))?;
        std::thread::sleep(std::time::Duration::from_millis(200));

        let mut buf = [0u8; 512];
        let n = self
            .port
            .read(&mut buf)
            .map_err(|e| format!("OE read: {}", e))?;
        let response = String::from_utf8_lossy(&buf[..n]).trim().to_string();

        audit.push(CommandAuditEntry {
            seq: audit.len() as u64,
            timestamp_unix_ms: ts,
            device_id: "oe1022d".into(),
            command: cmd.to_string(),
            command_class: "query".into(),
            allowed: true,
            sent_to_transport: true,
            rejection_reason: None,
            response_preview: Some(response.clone()),
            transport_error: None,
            safety_relevant: false,
        });

        Ok(response)
    }

    pub fn capture_frame(
        &mut self,
        audit: &mut Vec<CommandAuditEntry>,
        ts: u64,
        frame_delay_ms: u64,
    ) -> Result<(Vec<u8>, u64), String> {
        let cmd = "RALL?";
        self.port
            .write_all(b"RALL?\r")
            .map_err(|e| format!("OE write RALL?: {}", e))?;

        let start = std::time::Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(frame_delay_ms));

        let mut buf = vec![0u8; RALL_FRAME_BYTES];
        let mut total = 0usize;
        loop {
            match self.port.read(&mut buf[total..]) {
                Ok(0) => break,
                Ok(n) => {
                    total += n;
                    if total >= RALL_FRAME_BYTES {
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(e) => return Err(format!("OE read error: {}", e)),
            }
        }
        buf.truncate(total);

        let elapsed_ms = start.elapsed().as_millis() as u64;

        audit.push(CommandAuditEntry {
            seq: audit.len() as u64,
            timestamp_unix_ms: ts,
            device_id: "oe1022d".into(),
            command: cmd.to_string(),
            command_class: "oe_acquisition".into(),
            allowed: true,
            sent_to_transport: true,
            rejection_reason: None,
            response_preview: Some(format!("{} bytes", total)),
            transport_error: None,
            safety_relevant: false,
        });

        Ok((buf, elapsed_ms))
    }
}

// ---------------------------------------------------------------------------
// Fake OE transport for testing
// ---------------------------------------------------------------------------

pub struct FakeOeTransport {
    pub idn: String,
    pub frame_counter: u64,
}

impl FakeOeTransport {
    pub fn new() -> Self {
        FakeOeTransport {
            idn: "OE1022D,SN123456,1.0.0".into(),
            frame_counter: 0,
        }
    }

    pub fn query_identity(
        &mut self,
        audit: &mut Vec<CommandAuditEntry>,
        ts: u64,
    ) -> Result<String, String> {
        audit.push(CommandAuditEntry {
            seq: audit.len() as u64,
            timestamp_unix_ms: ts,
            device_id: "oe1022d".into(),
            command: "*IDN?".into(),
            command_class: "query".into(),
            allowed: true,
            sent_to_transport: true,
            rejection_reason: None,
            response_preview: Some(self.idn.clone()),
            transport_error: None,
            safety_relevant: false,
        });
        Ok(self.idn.clone())
    }

    pub fn capture_frame(
        &mut self,
        audit: &mut Vec<CommandAuditEntry>,
        ts: u64,
        _frame_delay_ms: u64,
    ) -> Result<(Vec<u8>, u64), String> {
        self.frame_counter += 1;
        // Generate a deterministic fake frame
        let mut frame = vec![0u8; RALL_FRAME_BYTES];
        // Fill with deterministic pattern based on counter
        for i in 0..RALL_FRAME_BYTES {
            frame[i] = ((self.frame_counter as u8).wrapping_add(i as u8)) & 0xFF;
        }

        audit.push(CommandAuditEntry {
            seq: audit.len() as u64,
            timestamp_unix_ms: ts,
            device_id: "oe1022d".into(),
            command: "RALL?".into(),
            command_class: "oe_acquisition".into(),
            allowed: true,
            sent_to_transport: true,
            rejection_reason: None,
            response_preview: Some(format!("{} bytes", RALL_FRAME_BYTES)),
            transport_error: None,
            safety_relevant: false,
        });

        Ok((frame, 800))
    }
}
