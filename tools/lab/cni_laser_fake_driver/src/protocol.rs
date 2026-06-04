//! CNI Laser binary frame protocol.
//!
//! Reference: `docs/equipment_manual/CNI Laser psu-sr/RS232语言协议_恒功率.md`
//!
//! Frame structure: `[0x55] [0xAA] [Command] [Data...] [Checksum]`
//! Checksum = sum(Command + all Data bytes) & 0xFF

/// Fixed frame header bytes.
pub const HEADER: [u8; 2] = [0x55, 0xAA];

/// Command codes.
pub const CMD_SET_POWER: u8 = 0x05;
pub const CMD_OUTPUT: u8 = 0x03;

/// Sub-command for output control.
pub const SUBCMD_LASER_OFF: u8 = 0x00;
pub const SUBCMD_LASER_ON: u8 = 0x01;

/// Sub-command for set power.
pub const SUBCMD_SET_POWER: u8 = 0x01;

/// Maximum software power limit (mW).
/// The device label says 0–150 mW; the software input range is 0–1500,
/// but values above 150 are clamped to 150.
pub const MAX_POWER_MW: u16 = 150;

/// A complete CNI laser command frame.
#[derive(Debug, Clone, PartialEq)]
pub struct CniFrame {
    pub command: u8,
    pub data: Vec<u8>,
    pub checksum: u8,
}

impl CniFrame {
    /// Build a "set power" frame.
    ///
    /// Power is clamped to [0, MAX_POWER_MW].
    ///
    /// Example: 100 mW → `55 AA 05 01 00 64 6A`
    pub fn set_power(power_mw: u16) -> Self {
        let power = power_mw.min(MAX_POWER_MW);
        let high = (power >> 8) as u8;
        let low = (power & 0xFF) as u8;
        let data = vec![SUBCMD_SET_POWER, high, low];
        let checksum = compute_checksum(CMD_SET_POWER, &data);
        Self {
            command: CMD_SET_POWER,
            data,
            checksum,
        }
    }

    /// Build a "laser off" frame: `55 AA 03 00 03`
    pub fn laser_off() -> Self {
        let data = vec![SUBCMD_LASER_OFF];
        let checksum = compute_checksum(CMD_OUTPUT, &data);
        Self {
            command: CMD_OUTPUT,
            data,
            checksum,
        }
    }

    /// Build a "laser on" frame: `55 AA 03 01 04`
    pub fn laser_on() -> Self {
        let data = vec![SUBCMD_LASER_ON];
        let checksum = compute_checksum(CMD_OUTPUT, &data);
        Self {
            command: CMD_OUTPUT,
            data,
            checksum,
        }
    }

    /// Serialize frame to bytes including header.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(2 + 1 + self.data.len() + 1);
        buf.extend_from_slice(&HEADER);
        buf.push(self.command);
        buf.extend_from_slice(&self.data);
        buf.push(self.checksum);
        buf
    }

    /// Parse a raw byte slice into a frame.
    ///
    /// Returns `Err` if header is wrong, slice is too short, or checksum fails.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 5 {
            return Err(format!("frame too short: {} bytes", bytes.len()));
        }
        if bytes[0] != HEADER[0] || bytes[1] != HEADER[1] {
            return Err(format!(
                "bad header: expected {:02X} {:02X}, got {:02X} {:02X}",
                HEADER[0], HEADER[1], bytes[0], bytes[1]
            ));
        }
        let command = bytes[2];
        let data_len = bytes.len() - 4; // minus header(2) + command(1) + checksum(1)
        let data = bytes[3..3 + data_len].to_vec();
        let checksum = bytes[bytes.len() - 1];

        let expected = compute_checksum(command, &data);
        if checksum != expected {
            return Err(format!(
                "checksum mismatch: expected {:02X}, got {:02X}",
                expected, checksum
            ));
        }

        Ok(Self {
            command,
            data,
            checksum,
        })
    }

    /// Verify checksum without full parsing.
    pub fn verify_checksum(bytes: &[u8]) -> bool {
        if bytes.len() < 5 {
            return false;
        }
        if bytes[0] != HEADER[0] || bytes[1] != HEADER[1] {
            return false;
        }
        let command = bytes[2];
        let data = &bytes[3..bytes.len() - 1];
        let checksum = bytes[bytes.len() - 1];
        compute_checksum(command, data) == checksum
    }

    /// Extract power setpoint from a set-power frame.
    pub fn power_mw(&self) -> Option<u16> {
        if self.command != CMD_SET_POWER || self.data.len() < 3 {
            return None;
        }
        Some(((self.data[1] as u16) << 8) | (self.data[2] as u16))
    }

    /// Returns true if this is a laser-off frame.
    pub fn is_laser_off(&self) -> bool {
        self.command == CMD_OUTPUT && self.data.first() == Some(&SUBCMD_LASER_OFF)
    }

    /// Returns true if this is a laser-on frame.
    pub fn is_laser_on(&self) -> bool {
        self.command == CMD_OUTPUT && self.data.first() == Some(&SUBCMD_LASER_ON)
    }
}

/// Compute checksum: sum of command + all data bytes, low 8 bits.
fn compute_checksum(command: u8, data: &[u8]) -> u8 {
    let sum = command as u16 + data.iter().map(|b| *b as u16).sum::<u16>();
    (sum & 0xFF) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_power_100mw() {
        // Document example: 100 mW → 55 AA 05 01 00 64 6A
        let frame = CniFrame::set_power(100);
        let bytes = frame.to_bytes();
        assert_eq!(bytes, vec![0x55, 0xAA, 0x05, 0x01, 0x00, 0x64, 0x6A]);
    }

    #[test]
    fn test_laser_off() {
        // Document: 55 AA 03 00 03
        let frame = CniFrame::laser_off();
        let bytes = frame.to_bytes();
        assert_eq!(bytes, vec![0x55, 0xAA, 0x03, 0x00, 0x03]);
    }

    #[test]
    fn test_laser_on() {
        // Document: 55 AA 03 01 04
        let frame = CniFrame::laser_on();
        let bytes = frame.to_bytes();
        assert_eq!(bytes, vec![0x55, 0xAA, 0x03, 0x01, 0x04]);
    }

    #[test]
    fn test_parse_roundtrip() {
        let original = CniFrame::set_power(100);
        let bytes = original.to_bytes();
        let parsed = CniFrame::parse(&bytes).unwrap();
        assert_eq!(original, parsed);
        assert_eq!(parsed.power_mw(), Some(100));
    }

    #[test]
    fn test_checksum_fail() {
        let mut bytes = CniFrame::set_power(100).to_bytes();
        bytes[6] ^= 0xFF; // corrupt checksum
        assert!(CniFrame::parse(&bytes).is_err());
    }

    #[test]
    fn test_power_clamping() {
        let frame = CniFrame::set_power(200); // above MAX_POWER_MW (150)
        assert_eq!(frame.power_mw(), Some(150));
    }

    #[test]
    fn test_verify_checksum() {
        let bytes = CniFrame::laser_off().to_bytes();
        assert!(CniFrame::verify_checksum(&bytes));

        let mut bad = bytes.clone();
        bad[3] = 0xFF;
        assert!(!CniFrame::verify_checksum(&bad));
    }

    #[test]
    fn test_is_laser_off() {
        assert!(CniFrame::laser_off().is_laser_off());
        assert!(!CniFrame::laser_on().is_laser_off());
        assert!(!CniFrame::set_power(50).is_laser_off());
    }

    #[test]
    fn test_is_laser_on() {
        assert!(CniFrame::laser_on().is_laser_on());
        assert!(!CniFrame::laser_off().is_laser_on());
        assert!(!CniFrame::set_power(50).is_laser_on());
    }
}
