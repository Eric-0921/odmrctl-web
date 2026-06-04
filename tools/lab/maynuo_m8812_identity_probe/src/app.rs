//! Core application logic: probe, classify, match, and report.

use crate::cli::CliArgs;
use crate::types::{
    AxisMapping, AxisMappingEntry, IdentitySnapshot, PortProbeResult, ProbeClassification,
    ProbeEvent, ProbeManifest, ProbeReport,
};
use odmr_mag::{expected_sn_from_idn, parse_maynuo_idn, MaynuoAxesProfile, MaynuoAxisProfile};
use odmr_maynuo_m8812::{
    MaynuoM8812Transport, MaynuoPortMetadata, MaynuoProbeError, MaynuoSerialPortConfig,
};
use odmr_types::DeviceId;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Load a MaynuoAxesProfile from a JSON file path.
pub fn load_profile(path: &Path) -> Result<MaynuoAxesProfile, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("read profile {}: {e}", path.display()))?;
    serde_json::from_str(&content).map_err(|e| format!("parse profile: {e}"))
}

/// Run the full identity probe workflow.
pub fn run(args: &CliArgs) -> Result<(), String> {
    let started_at = chrono_like_now();
    let profile_path = canonicalize_profile_path(&args.profile)?;
    let profile = load_profile(&profile_path)?;

    let all_ports = MaynuoM8812Transport::enumerate_ports()
        .map_err(|e| format!("enumerate ports: {e}"))?;
    let candidates = filter_ports(&all_ports, &args.include_port, &args.exclude_port, args.max_ports);

    let mut events: Vec<ProbeEvent> = Vec::new();
    events.push(event("probe_started", Some(format!("ports_candidates={}", candidates.len()))));

    let mut results: Vec<PortProbeResult> = Vec::new();
    let config = MaynuoSerialPortConfig {
        baudrate: args.baudrate,
        read_timeout_ms: args.timeout_ms,
        ..Default::default()
    };

    for meta in &candidates {
        if args.dry_run {
            results.push(PortProbeResult {
                port_path: meta.port_path.clone(),
                usb_serial_number: meta.usb_serial_number.clone(),
                probe_attempted: false,
                idn_raw: None,
                parsed: None,
                classification: ProbeClassification::NonTargetDevice,
                matched_axis_id: None,
                error: Some("dry-run: port not opened".into()),
            });
            events.push(event("port_skipped_dry_run", Some(meta.port_path.clone())));
        } else {
            let result = probe_one(&meta, &config);
            events.push(event(
                "port_probed",
                Some(format!(
                    "port={} classification={}",
                    meta.port_path,
                    result.classification.as_str()
                )),
            ));
            results.push(result);
        }
    }

    let (mapping, duplicate_sn, unknown_sn, missing_axes) =
        build_axis_mapping(&profile, &results);

    let ports_responded = results
        .iter()
        .filter(|r| r.probe_attempted && r.idn_raw.is_some())
        .count();
    let ports_matched = results
        .iter()
        .filter(|r| r.classification == ProbeClassification::MatchedAxis)
        .count();

    let strict_fail = args.strict && !unknown_sn.is_empty();
    let passed = missing_axes.is_empty() && duplicate_sn.is_empty() && !strict_fail;

    let report = ProbeReport {
        passed,
        missing_axes,
        duplicate_axes: duplicate_sn,
        unknown_sn,
        ports_scanned: candidates.len(),
        ports_responded,
        ports_matched,
        strict_mode: args.strict,
    };

    let snapshot = IdentitySnapshot {
        schema_version: "0.1.0".into(),
        observed_ports: results,
    };

    let completed_at = chrono_like_now();

    let manifest = ProbeManifest {
        schema_version: "0.1.0".into(),
        tool_name: "maynuo-m8812-identity-probe".into(),
        tool_version: "0.1.0".into(),
        started_at_utc: started_at.clone(),
        completed_at_utc: completed_at,
        profile_path: profile_path.display().to_string(),
        passed,
        artifact_files: vec![
            "manifest.json".into(),
            "maynuo_identity_snapshot.json".into(),
            "maynuo_identity_events.jsonl".into(),
            "maynuo_axis_mapping.json".into(),
            "maynuo_probe_report.json".into(),
        ],
        only_idn_queries_sent: true,
        no_current_commands_sent: true,
        no_output_commands_sent: true,
    };

    events.push(event(
        if passed { "probe_passed" } else { "probe_failed" },
        Some(format!("passed={passed}")),
    ));

    crate::artifacts::write_artifacts(&args.out_dir, &manifest, &snapshot, &mapping, &report, &events)?;

    eprintln!("Probe complete. passed={passed}. Artifacts written to {}", args.out_dir.display());
    Ok(())
}

/// Probe a single port: open, send *IDN?, parse, classify.
fn probe_one(meta: &MaynuoPortMetadata, config: &MaynuoSerialPortConfig) -> PortProbeResult {
    let device_id = DeviceId::new(format!("probe-{}", sanitize_id(&meta.port_path)));
    let mut transport = match MaynuoM8812Transport::open(device_id, &meta.port_path, config.clone()) {
        Ok(t) => t,
        Err(e) => {
            return PortProbeResult {
                port_path: meta.port_path.clone(),
                usb_serial_number: meta.usb_serial_number.clone(),
                probe_attempted: true,
                idn_raw: None,
                parsed: None,
                classification: ProbeClassification::IoError,
                matched_axis_id: None,
                error: Some(e.to_string()),
            };
        }
    };

    let idn_raw = match transport.query_idn() {
        Ok(s) => s,
        Err(e) => {
            let class = match &e {
                MaynuoProbeError::Timeout { .. } => ProbeClassification::Timeout,
                MaynuoProbeError::EmptyResponse { .. } => ProbeClassification::MalformedIdn,
                MaynuoProbeError::NonAsciiResponse { .. } => ProbeClassification::MalformedIdn,
                _ => ProbeClassification::IoError,
            };
            return PortProbeResult {
                port_path: meta.port_path.clone(),
                usb_serial_number: meta.usb_serial_number.clone(),
                probe_attempted: true,
                idn_raw: None,
                parsed: None,
                classification: class,
                matched_axis_id: None,
                error: Some(e.to_string()),
            };
        }
    };

    let parsed = match parse_maynuo_idn(&idn_raw) {
        Ok(p) => p,
        Err(e) => {
            return PortProbeResult {
                port_path: meta.port_path.clone(),
                usb_serial_number: meta.usb_serial_number.clone(),
                probe_attempted: true,
                idn_raw: Some(idn_raw),
                parsed: None,
                classification: ProbeClassification::MalformedIdn,
                matched_axis_id: None,
                error: Some(e.to_string()),
            };
        }
    };

    PortProbeResult {
        port_path: meta.port_path.clone(),
        usb_serial_number: meta.usb_serial_number.clone(),
        probe_attempted: true,
        idn_raw: Some(idn_raw),
        parsed: Some(parsed),
        classification: ProbeClassification::NonTargetDevice,
        matched_axis_id: None,
        error: None,
    }
}

/// Match probe results to logical axes by exact SN equality.
fn build_axis_mapping(
    profile: &MaynuoAxesProfile,
    results: &[PortProbeResult],
) -> (AxisMapping, Vec<String>, Vec<String>, Vec<String>) {
    let axes = [&profile.axes.x, &profile.axes.y, &profile.axes.z];
    let mut matched: BTreeMap<String, &PortProbeResult> = BTreeMap::new();
    let mut duplicate_sn: Vec<String> = Vec::new();
    let mut unknown_sn: Vec<String> = Vec::new();
    let mut missing_axes: Vec<String> = Vec::new();

    // First pass: match by exact SN
    for result in results {
        if result.classification == ProbeClassification::IoError
            || result.classification == ProbeClassification::Timeout
            || result.classification == ProbeClassification::MalformedIdn
        {
            continue;
        }
        let parsed = match &result.parsed {
            Some(p) => p,
            None => continue,
        };
        let sn = &parsed.serial_number;
        let mut found = false;
        for axis in &axes {
            let expected_sn = match expected_sn_from_idn(&axis.expected_idn) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if &expected_sn == sn {
                if matched.contains_key(&axis.axis_id) {
                    duplicate_sn.push(sn.clone());
                } else {
                    matched.insert(axis.axis_id.clone(), result);
                }
                found = true;
                break;
            }
        }
        if !found {
            unknown_sn.push(sn.clone());
        }
    }

    // Check for missing axes
    for axis in &axes {
        if !matched.contains_key(&axis.axis_id) {
            missing_axes.push(axis.axis_id.clone());
        }
    }

    let entry = |axis: &MaynuoAxisProfile| -> AxisMappingEntry {
        let expected_sn = expected_sn_from_idn(&axis.expected_idn).unwrap_or_default();
        if let Some(r) = matched.get(&axis.axis_id) {
            AxisMappingEntry {
                axis_id: axis.axis_id.clone(),
                expected_sn,
                observed_sn: r.parsed.as_ref().map(|p| p.serial_number.clone()),
                observed_idn: r.idn_raw.clone(),
                observed_port_path: Some(r.port_path.clone()),
                matched: true,
            }
        } else {
            AxisMappingEntry {
                axis_id: axis.axis_id.clone(),
                expected_sn,
                observed_sn: None,
                observed_idn: None,
                observed_port_path: None,
                matched: false,
            }
        }
    };

    let mapping = AxisMapping {
        x: entry(&profile.axes.x),
        y: entry(&profile.axes.y),
        z: entry(&profile.axes.z),
    };
    (mapping, duplicate_sn, unknown_sn, missing_axes)
}

fn filter_ports(
    ports: &[MaynuoPortMetadata],
    include: &[String],
    exclude: &[String],
    max_ports: Option<usize>,
) -> Vec<MaynuoPortMetadata> {
    let mut filtered: Vec<MaynuoPortMetadata> = ports
        .iter()
        .filter(|p| {
            if !include.is_empty() && !include.contains(&p.port_path) {
                return false;
            }
            if exclude.contains(&p.port_path) {
                return false;
            }
            true
        })
        .cloned()
        .collect();

    if let Some(max) = max_ports {
        filtered.truncate(max);
    }
    filtered
}

fn sanitize_id(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

fn event(event_type: &str, detail: Option<String>) -> ProbeEvent {
    ProbeEvent {
        event_type: event_type.into(),
        timestamp: chrono_like_now(),
        detail,
    }
}

fn chrono_like_now() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;

    // civil_from_days algorithm (Howard Hinnant), adapted for non-negative z.
    let z = days_since_epoch + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, m, s)
}

fn canonicalize_profile_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .ok_or("cannot determine workspace root")?;
        Ok(workspace_root.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use odmr_mag::{MaynuoAxes, MaynuoSerialSettings};
    use odmr_maynuo_m8812::MaynuoPortMetadata;

    fn example_profile() -> MaynuoAxesProfile {
        MaynuoAxesProfile {
            schema_version: "0.2.0".into(),
            kind: "maynuo_axes_profile".into(),
            id: "test".into(),
            name: Some("test".into()),
            description: None,
            serial_settings: MaynuoSerialSettings::default(),
            axes: MaynuoAxes {
                x: axis("mag_x", "2020", "MAYNUO,M8812,080020960220402020,V2.7", "COM4", 143.26),
                y: axis("mag_y", "2022", "MAYNUO,M8812,080020960220402022,V2.7", "COM6", 141.77),
                z: axis("mag_z", "2003", "MAYNUO,M8812,080020960220402003,V2.7", "COM3", 156.15),
            },
            safety_policy_id: "default".into(),
            calibration_date: "2026-01-01".into(),
            verified: true,
            verified_by: None,
            source: None,
            verification: None,
            note: None,
        }
    }

    fn axis(
        axis_id: &str,
        sn_tail: &str,
        expected_idn: &str,
        port: &str,
        coil_constant_nt_per_ma: f64,
    ) -> MaynuoAxisProfile {
        MaynuoAxisProfile {
            axis_id: axis_id.into(),
            display_name: None,
            last_known_port_name: port.into(),
            device_model: "MAYNUO M8812".into(),
            sn_tail: sn_tail.into(),
            expected_idn: expected_idn.into(),
            coil_constant_nt_per_ma,
            gain_t_per_a: coil_constant_nt_per_ma * 1e-6,
            zero_offset_ma: 0.0,
            zero_offset_a: 0.0,
            output_default: false,
            max_current_ma: 5000.0,
            max_current_a: 5.0,
            voltage_v: 75,
        }
    }

    fn result_with_idn(port: &str, idn: &str) -> PortProbeResult {
        let parsed = parse_maynuo_idn(idn).ok();
        PortProbeResult {
            port_path: port.into(),
            usb_serial_number: None,
            probe_attempted: true,
            idn_raw: Some(idn.into()),
            parsed,
            classification: ProbeClassification::NonTargetDevice,
            matched_axis_id: None,
            error: None,
        }
    }

    #[test]
    fn full_xyz_mapping_success() {
        let profile = example_profile();
        let results = vec![
            result_with_idn("COM4", "MAYNUO,M8812,080020960220402020,V2.7"),
            result_with_idn("COM6", "MAYNUO,M8812,080020960220402022,V2.7"),
            result_with_idn("COM3", "MAYNUO,M8812,080020960220402003,V2.7"),
        ];
        let (mapping, dup, unk, missing) = build_axis_mapping(&profile, &results);
        assert!(dup.is_empty());
        assert!(unk.is_empty());
        assert!(missing.is_empty());
        assert!(mapping.x.matched);
        assert!(mapping.y.matched);
        assert!(mapping.z.matched);
    }

    #[test]
    fn missing_axis_detected() {
        let profile = example_profile();
        let results = vec![
            result_with_idn("COM4", "MAYNUO,M8812,080020960220402020,V2.7"),
        ];
        let (_mapping, _dup, _unk, missing) = build_axis_mapping(&profile, &results);
        assert!(missing.contains(&"mag_y".into()));
        assert!(missing.contains(&"mag_z".into()));
    }

    #[test]
    fn unknown_sn_detected() {
        let profile = example_profile();
        let results = vec![
            result_with_idn("COM4", "MAYNUO,M8812,999999999999999999,V2.7"),
        ];
        let (_mapping, _dup, unk, _missing) = build_axis_mapping(&profile, &results);
        assert_eq!(unk, vec!["999999999999999999"]);
    }

    #[test]
    fn duplicate_sn_detected() {
        let profile = example_profile();
        let results = vec![
            result_with_idn("COM4", "MAYNUO,M8812,080020960220402020,V2.7"),
            result_with_idn("COM5", "MAYNUO,M8812,080020960220402020,V2.7"),
        ];
        let (_mapping, dup, _unk, _missing) = build_axis_mapping(&profile, &results);
        assert!(!dup.is_empty());
    }

    #[test]
    fn malformed_idn_classified() {
        let profile = example_profile();
        let results = vec![
            PortProbeResult {
                port_path: "COM4".into(),
                usb_serial_number: None,
                probe_attempted: true,
                idn_raw: Some("garbage".into()),
                parsed: None,
                classification: ProbeClassification::MalformedIdn,
                matched_axis_id: None,
                error: Some("malformed".into()),
            },
        ];
        let (_mapping, _dup, unk, missing) = build_axis_mapping(&profile, &results);
        assert!(unk.is_empty()); // malformed not counted as unknown SN
        assert!(!missing.is_empty()); // no valid results → all axes missing
    }

    #[test]
    fn port_filtering_include_empty() {
        let ports = vec![
            MaynuoPortMetadata {
                port_path: "COM1".into(),
                port_type: None,
                usb_serial_number: None,
                usb_vid: None,
                usb_pid: None,
                manufacturer: None,
                product: None,
            },
        ];
        let filtered = filter_ports(&ports, &[], &[], None);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn port_filtering_exclude() {
        let ports = vec![
            MaynuoPortMetadata {
                port_path: "COM1".into(),
                port_type: None,
                usb_serial_number: None,
                usb_vid: None,
                usb_pid: None,
                manufacturer: None,
                product: None,
            },
            MaynuoPortMetadata {
                port_path: "COM2".into(),
                port_type: None,
                usb_serial_number: None,
                usb_vid: None,
                usb_pid: None,
                manufacturer: None,
                product: None,
            },
        ];
        let filtered = filter_ports(&ports, &[], &["COM2".into()], None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].port_path, "COM1");
    }

    #[test]
    fn port_filtering_max_ports() {
        let ports: Vec<MaynuoPortMetadata> = (0..5)
            .map(|i| MaynuoPortMetadata {
                port_path: format!("COM{i}"),
                port_type: None,
                usb_serial_number: None,
                usb_vid: None,
                usb_pid: None,
                manufacturer: None,
                product: None,
            })
            .collect();
        let filtered = filter_ports(&ports, &[], &[], Some(2));
        assert_eq!(filtered.len(), 2);
    }
}
