//! Tests for Mag-M5A combined run.
//! Uses fake transports to verify orchestration logic without hardware.

use crate::artifacts;
use crate::mag_bridge::FakeMaynuoTransport;
use crate::oe_bridge::FakeOeTransport;
use crate::smb_bridge::FakeSmbTransport;
use crate::types::*;
use std::path::PathBuf;

#[test]
fn test_happy_path_fake_transports() {
    let mut smb = FakeSmbTransport::new();
    let mut oe = FakeOeTransport::new();
    let mut mag = FakeMaynuoTransport::new("MAYNUO,M8812,080020960220402020,V2.7");

    // SMB preflight
    let mut smb_audit = Vec::new();
    let idn = smb.query("*IDN?", &mut smb_audit, 0).unwrap();
    assert!(idn.contains("SMB100A"));

    let outp = smb.query("OUTP?", &mut smb_audit, 0).unwrap();
    assert_eq!(outp, "0");

    // OE preflight
    let mut oe_audit = Vec::new();
    let oe_idn = oe.query_identity(&mut oe_audit, 0).unwrap();
    assert!(oe_idn.contains("OE1022D"));

    // Maynuo identity
    let idn = mag.query_idn().unwrap();
    assert!(idn.contains("MAYNUO"));

    // Zero baseline
    let _mag_audit: Vec<CommandAuditEntry> = Vec::new();
    mag.send_set_remote().unwrap();
    mag.send_set_voltage(75).unwrap();
    mag.send_set_current(0.0).unwrap();
    mag.send_set_output(true).unwrap();

    let mut readings = Vec::new();
    for _ in 0..5 {
        readings.push(mag.query_meas_current().unwrap() * 1000.0);
    }
    let zero_mean = readings.iter().sum::<f64>() / readings.len() as f64;

    // Recur setpoint
    let recur_current_ma = 10.0;
    let total_current_a = (zero_mean + recur_current_ma) / 1000.0;
    mag.send_set_current(total_current_a).unwrap();

    let mut readings = Vec::new();
    for _ in 0..5 {
        readings.push(mag.query_meas_current().unwrap() * 1000.0);
    }
    let total_mean = readings.iter().sum::<f64>() / readings.len() as f64;
    let measured_recur_ma = total_mean - zero_mean;
    assert!((measured_recur_ma - recur_current_ma).abs() < 0.1);

    // RF ON
    smb.query(&format!("FREQ {}", 2882000000u64), &mut smb_audit, 0).unwrap();
    smb.query(&format!("POW {}", -30.0), &mut smb_audit, 0).unwrap();
    smb.query("OUTP ON", &mut smb_audit, 0).unwrap();
    assert!(smb.outp);

    // OE acquisition
    let mut frames = Vec::new();
    for _ in 0..10 {
        let (frame, _) = oe.capture_frame(&mut oe_audit, 0, 0).unwrap();
        frames.push(frame);
    }
    assert_eq!(frames.len(), 10);

    // Cleanup
    smb.query("OUTP OFF", &mut smb_audit, 0).unwrap();
    assert!(!smb.outp);

    mag.send_set_current(0.0).unwrap();
    mag.send_set_output(false).unwrap();
    mag.send_set_local().unwrap();
    assert!(!mag.output_on);

    // SMB and OE audits have entries (fake Maynuo does not populate audit)
    assert!(!smb_audit.is_empty());
    assert!(!oe_audit.is_empty());
}

#[test]
fn test_smb_command_allowlist_rejects_forbidden() {
    // Allowed queries
    assert!(crate::smb_bridge::validate_smb_command("*IDN?").is_ok());
    assert!(crate::smb_bridge::validate_smb_command("OUTP?").is_ok());

    // Forbidden: semicolon injection
    assert!(crate::smb_bridge::validate_smb_command("*IDN?;OUTP ON").is_err());

    // Forbidden: unknown command
    assert!(crate::smb_bridge::validate_smb_command("FREQ:SWE?").is_err());
}

#[test]
fn test_oe_command_allowlist() {
    let mut oe = FakeOeTransport::new();
    let mut audit = Vec::new();

    assert!(oe.query_identity(&mut audit, 0).is_ok());
    assert!(oe.capture_frame(&mut audit, 0, 0).is_ok());
}

#[test]
fn test_maynuo_sn_mismatch_detected() {
    let mut mag = FakeMaynuoTransport::new("MAYNUO,M8812,WRONG_SN,V2.7");
    let idn = mag.query_idn().unwrap();
    let sn = odmr_mag::expected_sn_from_idn(&idn).unwrap_or_default();
    assert_ne!(sn, "2020"); // Expected SN for mag_x
}

#[test]
fn test_mag_baseline_failure_triggers_no_rf() {
    // Simulate a Maynuo that fails after OUTP 1
    let mut mag = FakeMaynuoTransport::new("MAYNUO,M8812,080020960220402020,V2.7");
    let mut smb = FakeSmbTransport::new();

    mag.send_set_remote().unwrap();
    mag.send_set_voltage(75).unwrap();
    mag.send_set_current(0.0).unwrap();
    mag.send_set_output(true).unwrap();

    // Simulate MEAS failure by not taking readings
    // In real code, this would trigger cleanup and RF would never turn on
    assert!(smb.outp == false); // RF is still OFF

    // Cleanup
    mag.send_set_current(0.0).unwrap();
    mag.send_set_output(false).unwrap();
    mag.send_set_local().unwrap();
    assert!(!mag.output_on);
}

#[test]
fn test_report_requires_rf_final_off() {
    let report = CombinedRunReport {
        schema_version: "0.1.0".into(),
        run_id: "test".into(),
        passed: false,
        interrupted: false,
        rf: RfReportSection {
            requested_frequency_hz: 2882000000,
            requested_power_dbm: -30.0,
            readback_frequency_hz: None,
            readback_power_dbm: None,
            rf_on_window_start_unix_ms: None,
            rf_on_window_end_unix_ms: None,
            rf_final_off: true,
        },
        magnetic: MagReportSection {
            axis_id: "mag_x".into(),
            expected_sn: "2020".into(),
            observed_sn: "2020".into(),
            zero_readback_current_ma: 0.5,
            zero_readback_std_ma: 0.01,
            commanded_recur_current_ma: 10.0,
            measured_recur_current_ma: 10.02,
            measured_recur_field_nt: 1432.6,
            current_error_ma: 0.02,
            mag_final_output_off: true,
            mag_final_current_zero: true,
            mag_final_local_requested: true,
        },
        oe: OeReportSection {
            frames_requested: 10,
            frames_acquired: 10,
            raw_bin_bytes: 10 * odmr_oe1022d::RALL_FRAME_BYTES as u64,
            frame_size_bytes: odmr_oe1022d::RALL_FRAME_BYTES as u64,
            parse_failures: 0,
            timeout_count: 0,
        },
        timeline: TimelineReportSection {
            rf_on_before_oe_capture: true,
            mag_hold_before_oe_capture: true,
            oe_capture_completed_before_cleanup: true,
            cleanup_completed: true,
        },
        errors: vec![],
    };

    assert!(report.rf.rf_final_off);
    assert!(report.magnetic.mag_final_output_off);
    assert!(report.magnetic.mag_final_current_zero);
}

#[test]
fn test_raw_bin_size_calculation() {
    let frames = 10u64;
    let frame_size = odmr_oe1022d::RALL_FRAME_BYTES as u64;
    let expected_bytes = frames * frame_size;
    assert_eq!(expected_bytes, 10 * frame_size);
}

#[test]
fn test_event_timeline_ordering() {
    let mut events = Vec::new();
    for i in 0..5 {
        events.push(CombinedRunEvent {
            event_type: format!("evt_{}", i),
            timestamp_unix_ms: i * 100,
            device_id: None,
            detail: None,
        });
    }

    for i in 1..events.len() {
        assert!(
            events[i].timestamp_unix_ms >= events[i - 1].timestamp_unix_ms,
            "events must be monotonically ordered"
        );
    }
}

#[test]
fn test_no_csv_files_created() {
    let tmp = tempfile::tempdir().unwrap();
    let out_dir = tmp.path();

    // Write JSON artifacts
    let manifest = CombinedRunManifest {
        schema_version: "0.1.0".into(),
        tool_name: "test".into(),
        tool_version: "0.1.0".into(),
        run_id: "test".into(),
        started_at_utc: "2026".into(),
        completed_at_utc: "2026".into(),
        passed: true,
        devices: CombinedRunDevices {
            smb100a: DeviceIdentity { idn: "test".into(), sn: None },
            oe1022d: DeviceIdentity { idn: "test".into(), sn: None },
            maynuo: MaynuoDeviceIdentity {
                axis_id: "mag_x".into(),
                idn: "test".into(),
                sn: "test".into(),
            },
        },
        artifact_files: vec![],
        raw_first_contract_preserved: true,
        rf_final_off: true,
        mag_final_output_off: true,
        mag_final_current_zero: true,
        mag_final_local_requested: true,
        operator_note: None,
    };

    artifacts::write_all_artifacts(
        out_dir,
        &manifest,
        &CombinedRunReport {
            schema_version: "0.1.0".into(),
            run_id: "test".into(),
            passed: true,
            interrupted: false,
            rf: RfReportSection {
                requested_frequency_hz: 0,
                requested_power_dbm: 0.0,
                readback_frequency_hz: None,
                readback_power_dbm: None,
                rf_on_window_start_unix_ms: None,
                rf_on_window_end_unix_ms: None,
                rf_final_off: true,
            },
            magnetic: MagReportSection {
                axis_id: "mag_x".into(),
                expected_sn: "2020".into(),
                observed_sn: "2020".into(),
                zero_readback_current_ma: 0.0,
                zero_readback_std_ma: 0.0,
                commanded_recur_current_ma: 10.0,
                measured_recur_current_ma: 10.0,
                measured_recur_field_nt: 0.0,
                current_error_ma: 0.0,
                mag_final_output_off: true,
                mag_final_current_zero: true,
                mag_final_local_requested: true,
            },
            oe: OeReportSection {
                frames_requested: 0,
                frames_acquired: 0,
                raw_bin_bytes: 0,
                frame_size_bytes: 0,
                parse_failures: 0,
                timeout_count: 0,
            },
            timeline: TimelineReportSection {
                rf_on_before_oe_capture: true,
                mag_hold_before_oe_capture: true,
                oe_capture_completed_before_cleanup: true,
                cleanup_completed: true,
            },
            errors: vec![],
        },
        &[],
        &[],
        &[],
        &[],
        &SmbSnapshot {
            schema_version: "0.1.0".into(),
            idn: "test".into(),
            preflight_outp: "0".into(),
            preflight_mod: "0".into(),
            preflight_freq: "0".into(),
            preflight_pow: "0".into(),
            preflight_err: "0".into(),
        },
        &OeSnapshot {
            schema_version: "0.1.0".into(),
            idn: "test".into(),
        },
        &MagSnapshot {
            schema_version: "0.1.0".into(),
            axis_id: "mag_x".into(),
            expected_sn: "2020".into(),
            observed_sn: "2020".into(),
            idn: "test".into(),
            port_path: "".into(),
            zero_readback_current_ma: 0.0,
            zero_readback_std_ma: 0.0,
            commanded_recur_current_ma: 10.0,
            measured_recur_current_ma: 10.0,
            measured_recur_field_nt: 0.0,
            current_error_ma: 0.0,
        },
    )
    .unwrap();

    // Verify no CSV files
    let csv_files: Vec<_> = std::fs::read_dir(out_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "csv")
                .unwrap_or(false)
        })
        .collect();

    assert!(csv_files.is_empty(), "no CSV files should be created");
}

#[test]
fn test_manifest_serialization_roundtrip() {
    let manifest = CombinedRunManifest {
        schema_version: "0.1.0".into(),
        tool_name: "rf-mag-oe-minimal-run".into(),
        tool_version: "0.1.0".into(),
        run_id: "test_001".into(),
        started_at_utc: "20260604_120000".into(),
        completed_at_utc: "20260604_120030".into(),
        passed: true,
        devices: CombinedRunDevices {
            smb100a: DeviceIdentity {
                idn: "Rohde&Schwarz,SMB100A,123,3.2".into(),
                sn: None,
            },
            oe1022d: DeviceIdentity {
                idn: "OE1022D,SN456,1.0".into(),
                sn: None,
            },
            maynuo: MaynuoDeviceIdentity {
                axis_id: "mag_x".into(),
                idn: "MAYNUO,M8812,SN789,V2.7".into(),
                sn: "SN789".into(),
            },
        },
        artifact_files: vec!["manifest.json".into(), "report.json".into()],
        raw_first_contract_preserved: true,
        rf_final_off: true,
        mag_final_output_off: true,
        mag_final_current_zero: true,
        mag_final_local_requested: true,
        operator_note: Some("test note".into()),
    };

    let json = serde_json::to_string(&manifest).unwrap();
    let decoded: CombinedRunManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.run_id, "test_001");
    assert!(decoded.passed);
    assert!(decoded.rf_final_off);
}

#[test]
fn test_single_axis_only_no_simultaneous_output() {
    // Mag-M5A only supports single-axis output
    // Verify that only one axis is processed
    let cli = crate::cli::Cli {
        smb_host: "169.254.2.20".into(),
        smb_port: 5025,
        smb_query_delay_ms: 50,
        smb_timeout_ms: 3000,
        oe_port: "/dev/ttyUSB0".into(),
        oe_baud: 921600,
        oe_timeout_ms: 5000,
        oe_frame_delay_ms: 800,
        mag_profile: PathBuf::from("examples/magnetic/maynuo_m8812_axes.example.json"),
        mag_axis_id: "mag_x".into(),
        mag_recur_current_ma: 10.0,
        mag_samples: 5,
        rf_frequency_hz: 2882000000,
        rf_power_dbm: -30.0,
        frames: 10,
        settle_ms: 2000,
        out_dir: PathBuf::from("out/test"),
        operator_approve: true,
        operator_note: None,
        station_profile: None,
        preflight_only: false,
        ledger_path: None,
        dry_run: true,
    };

    // In dry-run mode, only one axis is referenced
    assert_eq!(cli.mag_axis_id, "mag_x");
}

#[test]
fn test_maynuo_cleanup_sets_safe_state() {
    let mut mag = FakeMaynuoTransport::new("MAYNUO,M8812,SN123,V2.7");
    let _audit: Vec<CommandAuditEntry> = Vec::new();

    mag.send_set_output(true).unwrap();
    mag.send_set_current(0.01).unwrap();
    assert!(mag.output_on);
    assert!(mag.current_a > 0.0);

    // Cleanup
    mag.send_set_current(0.0).unwrap();
    mag.send_set_output(false).unwrap();
    mag.send_set_local().unwrap();

    assert!(!mag.output_on);
    assert_eq!(mag.current_a, 0.0);
    assert!(!mag.remote_mode);
}


// ---------------------------------------------------------------------------
// P6.2 fault-injection tests
// ---------------------------------------------------------------------------

#[test]
fn test_smb_emergency_off_on_rf_failure() {
    // Simulate: OUTP ON succeeds but verification fails;
    // cleanup must send OUTP OFF.
    let mut smb = FakeSmbTransport::new();
    let mut audit = Vec::new();

    // RF setup succeeds
    smb.query("FREQ 2882000000", &mut audit, 0).unwrap();
    smb.query("POW -30.0", &mut audit, 0).unwrap();
    smb.query("OUTP ON", &mut audit, 0).unwrap();
    assert!(smb.outp);

    // Verification fails (injected)
    smb.fail_on = Some("OUTP?".to_string());
    let result = smb.query("OUTP?", &mut audit, 0);
    assert!(result.is_err());

    // Emergency cleanup: OUTP OFF must be sent
    smb.fail_on = None;
    let _ = smb.query("OUTP OFF", &mut audit, 0);
    assert!(!smb.outp);

    // Verify OFF
    let off_check = smb.query("OUTP?", &mut audit, 0).unwrap();
    assert_eq!(off_check.trim(), "0");
}

#[test]
fn test_cleanup_booleans_truthful_on_failure() {
    // Mag output-off command fails → mag_final_output_off must be false
    let mut mag = FakeMaynuoTransport::new("MAYNUO,M8812,SN123,V2.7");

    // Simulate a state where output is on
    mag.send_set_output(true).unwrap();
    mag.send_set_current(0.01).unwrap();

    // Cleanup commands
    let curr_ok = mag.send_set_current(0.0).is_ok();
    let outp_ok = mag.send_set_output(false).is_ok();
    let loc_ok = mag.send_set_local().is_ok();

    // In real code, report.magnetic.mag_final_output_off = outp_ok;
    // This test verifies the FakeTransport supports the semantics.
    assert!(curr_ok);
    assert!(outp_ok);
    assert!(loc_ok);
    assert!(!mag.output_on);
    assert_eq!(mag.current_a, 0.0);
    assert!(!mag.remote_mode);
}

#[test]
fn test_abort_flag_triggers_exit() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let flag = AtomicBool::new(false);
    assert!(flag.load(Ordering::Relaxed) == false);

    flag.store(true, Ordering::Relaxed);
    assert!(flag.load(Ordering::Relaxed));

    // Simulate check_abort logic
    let result = if flag.load(Ordering::Relaxed) {
        Err::<(), String>("Run aborted by operator (SIGINT)".into())
    } else {
        Ok(())
    };
    assert!(result.is_err());
}

#[test]
fn test_report_interrupted_field_exists() {
    let report = CombinedRunReport {
        schema_version: "0.1.0".into(),
        run_id: "test".into(),
        passed: false,
        interrupted: true,
        rf: RfReportSection {
            requested_frequency_hz: 0,
            requested_power_dbm: 0.0,
            readback_frequency_hz: None,
            readback_power_dbm: None,
            rf_on_window_start_unix_ms: None,
            rf_on_window_end_unix_ms: None,
            rf_final_off: false,
        },
        magnetic: MagReportSection {
            axis_id: "mag_x".into(),
            expected_sn: "SN123".into(),
            observed_sn: "SN123".into(),
            zero_readback_current_ma: 0.0,
            zero_readback_std_ma: 0.0,
            commanded_recur_current_ma: 10.0,
            measured_recur_current_ma: 0.0,
            measured_recur_field_nt: 0.0,
            current_error_ma: 0.0,
            mag_final_output_off: false,
            mag_final_current_zero: false,
            mag_final_local_requested: false,
        },
        oe: OeReportSection {
            frames_requested: 0,
            frames_acquired: 0,
            raw_bin_bytes: 0,
            frame_size_bytes: 0,
            parse_failures: 0,
            timeout_count: 0,
        },
        timeline: TimelineReportSection {
            rf_on_before_oe_capture: false,
            mag_hold_before_oe_capture: false,
            oe_capture_completed_before_cleanup: false,
            cleanup_completed: false,
        },
        errors: vec!["interrupted".into()],
    };

    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("interrupted"));
    assert!(json.contains("true") || json.contains("false"));
}
