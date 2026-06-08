//! Station Workbench commands — M5C-A

use crate::panels::load_station_safety;
use crate::workbench_state::WorkbenchState;
use odmr_preflight::{run_station_preflight_with_locks, StationProfile};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};

/// Snapshot of the workbench for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct WorkbenchSnapshot {
    pub profile_loaded: bool,
    pub profile_name: Option<String>,
    pub preflight_passed: bool,
    pub locks_held: Vec<String>,
    pub report: Option<odmr_preflight::StationPreflightReport>,
    /// Addresses from the loaded profile (device_id → address).
    pub profile_addresses: HashMap<String, String>,
}

/// Load a station profile from disk, parse safety limits, and cache in Tauri state.
#[tauri::command]
pub fn load_station_profile(
    state: tauri::State<WorkbenchState>,
    path: String,
) -> Result<StationProfile, String> {
    let profile = StationProfile::load(&path)?;
    let safety = load_station_safety(&path).unwrap_or_default();

    let mut guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    guard.profile = Some(profile.clone());
    guard.safety = safety;
    Ok(profile)
}

/// Load the built-in example station profile from the repo.
#[tauri::command]
pub fn load_example_station_profile(
    state: tauri::State<WorkbenchState>,
) -> Result<StationProfile, String> {
    // Search in several possible locations relative to project root / executable.
    let candidates: Vec<std::path::PathBuf> = {
        let mut v = vec![];
        // Relative to CWD (dev mode)
        v.push("examples/stations/odmr_station.full.example.json".into());
        v.push("../examples/stations/odmr_station.full.example.json".into());
        v.push("../../examples/stations/odmr_station.full.example.json".into());
        // Relative to executable (production)
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                v.push(dir.join("examples/stations/odmr_station.full.example.json"));
                v.push(dir.join("../examples/stations/odmr_station.full.example.json"));
            }
        }
        v
    };

    for path in &candidates {
        if path.exists() {
            return load_station_profile(state, path.to_string_lossy().to_string());
        }
    }

    // Fallback: create an in-memory mock profile.
    let mock = r#"{
        "name": "Mock NV Lab Station (Example)",
        "devices": [
            {"device_id":"smb100a_main","kind":"rf_source","transport":"tcp_raw_socket","address":"192.168.1.20:5025","expected_sn":null,"timeout_ms":5000},
            {"device_id":"oe1022d_main","kind":"lock_in","transport":"serial","address":"/dev/cu.usbmodem3361358734371","expected_sn":null,"timeout_ms":5000},
            {"device_id":"maynuo.mag_x","kind":"magnetic","transport":"serial","address":"auto","expected_sn":"2020","timeout_ms":5000},
            {"device_id":"maynuo.mag_y","kind":"magnetic","transport":"serial","address":"auto","expected_sn":"2022","timeout_ms":5000},
            {"device_id":"maynuo.mag_z","kind":"magnetic","transport":"serial","address":"auto","expected_sn":"2003","timeout_ms":5000},
            {"device_id":"cni_laser","kind":"laser","transport":"none","address":null,"expected_sn":null,"timeout_ms":0}
        ]
    }"#;
    let profile: StationProfile =
        serde_json::from_str(mock).map_err(|e| format!("parse mock profile: {e}"))?;

    let mut guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    guard.profile = Some(profile.clone());
    guard.safety = Default::default();
    Ok(profile)
}

/// Run preflight using the cached profile. Acquires and holds device locks in Tauri state.
#[tauri::command]
pub fn run_station_preflight_cmd(
    state: tauri::State<WorkbenchState>,
    operator_approved: bool,
) -> Result<odmr_preflight::StationPreflightReport, String> {
    let profile = {
        let guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard
            .profile
            .clone()
            .ok_or("No station profile loaded. Load a profile first.")?
    };

    let (report, locks) = run_station_preflight_with_locks(&profile, None, operator_approved)
        .map_err(|e| format!("preflight failed: {e}"))?;

    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.preflight_report = Some(report.clone());
        guard.locks = locks;
    }

    Ok(report)
}

/// Release all held device locks and clear the preflight report.
#[tauri::command]
pub fn release_all_locks(state: tauri::State<WorkbenchState>) -> Result<(), String> {
    let mut guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    guard.locks.clear(); // DeviceLock Drop releases flock
    guard.preflight_report = None;
    Ok(())
}

/// Connect a single device by providing its address directly (no station.json required).
///
/// Performs a lightweight identity check (*IDN? for SCPI, frame echo for laser)
/// and stores the address in workbench state for subsequent panel commands.
#[tauri::command]
pub fn connect_single_device(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    address: String,
    kind: String,
) -> Result<String, String> {
    // Quick identity probe based on device kind
    let idn = match kind.as_str() {
        "smb100a" | "rf_source" => {
            let stream = std::net::TcpStream::connect(&address)
                .map_err(|e| format!("TCP connect to {address}: {e}"))?;
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .ok();
            stream
                .set_write_timeout(Some(std::time::Duration::from_secs(3)))
                .ok();
            let mut stream = stream;
            stream
                .write_all(b"*IDN?\n")
                .map_err(|e| format!("write: {e}"))?;
            stream.flush().map_err(|e| format!("flush: {e}"))?;
            let mut buf = [0u8; 256];
            let n = stream.read(&mut buf).map_err(|e| format!("read: {e}"))?;
            String::from_utf8_lossy(&buf[..n]).trim().to_string()
        }
        "oe1022d" | "lock_in" => {
            let mut port = serialport::new(&address, 921_600)
                .timeout(std::time::Duration::from_secs(3))
                .open()
                .map_err(|e| format!("open serial {address}: {e}"))?;
            let _ = port.clear(serialport::ClearBuffer::Input);
            port.write_all(b"*IDN?\r")
                .map_err(|e| format!("write: {e}"))?;
            port.flush().map_err(|e| format!("flush: {e}"))?;
            std::thread::sleep(std::time::Duration::from_millis(500));
            let mut buf = vec![0u8; 256];
            let n = port.read(&mut buf).map_err(|e| format!("read: {e}"))?;
            String::from_utf8_lossy(&buf[..n]).trim().to_string()
        }
        "magnetic" | "magnet_xyz" | "maynuo" => {
            let mut port = serialport::new(&address, 9600)
                .timeout(std::time::Duration::from_secs(3))
                .open()
                .map_err(|e| format!("open serial {address}: {e}"))?;
            let _ = port.clear(serialport::ClearBuffer::Input);
            port.write_all(b"*IDN?\n")
                .map_err(|e| format!("write: {e}"))?;
            port.flush().map_err(|e| format!("flush: {e}"))?;
            std::thread::sleep(std::time::Duration::from_millis(200));
            let mut buf = vec![0u8; 256];
            let n = port.read(&mut buf).map_err(|e| format!("read: {e}"))?;
            String::from_utf8_lossy(&buf[..n]).trim().to_string()
        }
        "laser" | "cni_laser" => {
            // Laser has no query; just verify serial port opens
            let _port = serialport::new(&address, 9600)
                .timeout(std::time::Duration::from_secs(2))
                .open()
                .map_err(|e| format!("open serial {address}: {e}"))?;
            "CNI Laser (no IDN query)".to_string()
        }
        _ => return Err(format!("Unknown device kind: {kind}")),
    };

    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.dynamic_addresses.insert(device_id.clone(), address);
        guard.single_device_connected.insert(device_id);
    }

    Ok(idn)
}

/// Disconnect a single device that was connected via `connect_single_device`.
#[tauri::command]
pub fn disconnect_single_device(
    state: tauri::State<WorkbenchState>,
    device_id: String,
) -> Result<(), String> {
    let mut guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    guard.dynamic_addresses.remove(&device_id);
    guard.single_device_connected.remove(&device_id);
    Ok(())
}

/// Get a snapshot of the current workbench state.
#[tauri::command]
pub fn get_workbench_state(
    state: tauri::State<WorkbenchState>,
) -> Result<WorkbenchSnapshot, String> {
    let guard = state
        .inner
        .lock()
        .map_err(|e| format!("lock poison: {e}"))?;
    let mut locks_held: Vec<String> = guard
        .preflight_report
        .as_ref()
        .map(|r| {
            r.lock_status
                .iter()
                .filter(|ls| ls.acquired)
                .map(|ls| ls.device_id.clone())
                .collect()
        })
        .unwrap_or_default();
    // Also include single-device connected devices
    for id in &guard.single_device_connected {
        if !locks_held.contains(id) {
            locks_held.push(id.clone());
        }
    }

    let batch_passed = guard
        .preflight_report
        .as_ref()
        .map(|r| {
            r.all_devices_reachable && r.all_identities_verified && r.all_safe_states_confirmed
        })
        .unwrap_or(false);

    let profile_addresses: HashMap<String, String> = guard
        .profile
        .as_ref()
        .map(|p| {
            p.devices
                .iter()
                .map(|d| (d.device_id.clone(), d.address.clone()))
                .collect()
        })
        .unwrap_or_default();

    Ok(WorkbenchSnapshot {
        profile_loaded: guard.profile.is_some(),
        profile_name: guard.profile.as_ref().map(|p| p.name.clone()),
        preflight_passed: batch_passed || !guard.single_device_connected.is_empty(),
        locks_held,
        report: guard.preflight_report.clone(),
        profile_addresses,
    })
}
