//! odmr-types — Layer 0 base types for the ODMR automation system.
//!
//! Zero external dependencies (std only).

use std::fmt;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// DeviceId
// ---------------------------------------------------------------------------

/// Opaque identifier for a device instance in a station.
///
/// Rules (per PRD v0.2):
/// - lowercase letters, digits, underscores, hyphens
/// - no spaces, no slashes, no Chinese characters
/// - stable, should not change frequently
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeviceId(pub String);

impl DeviceId {
    pub fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
    }

    /// Validates that the ID conforms to PRD rules.
    pub fn validate(&self) -> Result<(), TypeError> {
        if self.0.is_empty() {
            return Err(TypeError::InvalidId("device_id is empty".into()));
        }
        if !self
            .0
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(TypeError::InvalidId(format!(
                "device_id '{}' contains invalid characters (allowed: a-z, 0-9, _, -)",
                self.0
            )));
        }
        Ok(())
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for DeviceId {
    type Err = TypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id = DeviceId(s.to_string());
        id.validate()?;
        Ok(id)
    }
}

// ---------------------------------------------------------------------------
// DeviceKind
// ---------------------------------------------------------------------------

/// Supported device kinds in the ODMR system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    /// R&S SMB100A RF / microwave signal generator.
    Smb100a,
    /// SSI OE1022D DSP lock-in amplifier.
    Oe1022d,
    /// Laser controller.
    Laser,
    /// XYZ magnetic field controller.
    MagnetXyz,
}

impl DeviceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceKind::Smb100a => "smb100a",
            DeviceKind::Oe1022d => "oe1022d",
            DeviceKind::Laser => "laser",
            DeviceKind::MagnetXyz => "magnet_xyz",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "smb100a" => Some(DeviceKind::Smb100a),
            "oe1022d" => Some(DeviceKind::Oe1022d),
            "laser" => Some(DeviceKind::Laser),
            "magnet_xyz" => Some(DeviceKind::MagnetXyz),
            _ => None,
        }
    }
}

impl fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DeviceKind {
    type Err = TypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_lossy(s).ok_or_else(|| TypeError::UnknownDeviceKind(s.to_string()))
    }
}

// ---------------------------------------------------------------------------
// RunId
// ---------------------------------------------------------------------------

/// Opaque identifier for a single experimental run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RunId(pub String);

impl RunId {
    pub fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    InvalidId(String),
    UnknownDeviceKind(String),
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::InvalidId(msg) => write!(f, "invalid id: {msg}"),
            TypeError::UnknownDeviceKind(k) => write!(f, "unknown device kind: {k}"),
        }
    }
}

impl std::error::Error for TypeError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_valid() {
        let id = DeviceId::new("smb100a_01");
        assert!(id.validate().is_ok());
    }

    #[test]
    fn device_id_rejects_uppercase() {
        let id = DeviceId::new("SMB100A_01");
        assert!(id.validate().is_err());
    }

    #[test]
    fn device_id_rejects_space() {
        let id = DeviceId::new("smb 01");
        assert!(id.validate().is_err());
    }

    #[test]
    fn device_kind_roundtrip() {
        assert_eq!(DeviceKind::Smb100a.to_string(), "smb100a");
        assert_eq!(
            DeviceKind::from_str("oe1022d").unwrap(),
            DeviceKind::Oe1022d
        );
    }

    #[test]
    fn run_id_display() {
        let run = RunId::new("run_20260528_001");
        assert_eq!(run.to_string(), "run_20260528_001");
    }
}
