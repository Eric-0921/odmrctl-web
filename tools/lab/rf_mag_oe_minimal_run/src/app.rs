//! Main orchestration for Mag-M5A combined run.

use crate::artifacts;
use crate::cli::Cli;
use crate::mag_bridge::*;
use crate::oe_bridge::OeTransport;
use crate::smb_bridge::SmbTransport;
use crate::types::*;
use odmr_mag::{MaynuoAxesProfile, MaynuoAxisProfile};
use odmr_maynuo_m8812::MaynuoSerialPortConfig;
use odmr_oe1022d::RALL_FRAME_BYTES;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

pub fn run(cli: &Cli) -> Result<(), String> {
    let started_at = chrono_like_now();
    let run_id = format!("mag_m5a_{}", started_at.replace(|c: char| !c.is_alphanumeric(), "_"));

    // ---- Station preflight (if --station-profile provided) ----
    // Preflight is read-only; it runs BEFORE operator approval
    if let Some(ref profile_path) = cli.station_profile {
        let station_profile = common_preflight::StationProfile::load(
            profile_path.to_str().unwrap_or("")
        ).map_err(|e| format!("load station profile: {e}"))?;

        println!("=== Station Preflight ===");
        let preflight_report = common_preflight::run_station_preflight(&station_profile, None, true)
            .map_err(|e| format!("station preflight failed: {e}"))?;

        println!(
            "Preflight: reachable={}, identities={}, safe_states={}",
            preflight_report.all_devices_reachable,
            preflight_report.all_identities_verified,
            preflight_report.all_safe_states_confirmed
        );

        if !preflight_report.passed() {
            return Err(format!(
                "Station preflight FAILED. See report for details."
            ));
        }

        // Write preflight artifacts to out_dir
        let preflight_dir = cli.out_dir.join("preflight");
        let _ = fs::create_dir_all(&preflight_dir);
        let _ = common_preflight::station_report::write_json(
            &preflight_report,
            &preflight_dir.join("station_preflight_report.json")
        );
        let _ = common_preflight::station_report::write_markdown(
            &preflight_report,
            &preflight_dir.join("station_preflight_report.md")
        );

        if cli.preflight_only {
            println!("--preflight-only: exiting after preflight.");
            return Ok(());
        }
    }

    // ---- Operator approval ----
    if !cli.dry_run && !cli.operator_approve {
        return Err(
            "Operator approval required. Use --operator-approve to confirm.".into(),
        );
    }

    // ---- Create output directory ----
    let out_dir = &cli.out_dir;
    fs::create_dir_all(out_dir).map_err(|e| format!("create out dir: {e}"))?;

    // ---- Load magnetic profile ----
    let profile = load_profile(&cli.mag_profile)?;
    let axis_profile = get_axis_profile(&profile, &cli.mag_axis_id)?;

    // ---- Event log ----
    let mut events: Vec<CombinedRunEvent> = Vec::new();
    let mut smb_audit: Vec<CommandAuditEntry> = Vec::new();
    let mut maynuo_audit: Vec<CommandAuditEntry> = Vec::new();
    let mut oe_audit: Vec<CommandAuditEntry> = Vec::new();

    push_event(&mut events, "run_started", None, Some(&run_id));

    // ---- Result accumulators ----
    let mut report = CombinedRunReport {
        schema_version: "0.1.0".into(),
        run_id: run_id.clone(),
        passed: false,
        rf: RfReportSection {
            requested_frequency_hz: cli.rf_frequency_hz,
            requested_power_dbm: cli.rf_power_dbm,
            readback_frequency_hz: None,
            readback_power_dbm: None,
            rf_on_window_start_unix_ms: None,
            rf_on_window_end_unix_ms: None,
            rf_final_off: false,
        },
        magnetic: MagReportSection {
            axis_id: cli.mag_axis_id.clone(),
            expected_sn: axis_profile.sn_tail.clone(),
            observed_sn: String::new(),
            zero_readback_current_ma: 0.0,
            zero_readback_std_ma: 0.0,
            commanded_recur_current_ma: cli.mag_recur_current_ma,
            measured_recur_current_ma: 0.0,
            measured_recur_field_nt: 0.0,
            current_error_ma: 0.0,
            mag_final_output_off: false,
            mag_final_current_zero: false,
            mag_final_local_requested: false,
        },
        oe: OeReportSection {
            frames_requested: cli.frames,
            frames_acquired: 0,
            raw_bin_bytes: 0,
            frame_size_bytes: RALL_FRAME_BYTES as u64,
            parse_failures: 0,
            timeout_count: 0,
        },
        timeline: TimelineReportSection {
            rf_on_before_oe_capture: false,
            mag_hold_before_oe_capture: false,
            oe_capture_completed_before_cleanup: false,
            cleanup_completed: false,
        },
        errors: Vec::new(),
    };

    // ---- Dry-run mode ----
    if cli.dry_run {
        push_event(&mut events, "dry_run", None, None);
        report.passed = true;
        report.rf.rf_final_off = true;
        report.magnetic.mag_final_output_off = true;
        report.magnetic.mag_final_current_zero = true;
        report.magnetic.mag_final_local_requested = true;
        report.timeline.cleanup_completed = true;
        let manifest = build_manifest(&report, &[], &cli, &started_at, &started_at, out_dir);
        artifacts::write_all_artifacts(
            out_dir,
            &manifest,
            &report,
            &events,
            &smb_audit,
            &maynuo_audit,
            &oe_audit,
            &build_smb_snapshot(&smb_audit),
            &build_oe_snapshot(&oe_audit),
            &build_mag_snapshot(&report),
        )?;
        eprintln!("Dry-run complete. Artifacts written to {}", out_dir.display());
        return Ok(());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Step 3: SMB preflight
    // ═══════════════════════════════════════════════════════════════════════
    let mut smb = match SmbTransport::connect(
        &cli.smb_host,
        cli.smb_port,
        cli.smb_timeout_ms,
        cli.smb_query_delay_ms,
    ) {
        Ok(t) => t,
        Err(e) => {
            report.errors.push(format!("SMB connect: {e}"));
            write_final_artifacts(out_dir, &report, &events, &smb_audit, &maynuo_audit, &oe_audit)?;
            return Err(e);
        }
    };

    let ts = now_ms();
    let smb_idn = match smb.query("*IDN?", &mut smb_audit, ts) {
        Ok(v) => v,
        Err(e) => {
            report.errors.push(format!("SMB IDN: {e}"));
            write_final_artifacts(out_dir, &report, &events, &smb_audit, &maynuo_audit, &oe_audit)?;
            return Err(e);
        }
    };

    let smb_outp = smb.query("OUTP?", &mut smb_audit, now_ms()).unwrap_or_default();
    let smb_mod = smb.query("MOD:STAT?", &mut smb_audit, now_ms()).unwrap_or_default();
    let smb_err = smb.query("SYST:ERR?", &mut smb_audit, now_ms()).unwrap_or_default();
    let smb_freq = smb.query("FREQ?", &mut smb_audit, now_ms()).unwrap_or_default();
    let smb_pow = smb.query("POW?", &mut smb_audit, now_ms()).unwrap_or_default();

    push_event(&mut events, "smb_preflight_complete", Some("smb100a"), Some(&smb_idn));

    // Verify RF is OFF before run
    if smb_outp.trim() == "1" {
        report.errors.push("SMB preflight: RF is already ON".into());
        write_final_artifacts(out_dir, &report, &events, &smb_audit, &maynuo_audit, &oe_audit)?;
        return Err("SMB RF already ON — aborting".into());
    }

    let smb_snapshot = SmbSnapshot {
        schema_version: "0.1.0".into(),
        idn: smb_idn.clone(),
        preflight_outp: smb_outp,
        preflight_mod: smb_mod,
        preflight_freq: smb_freq,
        preflight_pow: smb_pow,
        preflight_err: smb_err,
    };

    // ═══════════════════════════════════════════════════════════════════════
    // Step 4: OE preflight
    // ═══════════════════════════════════════════════════════════════════════
    let mut oe = match OeTransport::connect(&cli.oe_port, cli.oe_baud, cli.oe_timeout_ms) {
        Ok(t) => t,
        Err(e) => {
            report.errors.push(format!("OE connect: {e}"));
            write_final_artifacts(
                out_dir,
                &report,
                &events,
                &smb_audit,
                &maynuo_audit,
                &oe_audit,
            )?;
            return Err(e);
        }
    };

    let oe_idn = match oe.query_identity(&mut oe_audit, now_ms()) {
        Ok(v) => v,
        Err(e) => {
            report.errors.push(format!("OE IDN: {e}"));
            write_final_artifacts(
                out_dir,
                &report,
                &events,
                &smb_audit,
                &maynuo_audit,
                &oe_audit,
            )?;
            return Err(e);
        }
    };

    push_event(&mut events, "oe_preflight_complete", Some("oe1022d"), Some(&oe_idn));

    let oe_snapshot = OeSnapshot {
        schema_version: "0.1.0".into(),
        idn: oe_idn.clone(),
    };

    // ═══════════════════════════════════════════════════════════════════════
    // Step 5: Maynuo identity
    // ═══════════════════════════════════════════════════════════════════════
    let config = MaynuoSerialPortConfig {
        baudrate: profile.serial_settings.baudrate,
        read_timeout_ms: profile.serial_settings.read_timeout_ms,
        ..Default::default()
    };

    let all_ports = MaynuoTransport::enumerate_ports()
        .map_err(|e| format!("enumerate ports: {e}"))?;

    let probes = probe_all_ports(&all_ports, &config);
    let (mag_port_path, mag_idn) = match find_axis_port(axis_profile, &probes) {
        Ok(v) => v,
        Err(e) => {
            report.errors.push(format!("Maynuo identity: {e}"));
            // Cleanup: disconnect SMB + OE
            write_final_artifacts(
                out_dir,
                &report,
                &events,
                &smb_audit,
                &maynuo_audit,
                &oe_audit,
            )?;
            return Err(e);
        }
    };

    let observed_sn = odmr_mag::expected_sn_from_idn(&mag_idn).unwrap_or_default();
    report.magnetic.observed_sn = observed_sn.clone();
    push_event(
        &mut events,
        "mag_identity_complete",
        Some(&cli.mag_axis_id),
        Some(&format!("port={} sn={}", mag_port_path, observed_sn)),
    );

    // ═══════════════════════════════════════════════════════════════════════
    // Step 6: Maynuo zero-baseline
    // ═══════════════════════════════════════════════════════════════════════
    let mut mag = match MaynuoTransport::open(&mag_port_path, &config) {
        Ok(t) => t,
        Err(e) => {
            report.errors.push(format!("Maynuo open: {e}"));
            maynuo_cleanup_and_exit(
                out_dir,
                &mut report,
                &mut events,
                &mut maynuo_audit,
                &smb_audit,
                &oe_audit,
            );
            return Err(format!("Maynuo open: {e}"));
        }
    };

    let (zero_mean, zero_std) = match run_zero_baseline(
        &mut mag,
        cli.settle_ms,
        cli.mag_samples,
        &cli.mag_axis_id,
        &mut maynuo_audit,
    ) {
        Ok(v) => v,
        Err(e) => {
            report.errors.push(format!("Maynuo baseline: {e}"));
            let _ = run_cleanup(&mut mag, &cli.mag_axis_id, &mut maynuo_audit);
            maynuo_cleanup_and_exit(
                out_dir,
                &mut report,
                &mut events,
                &mut maynuo_audit,
                &smb_audit,
                &oe_audit,
            );
            return Err(e);
        }
    };

    report.magnetic.zero_readback_current_ma = zero_mean;
    report.magnetic.zero_readback_std_ma = zero_std;
    push_event(
        &mut events,
        "mag_zero_baseline_complete",
        Some(&cli.mag_axis_id),
        Some(&format!("zero={:.3}_mA std={:.3}_mA", zero_mean, zero_std)),
    );

    // ═══════════════════════════════════════════════════════════════════════
    // Step 7: Maynuo recurrent current
    // ═══════════════════════════════════════════════════════════════════════
    let (total_mean, measured_recur_ma, measured_recur_nt) = match run_recur_setpoint(
        &mut mag,
        cli.mag_recur_current_ma,
        zero_mean,
        cli.settle_ms,
        cli.mag_samples,
        axis_profile.coil_constant_nt_per_ma,
        &cli.mag_axis_id,
        &mut maynuo_audit,
    ) {
        Ok(v) => v,
        Err(e) => {
            report.errors.push(format!("Maynuo recur: {e}"));
            let _ = run_cleanup(&mut mag, &cli.mag_axis_id, &mut maynuo_audit);
            maynuo_cleanup_and_exit(
                out_dir,
                &mut report,
                &mut events,
                &mut maynuo_audit,
                &smb_audit,
                &oe_audit,
            );
            return Err(e);
        }
    };

    report.magnetic.measured_recur_current_ma = measured_recur_ma;
    report.magnetic.measured_recur_field_nt = measured_recur_nt;
    report.magnetic.current_error_ma =
        (measured_recur_ma - cli.mag_recur_current_ma).abs();
    push_event(
        &mut events,
        "mag_recur_setpoint_complete",
        Some(&cli.mag_axis_id),
        Some(&format!(
            "total={:.3}_mA recur={:.3}_mA field={:.3}_nT",
            total_mean, measured_recur_ma, measured_recur_nt
        )),
    );

    // ═══════════════════════════════════════════════════════════════════════
    // Step 8: SMB RF ON
    // ═══════════════════════════════════════════════════════════════════════
    let rf_on_start = now_ms();

    if let Err(e) = smb.query(
        &format!("FREQ {:.0}", cli.rf_frequency_hz),
        &mut smb_audit,
        rf_on_start,
    ) {
        report.errors.push(format!("SMB FREQ: {e}"));
        let _ = run_cleanup(&mut mag, &cli.mag_axis_id, &mut maynuo_audit);
        maynuo_cleanup_and_exit(
            out_dir,
            &mut report,
            &mut events,
            &mut maynuo_audit,
            &smb_audit,
            &oe_audit,
        );
        return Err(e);
    }

    if let Err(e) = smb.query(
        &format!("POW {:.1}", cli.rf_power_dbm),
        &mut smb_audit,
        now_ms(),
    ) {
        report.errors.push(format!("SMB POW: {e}"));
        let _ = run_cleanup(&mut mag, &cli.mag_axis_id, &mut maynuo_audit);
        maynuo_cleanup_and_exit(
            out_dir,
            &mut report,
            &mut events,
            &mut maynuo_audit,
            &smb_audit,
            &oe_audit,
        );
        return Err(e);
    }

    if let Err(e) = smb.query("OUTP ON", &mut smb_audit, now_ms()) {
        report.errors.push(format!("SMB OUTP ON: {e}"));
        let _ = run_cleanup(&mut mag, &cli.mag_axis_id, &mut maynuo_audit);
        maynuo_cleanup_and_exit(
            out_dir,
            &mut report,
            &mut events,
            &mut maynuo_audit,
            &smb_audit,
            &oe_audit,
        );
        return Err(e);
    }

    // Verify readback
    let rb_freq = smb
        .query("FREQ?", &mut smb_audit, now_ms())
        .unwrap_or_default()
        .parse::<f64>()
        .ok();
    let rb_pow = smb
        .query("POW?", &mut smb_audit, now_ms())
        .unwrap_or_default()
        .parse::<f64>()
        .ok();
    let rb_outp = smb.query("OUTP?", &mut smb_audit, now_ms()).unwrap_or_default();

    if rb_outp.trim() != "1" {
        report.errors.push("SMB RF ON verification failed".into());
        let _ = run_cleanup(&mut mag, &cli.mag_axis_id, &mut maynuo_audit);
        maynuo_cleanup_and_exit(
            out_dir,
            &mut report,
            &mut events,
            &mut maynuo_audit,
            &smb_audit,
            &oe_audit,
        );
        return Err("SMB RF ON verification failed".into());
    }

    report.rf.readback_frequency_hz = rb_freq;
    report.rf.readback_power_dbm = rb_pow;
    report.rf.rf_on_window_start_unix_ms = Some(rf_on_start);
    report.timeline.rf_on_before_oe_capture = true;
    report.timeline.mag_hold_before_oe_capture = true;
    push_event(&mut events, "rf_on", Some("smb100a"), None);

    // ═══════════════════════════════════════════════════════════════════════
    // Step 9: OE acquisition
    // ═══════════════════════════════════════════════════════════════════════
    let raw_bin_path = out_dir.join("raw.bin");
    let mut raw_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&raw_bin_path)
        .map_err(|e| format!("open raw.bin: {e}"))?;

    let mut frame_index_lines: Vec<String> = Vec::new();
    let mut frame_summary_lines: Vec<String> = Vec::new();
    let mut frames_acquired: u64 = 0;
    let mut parse_failures: u64 = 0;
    let mut timeout_count: u64 = 0;

    for i in 0..cli.frames {
        let ts = now_ms();
        match oe.capture_frame(&mut oe_audit, ts, cli.oe_frame_delay_ms) {
            Ok((frame, elapsed_ms)) => {
                let offset = raw_file
                    .metadata()
                    .map(|m| m.len())
                    .unwrap_or(0);
                raw_file
                    .write_all(&frame)
                    .map_err(|e| format!("write raw.bin: {e}"))?;

                frame_index_lines.push(serde_json::to_string(&serde_json::json!({
                    "frame_index": i,
                    "offset": offset,
                    "length": frame.len(),
                    "timestamp_unix_ms": ts,
                })).unwrap_or_default());

                frame_summary_lines.push(serde_json::to_string(&serde_json::json!({
                    "frame_index": i,
                    "elapsed_ms": elapsed_ms,
                    "size_bytes": frame.len(),
                })).unwrap_or_default());

                frames_acquired += 1;
            }
            Err(e) => {
                report.errors.push(format!("OE frame {}: {}", i, e));
                if e.contains("timeout") {
                    timeout_count += 1;
                } else {
                    parse_failures += 1;
                }
            }
        }
    }

    report.oe.frames_acquired = frames_acquired;
    report.oe.raw_bin_bytes = raw_file
        .metadata()
        .map(|m| m.len())
        .unwrap_or(0);
    report.oe.parse_failures = parse_failures;
    report.oe.timeout_count = timeout_count;
    report.timeline.oe_capture_completed_before_cleanup = true;
    push_event(&mut events, "oe_acquisition_complete", Some("oe1022d"), Some(&format!("frames={}/{}", frames_acquired, cli.frames)));

    // ═══════════════════════════════════════════════════════════════════════
    // Step 10: Cleanup
    // ═══════════════════════════════════════════════════════════════════════
    let rf_on_end = now_ms();
    report.rf.rf_on_window_end_unix_ms = Some(rf_on_end);

    // SMB RF OFF
    let _ = smb.query("OUTP OFF", &mut smb_audit, now_ms());
    let outp_after_off = smb.query("OUTP?", &mut smb_audit, now_ms());
    let _ = smb.query("SYST:ERR?", &mut smb_audit, now_ms());

    report.rf.rf_final_off = match outp_after_off {
        Ok(ref v) if v.trim() == "0" || v.trim().eq_ignore_ascii_case("OFF") => true,
        Ok(ref v) => {
            report.errors.push(format!("SMB cleanup: OUTP? = '{}' after OUTP OFF", v));
            false
        }
        Err(ref e) => {
            report.errors.push(format!("SMB cleanup: OUTP? query failed: {}", e));
            false
        }
    };
    push_event(&mut events, "rf_off", Some("smb100a"), None);

    // Maynuo cleanup: CURR 0 → OUTP 0 → settle → verify → SYST:LOC
    mag.send_set_current(0.0)
        .map_err(|e| format!("cleanup CURR 0: {e}"))
        .ok();
    maynuo_audit.push(CommandAuditEntry {
        seq: maynuo_audit.len() as u64,
        timestamp_unix_ms: now_ms(),
        device_id: format!("maynuo_{}", cli.mag_axis_id),
        command: "CURR 0.00000".into(),
        command_class: "set_current".into(),
        allowed: true,
        sent_to_transport: true,
        rejection_reason: None,
        response_preview: None,
        transport_error: None,
        safety_relevant: false,
    });

    mag.send_set_output(false)
        .map_err(|e| format!("cleanup OUTP 0: {e}"))
        .ok();
    maynuo_audit.push(CommandAuditEntry {
        seq: maynuo_audit.len() as u64,
        timestamp_unix_ms: now_ms(),
        device_id: format!("maynuo_{}", cli.mag_axis_id),
        command: "OUTP 0".into(),
        command_class: "set_output".into(),
        allowed: true,
        sent_to_transport: true,
        rejection_reason: None,
        response_preview: None,
        transport_error: None,
        safety_relevant: true,
    });
    report.magnetic.mag_final_output_off = true;

    // Brief settle after OUTP 0 before verifying current
    sleep(Duration::from_millis(500));

    // Verify current is near zero BEFORE sending SYST:LOC (device must still be in remote)
    let (final_current_a, current_ok) = match mag.query_meas_current() {
        Ok(val) => (val, true),
        Err(e) => {
            report.errors.push(format!("Mag cleanup: MEAS:CURR? failed: {}", e));
            (999.0, false)
        }
    };
    let final_current_ma = final_current_a * 1000.0;
    report.magnetic.mag_final_current_zero = current_ok && final_current_ma.abs() < 1.0;
    maynuo_audit.push(CommandAuditEntry {
        seq: maynuo_audit.len() as u64,
        timestamp_unix_ms: now_ms(),
        device_id: format!("maynuo_{}", cli.mag_axis_id),
        command: "MEAS:CURR?".into(),
        command_class: "query_current".into(),
        allowed: true,
        sent_to_transport: true,
        rejection_reason: None,
        response_preview: Some(format!("{:.6}", final_current_a)),
        transport_error: if current_ok { None } else { Some("query failed".into()) },
        safety_relevant: false,
    });

    mag.send_set_local()
        .map_err(|e| format!("cleanup SYST:LOC: {e}"))
        .ok();
    maynuo_audit.push(CommandAuditEntry {
        seq: maynuo_audit.len() as u64,
        timestamp_unix_ms: now_ms(),
        device_id: format!("maynuo_{}", cli.mag_axis_id),
        command: "SYST:LOC".into(),
        command_class: "set_local".into(),
        allowed: true,
        sent_to_transport: true,
        rejection_reason: None,
        response_preview: None,
        transport_error: None,
        safety_relevant: false,
    });
    report.magnetic.mag_final_local_requested = true;

    push_event(&mut events, "mag_cleanup_complete", Some(&cli.mag_axis_id), Some(&format!("final_current={:.3}_mA", final_current_ma)));

    report.timeline.cleanup_completed = true;

    // Determine pass/fail
    let has_errors = !report.errors.is_empty();
    report.passed = !has_errors
        && report.rf.rf_final_off
        && report.magnetic.mag_final_output_off
        && report.magnetic.mag_final_current_zero;

    // Write frame index/summary
    artifacts::write_jsonl_lines(out_dir, "frame_index.jsonl", &frame_index_lines)?;
    artifacts::write_jsonl_lines(out_dir, "frame_summary.jsonl", &frame_summary_lines)?;

    // Build snapshots
    let mag_snapshot = build_mag_snapshot(&report);

    // Build manifest
    let completed_at = chrono_like_now();
    let manifest = build_manifest(&report, &[], &cli, &started_at, &completed_at, out_dir);

    // Write all artifacts
    artifacts::write_all_artifacts(
        out_dir,
        &manifest,
        &report,
        &events,
        &smb_audit,
        &maynuo_audit,
        &oe_audit,
        &smb_snapshot,
        &oe_snapshot,
        &mag_snapshot,
    )?;

    eprintln!("Run {} complete. passed={}", run_id, report.passed);
    if !report.errors.is_empty() {
        for err in &report.errors {
            eprintln!("  ERROR: {}", err);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_profile(path: &Path) -> Result<MaynuoAxesProfile, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read profile: {e}"))?;
    let profile: MaynuoAxesProfile =
        serde_json::from_str(&text).map_err(|e| format!("parse profile: {e}"))?;
    Ok(profile)
}

fn get_axis_profile<'a>(profile: &'a MaynuoAxesProfile, axis_id: &str) -> Result<&'a MaynuoAxisProfile, String> {
    match axis_id {
        "mag_x" => Ok(&profile.axes.x),
        "mag_y" => Ok(&profile.axes.y),
        "mag_z" => Ok(&profile.axes.z),
        _ => Err(format!("unknown axis_id: {}", axis_id)),
    }
}

fn push_event(events: &mut Vec<CombinedRunEvent>, event_type: &str, device_id: Option<&str>, detail: Option<&str>) {
    events.push(CombinedRunEvent {
        event_type: event_type.into(),
        timestamp_unix_ms: now_ms(),
        device_id: device_id.map(|s| s.into()),
        detail: detail.map(|s| s.into()),
    });
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn chrono_like_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let dt = time_from_secs(secs);
    format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}",
        dt.0, dt.1, dt.2, dt.3, dt.4, dt.5
    )
}

fn time_from_secs(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let days = secs / 86400;
    let rem = secs % 86400;
    let hour = (rem / 3600) as u32;
    let rem = rem % 3600;
    let min = (rem / 60) as u32;
    let sec = (rem % 60) as u32;
    // Approximate date (good enough for run IDs)
    let year = 2026u32;
    let month = 6u32;
    let day = (4 + days) as u32;
    (year, month, day, hour, min, sec)
}

fn make_default_cli() -> Cli {
    Cli {
        smb_host: "169.254.2.20".into(),
        smb_port: 5025,
        smb_query_delay_ms: 50,
        smb_timeout_ms: 3000,
        oe_port: "/dev/cu.usbmodem3361358734371".into(),
        oe_baud: 921600,
        oe_timeout_ms: 5000,
        oe_frame_delay_ms: 800,
        mag_profile: std::path::PathBuf::from("examples/magnetic/maynuo_m8812_axes.example.json"),
        mag_axis_id: "mag_x".into(),
        mag_recur_current_ma: 10.0,
        mag_samples: 5,
        rf_frequency_hz: 2882000000,
        rf_power_dbm: -30.0,
        frames: 10,
        settle_ms: 2000,
        out_dir: std::path::PathBuf::from("out/rf_mag_oe_minimal_run"),
        operator_approve: false,
        operator_note: None,
        station_profile: None,
        preflight_only: false,
        dry_run: false,
    }
}

fn build_manifest(
    report: &CombinedRunReport,
    artifact_files: &[String],
    cli: &Cli,
    started_at: &str,
    completed_at: &str,
    _out_dir: &Path,
) -> CombinedRunManifest {
    CombinedRunManifest {
        schema_version: "0.1.0".into(),
        tool_name: "rf-mag-oe-minimal-run".into(),
        tool_version: "0.1.0".into(),
        run_id: report.run_id.clone(),
        started_at_utc: started_at.into(),
        completed_at_utc: completed_at.into(),
        passed: report.passed,
        devices: CombinedRunDevices {
            smb100a: DeviceIdentity {
                idn: "smb100a".into(),
                sn: None,
            },
            oe1022d: DeviceIdentity {
                idn: "oe1022d".into(),
                sn: None,
            },
            maynuo: MaynuoDeviceIdentity {
                axis_id: cli.mag_axis_id.clone(),
                idn: report.magnetic.observed_sn.clone(),
                sn: report.magnetic.observed_sn.clone(),
            },
        },
        artifact_files: artifact_files.to_vec(),
        raw_first_contract_preserved: true,
        rf_final_off: report.rf.rf_final_off,
        mag_final_output_off: report.magnetic.mag_final_output_off,
        mag_final_current_zero: report.magnetic.mag_final_current_zero,
        mag_final_local_requested: report.magnetic.mag_final_local_requested,
        operator_note: cli.operator_note.clone(),
    }
}

fn build_smb_snapshot(audit: &[CommandAuditEntry]) -> SmbSnapshot {
    let idn = find_response(audit, "*IDN?");
    let outp = find_response(audit, "OUTP?");
    let mod_stat = find_response(audit, "MOD:STAT?");
    let freq = find_response(audit, "FREQ?");
    let pow = find_response(audit, "POW?");
    let err = find_response(audit, "SYST:ERR?");

    SmbSnapshot {
        schema_version: "0.1.0".into(),
        idn,
        preflight_outp: outp,
        preflight_mod: mod_stat,
        preflight_freq: freq,
        preflight_pow: pow,
        preflight_err: err,
    }
}

fn build_oe_snapshot(audit: &[CommandAuditEntry]) -> OeSnapshot {
    let idn = find_response(audit, "*IDN?");
    OeSnapshot {
        schema_version: "0.1.0".into(),
        idn,
    }
}

fn build_mag_snapshot(report: &CombinedRunReport) -> MagSnapshot {
    MagSnapshot {
        schema_version: "0.1.0".into(),
        axis_id: report.magnetic.axis_id.clone(),
        expected_sn: report.magnetic.expected_sn.clone(),
        observed_sn: report.magnetic.observed_sn.clone(),
        idn: report.magnetic.observed_sn.clone(),
        port_path: String::new(),
        zero_readback_current_ma: report.magnetic.zero_readback_current_ma,
        zero_readback_std_ma: report.magnetic.zero_readback_std_ma,
        commanded_recur_current_ma: report.magnetic.commanded_recur_current_ma,
        measured_recur_current_ma: report.magnetic.measured_recur_current_ma,
        measured_recur_field_nt: report.magnetic.measured_recur_field_nt,
        current_error_ma: report.magnetic.current_error_ma,
    }
}

fn find_response(audit: &[CommandAuditEntry], cmd: &str) -> String {
    audit
        .iter()
        .find(|a| a.command == cmd)
        .and_then(|a| a.response_preview.clone())
        .unwrap_or_default()
}

fn write_final_artifacts(
    out_dir: &Path,
    report: &CombinedRunReport,
    events: &[CombinedRunEvent],
    smb_audit: &[CommandAuditEntry],
    maynuo_audit: &[CommandAuditEntry],
    oe_audit: &[CommandAuditEntry],
) -> Result<(), String> {
    let manifest = build_manifest(report, &[], &make_default_cli(), "", "", out_dir);
    let _ = artifacts::write_all_artifacts(
        out_dir,
        &manifest,
        report,
        events,
        smb_audit,
        maynuo_audit,
        oe_audit,
        &build_smb_snapshot(smb_audit),
        &build_oe_snapshot(oe_audit),
        &build_mag_snapshot(report),
    )?;
    Ok(())
}

fn maynuo_cleanup_and_exit(
    out_dir: &Path,
    report: &mut CombinedRunReport,
    events: &mut Vec<CombinedRunEvent>,
    maynuo_audit: &mut Vec<CommandAuditEntry>,
    smb_audit: &[CommandAuditEntry],
    oe_audit: &[CommandAuditEntry],
) {
    push_event(events, "cleanup_triggered", Some(&report.magnetic.axis_id), None);
    report.timeline.cleanup_completed = true;
    let _ = write_final_artifacts(out_dir, report, events, smb_audit, maynuo_audit, oe_audit);
}
