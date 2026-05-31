//! OE1022D serial transport bridge for M3.4 real mode.

use crate::types::M3_4CommandAuditEntry;
use odmr_oe1022d::RALL_FRAME_BYTES;

pub struct OeSerialTransport {
    port: Box<dyn serialport::SerialPort>,
}

impl OeSerialTransport {
    pub fn connect(port: &str, baud: u32, timeout_ms: u64) -> Result<Self, String> {
        let sp = serialport::new(port, baud)
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .open()
            .map_err(|e| format!("OE connect {}: {}", port, e))?;
        Ok(OeSerialTransport { port: sp })
    }

    pub fn query_identity(
        &mut self,
        audit: &mut Vec<M3_4CommandAuditEntry>,
        ts: u64,
    ) -> Result<String, String> {
        let cmd = "*IDN?";
        validate_oe_command(cmd)?;

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

        audit.push(M3_4CommandAuditEntry {
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
        audit: &mut Vec<M3_4CommandAuditEntry>,
        ts: u64,
        frame_delay_ms: u64,
    ) -> Result<(Vec<u8>, u64), String> {
        let cmd = "RALL?";
        validate_oe_command(cmd)?;

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

        audit.push(M3_4CommandAuditEntry {
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

const OE_ALLOWLIST: &[&str] = &["*IDN?", "RALL?"];
const OE_FORBIDDEN: &[&str] = &[
    "SSETD", "RSETD", "APHSD", "FMODD", "PHASD", "ISRCD", "SENSD", "OFLTD", "OFSLD", "HARMD",
    "SLVLD", "SWVTD",
];

fn validate_oe_command(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if trimmed.contains(';') {
        return Err(format!("OE command '{}' contains semicolon", trimmed));
    }
    let upper = trimmed.to_ascii_uppercase();
    for pat in OE_FORBIDDEN {
        if upper.contains(pat) {
            return Err(format!(
                "OE command '{}' matches forbidden pattern '{}'",
                trimmed, pat
            ));
        }
    }
    if !OE_ALLOWLIST.contains(&trimmed) {
        return Err(format!("OE command '{}' not in allowlist", trimmed));
    }
    Ok(())
}
