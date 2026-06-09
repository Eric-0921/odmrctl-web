# Config Compatibility Mapping

## Goal

`odmr-config` is now the canonical JSON configuration entrypoint for station/runtime loading. Legacy station profile JSON is still accepted, but it is normalized into the canonical model before preflight, executor runtime, replay defaults, and Tauri workbench defaults consume it.

Manual sources locked into defaults:

- `docs/equipment_manual/smb100a/05_remote_control_basics.md`
- `docs/equipment_manual/oe1022d/05_oe1022d_rall_global_data_config_reading.md`
- `docs/equipment_manual/maynuo_dc-power-supply/m8812_remote_control_reference.md`
- `docs/equipment_manual/CNI Laser psu-sr/RS232语言协议_恒功率.md`

## Old To New Mapping

| Legacy field | Canonical field | Notes |
|---|---|---|
| `name` | `StationConfig.name` / `station_id` | Legacy `name` is preserved as both display name and fallback ID. |
| `devices[].device_id` | `devices[].device_id` | Preserved as-is; validation normalizes `.` only for internal ID checks. |
| `devices[].kind = rf_source` | `devices[].device_type = smb100a` | Normalized by loader. |
| `devices[].kind = lock_in` | `devices[].device_type = oe1022d` | Normalized by loader. |
| `devices[].kind = magnetic` / `maynuo` | `devices[].device_type = magnet_xyz` | Normalized by loader. |
| `devices[].kind = laser` / `cni` / `cni_laser` | `devices[].device_type = laser` | Normalized by loader. |
| `devices[].transport = tcp` | `devices[].transport.kind = tcp_scpi` | SMB100A default port fixed to `5025`. |
| `devices[].address = host:port` | `devices[].transport.host` + `port` | Port falls back to `5025` when omitted. |
| `devices[].transport = serial` | `devices[].transport.kind = serial` | Per-device manual defaults fill baud/parity/DTR/line terminator. |
| `devices[].expected_sn` | `devices[].identity.expected_sn` | Also appended to `expected_contains` for Maynuo matching. |
| `devices[].timeout_ms` | `devices[].transport.timeout_ms` | Preserved. |
| `safety.smb100a_max_power_dbm` | `StationConfig.safety.smb100a_max_power_dbm` | Now shared by Tauri and executor. |
| `safety.smb100a_min_freq_hz` | `StationConfig.safety.smb100a_min_freq_hz` | Canonical. |
| `safety.smb100a_max_freq_hz` | `StationConfig.safety.smb100a_max_freq_hz` | Canonical. |
| `safety.mag_max_current_a_per_axis` | `StationConfig.safety.mag_max_current_a_per_axis` | Canonical. |
| `safety.laser_max_power_mw` | `StationConfig.safety.laser_max_power_mw` | Hard-capped by manual-derived `150 mW`. |

## Manual-Derived Defaults

### SMB100A

- Transport: raw TCP SCPI socket
- Default port: `5025`
- Cleanup: `OUTP OFF`, `MOD:STAT OFF`, `FM:STAT OFF`

### OE1022D

- RALL frame size: `12288 B`
- Samples per frame: `50`
- Refresh cadence: `50 ms` nominal
- Canonical acquisition transport: USB serial RALL path

### Maynuo M8812

- Serial: `9600 8N1`
- Line terminator: `LF`
- Session control: `SYST:REM` / `SYST:LOC`
- Runtime commands fixed into defaults: `CURR`, `OUTP`, `MEAS:CURR?`

### CNI Laser

- Serial: `9600 8N1`
- `laser_off`: `55 AA 03 00 03`
- `laser_on`: `55 AA 03 01 04`
- Power frame: `55 AA 05 01 <hi> <lo> <checksum>`
- Cleanup: emergency off path is mandatory

## Runtime Consumers Now Routed Through `odmr-config`

- `odmr-preflight::StationProfile::load(...)`
- Tauri `load_station_safety(...)`
- Tauri hardware run adapter (`ExperimentPlan` → `HardwareRunConfig`)
- Replay defaults and canonical/legacy migration entrypoints

## Remaining Follow-up

- Promote `AppConfig` persistence into a first-class checked file under the desktop app runtime root.
- Replace remaining ad hoc station snapshot JSON emitters with canonical `StationConfig` serialization.
- Move GUI replay transport from raw `serde_json::Value` responses to dedicated frontend types.
