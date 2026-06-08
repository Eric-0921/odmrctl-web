//! WorkbenchState — persistent Tauri state for M5C-A Device Workbench.
//!
//! Holds:
//! - Loaded station profile + parsed safety limits + coil constants
//! - Last preflight report
//! - Acquired device locks (cross-process flock)
//! - Per-axis magnetic state (zero bias, recur current, lock-zero flag)
//! - Dynamic device addresses (for per-card connect without station.json)
//!
//! Locks are held across Tauri commands until `release_locks()` is called.

use crate::panels::StationSafety;
use odmr_preflight::{DeviceLock, StationPreflightReport, StationProfile};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// Runtime zero baseline measured during this application session.
///
/// This is intentionally separate from station/profile/recipe JSON. Profiles
/// hold long-lived calibration; this value is measured after the station is
/// connected and locked.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeZeroBaseline {
    pub schema_version: String,
    pub kind: String,
    pub session_id: String,
    pub locked_at: String,
    pub axes: HashMap<String, RuntimeZeroAxisBaseline>,
}

/// Per-axis zero baseline measurement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeZeroAxisBaseline {
    pub device_id: String,
    pub axis: String,
    pub identity: Option<String>,
    pub zero_samples_a: Vec<f64>,
    pub zero_mean_a: f64,
    pub zero_std_a: f64,
    pub coil_constant_nt_per_ma: f64,
}

/// Inner mutable state.
#[derive(Default)]
pub struct WorkbenchStateInner {
    pub profile: Option<StationProfile>,
    pub safety: StationSafety,
    pub preflight_report: Option<StationPreflightReport>,
    pub locks: Vec<DeviceLock>,

    // --- Per-device dynamic connection (no station.json required) ---
    /// Device addresses provided directly via Devices page connect.
    pub dynamic_addresses: HashMap<String, String>,
    /// Device addresses inferred by serial auto-identification but not yet connected.
    pub auto_bound_addresses: HashMap<String, String>,
    /// Devices that passed single-device identity check.
    pub single_device_connected: HashSet<String>,

    // --- Magnetic axis state (matches original GUI logic) ---
    /// Zero-field bias current per axis (A).  User-adjustable until lock-zero.
    pub mag_zero_bias: HashMap<String, f64>,
    /// Recurrent (reproduction) current per axis (A).  Only effective when lock-zero=ON.
    pub mag_recur_current: HashMap<String, f64>,
    /// Lock-zero flag per axis.  When true, zero_bias is frozen and recur_current is added.
    pub mag_lock_zero: HashMap<String, bool>,
    /// Output flag per axis tracked by workbench commands.
    pub mag_output_on: HashMap<String, bool>,
    /// Coil constant per axis (nT / mA).  Used for Mag ↔ Current conversion.
    pub mag_coil_constant: HashMap<String, f64>,
    /// Runtime zero baseline measured and locked for the current session.
    pub runtime_zero_baseline: Option<RuntimeZeroBaseline>,
    /// Loaded experiment plan JSON. This is a plan preview artifact only.
    pub experiment_plan: Option<serde_json::Value>,
    /// In-memory experiment draft edited by GUI tables. Reset on app restart.
    pub experiment_plan_draft: Option<serde_json::Value>,
    /// Per-device preset drafts edited in Device Workbench. Reset on app restart.
    pub device_preset_drafts: HashMap<String, serde_json::Value>,
    /// Selected default package id per device for GUI continuity.
    pub selected_default_packages: HashMap<String, String>,
    /// Last experiment-plan run launcher status. Stored as JSON to keep the
    /// session state independent from the experiment-plan command module.
    pub experiment_run_status: Option<serde_json::Value>,
    /// Cooperative stop flag for the current experiment-plan run launcher.
    pub experiment_run_cancel_requested: bool,
}

/// Tauri-managed state wrapper.
pub struct WorkbenchState {
    pub inner: Mutex<WorkbenchStateInner>,
}

impl Default for WorkbenchState {
    fn default() -> Self {
        let mut inner = WorkbenchStateInner::default();
        inner
            .mag_coil_constant
            .insert("maynuo.mag_x".to_string(), 143.26);
        inner
            .mag_coil_constant
            .insert("maynuo.mag_y".to_string(), 141.77);
        inner
            .mag_coil_constant
            .insert("maynuo.mag_z".to_string(), 156.15);
        Self {
            inner: Mutex::new(inner),
        }
    }
}

impl WorkbenchState {
    /// True if the device is accessible:
    /// - either locked via batch preflight, OR
    /// - connected via single-device connect.
    pub fn is_accessible(&self, device_id: &str) -> bool {
        let guard = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };
        // Batch preflight lock?
        let batch_locked = guard
            .preflight_report
            .as_ref()
            .map(|r| {
                r.lock_status
                    .iter()
                    .any(|ls| ls.device_id == device_id && ls.acquired)
            })
            .unwrap_or(false);
        // Single-device connect?
        let single_connected = guard.single_device_connected.contains(device_id);
        batch_locked || single_connected
    }

    /// Get address for a device: dynamic first, then profile.
    pub fn device_address(&self, device_id: &str) -> Option<String> {
        let guard = self.inner.lock().ok()?;
        // 1. Dynamic address (single-device connect)
        if let Some(addr) = guard.dynamic_addresses.get(device_id) {
            return Some(addr.clone());
        }
        // 2. Auto-bound address from serial identification
        if let Some(addr) = guard.auto_bound_addresses.get(device_id) {
            return Some(addr.clone());
        }
        // 3. Profile address
        let profile = guard.profile.as_ref()?;
        profile
            .devices
            .iter()
            .find(|d| d.device_id == device_id)
            .map(|d| d.address.clone())
    }

    /// Get a clone of the cached safety limits.
    pub fn safety(&self) -> StationSafety {
        let guard = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return StationSafety::default(),
        };
        guard.safety.clone()
    }

    // --- Magnetic helpers ---

    pub fn mag_zero_bias(&self, device_id: &str) -> f64 {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.mag_zero_bias.get(device_id).copied())
            .unwrap_or(0.0)
    }

    pub fn mag_recur_current(&self, device_id: &str) -> f64 {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.mag_recur_current.get(device_id).copied())
            .unwrap_or(0.0)
    }

    pub fn mag_lock_zero(&self, device_id: &str) -> bool {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.mag_lock_zero.get(device_id).copied())
            .unwrap_or(false)
    }

    pub fn mag_output_on(&self, device_id: &str) -> bool {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.mag_output_on.get(device_id).copied())
            .unwrap_or(false)
    }

    pub fn mag_coil_constant(&self, device_id: &str) -> f64 {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.mag_coil_constant.get(device_id).copied())
            .unwrap_or(143.0)
    }

    pub fn mag_total_current(&self, device_id: &str) -> f64 {
        let bias = self.mag_zero_bias(device_id);
        let recur = self.mag_recur_current(device_id);
        let locked = self.mag_lock_zero(device_id);
        if locked {
            bias + recur
        } else {
            bias
        }
    }
}
