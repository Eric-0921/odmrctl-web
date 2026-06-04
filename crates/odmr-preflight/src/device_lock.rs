use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

/// Cross-process device lock using POSIX file locking (flock).
///
/// The lock is automatically released when the `DeviceLock` is dropped.
/// Works on macOS, Linux, and Windows.
pub struct DeviceLock {
    _file: File,
    device_id: String,
}

impl DeviceLock {
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
    pub fn lock_file(&self) -> PathBuf {
        lock_file_path(&self.device_id)
    }
}

#[derive(Debug, Clone)]
pub enum LockError {
    AlreadyLocked { device_id: String, pid: Option<u32> },
    IoError { detail: String },
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::AlreadyLocked { device_id, pid } => {
                write!(f, "Device '{}' already locked by pid={:?}", device_id, pid)
            }
            LockError::IoError { detail } => write!(f, "Lock I/O error: {}", detail),
        }
    }
}

impl DeviceLock {
    /// Attempt to acquire an exclusive lock for a device.
    ///
    /// Returns `Ok(DeviceLock)` if acquired, or `Err(LockError::AlreadyLocked)`
    /// if another process holds the lock.
    pub fn try_acquire(device_id: &str) -> Result<Self, LockError> {
        let path = lock_file_path(device_id);
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| LockError::IoError {
                detail: format!("open lock file {}: {}", path.display(), e),
            })?;

        match file.try_lock_exclusive() {
            Ok(()) => Ok(DeviceLock {
                _file: file,
                device_id: device_id.to_string(),
            }),
            Err(_) => Err(LockError::AlreadyLocked {
                device_id: device_id.to_string(),
                pid: None, // TODO: read PID from lock file content
            }),
        }
    }

    /// Block until the lock is acquired.
    pub fn acquire(device_id: &str) -> Result<Self, LockError> {
        let path = lock_file_path(device_id);
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| LockError::IoError {
                detail: format!("open lock file {}: {}", path.display(), e),
            })?;

        file.lock_exclusive().map_err(|e| LockError::IoError {
            detail: format!("flock failed: {}", e),
        })?;

        Ok(DeviceLock {
            _file: file,
            device_id: device_id.to_string(),
        })
    }
}

impl Drop for DeviceLock {
    fn drop(&mut self) {
        // flock is automatically released when the file descriptor is closed
        // fs2's FileExt implementation does this on drop
        let _ = self._file.unlock();
    }
}

pub fn lock_file_path(device_id: &str) -> PathBuf {
    let sanitized: String = device_id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    std::env::temp_dir().join(format!("odmr_lock_{}.lock", sanitized))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_acquire_and_release() {
        let device_id = "test_device_42";
        {
            let lock = DeviceLock::try_acquire(device_id).unwrap();
            assert_eq!(lock.device_id, device_id);
            // Lock should be held here
        }
        // Lock released on drop; should be able to re-acquire
        let _lock2 = DeviceLock::try_acquire(device_id).unwrap();
    }

    #[test]
    fn lock_contention_detected() {
        let device_id = "test_device_contention";
        let _lock1 = DeviceLock::try_acquire(device_id).unwrap();
        let result = DeviceLock::try_acquire(device_id);
        assert!(matches!(result, Err(LockError::AlreadyLocked { .. })));
    }
}
