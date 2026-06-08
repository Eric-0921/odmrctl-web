//! Station Workbench commands — M5C-A

use crate::panels::load_station_safety;
use crate::workbench_state::WorkbenchState;
use odmr_preflight::{run_station_preflight_with_locks, StationProfile};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialPortInfo {
    pub port_name: String,
    pub port_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifiedSerialDevice {
    pub port: String,
    pub detected_kind: String,
    pub idn: Option<String>,
    pub serial_number: Option<String>,
    pub confidence: String,
    pub suggested_role: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerialIdentifyReport {
    pub ports: Vec<SerialPortInfo>,
    pub devices: Vec<IdentifiedSerialDevice>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceRoleRequest {
    pub device_id: String,
    pub kind: String,
    pub expected_sn: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutoBoundDevice {
    pub device_id: String,
    pub kind: String,
    pub address: Option<String>,
    pub idn: Option<String>,
    pub serial_number: Option<String>,
    pub confidence: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutoBindReport {
    pub bound: Vec<AutoBoundDevice>,
    pub blocked: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub transport: String,
    pub address: String,
    pub detected_kind: String,
    pub idn: Option<String>,
    pub serial_number: Option<String>,
    pub model: Option<String>,
    pub confidence: String,
    pub suggested_role: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDiscoveryReport {
    pub serial_ports: Vec<SerialPortInfo>,
    pub tcp_targets: Vec<String>,
    pub usb_resources: Vec<String>,
    pub devices: Vec<DiscoveredDevice>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceProbeRequest {
    pub requested_kinds: Vec<String>,
    pub smb100a_tcp_targets: Vec<String>,
    pub enable_usb_probe: bool,
}

fn serial_number_from_idn(idn: &str) -> Option<String> {
    for token in idn.split(|c: char| !c.is_ascii_alphanumeric()) {
        if token.len() >= 4 && token.chars().all(|c| c.is_ascii_digit()) {
            return Some(token.to_string());
        }
    }
    None
}

fn smb100a_serial_from_idn(idn: &str) -> Option<String> {
    idn.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 4 && token.chars().all(|c| c.is_ascii_digit()))
        .last()
        .map(str::to_string)
}

fn smb100a_model_from_idn(idn: &str) -> Option<String> {
    idn.split(',')
        .find(|part| part.to_ascii_uppercase().contains("SMB100A"))
        .map(|part| part.trim().to_string())
        .or_else(|| {
            idn.split_whitespace()
                .find(|part| part.to_ascii_uppercase().contains("SMB100A"))
                .map(|part| part.trim().to_string())
        })
}

fn is_smb100a_idn(idn: &str) -> bool {
    let upper = idn.to_ascii_uppercase();
    upper.contains("SMB100A") || (upper.contains("ROHDE") && upper.contains("SCHWARZ"))
}

fn discovered_from_serial(device: IdentifiedSerialDevice) -> DiscoveredDevice {
    DiscoveredDevice {
        transport: "serial".to_string(),
        address: device.port,
        detected_kind: device.detected_kind,
        idn: device.idn,
        serial_number: device.serial_number,
        model: None,
        confidence: device.confidence,
        suggested_role: device.suggested_role,
        status: device.status,
    }
}

fn list_serial_ports_inner() -> Result<Vec<SerialPortInfo>, String> {
    let mut ports: Vec<SerialPortInfo> = serialport::available_ports()
        .map_err(|e| format!("list serial ports: {e}"))?
        .into_iter()
        .map(|port| SerialPortInfo {
            port_name: port.port_name,
            port_type: format!("{:?}", port.port_type),
        })
        .collect();
    ports.sort_by(|a, b| a.port_name.cmp(&b.port_name));
    Ok(ports)
}

fn should_probe_serial_port(port_name: &str) -> bool {
    let lower = port_name.to_ascii_lowercase();
    if lower.contains("bluetooth")
        || lower.contains("incoming")
        || lower.contains("debug-console")
        || lower.contains("freebuds")
        || lower.contains("freelace")
        || lower.contains("soundpeats")
    {
        return false;
    }
    lower.starts_with("com")
        || lower.contains("usb")
        || lower.contains("pl2303")
        || lower.contains("ftdi")
        || lower.contains("ch340")
        || lower.contains("ttyusb")
        || lower.contains("ttyacm")
}

fn likely_maynuo_serial_port(port_name: &str) -> bool {
    let lower = port_name.to_ascii_lowercase();
    lower.contains("pl2303") || lower.contains("usbserial")
}

fn serial_query(
    port_name: &str,
    baud: u32,
    command: &[u8],
    wait_ms: u64,
) -> Result<String, String> {
    let mut port = serialport::new(port_name, baud)
        .timeout(Duration::from_millis(700))
        .open()
        .map_err(|e| format!("open serial {port_name}: {e}"))?;
    let _ = port.clear(serialport::ClearBuffer::Input);
    port.write_all(command)
        .map_err(|e| format!("write {port_name}: {e}"))?;
    port.flush()
        .map_err(|e| format!("flush {port_name}: {e}"))?;
    std::thread::sleep(Duration::from_millis(wait_ms));
    let mut buf = vec![0u8; 512];
    let n = port
        .read(&mut buf)
        .map_err(|e| format!("read {port_name}: {e}"))?;
    Ok(String::from_utf8_lossy(&buf[..n]).trim().to_string())
}

fn identify_serial_port(
    port_name: &str,
    requested_kinds: &[String],
) -> Option<IdentifiedSerialDevice> {
    let wants =
        |kind: &str| requested_kinds.is_empty() || requested_kinds.iter().any(|k| k == kind);

    if likely_maynuo_serial_port(port_name) && (wants("magnetic") || wants("maynuo")) {
        if let Ok(idn) = serial_query(port_name, 9600, b"*IDN?\n", 200) {
            let serial_number = serial_number_from_idn(&idn);
            let suggested_role = match serial_number.as_deref() {
                Some("2020") => Some("maynuo.mag_x".to_string()),
                Some("2022") => Some("maynuo.mag_y".to_string()),
                Some("2003") => Some("maynuo.mag_z".to_string()),
                _ => None,
            };
            let upper = idn.to_ascii_uppercase();
            if upper.contains("MAYNUO") || upper.contains("M8812") || suggested_role.is_some() {
                return Some(IdentifiedSerialDevice {
                    port: port_name.to_string(),
                    detected_kind: "magnetic".to_string(),
                    idn: Some(idn),
                    serial_number,
                    confidence: if suggested_role.is_some() {
                        "high"
                    } else {
                        "medium"
                    }
                    .to_string(),
                    suggested_role,
                    status: "identified".to_string(),
                });
            }
        }
    }

    if wants("oe1022d") || wants("lock_in") {
        if let Ok(idn) = serial_query(port_name, 921_600, b"*IDN?\r", 500) {
            let upper = idn.to_ascii_uppercase();
            if upper.contains("OE1022") || upper.contains("SSI") {
                return Some(IdentifiedSerialDevice {
                    port: port_name.to_string(),
                    detected_kind: "oe1022d".to_string(),
                    serial_number: serial_number_from_idn(&idn),
                    idn: Some(idn),
                    confidence: "high".to_string(),
                    suggested_role: Some("oe1022d_main".to_string()),
                    status: "identified".to_string(),
                });
            }
        }
    }

    if wants("magnetic") || wants("maynuo") {
        if let Ok(idn) = serial_query(port_name, 9600, b"*IDN?\n", 200) {
            let serial_number = serial_number_from_idn(&idn);
            let suggested_role = match serial_number.as_deref() {
                Some("2020") => Some("maynuo.mag_x".to_string()),
                Some("2022") => Some("maynuo.mag_y".to_string()),
                Some("2003") => Some("maynuo.mag_z".to_string()),
                _ => None,
            };
            let upper = idn.to_ascii_uppercase();
            if upper.contains("MAYNUO") || upper.contains("M8812") || suggested_role.is_some() {
                return Some(IdentifiedSerialDevice {
                    port: port_name.to_string(),
                    detected_kind: "magnetic".to_string(),
                    idn: Some(idn),
                    serial_number,
                    confidence: if suggested_role.is_some() {
                        "high"
                    } else {
                        "medium"
                    }
                    .to_string(),
                    suggested_role,
                    status: "identified".to_string(),
                });
            }
        }
    }

    if wants("laser") || wants("cni_laser") {
        if serialport::new(port_name, 9600)
            .timeout(Duration::from_millis(250))
            .open()
            .is_ok()
        {
            return Some(IdentifiedSerialDevice {
                port: port_name.to_string(),
                detected_kind: "laser".to_string(),
                idn: Some("CNI Laser candidate (open-only probe; no reliable IDN)".to_string()),
                serial_number: None,
                confidence: "low".to_string(),
                suggested_role: Some("cni_laser".to_string()),
                status: "identified_low_confidence".to_string(),
            });
        }
    }

    None
}

fn tcp_query_idn(address: &str) -> Result<String, String> {
    let socket_addr = address
        .to_socket_addrs()
        .map_err(|e| format!("resolve {address}: {e}"))?
        .next()
        .ok_or_else(|| format!("resolve {address}: no socket address"))?;
    let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_millis(500))
        .map_err(|e| format!("TCP connect to {address}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_millis(700)))
        .map_err(|e| format!("set read timeout {address}: {e}"))?;
    stream
        .set_write_timeout(Some(Duration::from_millis(500)))
        .map_err(|e| format!("set write timeout {address}: {e}"))?;
    stream
        .write_all(b"*IDN?\n")
        .map_err(|e| format!("write *IDN? to {address}: {e}"))?;
    stream
        .flush()
        .map_err(|e| format!("flush {address}: {e}"))?;
    let mut buf = [0u8; 512];
    let n = stream
        .read(&mut buf)
        .map_err(|e| format!("read *IDN? from {address}: {e}"))?;
    Ok(String::from_utf8_lossy(&buf[..n]).trim().to_string())
}

fn discover_smb100a_tcp_targets_with_query<F>(
    targets: &[String],
    mut query: F,
) -> (Vec<DiscoveredDevice>, Vec<String>)
where
    F: FnMut(&str) -> Result<String, String>,
{
    let mut devices = Vec::new();
    let mut warnings = Vec::new();
    for target in targets {
        let target = target.trim();
        if target.is_empty() {
            continue;
        }
        match query(target) {
            Ok(idn) if is_smb100a_idn(&idn) => devices.push(DiscoveredDevice {
                transport: "tcp".to_string(),
                address: target.to_string(),
                detected_kind: "rf_source".to_string(),
                serial_number: smb100a_serial_from_idn(&idn),
                model: smb100a_model_from_idn(&idn),
                idn: Some(idn),
                confidence: "high".to_string(),
                suggested_role: Some("smb100a_main".to_string()),
                status: "identified".to_string(),
            }),
            Ok(idn) if idn.is_empty() => {
                warnings.push(format!("{target}: empty *IDN? response"));
            }
            Ok(idn) => warnings.push(format!("{target}: non-SMB100A *IDN? response: {idn}")),
            Err(err) => warnings.push(err),
        }
    }
    (devices, warnings)
}

fn default_smb100a_tcp_targets() -> Vec<String> {
    vec![
        "169.254.2.20:5025".to_string(),
        "192.168.1.20:5025".to_string(),
        "192.168.0.20:5025".to_string(),
    ]
}

fn normalized_tcp_targets(input: &[String]) -> Vec<String> {
    let mut targets = Vec::new();
    let defaults = default_smb100a_tcp_targets();
    for target in input.iter().chain(defaults.iter()) {
        let trimmed = target.trim();
        if trimmed.is_empty() || trimmed == "auto" {
            continue;
        }
        if !targets.iter().any(|existing| existing == trimmed) {
            targets.push(trimmed.to_string());
        }
    }
    targets
}

fn auto_bind_report_from_discovered_devices(
    devices: &[DiscoveredDevice],
    requested_roles: Vec<DeviceRoleRequest>,
) -> (AutoBindReport, HashMap<String, String>) {
    let mut bound = Vec::new();
    let mut blocked = Vec::new();
    let mut bindings: HashMap<String, String> = HashMap::new();

    for role in requested_roles {
        let found = devices.iter().find(|device| {
            if device.detected_kind != role.kind
                && !(role.kind == "magnetic" && device.detected_kind == "maynuo")
                && !(role.kind == "smb100a" && device.detected_kind == "rf_source")
                && !(role.kind == "rf_source" && device.detected_kind == "smb100a")
                && !(role.kind == "laser" && device.detected_kind == "cni_laser")
            {
                return false;
            }
            if let Some(expected_sn) = &role.expected_sn {
                return device.serial_number.as_deref() == Some(expected_sn.as_str());
            }
            device.suggested_role.as_deref() == Some(role.device_id.as_str())
                || role.expected_sn.is_none()
        });

        if let Some(device) = found {
            if device.confidence == "low" {
                blocked.push(format!(
                    "{} ({}) identified at {} with low confidence; confirm manually before binding",
                    role.device_id, role.kind, device.address
                ));
                continue;
            }
            bindings.insert(role.device_id.clone(), device.address.clone());
            bound.push(AutoBoundDevice {
                device_id: role.device_id,
                kind: role.kind,
                address: Some(device.address.clone()),
                idn: device.idn.clone(),
                serial_number: device.serial_number.clone(),
                confidence: device.confidence.clone(),
                status: "bound".to_string(),
            });
        } else {
            blocked.push(format!(
                "{} ({}) not identified{}",
                role.device_id,
                role.kind,
                role.expected_sn
                    .as_ref()
                    .map(|sn| format!(" with SN {sn}"))
                    .unwrap_or_default()
            ));
        }
    }

    (AutoBindReport { bound, blocked }, bindings)
}

fn auto_bind_report_from_identified_devices(
    devices: &[IdentifiedSerialDevice],
    requested_roles: Vec<DeviceRoleRequest>,
) -> (AutoBindReport, HashMap<String, String>) {
    let discovered: Vec<DiscoveredDevice> = devices
        .iter()
        .cloned()
        .map(discovered_from_serial)
        .collect();
    auto_bind_report_from_discovered_devices(&discovered, requested_roles)
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

#[tauri::command]
pub fn serial_list_ports() -> Result<Vec<SerialPortInfo>, String> {
    list_serial_ports_inner()
}

#[tauri::command]
pub fn serial_identify_devices(
    requested_kinds: Vec<String>,
) -> Result<SerialIdentifyReport, String> {
    let ports = list_serial_ports_inner()?;
    let mut devices = Vec::new();
    let mut warnings = Vec::new();
    for port in &ports {
        if !should_probe_serial_port(&port.port_name) {
            warnings.push(format!("{}: skipped non-lab serial port", port.port_name));
            continue;
        }
        match identify_serial_port(&port.port_name, &requested_kinds) {
            Some(device) => devices.push(device),
            None => warnings.push(format!(
                "{}: no supported device identified",
                port.port_name
            )),
        }
    }
    Ok(SerialIdentifyReport {
        ports,
        devices,
        warnings,
    })
}

#[tauri::command]
pub fn discover_devices(request: DeviceProbeRequest) -> Result<DeviceDiscoveryReport, String> {
    let wants = |kind: &str| {
        request.requested_kinds.is_empty() || request.requested_kinds.iter().any(|k| k == kind)
    };
    let mut devices = Vec::new();
    let mut warnings = Vec::new();

    let serial_ports = list_serial_ports_inner()?;
    if wants("oe1022d") || wants("lock_in") || wants("magnetic") || wants("laser") {
        for port in &serial_ports {
            if !should_probe_serial_port(&port.port_name) {
                warnings.push(format!("{}: skipped non-lab serial port", port.port_name));
                continue;
            }
            match identify_serial_port(&port.port_name, &request.requested_kinds) {
                Some(device) => devices.push(discovered_from_serial(device)),
                None => warnings.push(format!(
                    "{}: no supported serial device identified",
                    port.port_name
                )),
            }
        }
    }

    let tcp_targets = if wants("smb100a") || wants("rf_source") {
        normalized_tcp_targets(&request.smb100a_tcp_targets)
    } else {
        Vec::new()
    };
    if !tcp_targets.is_empty() {
        let (mut tcp_devices, mut tcp_warnings) =
            discover_smb100a_tcp_targets_with_query(&tcp_targets, tcp_query_idn);
        devices.append(&mut tcp_devices);
        warnings.append(&mut tcp_warnings);
    }

    let usb_resources = Vec::new();
    if request.enable_usb_probe && (wants("smb100a") || wants("rf_source")) {
        warnings.push(
            "SMB100A USB/VISA discovery is not enabled in this build; use TCP or manual address."
                .to_string(),
        );
    }

    Ok(DeviceDiscoveryReport {
        serial_ports,
        tcp_targets,
        usb_resources,
        devices,
        warnings,
    })
}

#[tauri::command]
pub fn auto_bind_identified_devices(
    state: tauri::State<WorkbenchState>,
    requested_roles: Vec<DeviceRoleRequest>,
) -> Result<AutoBindReport, String> {
    let kinds: Vec<String> = requested_roles
        .iter()
        .map(|role| role.kind.clone())
        .collect();
    let report = serial_identify_devices(kinds)?;
    let (bind_report, bindings) =
        auto_bind_report_from_identified_devices(&report.devices, requested_roles);

    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        for (device_id, address) in bindings {
            guard.auto_bound_addresses.insert(device_id, address);
        }
    }

    Ok(bind_report)
}

#[tauri::command]
pub fn auto_bind_discovered_devices(
    state: tauri::State<WorkbenchState>,
    requested_roles: Vec<DeviceRoleRequest>,
    discovery: Option<DeviceDiscoveryReport>,
) -> Result<AutoBindReport, String> {
    let discovery = match discovery {
        Some(report) => report,
        None => {
            let requested_kinds = requested_roles
                .iter()
                .map(|role| role.kind.clone())
                .collect();
            discover_devices(DeviceProbeRequest {
                requested_kinds,
                smb100a_tcp_targets: Vec::new(),
                enable_usb_probe: false,
            })?
        }
    };
    let (bind_report, bindings) =
        auto_bind_report_from_discovered_devices(&discovery.devices, requested_roles);

    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        for (device_id, address) in bindings {
            guard.auto_bound_addresses.insert(device_id, address);
        }
    }

    Ok(bind_report)
}

#[tauri::command]
pub fn connect_bound_devices(
    state: tauri::State<WorkbenchState>,
) -> Result<WorkbenchSnapshot, String> {
    let bindings = {
        let guard = state
            .inner
            .lock()
            .map_err(|e| format!("lock poison: {e}"))?;
        guard.auto_bound_addresses.clone()
    };
    for (device_id, address) in bindings {
        if address.starts_with("visa::") || address.starts_with("usbtmc::") {
            return Err(format!(
                "{device_id}: SMB100A USB/VISA identified but connect path not implemented"
            ));
        }
        let kind = if device_id.starts_with("maynuo.") {
            "magnetic"
        } else if device_id.contains("smb100a") {
            "smb100a"
        } else if device_id.contains("oe1022d") {
            "oe1022d"
        } else if device_id.contains("laser") {
            "laser"
        } else {
            continue;
        };
        connect_single_device(state.clone(), device_id, address, kind.to_string())?;
    }
    get_workbench_state(state)
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

    let mut profile_addresses: HashMap<String, String> = guard
        .profile
        .as_ref()
        .map(|p| {
            p.devices
                .iter()
                .map(|d| (d.device_id.clone(), d.address.clone()))
                .collect()
        })
        .unwrap_or_default();
    for (device_id, address) in &guard.auto_bound_addresses {
        profile_addresses.insert(device_id.clone(), address.clone());
    }
    for (device_id, address) in &guard.dynamic_addresses {
        profile_addresses.insert(device_id.clone(), address.clone());
    }

    Ok(WorkbenchSnapshot {
        profile_loaded: guard.profile.is_some(),
        profile_name: guard.profile.as_ref().map(|p| p.name.clone()),
        preflight_passed: batch_passed || !guard.single_device_connected.is_empty(),
        locks_held,
        report: guard.preflight_report.clone(),
        profile_addresses,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identified_magnetic(port: &str, serial_number: &str, role: &str) -> IdentifiedSerialDevice {
        IdentifiedSerialDevice {
            port: port.to_string(),
            detected_kind: "magnetic".to_string(),
            idn: Some(format!("MAYNUO,M8812,{serial_number}")),
            serial_number: Some(serial_number.to_string()),
            confidence: "high".to_string(),
            suggested_role: Some(role.to_string()),
            status: "identified".to_string(),
        }
    }

    fn discovered_smb(address: &str) -> DiscoveredDevice {
        DiscoveredDevice {
            transport: "tcp".to_string(),
            address: address.to_string(),
            detected_kind: "rf_source".to_string(),
            idn: Some("Rohde&Schwarz,SMB100A,123456,1.0".to_string()),
            serial_number: Some("123456".to_string()),
            model: Some("SMB100A".to_string()),
            confidence: "high".to_string(),
            suggested_role: Some("smb100a_main".to_string()),
            status: "identified".to_string(),
        }
    }

    #[test]
    fn discover_devices_filters_non_lab_serial_ports() {
        assert!(!should_probe_serial_port("/dev/cu.Bluetooth-Incoming-Port"));
        assert!(!should_probe_serial_port("/dev/cu.HUAWEIFreeBuds7i"));
        assert!(!should_probe_serial_port("/dev/cu.debug-console"));
        assert!(should_probe_serial_port("/dev/cu.PL2303G-USBtoUART13220"));
        assert!(should_probe_serial_port("/dev/cu.usbmodem3361358734371"));
        assert!(should_probe_serial_port("COM3"));
        assert!(should_probe_serial_port("/dev/ttyUSB0"));
        assert!(should_probe_serial_port("/dev/ttyACM0"));
    }

    #[test]
    fn discover_devices_smb100a_idn_parser_accepts_rohde_schwarz_response() {
        let idn = "Rohde&Schwarz,SMB100A,1412.4000K02/123456,3.1";
        assert!(is_smb100a_idn(idn));
        assert_eq!(smb100a_model_from_idn(idn).as_deref(), Some("SMB100A"));
        assert_eq!(smb100a_serial_from_idn(idn).as_deref(), Some("123456"));
    }

    #[test]
    fn discover_devices_tcp_probe_isolates_success_timeout_and_non_smb() {
        let targets = vec![
            "169.254.2.20:5025".to_string(),
            "192.168.1.20:5025".to_string(),
            "192.168.0.20:5025".to_string(),
            "10.0.0.5:5025".to_string(),
        ];
        let (devices, warnings) =
            discover_smb100a_tcp_targets_with_query(&targets, |target| match target {
                "169.254.2.20:5025" => Ok("Rohde&Schwarz,SMB100A,123456,1.0".to_string()),
                "192.168.1.20:5025" => Err("timeout".to_string()),
                "192.168.0.20:5025" => Ok("Other,Device,0000".to_string()),
                _ => Ok(String::new()),
            });

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].suggested_role.as_deref(), Some("smb100a_main"));
        assert_eq!(warnings.len(), 3);
    }

    #[test]
    fn auto_bind_binds_high_confidence_smb100a_to_main_role() {
        let devices = vec![discovered_smb("169.254.2.20:5025")];
        let roles = vec![DeviceRoleRequest {
            device_id: "smb100a_main".to_string(),
            kind: "rf_source".to_string(),
            expected_sn: None,
        }];

        let (report, bindings) = auto_bind_report_from_discovered_devices(&devices, roles);

        assert_eq!(report.bound.len(), 1);
        assert!(report.blocked.is_empty());
        assert_eq!(
            bindings.get("smb100a_main").map(String::as_str),
            Some("169.254.2.20:5025")
        );
    }

    #[test]
    fn auto_bind_matches_maynuo_axes_by_expected_sn() {
        let devices = vec![
            identified_magnetic("/dev/cu.mag-x", "2020", "maynuo.mag_x"),
            identified_magnetic("/dev/cu.mag-y", "2022", "maynuo.mag_y"),
            identified_magnetic("/dev/cu.mag-z", "2003", "maynuo.mag_z"),
        ];
        let roles = vec![
            DeviceRoleRequest {
                device_id: "maynuo.mag_x".to_string(),
                kind: "magnetic".to_string(),
                expected_sn: Some("2020".to_string()),
            },
            DeviceRoleRequest {
                device_id: "maynuo.mag_y".to_string(),
                kind: "magnetic".to_string(),
                expected_sn: Some("2022".to_string()),
            },
            DeviceRoleRequest {
                device_id: "maynuo.mag_z".to_string(),
                kind: "magnetic".to_string(),
                expected_sn: Some("2003".to_string()),
            },
        ];

        let (report, bindings) = auto_bind_report_from_identified_devices(&devices, roles);

        assert_eq!(report.bound.len(), 3);
        assert!(report.blocked.is_empty());
        assert_eq!(
            bindings.get("maynuo.mag_x").map(String::as_str),
            Some("/dev/cu.mag-x")
        );
        assert_eq!(
            bindings.get("maynuo.mag_y").map(String::as_str),
            Some("/dev/cu.mag-y")
        );
        assert_eq!(
            bindings.get("maynuo.mag_z").map(String::as_str),
            Some("/dev/cu.mag-z")
        );
    }

    #[test]
    fn auto_bind_blocks_wrong_maynuo_sn() {
        let devices = vec![identified_magnetic("/dev/cu.wrong", "9999", "maynuo.mag_x")];
        let roles = vec![DeviceRoleRequest {
            device_id: "maynuo.mag_x".to_string(),
            kind: "magnetic".to_string(),
            expected_sn: Some("2020".to_string()),
        }];

        let (report, bindings) = auto_bind_report_from_identified_devices(&devices, roles);

        assert!(report.bound.is_empty());
        assert!(bindings.is_empty());
        assert_eq!(report.blocked.len(), 1);
    }

    #[test]
    fn auto_bind_does_not_bind_low_confidence_laser_candidate() {
        let devices = vec![IdentifiedSerialDevice {
            port: "/dev/cu.any-open-port".to_string(),
            detected_kind: "laser".to_string(),
            idn: Some("CNI Laser candidate (open-only probe; no reliable IDN)".to_string()),
            serial_number: None,
            confidence: "low".to_string(),
            suggested_role: Some("cni_laser".to_string()),
            status: "identified_low_confidence".to_string(),
        }];
        let roles = vec![DeviceRoleRequest {
            device_id: "cni_laser".to_string(),
            kind: "laser".to_string(),
            expected_sn: None,
        }];

        let (report, bindings) = auto_bind_report_from_identified_devices(&devices, roles);

        assert!(report.bound.is_empty());
        assert!(bindings.is_empty());
        assert_eq!(report.blocked.len(), 1);
        assert!(report.blocked[0].contains("low confidence"));
    }
}
