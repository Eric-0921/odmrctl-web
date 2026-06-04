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
//! let report = run_station_preflight(&profile, None, true).unwrap();
//! assert!(report.all_devices_reachable);
//! assert!(report.all_safe_states_confirmed);
//! ```

pub mod error;
pub mod types;
pub mod smb_probe;
pub mod oe_probe;
pub mod maynuo_probe;
pub mod cni_laser_probe;
pub mod device_lock;
pub mod station_report;
pub mod ledger;

pub use error::{PreflightError, PreflightResult};
pub use types::{StationProfile, StationPreflightReport, DevicePreflightReport};
pub use device_lock::{DeviceLock, LockError};
pub use ledger::{StationLedger, DeviceLedgerEntry, mark_safe, mark_unsafe, new_ledger};

use std::path::PathBuf;
use std::time::Instant;

/// Run the full station preflight (Phase A).
///
/// This is a blocking, synchronous operation that probes all devices
/// defined in the station profile. It does NOT enable any outputs.
///
/// If `ledger_path` is provided, the ledger is loaded before probing
/// and saved after. Devices that were previously marked unsafe trigger
/// an extended preflight mode which **requires** `operator_approved`
/// to be `true` — otherwise the preflight is rejected.
///
/// If `operator_approved` is `true`, the report's `operator_approved`
/// field will be set accordingly; this does NOT skip any safety checks.
pub fn run_station_preflight(
    profile: &StationProfile,
    ledger_path: Option<&PathBuf>,
    operator_approved: bool,
) -> Result<StationPreflightReport, PreflightError> {
    let started_at = Instant::now();

    // Load ledger if path provided
    let mut ledger = ledger_path
        .and_then(StationLedger::load)
        .unwrap_or_else(new_ledger);

    // Check for previously unsafe devices
    if ledger.any_unsafe() {
        eprintln!("⚠️  EXTENDED PREFLIGHT MODE: Previous run left devices in unsafe state.");
        for (id, entry) in &ledger.devices {
            if !entry.last_safe_state {
                eprintln!("   - {} was UNSAFE at {}", id, entry.last_seen);
            }
        }
        if !operator_approved {
            eprintln!("\n❌ EXTENDED MODE BLOCKED — Pass --operator-approve to acknowledge.");
            return Err(PreflightError::Other {
                detail: "Extended preflight mode requires --operator-approve".into(),
            });
        }
        eprintln!("   Operator approval acknowledged. Extra verification will be performed.");
    }

    // Acquire device locks before probing
    let mut locks: Vec<device_lock::DeviceLock> = Vec::new();
    let mut lock_failures: Vec<String> = Vec::new();
    for device in &profile.devices {
        match device_lock::DeviceLock::try_acquire(&device.device_id) {
            Ok(lock) => locks.push(lock),
            Err(e) => {
                lock_failures.push(format!("[{}] Lock failed: {}", device.device_id, e));
            }
        }
    }
    if !lock_failures.is_empty() {
        for err in &lock_failures {
            eprintln!("ERROR: {}", err);
        }
        return Err(PreflightError::DeviceBusy {
            device_id: profile.devices.first().map(|d| d.device_id.clone()).unwrap_or_default(),
            pid: None,
        });
    }

    let mut reports: Vec<DevicePreflightReport> = Vec::new();
    let mut all_ok = true;

    for device in &profile.devices {
        let report = match probe_device(device) {
            Ok(r) => r,
            Err(e) => {
                all_ok = false;
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

    let report = StationPreflightReport {
        schema_version: "0.1.0".to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        station_profile: profile.name.clone(),
        all_devices_reachable: reports.iter().all(|r| r.reachability),
        all_identities_verified: reports.iter().all(|r| r.identity_display.is_some()),
        all_safe_states_confirmed: reports.iter().all(|r| r.safe_state.as_ref().map(|s| s.confirmed).unwrap_or(false)),
        operator_approved,
        elapsed_ms: started_at.elapsed().as_millis() as u64,
        devices: reports,
    };

    // Update ledger based on results
    for d in &report.devices {
        let sn = d.identity_display.as_deref();
        if d.reachability && d.safe_state.as_ref().map(|s| s.confirmed).unwrap_or(false) {
            mark_safe(&mut ledger, &d.device_id, sn);
        } else {
            mark_unsafe(&mut ledger, &d.device_id, sn);
        }
    }

    // Save ledger
    if let Some(path) = ledger_path {
        if let Err(e) = ledger.save(path) {
            eprintln!("Warning: failed to save station ledger: {}", e);
        }
    }

    if !all_ok {
        return Err(PreflightError::Other {
            detail: "One or more devices failed preflight".into(),
        });
    }

    Ok(report)
}

fn probe_device(device: &types::DeviceConfig) -> PreflightResult<DevicePreflightReport> {
    match device.kind.as_str() {
        "rf_source" | "smb100a" => smb_probe::probe(device),
        "lock_in" | "oe1022d" => oe_probe::probe(device),
        "magnetic" | "maynuo" => maynuo_probe::probe(device),
        "laser" | "cni" | "cni_laser" => cni_laser_probe::probe(device),
        _ => Err(PreflightError::UnsupportedDeviceKind {
            kind: device.kind.clone(),
        }),
    }
}
