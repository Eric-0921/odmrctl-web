//! `*IDN?` probe and response parsing.
//!
//! Handles K1 (clear input buffer before reading), K5 (trailing NUL is
//! expected, not an error), and validates the response shape against
//! the OE1022D identity pattern.
//!
//! Expected identity response shape (from `oe1022d_rust_demo` and the
//! OE1022D manual):
//!
//! ```text
//! SSI,LIA-OE1022D,D6522078,Ver6.3200831
//! ```
//!
//! 4 comma-separated fields. The last field (firmware version) may
//! contain a trailing NUL due to the device's fixed-length identity
//! buffer.

use std::io::Read;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::constants::{
    IDN_COMMAND, IDN_READ_TIMEOUT, OE1022D_BAUD_RATE, OE1022D_DATA_BITS,
    OE1022D_FLOW_CONTROL, OE1022D_PARITY, OE1022D_STOP_BITS,
};

/// Parsed identity response from a successful `*IDN?` probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdnResponse {
    /// e.g. `"SSI"`
    pub manufacturer: String,
    /// e.g. `"LIA-OE1022D"`
    pub model: String,
    /// e.g. `"D6522078"`. **This is the stable device identity**;
    /// USB port paths change between reboots but the SN does not.
    pub serial_number: String,
    /// e.g. `"Ver6.3200831"`
    pub firmware_version: String,
    /// Raw response bytes (with trailing NULs preserved). Useful for
    /// fingerprint persistence and for debugging transport edge cases.
    pub raw: Vec<u8>,
}

impl IdnResponse {
    /// The stable identity fingerprint of the device. Two probes of the
    /// same physical OE1022D (regardless of USB port) yield the same
    /// `device_id()`.
    pub fn device_id(&self) -> String {
        format!("{}:{}:{}", self.manufacturer, self.model, self.serial_number)
    }
}

#[derive(Debug, Error)]
pub enum IdnProbeError {
    #[error("failed to open serial port {port}: {source}")]
    OpenFailed {
        port: String,
        #[source]
        source: serialport::Error,
    },
    #[error("failed to clear input buffer on {port}: {source}")]
    ClearInputFailed {
        port: String,
        #[source]
        source: serialport::Error,
    },
    #[error("failed to write *IDN? to {port}: {source}")]
    WriteFailed {
        port: String,
        #[source]
        source: std::io::Error,
    },
    #[error("read timeout on {port} after {elapsed_ms} ms")]
    ReadTimeout { port: String, elapsed_ms: u64 },
    #[error("read error on {port}: {source}")]
    ReadError {
        port: String,
        #[source]
        source: std::io::Error,
    },
    #[error("IDN? response from {port} was empty")]
    EmptyResponse { port: String },
    #[error("IDN? response from {port} was not valid UTF-8: {source}")]
    InvalidUtf8 {
        port: String,
        #[source]
        source: std::string::FromUtf8Error,
    },
    #[error("IDN? response from {port} had wrong field count: expected 4 comma-separated fields, got {actual} in {raw:?}")]
    WrongFieldCount {
        port: String,
        actual: usize,
        raw: Vec<u8>,
    },
}

/// Open a port and probe its identity via `*IDN?`.
///
/// **K1 fix**: before reading the response, we call
/// `port.clear(ClearBuffer::Input)` to discard any residue from a
/// previous command.
///
/// **K5 fix**: we read until LF or CR, then strip trailing NULs and
/// whitespace from the raw response before parsing.
pub fn probe_idn(port_path: &str) -> Result<IdnResponse, IdnProbeError> {
    // Open with the canonical OE1022D settings.
    // Note: serialport 4.x only exposes a single `timeout` builder
    // method; it controls both read and write timeouts. We use
    // IDN_READ_TIMEOUT (the longer of the two) so the probe tolerates
    // slow devices. Per-direction timeouts can be enforced later by
    // the `RALL?` reader which uses `port.set_timeout` directly.
    let mut port = serialport::new(port_path, OE1022D_BAUD_RATE)
        .data_bits(OE1022D_DATA_BITS)
        .parity(OE1022D_PARITY)
        .stop_bits(OE1022D_STOP_BITS)
        .flow_control(OE1022D_FLOW_CONTROL)
        .timeout(IDN_READ_TIMEOUT)
        .open()
        .map_err(|source| IdnProbeError::OpenFailed {
            port: port_path.to_string(),
            source,
        })?;

    // K1: Clear the input buffer to discard any prior residue.
    port.clear(serialport::ClearBuffer::Input)
        .map_err(|source| IdnProbeError::ClearInputFailed {
            port: port_path.to_string(),
            source,
        })?;

    // Write *IDN?
    port.write_all(IDN_COMMAND).map_err(|source| IdnProbeError::WriteFailed {
        port: port_path.to_string(),
        source,
    })?;
    port.flush().map_err(|source| IdnProbeError::WriteFailed {
        port: port_path.to_string(),
        source,
    })?;

    // Read until we see a CR or LF, or hit the timeout.
    let mut raw = Vec::with_capacity(128);
    let mut buf = [0u8; 64];
    let started = std::time::Instant::now();
    loop {
        match port.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                raw.extend_from_slice(&buf[..n]);
                if raw.iter().any(|&b| b == b'\n' || b == b'\r') {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                return Err(IdnProbeError::ReadTimeout {
                    port: port_path.to_string(),
                    elapsed_ms: started.elapsed().as_millis() as u64,
                });
            }
            Err(source) => {
                return Err(IdnProbeError::ReadError {
                    port: port_path.to_string(),
                    source,
                });
            }
        }
    }

    if raw.is_empty() {
        return Err(IdnProbeError::EmptyResponse {
            port: port_path.to_string(),
        });
    }

    parse_idn(&raw, port_path)
}

/// Parse a raw `*IDN?` response into structured fields.
///
/// **K5 fix**: trailing NULs (from the device's fixed-length identity
/// buffer) and trailing CR/LF are stripped before splitting on commas.
pub fn parse_idn(raw: &[u8], port_path: &str) -> Result<IdnResponse, IdnProbeError> {
    // Strip trailing CR/LF and NULs, but keep the raw bytes for the
    // `raw` field so callers can persist the exact response.
    let mut trimmed = raw.to_vec();
    while let Some(&last) = trimmed.last() {
        if last == b'\n' || last == b'\r' || last == 0 {
            trimmed.pop();
        } else {
            break;
        }
    }
    if trimmed.is_empty() {
        return Err(IdnProbeError::EmptyResponse {
            port: port_path.to_string(),
        });
    }

    let s = String::from_utf8(trimmed).map_err(|source| IdnProbeError::InvalidUtf8 {
        port: port_path.to_string(),
        source,
    })?;
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return Err(IdnProbeError::WrongFieldCount {
            port: port_path.to_string(),
            actual: parts.len(),
            raw: raw.to_vec(),
        });
    }
    Ok(IdnResponse {
        manufacturer: parts[0].trim().to_string(),
        model: parts[1].trim().to_string(),
        serial_number: parts[2].trim().to_string(),
        firmware_version: parts[3].trim().to_string(),
        raw: raw.to_vec(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// K5 fixture: a real OE1022D identity response with trailing NUL.
    /// From `docs/lab-bringup/oe1022d_acquire_2026-05-31/acquisition_report.md`.
    const REAL_IDN: &[u8] =
        b"SSI,LIA-OE1022D,D6522078,Ver6.3200831\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

    #[test]
    fn k5_parse_real_oe1022d_response() {
        let idn = parse_idn(REAL_IDN, "/dev/cu.usbmodem3361358734371").unwrap();
        assert_eq!(idn.manufacturer, "SSI");
        assert_eq!(idn.model, "LIA-OE1022D");
        assert_eq!(idn.serial_number, "D6522078");
        assert_eq!(idn.firmware_version, "Ver6.3200831");
        // raw bytes preserved for fingerprint persistence:
        assert_eq!(idn.raw, REAL_IDN);
    }

    #[test]
    fn parse_with_crlf_terminator() {
        let raw = b"SSI,LIA-OE1022D,D6522078,Ver6.3200831\r\n";
        let idn = parse_idn(raw, "test").unwrap();
        assert_eq!(idn.serial_number, "D6522078");
    }

    #[test]
    fn parse_with_lf_only_terminator() {
        let raw = b"SSI,LIA-OE1022D,D6522078,Ver6.3200831\n";
        let idn = parse_idn(raw, "test").unwrap();
        assert_eq!(idn.serial_number, "D6522078");
    }

    #[test]
    fn parse_wrong_field_count() {
        let raw = b"SSI,LIA-OE1022D,only_three_fields\n";
        let err = parse_idn(raw, "test").unwrap_err();
        match err {
            IdnProbeError::WrongFieldCount { actual, .. } => assert_eq!(actual, 3),
            other => panic!("expected WrongFieldCount, got {other:?}"),
        }
    }

    #[test]
    fn parse_empty_after_trim() {
        // All NULs should fail as empty response.
        let raw = b"\0\0\0";
        let err = parse_idn(raw, "test").unwrap_err();
        assert!(matches!(err, IdnProbeError::EmptyResponse { .. }));
    }

    #[test]
    fn device_id_is_stable_across_ports() {
        // Two responses from the same physical device, opened on
        // different ports, must produce identical device_id().
        let idn_a = parse_idn(REAL_IDN, "/dev/cu.usbmodemPORT_A").unwrap();
        let idn_b = parse_idn(REAL_IDN, "/dev/cu.usbmodemPORT_B").unwrap();
        assert_eq!(idn_a.device_id(), idn_b.device_id());
        assert_eq!(idn_a.device_id(), "SSI:LIA-OE1022D:D6522078");
    }
}
