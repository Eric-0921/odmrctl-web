//! odmr-laser — Layer 1 CNI laser driver.
//!
//! Protocol source:
//! - `docs/equipment_manual/CNI Laser psu-sr/RS232语言协议_恒功率.md`
//! - `docs/equipment_manual/CNI Laser psu-sr/激光器使用说明书.md`
//!
//! This crate owns the binary serial protocol. GUI/Tauri callers must use the
//! typed API here rather than constructing frames directly.

use odmr_types::DeviceId;
use std::io::{Read, Write};
use std::time::Duration;

pub const HEADER: [u8; 2] = [0x55, 0xAA];
pub const CMD_SET_POWER: u8 = 0x05;
pub const CMD_OUTPUT: u8 = 0x03;
pub const SUBCMD_LASER_OFF: u8 = 0x00;
pub const SUBCMD_LASER_ON: u8 = 0x01;
pub const SUBCMD_SET_POWER: u8 = 0x01;
pub const MANUAL_MAX_POWER_MW: u16 = 150;

#[derive(Debug)]
pub enum LaserError {
    Serial(serialport::Error),
    Io(std::io::Error),
    Protocol(String),
}

impl std::fmt::Display for LaserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serial(e) => write!(f, "serial error: {e}"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Protocol(msg) => write!(f, "protocol error: {msg}"),
        }
    }
}

impl std::error::Error for LaserError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serial(e) => Some(e),
            Self::Io(e) => Some(e),
            Self::Protocol(_) => None,
        }
    }
}

impl From<serialport::Error> for LaserError {
    fn from(value: serialport::Error) -> Self {
        Self::Serial(value)
    }
}

impl From<std::io::Error> for LaserError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaserSerialConfig {
    pub baud_rate: u32,
    pub timeout_ms: u64,
    pub max_power_mw: u16,
}

impl Default for LaserSerialConfig {
    fn default() -> Self {
        Self {
            baud_rate: 9600,
            timeout_ms: 2000,
            max_power_mw: MANUAL_MAX_POWER_MW,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaserFrame {
    pub command: u8,
    pub data: Vec<u8>,
    pub checksum: u8,
}

impl LaserFrame {
    pub fn set_power(power_mw: u16) -> Self {
        let power = power_mw.min(MANUAL_MAX_POWER_MW);
        let data = vec![
            SUBCMD_SET_POWER,
            ((power >> 8) & 0xFF) as u8,
            (power & 0xFF) as u8,
        ];
        Self {
            command: CMD_SET_POWER,
            checksum: compute_checksum(CMD_SET_POWER, &data),
            data,
        }
    }

    pub fn laser_off() -> Self {
        let data = vec![SUBCMD_LASER_OFF];
        Self {
            command: CMD_OUTPUT,
            checksum: compute_checksum(CMD_OUTPUT, &data),
            data,
        }
    }

    pub fn laser_on() -> Self {
        let data = vec![SUBCMD_LASER_ON];
        Self {
            command: CMD_OUTPUT,
            checksum: compute_checksum(CMD_OUTPUT, &data),
            data,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.data.len() + 4);
        bytes.extend_from_slice(&HEADER);
        bytes.push(self.command);
        bytes.extend_from_slice(&self.data);
        bytes.push(self.checksum);
        bytes
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, LaserError> {
        if bytes.len() < 5 {
            return Err(LaserError::Protocol(format!(
                "frame too short: {} bytes",
                bytes.len()
            )));
        }
        if bytes[0..2] != HEADER {
            return Err(LaserError::Protocol("invalid frame header".into()));
        }
        let command = bytes[2];
        let data = bytes[3..bytes.len() - 1].to_vec();
        let checksum = bytes[bytes.len() - 1];
        let expected = compute_checksum(command, &data);
        if checksum != expected {
            return Err(LaserError::Protocol(format!(
                "checksum mismatch: expected {expected:02X}, got {checksum:02X}"
            )));
        }
        Ok(Self {
            command,
            data,
            checksum,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaserEchoCheck {
    pub matched_echo: bool,
    pub echoed_bytes: Vec<u8>,
}

pub struct LaserClient {
    device_id: DeviceId,
    port_path: String,
    config: LaserSerialConfig,
    port: Box<dyn serialport::SerialPort>,
}

impl LaserClient {
    pub fn open(
        device_id: DeviceId,
        port_path: impl Into<String>,
        config: LaserSerialConfig,
    ) -> Result<Self, LaserError> {
        let port_path = port_path.into();
        let port = serialport::new(&port_path, config.baud_rate)
            .timeout(Duration::from_millis(config.timeout_ms))
            .open()?;
        Ok(Self {
            device_id,
            port_path,
            config,
            port,
        })
    }

    pub fn device_id(&self) -> &DeviceId {
        &self.device_id
    }

    pub fn port_path(&self) -> &str {
        &self.port_path
    }

    pub fn config(&self) -> &LaserSerialConfig {
        &self.config
    }

    pub fn set_power(&mut self, power_mw: u16) -> Result<u16, LaserError> {
        let applied = power_mw.min(self.config.max_power_mw);
        self.send_frame(&LaserFrame::set_power(applied), false)?;
        Ok(applied)
    }

    pub fn set_enabled(&mut self, enabled: bool) -> Result<(), LaserError> {
        let frame = if enabled {
            LaserFrame::laser_on()
        } else {
            LaserFrame::laser_off()
        };
        self.send_frame(&frame, false)?;
        Ok(())
    }

    pub fn emergency_off(&mut self) -> Result<(), LaserError> {
        self.send_frame(&LaserFrame::laser_off(), false)?;
        self.send_frame(&LaserFrame::set_power(0), false)?;
        Ok(())
    }

    pub fn identity_or_echo_check(&mut self) -> Result<LaserEchoCheck, LaserError> {
        let frame = LaserFrame::laser_off();
        let sent = frame.to_bytes();
        let echoed = self.send_frame(&frame, true)?.unwrap_or_default();
        Ok(LaserEchoCheck {
            matched_echo: echoed == sent,
            echoed_bytes: echoed,
        })
    }

    pub fn send_frame(
        &mut self,
        frame: &LaserFrame,
        read_echo: bool,
    ) -> Result<Option<Vec<u8>>, LaserError> {
        let bytes = frame.to_bytes();
        let _ = self.port.clear(serialport::ClearBuffer::Input);
        self.port.write_all(&bytes)?;
        self.port.flush()?;
        if !read_echo {
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(150));
        let mut buf = vec![0u8; bytes.len()];
        let n = self.port.read(&mut buf)?;
        buf.truncate(n);
        Ok(Some(buf))
    }
}

fn compute_checksum(command: u8, data: &[u8]) -> u8 {
    let sum = command as u16 + data.iter().map(|value| *value as u16).sum::<u16>();
    (sum & 0xFF) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_frame_matches_manual_example() {
        let bytes = LaserFrame::set_power(100).to_bytes();
        assert_eq!(bytes, vec![0x55, 0xAA, 0x05, 0x01, 0x00, 0x64, 0x6A]);
    }

    #[test]
    fn off_frame_matches_manual_example() {
        assert_eq!(
            LaserFrame::laser_off().to_bytes(),
            vec![0x55, 0xAA, 0x03, 0x00, 0x03]
        );
    }

    #[test]
    fn on_frame_matches_manual_example() {
        assert_eq!(
            LaserFrame::laser_on().to_bytes(),
            vec![0x55, 0xAA, 0x03, 0x01, 0x04]
        );
    }

    #[test]
    fn parser_rejects_bad_checksum() {
        let mut bytes = LaserFrame::set_power(100).to_bytes();
        bytes[6] ^= 0xFF;
        assert!(LaserFrame::parse(&bytes).is_err());
    }
}
