//! Fake CNI Laser device for protocol testing (M1).
//!
//! Simulates the laser PSU-SR behavior in memory without any real hardware.
//! This is used to validate the protocol implementation before connecting
//! to a live laser.

use crate::protocol::{CniFrame, MAX_POWER_MW};

/// Simulated laser state.
#[derive(Debug, Clone, PartialEq)]
pub struct FakeCniLaser {
    pub power_setpoint_mw: u16,
    pub output_enabled: bool,
    pub max_power_mw: u16,
}

impl Default for FakeCniLaser {
    fn default() -> Self {
        Self {
            power_setpoint_mw: 0,
            output_enabled: false,
            max_power_mw: MAX_POWER_MW,
        }
    }
}

impl FakeCniLaser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process an incoming frame and update internal state.
    ///
    /// Returns `Ok(())` if the frame was valid and accepted.
    /// Returns `Err` if the frame was malformed or violated a limit.
    pub fn handle_frame(&mut self, frame: &CniFrame) -> Result<(), String> {
        match frame.command {
            0x05 => {
                // Set power
                if let Some(power) = frame.power_mw() {
                    self.power_setpoint_mw = power.min(self.max_power_mw);
                    Ok(())
                } else {
                    Err("Invalid set-power frame: missing power data".into())
                }
            }
            0x03 => {
                // Output control
                match frame.data.first() {
                    Some(&0x00) => {
                        self.output_enabled = false;
                        Ok(())
                    }
                    Some(&0x01) => {
                        // Laser on: only allow if power setpoint > 0
                        if self.power_setpoint_mw == 0 {
                            return Err(
                                "Refused to enable laser: power setpoint is 0 mW".into()
                            );
                        }
                        self.output_enabled = true;
                        Ok(())
                    }
                    _ => Err("Invalid output sub-command".into()),
                }
            }
            _ => Err(format!("Unknown command: 0x{:02X}", frame.command)),
        }
    }

    /// Convenience: handle raw bytes directly.
    pub fn handle_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        let frame = CniFrame::parse(bytes)?;
        self.handle_frame(&frame)
    }

    /// Returns true if the laser is in a safe state (output disabled).
    pub fn is_safe(&self) -> bool {
        !self.output_enabled
    }

    /// Returns the actual output power (0 if disabled).
    pub fn actual_power_mw(&self) -> u16 {
        if self.output_enabled {
            self.power_setpoint_mw
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let laser = FakeCniLaser::new();
        assert!(!laser.output_enabled);
        assert_eq!(laser.power_setpoint_mw, 0);
        assert!(laser.is_safe());
        assert_eq!(laser.actual_power_mw(), 0);
    }

    #[test]
    fn test_set_power() {
        let mut laser = FakeCniLaser::new();
        let frame = CniFrame::set_power(100);
        laser.handle_frame(&frame).unwrap();
        assert_eq!(laser.power_setpoint_mw, 100);
        assert!(!laser.output_enabled); // setting power does not enable output
    }

    #[test]
    fn test_power_clamped_by_fake() {
        let mut laser = FakeCniLaser::new();
        let frame = CniFrame::set_power(200);
        laser.handle_frame(&frame).unwrap();
        assert_eq!(laser.power_setpoint_mw, 150); // clamped to max
    }

    #[test]
    fn test_laser_on_off() {
        let mut laser = FakeCniLaser::new();

        // Cannot turn on with 0 power
        let frame = CniFrame::laser_on();
        assert!(laser.handle_frame(&frame).is_err());

        // Set power first
        laser.handle_frame(&CniFrame::set_power(50)).unwrap();

        // Now can turn on
        laser.handle_frame(&CniFrame::laser_on()).unwrap();
        assert!(laser.output_enabled);
        assert_eq!(laser.actual_power_mw(), 50);
        assert!(!laser.is_safe());

        // Turn off
        laser.handle_frame(&CniFrame::laser_off()).unwrap();
        assert!(!laser.output_enabled);
        assert!(laser.is_safe());
        assert_eq!(laser.actual_power_mw(), 0);
    }

    #[test]
    fn test_handle_bytes_roundtrip() {
        let mut laser = FakeCniLaser::new();
        let bytes = CniFrame::set_power(75).to_bytes();
        laser.handle_bytes(&bytes).unwrap();
        assert_eq!(laser.power_setpoint_mw, 75);
    }

    #[test]
    fn test_bad_checksum_rejected() {
        let mut laser = FakeCniLaser::new();
        let mut bytes = CniFrame::set_power(50).to_bytes();
        bytes[6] ^= 0xFF; // corrupt
        assert!(laser.handle_bytes(&bytes).is_err());
    }

    #[test]
    fn test_power_limit_enforcement() {
        let mut laser = FakeCniLaser::new();
        // Try to set 500 mW (way above 150 limit)
        laser.handle_frame(&CniFrame::set_power(500)).unwrap();
        assert_eq!(laser.power_setpoint_mw, 150);
    }
}
