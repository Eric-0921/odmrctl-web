//! odmr-device — Shared device abstractions, traits, and types.
//!
//! This crate defines the interface boundary between the executor and concrete
//! device drivers.  No hardware access lives here; only traits and common
//! structures.

use odmr_types::{DeviceId, DeviceKind};
use std::fmt;

// ---------------------------------------------------------------------------
// Device trait
// ---------------------------------------------------------------------------

/// Common interface for every instrument in the ODMR system.
pub trait Device {
    /// Return the stable device identifier.
    fn id(&self) -> &DeviceId;

    /// Return the kind of instrument.
    fn kind(&self) -> DeviceKind;

    /// Return the current connection / health status.
    fn status(&self) -> DeviceStatus;
}

// ---------------------------------------------------------------------------
// FakeDevice trait
// ---------------------------------------------------------------------------

/// Trait for test-doubles that can accept textual commands and produce
/// deterministic responses without touching real hardware.
pub trait FakeDevice: Device {
    /// Accept a command string (SCPI, OE1022D ASCII, etc.) and update
    /// internal state.  Returns an acknowledgement or error.
    fn send_command(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError>;

    /// Accept a query command and return the current state as a string.
    fn query(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError>;

    /// Return the identity response (equivalent to `*IDN?`).
    fn idn(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceStatus {
    pub connected: bool,
    pub error_queue_len: usize,
    pub busy: bool,
}

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceResponse {
    /// Command accepted, no data returned.
    Ack,
    /// Query response payload.
    Value(String),
    /// Command rejected with a reason string.
    Error(String),
}

impl fmt::Display for DeviceResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceResponse::Ack => write!(f, "ACK"),
            DeviceResponse::Value(v) => write!(f, "{v}"),
            DeviceResponse::Error(e) => write!(f, "ERR: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceError {
    UnknownCommand(String),
    InvalidParameter { cmd: String, reason: String },
    QueryOnSetter(String),
    SetOnQuerier(String),
    NotConnected,
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceError::UnknownCommand(c) => write!(f, "unknown command: {c}"),
            DeviceError::InvalidParameter { cmd, reason } => {
                write!(f, "invalid parameter in '{cmd}': {reason}")
            }
            DeviceError::QueryOnSetter(c) => write!(f, "'{c}' is not a query"),
            DeviceError::SetOnQuerier(c) => write!(f, "'{c}' is a query, cannot set"),
            DeviceError::NotConnected => write!(f, "device not connected"),
        }
    }
}

impl std::error::Error for DeviceError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_response_display() {
        assert_eq!(DeviceResponse::Ack.to_string(), "ACK");
        assert_eq!(
            DeviceResponse::Value("2.882GHz".to_string()).to_string(),
            "2.882GHz"
        );
    }

    #[test]
    fn device_error_display() {
        let e = DeviceError::UnknownCommand("FOO".to_string());
        assert!(e.to_string().contains("FOO"));
    }
}
