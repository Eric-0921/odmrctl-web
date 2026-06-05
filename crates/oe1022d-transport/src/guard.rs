//! K8: Single-process device lock to prevent concurrent takeover of the
//! same physical OE1022D.
//!
//! Even though our small project does not run a full Device Registry
//! (per the v0.1 PRD), we still need a simple `Mutex<()>`-style guard
//! so that two acquisition handles in the same process cannot both
//! open the same port (which would interleave `RALL?` requests and
//! produce unreadable frames).
//!
//! The lock is keyed by **device id (SN)** rather than by port path,
//! so it stays valid across USB re-enumeration.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Opaque handle returned by `DeviceLock::acquire`. Drop the handle to
/// release the lock; do not call `clone()` on it.
#[derive(Debug)]
pub struct DeviceLockGuard {
    /// We hold an Arc to the inner state so the same global map keeps
    /// a reference even after this guard drops.
    _registry: Arc<Mutex<LockRegistry>>,
    /// Device key (SN).
    device_id: String,
}

impl Drop for DeviceLockGuard {
    fn drop(&mut self) {
        if let Ok(mut reg) = self._registry.lock() {
            reg.holders.remove(&self.device_id);
        }
    }
}

#[derive(Default, Debug)]
struct LockRegistry {
    holders: HashMap<String, String>,
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum DeviceLockError {
    #[error("device {device_id} is already locked by {holder} (port {port})")]
    AlreadyLocked {
        device_id: String,
        holder: String,
        port: String,
    },
}

/// Global device lock registry. Construct one per process and share it
/// (via `Arc`) wherever acquisition handles are created.
#[derive(Debug, Clone, Default)]
pub struct DeviceLock {
    inner: Arc<Mutex<LockRegistry>>,
}

impl DeviceLock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to acquire the lock for `device_id`. The `holder` string is
    /// for diagnostics only (e.g. "AcquisitionThread", "IdnProbe").
    pub fn acquire(
        &self,
        device_id: &str,
        holder: &str,
        port: &str,
    ) -> Result<DeviceLockGuard, DeviceLockError> {
        let mut reg = self.inner.lock().expect("DeviceLock mutex poisoned");
        if let Some(existing) = reg.holders.get(device_id) {
            return Err(DeviceLockError::AlreadyLocked {
                device_id: device_id.to_string(),
                holder: existing.clone(),
                port: port.to_string(),
            });
        }
        reg.holders
            .insert(device_id.to_string(), holder.to_string());
        Ok(DeviceLockGuard {
            _registry: self.inner.clone(),
            device_id: device_id.to_string(),
        })
    }

    /// Check whether the device is currently held.
    pub fn is_locked(&self, device_id: &str) -> bool {
        let reg = self.inner.lock().expect("DeviceLock mutex poisoned");
        reg.holders.contains_key(device_id)
    }

    /// Snapshot of current holders. Useful for `*IDN?` probe to avoid
    /// opening the same port twice.
    pub fn holders(&self) -> Vec<String> {
        let reg = self.inner.lock().expect("DeviceLock mutex poisoned");
        reg.holders.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_and_release() {
        let lock = DeviceLock::new();
        let g = lock.acquire("SN001", "test", "/dev/cu.usbmodemA").unwrap();
        assert!(lock.is_locked("SN001"));
        drop(g);
        assert!(!lock.is_locked("SN001"));
    }

    #[test]
    fn k8_double_acquire_rejected() {
        let lock = DeviceLock::new();
        let _g1 = lock.acquire("SN002", "acq", "/dev/cu.usbmodemA").unwrap();
        let err = lock.acquire("SN002", "probe", "/dev/cu.usbmodemA").unwrap_err();
        assert!(matches!(err, DeviceLockError::AlreadyLocked { .. }));
    }

    #[test]
    fn lock_key_is_sn_not_port() {
        // K8 semantics: the lock is keyed by device id (SN), not by
        // port path. Two requests with the same SN but different
        // ports must be treated as the same device.
        let lock = DeviceLock::new();
        let _g1 = lock
            .acquire("SN003", "first", "/dev/cu.usbmodemPORT_A")
            .unwrap();
        let err = lock
            .acquire("SN003", "second", "/dev/cu.usbmodemPORT_B")
            .unwrap_err();
        assert!(matches!(err, DeviceLockError::AlreadyLocked { .. }));
    }

    #[test]
    fn different_sns_are_independent() {
        let lock = DeviceLock::new();
        let _g1 = lock.acquire("SN_A", "a", "/dev/cu.usbmodemA").unwrap();
        let _g2 = lock.acquire("SN_B", "b", "/dev/cu.usbmodemB").unwrap();
        assert!(lock.is_locked("SN_A"));
        assert!(lock.is_locked("SN_B"));
    }
}
