use std::fmt;

pub type PreflightResult<T> = Result<T, PreflightError>;

#[derive(Debug, Clone, PartialEq)]
pub enum PreflightError {
    /// Physical layer unreachable (ping failed, serial port not found)
    PhysicalUnreachable {
        device_id: String,
        detail: String,
    },
    /// Transport opened but identity mismatch or no response
    IdentityMismatch {
        device_id: String,
        expected: Option<String>,
        observed: Option<String>,
    },
    /// Device responded but error queue is not empty
    ErrorQueueNonEmpty {
        device_id: String,
        errors: Vec<String>,
    },
    /// Safe state verification failed (e.g., RF output still ON)
    SafeStateFailed {
        device_id: String,
        expected: String,
        observed: String,
    },
    /// Device is locked by another process
    DeviceBusy {
        device_id: String,
        pid: Option<u32>,
    },
    /// Unsupported device kind in profile
    UnsupportedDeviceKind {
        kind: String,
    },
    /// Serial port I/O error
    SerialError {
        device_id: String,
        detail: String,
    },
    /// TCP socket I/O error
    TcpError {
        device_id: String,
        detail: String,
    },
    /// Timeout waiting for device response
    Timeout {
        device_id: String,
        command: String,
        timeout_ms: u64,
    },
    /// Generic catch-all
    Other {
        detail: String,
    },
}

impl fmt::Display for PreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreflightError::PhysicalUnreachable { device_id, detail } => {
                write!(f, "[{}] Physical unreachable: {}", device_id, detail)
            }
            PreflightError::IdentityMismatch { device_id, expected, observed } => {
                write!(f, "[{}] Identity mismatch: expected={:?}, observed={:?}", device_id, expected, observed)
            }
            PreflightError::ErrorQueueNonEmpty { device_id, errors } => {
                write!(f, "[{}] Error queue non-empty: {:?}", device_id, errors)
            }
            PreflightError::SafeStateFailed { device_id, expected, observed } => {
                write!(f, "[{}] Safe state failed: expected={}, observed={}", device_id, expected, observed)
            }
            PreflightError::DeviceBusy { device_id, pid } => {
                write!(f, "[{}] Device busy (locked by pid={:?})", device_id, pid)
            }
            PreflightError::UnsupportedDeviceKind { kind } => {
                write!(f, "Unsupported device kind: {}", kind)
            }
            PreflightError::SerialError { device_id, detail } => {
                write!(f, "[{}] Serial error: {}", device_id, detail)
            }
            PreflightError::TcpError { device_id, detail } => {
                write!(f, "[{}] TCP error: {}", device_id, detail)
            }
            PreflightError::Timeout { device_id, command, timeout_ms } => {
                write!(f, "[{}] Timeout on '{}' after {}ms", device_id, command, timeout_ms)
            }
            PreflightError::Other { detail } => {
                write!(f, "{}", detail)
            }
        }
    }
}

impl std::error::Error for PreflightError {}
