//! Auto-discovery for OE1022D devices: enumerate serial ports, probe
//! each with `*IDN?`, and return any device that identifies as
//! `SSI LIA-OE1022D`.
//!
//! ## Rationale
//!
//! The Mac lab environment hands out different `/dev/cu.usbmodem*`
//! paths on every reboot, depending on which USB port the device is
//! plugged into. We cannot hard-code a port path; instead, the
//! discovery flow is:
//!
//! 1. List every available serial port via
//!    `serialport::available_ports()`.
//! 2. For each port, open it at 921600/8N1/no-flow-control.
//! 3. Clear the input buffer (K1 fix), write `*IDN?\r`, read until
//!    CR/LF or 300 ms timeout.
//! 4. Parse the response. If the manufacturer is "SSI" and the model
//!    contains "OE1022D", record this port as a candidate.
//! 5. Close the port and move on.
//!
//! The whole flow is wrapped in a 5-second deadline so a flaky port
//! cannot stall acquisition startup.
//!
//! ## Identity vs port
//!
//! The "device id" is the **serial number** from the IDN? response
//! (e.g. "SSI:LIA-OE1022D:D6522078"). The port path is volatile; the
//! SN is the stable identifier. See [`crate::idn::IdnResponse::device_id`].

use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::constants::IDN_READ_TIMEOUT;
use crate::idn::{probe_idn, IdnResponse};
use crate::port::{enumerate_ports, PortInfo};

/// Result of a successful discovery: a port that identified as an
/// OE1022D, plus its identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredOe1022d {
    pub port: PortInfo,
    pub idn: IdnResponse,
}

#[derive(Debug, Error)]
pub enum DiscoverError {
    #[error("port enumeration failed: {0}")]
    EnumerateFailed(#[from] serialport::Error),
    #[error("discovery deadline ({timeout_ms} ms) exceeded after scanning {scanned} ports; found {found} candidate(s)")]
    DeadlineExceeded {
        timeout_ms: u64,
        scanned: usize,
        found: usize,
    },
    #[error("no OE1022D found in {scanned} candidate port(s); check cable and baud rate (must be 921600)")]
    NoDeviceFound { scanned: usize },
}

/// Scan every available serial port and return the first one that
/// identifies as an OE1022D. If multiple are present, returns all of
/// them; callers can pick the one they want.
///
/// `per_port_timeout` is the IDN? probe timeout; the overall
/// `deadline` bounds the total wall-clock time spent in the scan.
///
/// C5: this is the bring-up flow for the real-device smoke test. In
/// the v0.1 product path it will be called by the Tauri command
/// `discover_devices`.
pub fn discover_oe1022d(
    per_port_timeout: Duration,
    deadline: Duration,
) -> Result<Vec<DiscoveredOe1022d>, DiscoverError> {
    let started = Instant::now();
    let candidates = enumerate_ports()?;
    let mut found = Vec::new();
    let mut scanned = 0usize;

    for port in candidates {
        if started.elapsed() > deadline {
            return Err(DiscoverError::DeadlineExceeded {
                timeout_ms: deadline.as_millis() as u64,
                scanned,
                found: found.len(),
            });
        }
        scanned += 1;
        match probe_idn_with_timeout(&port.name, per_port_timeout) {
            Ok(idn) => {
                let is_match = is_oe1022d(&idn);
                eprintln!(
                    "[discover]   -> {} | model='{}' | SN={} | match={}",
                    idn.manufacturer,
                    idn.model,
                    idn.serial_number,
                    is_match
                );
                if is_match {
                    found.push(DiscoveredOe1022d { port, idn });
                }
            }
            Err(e) => {
                eprintln!("[discover]   -> probe failed: {e}");
            }
        }
    }

    if found.is_empty() {
        return Err(DiscoverError::NoDeviceFound { scanned });
    }
    Ok(found)
}

/// True if the response is from an OE1022D lock-in amplifier.
fn is_oe1022d(idn: &IdnResponse) -> bool {
    let mfg_match = idn.manufacturer.eq_ignore_ascii_case("SSI");
    let model_match = idn.model.to_ascii_uppercase().contains("OE1022D");
    eprintln!(
        "[is_oe1022d] mfg={:?} (match={}) | model={:?} (match={})",
        idn.manufacturer, mfg_match, idn.model, model_match
    );
    mfg_match && model_match
}

/// Probe a single port with a custom per-read timeout. Wraps
/// [`crate::idn::probe_idn`] but allows the caller to pass a tighter
/// deadline than the default 300 ms.
fn probe_idn_with_timeout(
    port_path: &str,
    timeout: Duration,
) -> Result<IdnResponse, crate::idn::IdnProbeError> {
    // Re-implement the probe with a configurable timeout by opening
    // the port ourselves. The library probe uses a fixed 300 ms
    // timeout, which is too long when we are scanning 20+ ports.
    use std::io::Read;
    let mut port = serialport::new(port_path, crate::constants::OE1022D_BAUD_RATE)
        .data_bits(crate::constants::OE1022D_DATA_BITS)
        .parity(crate::constants::OE1022D_PARITY)
        .stop_bits(crate::constants::OE1022D_STOP_BITS)
        .flow_control(crate::constants::OE1022D_FLOW_CONTROL)
        .timeout(timeout)
        .open()
        .map_err(|source| crate::idn::IdnProbeError::OpenFailed {
            port: port_path.to_string(),
            source,
        })?;
    if let Err(source) = port.clear(serialport::ClearBuffer::Input) {
        return Err(crate::idn::IdnProbeError::ClearInputFailed {
            port: port_path.to_string(),
            source,
        });
    }
    use std::io::Write;
    if let Err(source) = port.write_all(crate::constants::IDN_COMMAND) {
        return Err(crate::idn::IdnProbeError::WriteFailed {
            port: port_path.to_string(),
            source,
        });
    }
    if let Err(source) = port.flush() {
        return Err(crate::idn::IdnProbeError::WriteFailed {
            port: port_path.to_string(),
            source,
        });
    }
    let mut raw = Vec::with_capacity(128);
    let mut buf = [0u8; 64];
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > timeout {
            return Err(crate::idn::IdnProbeError::ReadTimeout {
                port: port_path.to_string(),
                elapsed_ms: started.elapsed().as_millis() as u64,
            });
        }
        match port.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                raw.extend_from_slice(&buf[..n]);
                if raw.iter().any(|&b| b == b'\n' || b == b'\r') {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                return Err(crate::idn::IdnProbeError::ReadTimeout {
                    port: port_path.to_string(),
                    elapsed_ms: started.elapsed().as_millis() as u64,
                });
            }
            Err(source) => {
                return Err(crate::idn::IdnProbeError::ReadError {
                    port: port_path.to_string(),
                    source,
                });
            }
        }
    }
    if raw.is_empty() {
        return Err(crate::idn::IdnProbeError::EmptyResponse {
            port: port_path.to_string(),
        });
    }
    crate::idn::parse_idn(&raw, port_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_oe1022d_accepts_known_good() {
        let idn = IdnResponse {
            manufacturer: "SSI".into(),
            model: "LIA-OE1022D".into(),
            serial_number: "D6522078".into(),
            firmware_version: "Ver6.3200831".into(),
            raw: vec![],
        };
        assert!(is_oe1022d(&idn));
    }

    #[test]
    fn is_oe1022d_rejects_other_brands() {
        let idn = IdnResponse {
            manufacturer: "Rohde&Schwarz".into(),
            model: "SMB100A".into(),
            serial_number: "1234".into(),
            firmware_version: "3.0".into(),
            raw: vec![],
        };
        assert!(!is_oe1022d(&idn));
    }

    #[test]
    fn is_oe1022d_is_case_insensitive() {
        let idn = IdnResponse {
            manufacturer: "ssi".into(),
            model: "lia-oe1022d".into(),
            serial_number: "X".into(),
            firmware_version: "1".into(),
            raw: vec![],
        };
        assert!(is_oe1022d(&idn));
    }

    /// Live discovery smoke test. Scans the host's serial ports and
    /// asserts the function does not panic; on machines with no
    /// ports it should return `NoDeviceFound` (not crash).
    #[test]
    fn discover_does_not_panic() {
        let result = discover_oe1022d(Duration::from_millis(200), Duration::from_secs(5));
        match result {
            Ok(devices) => {
                eprintln!("discover: found {} OE1022D device(s)", devices.len());
                for d in &devices {
                    eprintln!("  - {} ({})", d.port.name, d.idn.device_id());
                }
            }
            Err(DiscoverError::NoDeviceFound { scanned }) => {
                eprintln!("discover: scanned {scanned} port(s), none matched");
            }
            Err(DiscoverError::DeadlineExceeded { .. }) => {
                eprintln!("discover: deadline exceeded");
            }
            Err(e) => eprintln!("discover: {e}"),
        }
    }
}
