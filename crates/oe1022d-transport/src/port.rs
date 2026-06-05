//! Serial port enumeration and classification.
//!
//! Wraps `serialport::available_ports()` with a small classification
//! that lets callers filter USB CDC vs. legacy RS232 paths (K7).

use serde::{Deserialize, Serialize};

/// Classified view of an available serial port. The `kind` field is
/// our best guess based on path conventions — never authoritative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortInfo {
    /// OS-assigned port path, e.g. `/dev/cu.usbmodem3361358734371` on
    /// macOS or `COM7` on Windows.
    pub name: String,
    /// Our classification. K7: `/dev/cu.usbmodem*` paths are **USB CDC**
    /// even though they look like a TTY.
    pub kind: PortKind,
}

/// Coarse port kind. Always treat as advisory; the only authoritative
/// identity check is `*IDN?`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortKind {
    /// macOS USB CDC path: `/dev/cu.usbmodem*` or `/dev/tty.usbmodem*`.
    /// K7: do not assume these are RS232.
    UsbCdcMac,
    /// macOS legacy serial port (rare on modern hardware).
    LegacySerialMac,
    /// Windows COM port: `COM1`..`COM255`.
    ComWindows,
    /// Linux USB CDC: `/dev/ttyACM*`.
    UsbCdcLinux,
    /// Linux USB-to-serial: `/dev/ttyUSB*`.
    UsbSerialLinux,
    /// Anything else — still worth probing, classification is best-effort.
    Unknown,
}

impl PortKind {
    /// Classify a port name by string prefix conventions only.
    pub fn classify(name: &str) -> Self {
        // Order matters: `cu.usbmodem*` and `tty.usbmodem*` are USB CDC.
        if name.starts_with("/dev/cu.usbmodem") || name.starts_with("/dev/tty.usbmodem") {
            Self::UsbCdcMac
        } else if name.starts_with("/dev/cu.") || name.starts_with("/dev/tty.") {
            Self::LegacySerialMac
        } else if name.starts_with("/dev/ttyACM") {
            Self::UsbCdcLinux
        } else if name.starts_with("/dev/ttyUSB") {
            Self::UsbSerialLinux
        } else if name.len() >= 4
            && name[..3].eq_ignore_ascii_case("COM")
            && name[3..].chars().all(|c| c.is_ascii_digit())
        {
            Self::ComWindows
        } else {
            Self::Unknown
        }
    }
}

/// Enumerate all serial ports currently visible to the OS.
///
/// Returns an empty Vec if no ports are found (NOT an error).
/// Returns an error only if the OS enumeration itself fails.
pub fn enumerate_ports() -> Result<Vec<PortInfo>, serialport::Error> {
    let raw = serialport::available_ports()?;
    Ok(raw
        .into_iter()
        .map(|p| {
            let name = p.port_name.clone();
            let kind = PortKind::classify(&name);
            PortInfo { name, kind }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_macos_usb_cdc() {
        // K7: /dev/cu.usbmodem* is USB CDC, NOT RS232.
        assert_eq!(
            PortKind::classify("/dev/cu.usbmodem3361358734371"),
            PortKind::UsbCdcMac
        );
        assert_eq!(
            PortKind::classify("/dev/tty.usbmodem3361358734371"),
            PortKind::UsbCdcMac
        );
    }

    #[test]
    fn classify_macos_legacy() {
        assert_eq!(
            PortKind::classify("/dev/cu.Bluetooth-Incoming-Port"),
            PortKind::LegacySerialMac
        );
    }

    #[test]
    fn classify_linux() {
        assert_eq!(PortKind::classify("/dev/ttyACM0"), PortKind::UsbCdcLinux);
        assert_eq!(PortKind::classify("/dev/ttyUSB0"), PortKind::UsbSerialLinux);
    }

    #[test]
    fn classify_windows() {
        assert_eq!(PortKind::classify("COM1"), PortKind::ComWindows);
        assert_eq!(PortKind::classify("COM7"), PortKind::ComWindows);
        assert_eq!(PortKind::classify("COM255"), PortKind::ComWindows);
        // Lower-case tolerated:
        assert_eq!(PortKind::classify("com3"), PortKind::ComWindows);
    }

    #[test]
    fn classify_unknown() {
        assert_eq!(PortKind::classify("/dev/foo"), PortKind::Unknown);
        assert_eq!(PortKind::classify(""), PortKind::Unknown);
        // COM-like but not numeric:
        assert_eq!(PortKind::classify("COMx"), PortKind::Unknown);
    }

    #[test]
    fn enumerate_does_not_panic() {
        // On any platform, this should not panic. May return an empty
        // list (CI without serial ports) or fail with an OS-specific
        // error — both are acceptable.
        let result = enumerate_ports();
        match result {
            Ok(ports) => {
                for p in &ports {
                    // Each enumerated port must have a non-empty name.
                    assert!(!p.name.is_empty());
                }
            }
            Err(e) => {
                eprintln!("enumerate_ports() returned error: {e}");
            }
        }
    }
}
