//! odmr-preflight — Unified station preflight for ODMR devices.
//!
//! Layer 1+2 boundary crate. Provides:
//! - Device discovery and identity verification
//! - Safe-state probing (query-only or safe-state-write)
//! - Cross-process device locks (`DeviceLock`)
//! - Station ledger persistence
//! - Preflight report generation
//!
//! ## Usage
//! ```no_run
//! use odmr_preflight::{StationProfile, run_station_preflight};
//!
//! let profile = StationProfile::load("station.json").unwrap();
//! let report = run_station_preflight(&profile, None, true).unwrap();
//! assert!(report.all_devices_reachable);
//! assert!(report.all_safe_states_confirmed);
//! ```

pub mod cni_laser_probe;
pub mod device_lock;
pub mod error;
pub mod ledger;
pub mod maynuo_probe;
pub mod oe_probe;
pub mod smb_probe;
pub mod station_report;
pub mod types;

pub use device_lock::{DeviceLock, LockError};
pub use error::{PreflightError, PreflightResult};
pub use ledger::{mark_safe, mark_unsafe, new_ledger, DeviceLedgerEntry, StationLedger};
pub use types::{DevicePreflightReport, StationPreflightReport, StationProfile};

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
    let (report, _locks) =
        run_station_preflight_with_locks(profile, ledger_path, operator_approved)?;
    Ok(report)
}

/// Run preflight and return both the report and the acquired device locks.
///
/// Callers that need to hold locks through execution (e.g. M5A) should use
/// this function and keep the `Vec<DeviceLock>` alive until cleanup completes.
pub fn run_station_preflight_with_locks(
    profile: &StationProfile,
    ledger_path: Option<&PathBuf>,
    operator_approved: bool,
) -> Result<(StationPreflightReport, Vec<device_lock::DeviceLock>), PreflightError> {
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
    let mut lock_status: Vec<types::DeviceLockStatus> = Vec::new();
    let mut any_lock_failed = false;
    for device in &profile.devices {
        let lock_file = device_lock::lock_file_path(&device.device_id);
        match device_lock::DeviceLock::try_acquire(&device.device_id) {
            Ok(lock) => {
                lock_status.push(types::DeviceLockStatus {
                    device_id: device.device_id.clone(),
                    acquired: true,
                    lock_file: lock_file.display().to_string(),
                    pid: None,
                    error: None,
                });
                locks.push(lock);
            }
            Err(e) => {
                any_lock_failed = true;
                let err_str = format!("{}", e);
                let pid = match &e {
                    device_lock::LockError::AlreadyLocked { pid, .. } => *pid,
                    _ => None,
                };
                lock_status.push(types::DeviceLockStatus {
                    device_id: device.device_id.clone(),
                    acquired: false,
                    lock_file: lock_file.display().to_string(),
                    pid,
                    error: Some(err_str),
                });
                eprintln!("ERROR: [{}] Lock failed: {}", device.device_id, e);
            }
        }
    }
    if any_lock_failed {
        let _report = StationPreflightReport {
            schema_version: "0.1.0".to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            station_profile: profile.name.clone(),
            all_devices_reachable: false,
            all_identities_verified: false,
            all_safe_states_confirmed: false,
            operator_approved,
            elapsed_ms: started_at.elapsed().as_millis() as u64,
            devices: vec![],
            lock_status,
        };
        return Err(PreflightError::DeviceBusy {
            device_id: profile
                .devices
                .first()
                .map(|d| d.device_id.clone())
                .unwrap_or_default(),
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
                    commands_sent: None,
                    laser_on_sent: None,
                    nonzero_power_sent: None,
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
        all_safe_states_confirmed: reports
            .iter()
            .all(|r| r.safe_state.as_ref().map(|s| s.confirmed).unwrap_or(false)),
        operator_approved,
        elapsed_ms: started_at.elapsed().as_millis() as u64,
        devices: reports,
        lock_status,
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

    Ok((report, locks))
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
