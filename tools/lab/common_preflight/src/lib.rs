//! Common Preflight — Unified device connection and initialization for ODMR lab tools.
//!
//! This crate provides Phase A (passive preflight) and Phase B (armed execution gate)
//! for all real-device lab tools. It is NOT a core workspace crate; it lives in
//! `tools/lab/` to allow rapid iteration before stabilization.
//!
//! ## Usage
//! ```no_run
//! use common_preflight::{StationProfile, run_station_preflight};
//!
//! let profile = StationProfile::load("station.json").unwrap();
//! let report = run_station_preflight(&profile).unwrap();
//! assert!(report.all_devices_reachable);
//! assert!(report.all_safe_states_confirmed);
//! ```

pub mod error;
pub mod types;
pub mod smb_probe;
pub mod oe_probe;
pub mod maynuo_probe;
pub mod device_lock;
pub mod station_report;
pub mod ledger;

pub use error::{PreflightError, PreflightResult};
pub use types::{StationProfile, StationPreflightReport, DevicePreflightReport};
pub use device_lock::{DeviceLock, LockError};

use std::time::Instant;

/// Run the full station preflight (Phase A).
///
/// This is a blocking, synchronous operation that probes all devices
/// defined in the station profile. It does NOT enable any outputs.
pub fn run_station_preflight(profile: &StationProfile) -> Result<StationPreflightReport, PreflightError> {
    let started_at = Instant::now();
    let mut reports: Vec<DevicePreflightReport> = Vec::new();
    for device in &profile.devices {
        let report = match probe_device(device) {
            Ok(r) => r,
            Err(e) => {
                DevicePreflightReport {
                    device_id: device.device_id.clone(),
                    kind: device.kind.clone(),
                    reachability: false,
                    identity_raw: None,
                    identity_display: None,
                    error_queue: vec![format!("{}", e)],
                    safe_state: None,
                    warnings: vec![format!("Probe failed: {}", e)],
                }
            }
        };
        reports.push(report);
    }

    Ok(StationPreflightReport {
        schema_version: "0.1.0".to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        station_profile: profile.name.clone(),
        all_devices_reachable: reports.iter().all(|r| r.reachability),
        all_identities_verified: reports.iter().all(|r| r.identity_display.is_some()),
        all_safe_states_confirmed: reports.iter().all(|r| r.safe_state.as_ref().map(|s| s.confirmed).unwrap_or(false)),
        operator_approved: false,
        elapsed_ms: started_at.elapsed().as_millis() as u64,
        devices: reports,
    })
}

fn probe_device(device: &types::DeviceConfig) -> PreflightResult<DevicePreflightReport> {
    match device.kind.as_str() {
        "rf_source" | "smb100a" => smb_probe::probe(device),
        "lock_in" | "oe1022d" => oe_probe::probe(device),
        "magnetic" | "maynuo" => maynuo_probe::probe(device),
        _ => Err(PreflightError::UnsupportedDeviceKind {
            kind: device.kind.clone(),
        }),
    }
}
