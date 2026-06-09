//! CNI Laser minimal device panel — M5C-A
//!
//! All commands require the device to be locked via preflight first.
//! **IMPORTANT**: The CNI Laser protocol has no query commands.
//! All "status" is session-local.

use super::with_device_access;
use crate::workbench_state::WorkbenchState;
use odmr_laser::{LaserClient, LaserSerialConfig};
use odmr_types::DeviceId;
use serde::Serialize;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize)]
pub struct LaserStatus {
    pub connected: bool,
    pub power_setpoint_mw: u16,
    pub enabled: bool,
    pub note: &'static str,
    pub last_command_time: String,
}

fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}

/// Session-local laser state (no hardware readback exists).
#[derive(Debug, Clone, Default)]
struct LaserSessionState {
    power_mw: u16,
    enabled: bool,
}

static LASER_STATE: OnceLock<Mutex<LaserSessionState>> = OnceLock::new();

fn laser_state() -> &'static Mutex<LaserSessionState> {
    LASER_STATE.get_or_init(|| Mutex::new(LaserSessionState::default()))
}

fn get_state() -> LaserSessionState {
    laser_state().lock().map(|g| g.clone()).unwrap_or_default()
}

fn set_state(power_mw: u16, enabled: bool) {
    let _ = laser_state().lock().map(|mut g| {
        g.power_mw = power_mw;
        g.enabled = enabled;
    });
}

#[tauri::command]
pub fn laser_get_status(
    _state: tauri::State<WorkbenchState>,
    _device_id: String,
) -> Result<LaserStatus, String> {
    let st = get_state();
    Ok(LaserStatus {
        connected: true,
        power_setpoint_mw: st.power_mw,
        enabled: st.enabled,
        note: "No hardware readback available. State is session-only.",
        last_command_time: now_rfc3339(),
    })
}

#[tauri::command]
pub fn laser_set_power(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    power_mw: u16,
) -> Result<LaserStatus, String> {
    let safety = state.safety();
    let power = power_mw.min(safety.laser_max_power_mw);

    let port_path = with_device_access(&state, &device_id)?;
    let mut client = LaserClient::open(
        DeviceId::new(device_id.clone()),
        port_path,
        LaserSerialConfig {
            max_power_mw: safety.laser_max_power_mw,
            ..LaserSerialConfig::default()
        },
    )
    .map_err(|e| format!("open laser client: {e}"))?;
    let power = client
        .set_power(power)
        .map_err(|e| format!("set laser power: {e}"))?;

    set_state(power, get_state().enabled);
    Ok(LaserStatus {
        connected: true,
        power_setpoint_mw: power,
        enabled: get_state().enabled,
        note: "No hardware readback available. State is session-only.",
        last_command_time: now_rfc3339(),
    })
}

#[tauri::command]
pub fn laser_set_enabled(
    state: tauri::State<WorkbenchState>,
    device_id: String,
    enabled: bool,
) -> Result<LaserStatus, String> {
    let st = get_state();
    if enabled && st.power_mw == 0 {
        return Err("Laser enable blocked: power setpoint is 0 mW. Set power > 0 first.".into());
    }

    let port_path = with_device_access(&state, &device_id)?;
    let mut client = LaserClient::open(
        DeviceId::new(device_id.clone()),
        port_path,
        LaserSerialConfig {
            max_power_mw: state.safety().laser_max_power_mw,
            ..LaserSerialConfig::default()
        },
    )
    .map_err(|e| format!("open laser client: {e}"))?;
    client
        .set_enabled(enabled)
        .map_err(|e| format!("set laser enabled={enabled}: {e}"))?;

    set_state(st.power_mw, enabled);
    Ok(LaserStatus {
        connected: true,
        power_setpoint_mw: st.power_mw,
        enabled,
        note: "No hardware readback available. State is session-only.",
        last_command_time: now_rfc3339(),
    })
}

#[tauri::command]
pub fn laser_emergency_off(
    state: tauri::State<WorkbenchState>,
    device_id: String,
) -> Result<LaserStatus, String> {
    let port_path = with_device_access(&state, &device_id)?;
    let mut client = LaserClient::open(
        DeviceId::new(device_id),
        port_path,
        LaserSerialConfig::default(),
    )
    .map_err(|e| format!("open laser client: {e}"))?;
    let _ = client.emergency_off();

    set_state(0, false);
    Ok(LaserStatus {
        connected: true,
        power_setpoint_mw: 0,
        enabled: false,
        note: "EMERGENCY OFF sent. Power set to 0, emission disabled.",
        last_command_time: now_rfc3339(),
    })
}
