use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Persistent station state ledger.
///
/// Written on normal exit, read on next preflight.
/// If `last_safe_state == false`, next preflight enters extended mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationLedger {
    pub schema_version: String,
    pub last_updated: String,
    pub devices: HashMap<String, DeviceLedgerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLedgerEntry {
    pub last_seen: String,
    pub last_safe_state: bool,
    pub last_sn: Option<String>,
}

impl StationLedger {
    pub fn default_path() -> PathBuf {
        std::env::temp_dir().join("odmr_station_state.json")
    }

    pub fn load(path: &PathBuf) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), String> {
        let json =
            serde_json::to_string_pretty(self).map_err(|e| format!("serialize ledger: {e}"))?;
        std::fs::write(path, json).map_err(|e| format!("write ledger: {e}"))
    }

    pub fn any_unsafe(&self) -> bool {
        self.devices.values().any(|d| !d.last_safe_state)
    }
}

/// Mark all devices as safe in the ledger.
pub fn mark_safe(ledger: &mut StationLedger, device_id: &str, sn: Option<&str>) {
    ledger.devices.insert(
        device_id.to_string(),
        DeviceLedgerEntry {
            last_seen: chrono::Utc::now().to_rfc3339(),
            last_safe_state: true,
            last_sn: sn.map(|s| s.to_string()),
        },
    );
}

/// Mark a device as unsafe (e.g., after a crash or failed cleanup).
pub fn mark_unsafe(ledger: &mut StationLedger, device_id: &str, sn: Option<&str>) {
    ledger.devices.insert(
        device_id.to_string(),
        DeviceLedgerEntry {
            last_seen: chrono::Utc::now().to_rfc3339(),
            last_safe_state: false,
            last_sn: sn.map(|s| s.to_string()),
        },
    );
}

/// Create a new empty ledger.
pub fn new_ledger() -> StationLedger {
    StationLedger {
        schema_version: "0.1.0".to_string(),
        last_updated: chrono::Utc::now().to_rfc3339(),
        devices: HashMap::new(),
    }
}
