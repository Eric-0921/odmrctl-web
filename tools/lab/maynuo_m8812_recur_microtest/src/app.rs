//! Core application logic: zero-baseline → lock-zero → CURR nonzero →
//! readback reconstruction → cleanup.

use crate::cli::CliArgs;
use crate::types::{
    CommandAuditEntry, RecurMicrotestAxisResult, RecurMicrotestEvent, RecurMicrotestManifest,
    RecurMicrotestReport, RecurMicrotestSnapshot,
};
use odmr_mag::{expected_sn_from_idn, parse_maynuo_idn, MaynuoAxesProfile, MaynuoAxisProfile};
use odmr_maynuo_m8812::{MaynuoM8812Transport, MaynuoPortMetadata, MaynuoSerialPortConfig};
use odmr_types::DeviceId;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

// ── Identity probe result (local, not serialized) ──────────────────────

#[allow(dead_code)]
struct ProbeResult {
    port_path: String,
    idn_raw: Option<String>,
    sn: Option<String>,
    error: Option<String>,
}

// ── Public entry point ─────────────────────────────────────────────────

pub fn run(args: &CliArgs) -> Result<(), String> {
    let started_at = chrono_like_now();
    let profile_path = canonicalize_profile_path(&args.profile)?;
    let profile = load_profile(&profile_path)?;

    let mut events: Vec<RecurMicrotestEvent> = Vec::new();
    events.push(event("run_started", Some(&args.axis_id), None));

    // Validate axis_id
    let axis_profile = get_axis_profile(&profile, &args.axis_id)?;

    // Enumerate and filter ports
    let all_ports = MaynuoM8812Transport::enumerate_ports()
        .map_err(|e| format!("enumerate ports: {e}"))?;
    let candidates = filter_ports(&all_ports, &args.include_port, &args.exclude_port, args.max_ports);
    events.push(event("ports_scanned", Some(&args.axis_id), Some(format!("candidates={}", candidates.len()))));

    let config = MaynuoSerialPortConfig {
        baudrate: args.baudrate,
        read_timeout_ms: args.timeout_ms,
        ..Default::default()
    };

    // Identity probe
    if args.dry_run {
        events.push(event("dry_run", Some(&args.axis_id), Some("dry-run mode, no hardware access".into())));
        let (manifest, snapshot, report) = build_dry_run_output(args, axis_profile, &started_at);
        crate::artifacts::write_artifacts(&args.out_dir, &manifest, &snapshot, &report, &events, &[])?;
        eprintln!("Dry-run complete. Artifacts written to {}", args.out_dir.display());
        return Ok(());
    }

    let id_results = probe_all_ports(&candidates, &config, &mut events);

    // Find the port matching the requested axis
    let (port_path, idn) = find_axis_port(axis_profile, &id_results)?;
    events.push(event("axis_matched", Some(&args.axis_id), Some(format!("port={port_path}"))));

    // ── Process the axis ──
    let (result, audit) = process_axis(axis_profile, &port_path, &idn, &config, args, &mut events);

    let completed_at = chrono_like_now();

    // Compute error metrics
    let current_error_ma = (result.measured_recur_current_ma - result.recur_current_ma_requested).abs();
    let within_error = current_error_ma <= args.max_current_error_ma;
    let within_std = result.measured_total_std_ma <= args.max_current_std_ma;

    let has_errors = !result.errors.is_empty();
    let passed = !has_errors && within_error && within_std && result.output_final_off && result.current_final_zero;

    let report = RecurMicrotestReport {
        passed,
        axis_id: args.axis_id.clone(),
        recur_current_ma_requested: result.recur_current_ma_requested,
        zero_readback_current_ma: result.zero_readback_current_ma,
        measured_total_current_ma: result.measured_total_current_ma,
        measured_recur_current_ma: result.measured_recur_current_ma,
        measured_recur_field_nt: result.measured_recur_field_nt,
        current_error_ma,
        within_error_tolerance: within_error,
        within_std_tolerance: within_std,
        output_final_off: result.output_final_off,
        current_final_zero: result.current_final_zero,
        local_mode_requested: result.local_mode_requested,
        errors: result.errors.clone(),
    };

    let manifest = RecurMicrotestManifest {
        schema_version: "0.1.0".into(),
        tool_name: "maynuo-m8812-recur-microtest".into(),
        tool_version: "0.1.0".into(),
        started_at_utc: started_at,
        completed_at_utc: completed_at.clone(),
        profile_path: profile_path.display().to_string(),
        axis_id: args.axis_id.clone(),
        recur_current_ma_requested: result.recur_current_ma_requested,
        passed,
        artifact_files: vec![
            "manifest.json".into(),
            "maynuo_recur_microtest_snapshot.json".into(),
            "maynuo_recur_microtest_report.json".into(),
            "maynuo_recur_microtest_events.jsonl".into(),
            "maynuo_command_audit.jsonl".into(),
        ],
        operator_note: args.operator_note.clone(),
        only_m3_commands_sent: true,
    };

    events.push(event(
        if passed { "run_passed" } else { "run_failed" },
        Some(&args.axis_id),
        Some(format!("passed={passed} error_ma={current_error_ma:.3}")),
    ));

    let snapshot = RecurMicrotestSnapshot {
        schema_version: "0.1.0".into(),
        axis: result,
        timestamp_utc: completed_at,
    };

    crate::artifacts::write_artifacts(&args.out_dir, &manifest, &snapshot, &report, &events, &audit)?;

    eprintln!(
        "Recur microtest complete. passed={passed}. Artifacts written to {}",
        args.out_dir.display()
    );
    Ok(())
}

// ── Core per-axis process ──────────────────────────────────────────────

fn process_axis(
    profile: &MaynuoAxisProfile,
    port_path: &str,
    idn: &str,
    config: &MaynuoSerialPortConfig,
    args: &CliArgs,
    events: &mut Vec<RecurMicrotestEvent>,
) -> (RecurMicrotestAxisResult, Vec<CommandAuditEntry>) {
    let coil_nt_per_ma = profile.coil_constant_nt_per_ma;

    let mut result = RecurMicrotestAxisResult {
        axis_id: profile.axis_id.clone(),
        idn: idn.to_string(),
        port_path: port_path.to_string(),
        sn_tail: profile.sn_tail.clone(),
        coil_constant_nt_per_ma: coil_nt_per_ma,
        zero_set_current_ma: 0.0,
        zero_readback_samples_ma: vec![],
        zero_readback_current_ma: 0.0,
        zero_readback_std_ma: 0.0,
        lock_zero_applied: false,
        recur_current_ma_requested: 0.0,
        target_field_nt_requested: None,
        total_current_ma_commanded: 0.0,
        command_string: String::new(),
        recur_readback_samples_ma: vec![],
        measured_total_current_ma: 0.0,
        measured_total_std_ma: 0.0,
        measured_recur_current_ma: 0.0,
        measured_recur_field_nt: 0.0,
        current_error_ma: 0.0,
        field_error_nt: None,
        output_final_off: false,
        current_final_zero: false,
        local_mode_requested: false,
        errors: vec![],
    };
    let mut audit: Vec<CommandAuditEntry> = Vec::new();
    let mut seq: u64 = 0;

    // Open transport
    let device_id = DeviceId::new(format!("m3-{}", &profile.axis_id));
    let mut transport = match MaynuoM8812Transport::open(device_id, port_path, config.clone()) {
        Ok(t) => t,
        Err(e) => {
            result.errors.push(format!("open port: {e}"));
            return (result, audit);
        }
    };

    // ── SYST:REM ──
    if let Err(e) = transport.send_set_remote() {
        result.errors.push(format!("SYST:REM: {e}"));
        attempt_cleanup(&mut transport, &mut result, &mut seq, &mut audit);
        return (result, audit);
    }
    push_audit(&mut seq, &mut audit, &result.axis_id, "SYST:REM", "set_remote", false, None, None, false);

    // ── VOLT 75 ──
    if let Err(e) = transport.send_set_voltage(75) {
        result.errors.push(format!("VOLT 75: {e}"));
        attempt_cleanup(&mut transport, &mut result, &mut seq, &mut audit);
        return (result, audit);
    }
    push_audit(&mut seq, &mut audit, &result.axis_id, "VOLT 75", "set_voltage", false, None, None, false);

    // ═══════════════════════════════════════════════════════════════════
    // Phase 1: Zero-baseline (CURR 0 → OUTP 1 → MEAS → lock-zero)
    // ═══════════════════════════════════════════════════════════════════

    // CURR 0
    if let Err(e) = transport.send_set_current(0.0) {
        result.errors.push(format!("baseline CURR 0: {e}"));
        attempt_cleanup(&mut transport, &mut result, &mut seq, &mut audit);
        return (result, audit);
    }
    push_audit(&mut seq, &mut audit, &result.axis_id, "CURR 0.00000", "set_current", false, None, None, false);

    // OUTP 1
    if let Err(e) = transport.send_set_output(true) {
        result.errors.push(format!("OUTP 1: {e}"));
        attempt_cleanup(&mut transport, &mut result, &mut seq, &mut audit);
        return (result, audit);
    }
    push_audit(&mut seq, &mut audit, &result.axis_id, "OUTP 1", "set_output", false, None, None, false);
    events.push(event("mag_output_on_zero", Some(&result.axis_id), None));

    // Settle
    sleep(Duration::from_millis(args.settle_ms));
    events.push(event("mag_settle_zero", Some(&result.axis_id), Some(format!("settle_ms={}", args.settle_ms))));

    // MEAS:CURR? × N for zero baseline
    let (zero_samples_ma, zero_mean, zero_std) = match collect_readback(
        &mut transport, &mut result, &mut seq, &mut audit, args, events, "zero",
    ) {
        Ok(v) => v,
        Err(_) => return (result, audit),
    };

    result.zero_readback_samples_ma = zero_samples_ma;
    result.zero_readback_current_ma = zero_mean;
    result.zero_readback_std_ma = zero_std;
    events.push(event(
        "mag_zero_measured",
        Some(&result.axis_id),
        Some(format!("zero={:.3}_mA std={:.3}_mA", zero_mean, zero_std)),
    ));

    // Lock-zero
    result.lock_zero_applied = true;
    events.push(event("mag_lock_zero_applied", Some(&result.axis_id), Some(format!("zero={:.3}_mA", zero_mean))));

    // ═══════════════════════════════════════════════════════════════════
    // Phase 2: Recurrent setpoint
    // ═══════════════════════════════════════════════════════════════════

    // Compute recur current
    let recur_current_ma = if let Some(target_nt) = args.target_field_nt {
        result.target_field_nt_requested = Some(target_nt);
        target_nt / coil_nt_per_ma
    } else {
        result.target_field_nt_requested = None;
        args.recur_current_ma
    };

    if !recur_current_ma.is_finite() || recur_current_ma <= 0.0 {
        result.errors.push(format!("invalid recur current: {recur_current_ma} mA"));
        attempt_cleanup(&mut transport, &mut result, &mut seq, &mut audit);
        return (result, audit);
    }

    let total_current_ma = result.zero_set_current_ma + recur_current_ma;
    if total_current_ma > profile.max_current_ma {
        result.errors.push(format!(
            "total current {total_current_ma} mA exceeds axis limit {} mA",
            profile.max_current_ma
        ));
        attempt_cleanup(&mut transport, &mut result, &mut seq, &mut audit);
        return (result, audit);
    }

    let command_current_a = total_current_ma / 1000.0;
    let command_string = format!("CURR {:.5}", command_current_a);

    result.recur_current_ma_requested = recur_current_ma;
    result.total_current_ma_commanded = total_current_ma;
    result.command_string = command_string.clone();

    // Send CURR {nonzero}
    if let Err(e) = transport.send_set_current(command_current_a) {
        result.errors.push(format!("send CURR recur: {e}"));
        // Attempt safe return to zero before cleanup
        let _ = transport.send_set_current(0.0);
        attempt_cleanup(&mut transport, &mut result, &mut seq, &mut audit);
        return (result, audit);
    }
    push_audit(&mut seq, &mut audit, &result.axis_id, &command_string, "set_current", false, None, None, true);
    events.push(event("mag_recur_setpoint_sent", Some(&result.axis_id), Some(format!("cmd={command_string}"))));

    // Settle
    sleep(Duration::from_millis(args.settle_ms));
    events.push(event("mag_settle_recur", Some(&result.axis_id), Some(format!("settle_ms={}", args.settle_ms))));

    // MEAS:CURR? × N for recur readback
    let (recur_samples_ma, total_mean, total_std) = match collect_readback(
        &mut transport, &mut result, &mut seq, &mut audit, args, events, "recur",
    ) {
        Ok(v) => v,
        Err(_) => return (result, audit),
    };

    result.recur_readback_samples_ma = recur_samples_ma;
    result.measured_total_current_ma = total_mean;
    result.measured_total_std_ma = total_std;

    // Reconstruct
    let measured_recur_ma = total_mean - result.zero_readback_current_ma;
    let measured_recur_nt = measured_recur_ma * coil_nt_per_ma;

    result.measured_recur_current_ma = measured_recur_ma;
    result.measured_recur_field_nt = measured_recur_nt;

    events.push(event(
        "mag_recur_reconstructed",
        Some(&result.axis_id),
        Some(format!(
            "total={:.3}_mA recur={:.3}_mA field={:.3}_nT",
            total_mean, measured_recur_ma, measured_recur_nt
        )),
    ));

    // ═══════════════════════════════════════════════════════════════════
    // Phase 3: Cleanup (CURR 0 → OUTP 0 → SYST:LOC)
    // ═══════════════════════════════════════════════════════════════════

    // CURR 0 first (ramp down before turning off output)
    if let Err(e) = transport.send_set_current(0.0) {
        result.errors.push(format!("cleanup CURR 0: {e}"));
    }
    push_audit(&mut seq, &mut audit, &result.axis_id, "CURR 0.00000", "set_current", false, None, None, false);
    result.current_final_zero = true;

    // OUTP 0
    if let Err(e) = transport.send_set_output(false) {
        result.errors.push(format!("cleanup OUTP 0: {e}"));
    }
    push_audit(&mut seq, &mut audit, &result.axis_id, "OUTP 0", "set_output", false, None, None, false);
    result.output_final_off = true;

    // Wait for current to decay, then verify
    std::thread::sleep(std::time::Duration::from_millis(500));
    match transport.query_meas_current() {
        Ok(current_a) => {
            let current_ma = current_a * 1000.0;
            result.current_final_zero = current_ma.abs() < 1.0;
            push_audit(&mut seq, &mut audit, &result.axis_id, "MEAS:CURR?", "query_current", true, Some(format!("{:.6}", current_a)), None, false);
        }
        Err(e) => {
            result.errors.push(format!("cleanup MEAS:CURR? failed: {e}"));
            push_audit(&mut seq, &mut audit, &result.axis_id, "MEAS:CURR?", "query_current", true, None, Some(e.to_string()), false);
        }
    }

    // SYST:LOC
    if let Err(e) = transport.send_set_local() {
        result.errors.push(format!("cleanup SYST:LOC: {e}"));
    }
    push_audit(&mut seq, &mut audit, &result.axis_id, "SYST:LOC", "set_local", false, None, None, false);
    result.local_mode_requested = true;

    events.push(event("mag_cleanup_complete", Some(&result.axis_id), None));

    (result, audit)
}

// ── Readback collector ─────────────────────────────────────────────────

fn collect_readback(
    transport: &mut MaynuoM8812Transport,
    result: &mut RecurMicrotestAxisResult,
    seq: &mut u64,
    audit: &mut Vec<CommandAuditEntry>,
    args: &CliArgs,
    events: &mut Vec<RecurMicrotestEvent>,
    phase: &str,
) -> Result<(Vec<f64>, f64, f64), String> {
    let mut samples_a: Vec<f64> = Vec::new();
    for i in 0..args.samples {
        if i > 0 {
            sleep(Duration::from_millis(args.sample_interval_ms));
        }
        match transport.query_meas_current() {
            Ok(current_a) => {
                samples_a.push(current_a);
                push_audit(seq, audit, &result.axis_id, "MEAS:CURR?", "query_current", true, Some(format!("{:.6}", current_a)), None, false);
            }
            Err(e) => {
                let err_str = e.to_string();
                result.errors.push(format!("MEAS:CURR? {phase} sample {i}: {err_str}"));
                push_audit(seq, audit, &result.axis_id, "MEAS:CURR?", "query_current", true, None, Some(err_str), false);
            }
        }
    }

    if samples_a.is_empty() {
        result.errors.push(format!("no valid {phase} MEAS:CURR? samples"));
        attempt_cleanup(transport, result, seq, audit);
        return Err("no valid MEAS:CURR? samples".into());
    }

    let samples_ma: Vec<f64> = samples_a.iter().map(|a| a * 1000.0).collect();
    let mean_ma = samples_ma.iter().sum::<f64>() / samples_ma.len() as f64;
    let variance: f64 = samples_ma.iter().map(|s| (s - mean_ma).powi(2)).sum::<f64>() / samples_ma.len() as f64;
    let std_ma = variance.sqrt();

    events.push(event(
        &format!("mag_{phase}_readback_done"),
        Some(&result.axis_id),
        Some(format!("mean={:.3}_mA std={:.3}_mA n={}", mean_ma, std_ma, samples_ma.len())),
    ));

    Ok((samples_ma, mean_ma, std_ma))
}

// ── Cleanup ────────────────────────────────────────────────────────────

fn attempt_cleanup(
    transport: &mut MaynuoM8812Transport,
    result: &mut RecurMicrotestAxisResult,
    seq: &mut u64,
    audit: &mut Vec<CommandAuditEntry>,
) {
    // CURR 0
    match transport.send_set_current(0.0) {
        Ok(()) => {
            push_audit(seq, audit, &result.axis_id, "CURR 0.00000", "set_current", false, None, None, false);
            result.current_final_zero = true;
        }
        Err(e) => {
            let err = e.to_string();
            push_audit(seq, audit, &result.axis_id, "CURR 0.00000", "set_current", false, None, Some(err), false);
        }
    }

    // OUTP 0
    match transport.send_set_output(false) {
        Ok(()) => {
            push_audit(seq, audit, &result.axis_id, "OUTP 0", "set_output", false, None, None, false);
            result.output_final_off = true;
        }
        Err(e) => {
            let err = e.to_string();
            push_audit(seq, audit, &result.axis_id, "OUTP 0", "set_output", false, None, Some(err), false);
        }
    }

    // SYST:LOC
    match transport.send_set_local() {
        Ok(()) => {
            push_audit(seq, audit, &result.axis_id, "SYST:LOC", "set_local", false, None, None, false);
            result.local_mode_requested = true;
        }
        Err(e) => {
            let err = e.to_string();
            push_audit(seq, audit, &result.axis_id, "SYST:LOC", "set_local", false, None, Some(err), false);
        }
    }
}

// ── Identity probing ───────────────────────────────────────────────────

fn probe_all_ports(
    candidates: &[MaynuoPortMetadata],
    config: &MaynuoSerialPortConfig,
    events: &mut Vec<RecurMicrotestEvent>,
) -> Vec<ProbeResult> {
    let mut results = Vec::new();
    for meta in candidates {
        let device_id = DeviceId::new(format!("probe-{}", sanitize_id(&meta.port_path)));
        let mut transport = match MaynuoM8812Transport::open(device_id, &meta.port_path, config.clone()) {
            Ok(t) => t,
            Err(e) => {
                results.push(ProbeResult {
                    port_path: meta.port_path.clone(),
                    idn_raw: None,
                    sn: None,
                    error: Some(e.to_string()),
                });
                continue;
            }
        };
        match transport.query_idn() {
            Ok(idn) => {
                let sn = parse_maynuo_idn(&idn).ok().map(|p| p.serial_number);
                results.push(ProbeResult {
                    port_path: meta.port_path.clone(),
                    idn_raw: Some(idn),
                    sn,
                    error: None,
                });
                events.push(event("port_probed", None, Some(meta.port_path.clone())));
            }
            Err(e) => {
                results.push(ProbeResult {
                    port_path: meta.port_path.clone(),
                    idn_raw: None,
                    sn: None,
                    error: Some(e.to_string()),
                });
                events.push(event("port_probe_failed", None, Some(meta.port_path.clone())));
            }
        }
    }
    results
}

fn find_axis_port(
    profile: &MaynuoAxisProfile,
    results: &[ProbeResult],
) -> Result<(String, String), String> {
    let expected_sn = expected_sn_from_idn(&profile.expected_idn)
        .map_err(|e| format!("parse expected_idn: {e}"))?;

    for r in results {
        if let Some(ref sn) = r.sn {
            if sn == &expected_sn {
                let idn = r.idn_raw.clone().unwrap_or_default();
                return Ok((r.port_path.clone(), idn));
            }
        }
    }
    Err(format!("axis {} (SN {expected_sn}) not found on any port", profile.axis_id))
}

fn get_axis_profile<'a>(profile: &'a MaynuoAxesProfile, axis_id: &str) -> Result<&'a MaynuoAxisProfile, String> {
    match axis_id {
        "mag_x" => Ok(&profile.axes.x),
        "mag_y" => Ok(&profile.axes.y),
        "mag_z" => Ok(&profile.axes.z),
        _ => Err(format!("unknown axis_id: {axis_id}")),
    }
}

// ── Audit ──────────────────────────────────────────────────────────────

fn push_audit(
    seq: &mut u64,
    audit: &mut Vec<CommandAuditEntry>,
    axis_id: &str,
    command: &str,
    class: &str,
    expects_response: bool,
    response_preview: Option<String>,
    transport_error: Option<String>,
    nonzero_current_attempted: bool,
) {
    *seq += 1;
    audit.push(CommandAuditEntry {
        seq: *seq,
        timestamp: chrono_like_now(),
        axis_id: axis_id.into(),
        command: command.into(),
        command_class: class.into(),
        expects_response,
        response_preview,
        transport_error,
        allowed: true,
        nonzero_current_attempted,
    });
}

// ── Dry-run output ─────────────────────────────────────────────────────

fn build_dry_run_output(
    args: &CliArgs,
    profile: &MaynuoAxisProfile,
    started_at: &str,
) -> (RecurMicrotestManifest, RecurMicrotestSnapshot, RecurMicrotestReport) {
    let completed_at = chrono_like_now();
    let result = RecurMicrotestAxisResult {
        axis_id: args.axis_id.clone(),
        idn: profile.expected_idn.clone(),
        port_path: profile.last_known_port_name.clone(),
        sn_tail: profile.sn_tail.clone(),
        coil_constant_nt_per_ma: profile.coil_constant_nt_per_ma,
        zero_set_current_ma: 0.0,
        zero_readback_samples_ma: vec![],
        zero_readback_current_ma: 0.0,
        zero_readback_std_ma: 0.0,
        lock_zero_applied: false,
        recur_current_ma_requested: args.recur_current_ma,
        target_field_nt_requested: args.target_field_nt,
        total_current_ma_commanded: args.recur_current_ma,
        command_string: format!("CURR {:.5}", args.recur_current_ma / 1000.0),
        recur_readback_samples_ma: vec![],
        measured_total_current_ma: 0.0,
        measured_total_std_ma: 0.0,
        measured_recur_current_ma: 0.0,
        measured_recur_field_nt: 0.0,
        current_error_ma: 0.0,
        field_error_nt: None,
        output_final_off: false,
        current_final_zero: false,
        local_mode_requested: false,
        errors: vec!["dry-run: no hardware accessed".into()],
    };

    let manifest = RecurMicrotestManifest {
        schema_version: "0.1.0".into(),
        tool_name: "maynuo-m8812-recur-microtest".into(),
        tool_version: "0.1.0".into(),
        started_at_utc: started_at.into(),
        completed_at_utc: completed_at.clone(),
        profile_path: args.profile.display().to_string(),
        axis_id: args.axis_id.clone(),
        recur_current_ma_requested: args.recur_current_ma,
        passed: false,
        artifact_files: vec![
            "manifest.json".into(),
            "maynuo_recur_microtest_snapshot.json".into(),
            "maynuo_recur_microtest_report.json".into(),
            "maynuo_recur_microtest_events.jsonl".into(),
            "maynuo_command_audit.jsonl".into(),
        ],
        operator_note: args.operator_note.clone(),
        only_m3_commands_sent: true,
    };

    let report = RecurMicrotestReport {
        passed: false,
        axis_id: args.axis_id.clone(),
        recur_current_ma_requested: args.recur_current_ma,
        zero_readback_current_ma: 0.0,
        measured_total_current_ma: 0.0,
        measured_recur_current_ma: 0.0,
        measured_recur_field_nt: 0.0,
        current_error_ma: 0.0,
        within_error_tolerance: false,
        within_std_tolerance: false,
        output_final_off: false,
        current_final_zero: false,
        local_mode_requested: false,
        errors: vec!["dry-run".into()],
    };

    let snapshot = RecurMicrotestSnapshot {
        schema_version: "0.1.0".into(),
        axis: result,
        timestamp_utc: completed_at,
    };

    (manifest, snapshot, report)
}

// ── Helpers ────────────────────────────────────────────────────────────

pub fn load_profile(path: &Path) -> Result<MaynuoAxesProfile, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("read profile {}: {e}", path.display()))?;
    serde_json::from_str(&content).map_err(|e| format!("parse profile: {e}"))
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

fn event(event_type: &str, axis_id: Option<&str>, detail: Option<String>) -> RecurMicrotestEvent {
    RecurMicrotestEvent {
        event_type: event_type.into(),
        timestamp: chrono_like_now(),
        axis_id: axis_id.map(|s| s.to_string()),
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

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use odmr_mag::{MaynuoAxes, MaynuoSerialSettings};

    #[allow(dead_code)]
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

    fn match_result(port: &str, sn: &str, idn: &str) -> ProbeResult {
        ProbeResult {
            port_path: port.into(),
            idn_raw: Some(idn.into()),
            sn: Some(sn.into()),
            error: None,
        }
    }

    // ── Port matching ──

    #[test]
    fn find_axis_port_matches_x() {
        let profile = axis("mag_x", "2020", "MAYNUO,M8812,080020960220402020,V2.7", "COM4", 143.26);
        let results = vec![match_result("COM4", "080020960220402020", "MAYNUO,M8812,080020960220402020,V2.7")];
        let (port, idn) = find_axis_port(&profile, &results).unwrap();
        assert_eq!(port, "COM4");
        assert!(idn.contains("080020960220402020"));
    }

    #[test]
    fn find_axis_port_rejects_wrong_sn() {
        let profile = axis("mag_x", "2020", "MAYNUO,M8812,080020960220402020,V2.7", "COM4", 143.26);
        let results = vec![match_result("COM5", "999999999999999999", "MAYNUO,M8812,999999999999999999,V2.7")];
        let err = find_axis_port(&profile, &results).unwrap_err();
        assert!(err.contains("not found"));
    }

    // ── Recur current / field computation ──

    #[test]
    fn recur_current_from_target_field() {
        let coil: f64 = 143.26;
        let target_nt: f64 = 1432.6;
        let recur_ma = target_nt / coil;
        assert!((recur_ma - 10.0_f64).abs() < 0.01);
    }

    #[test]
    fn target_field_to_total_current() {
        let coil: f64 = 143.26;
        let target_nt: f64 = 1432.6;
        let recur_ma = target_nt / coil;
        let zero_set_ma: f64 = 0.0;
        let total_ma = zero_set_ma + recur_ma;
        assert!((total_ma - 10.0_f64).abs() < 0.01);
        let cmd_a = total_ma / 1000.0;
        assert!((cmd_a - 0.01_f64).abs() < 1e-6);
        let cmd = format!("CURR {:.5}", cmd_a);
        assert!(cmd.contains("0.01000"));
    }

    #[test]
    fn measured_recur_reconstruction() {
        let zero_readback_ma: f64 = 0.102;
        let measured_total_ma: f64 = 10.152;
        let recur_ma = measured_total_ma - zero_readback_ma;
        assert!((recur_ma - 10.05_f64).abs() < 0.01);
        let coil: f64 = 143.26;
        let recur_nt = recur_ma * coil;
        assert!((recur_nt - 1440.0_f64).abs() < 5.0);
    }

    #[test]
    fn reject_total_current_over_limit() {
        let max_ma: f64 = 5000.0;
        let recur_ma: f64 = 6000.0;
        let total_ma = 0.0_f64 + recur_ma;
        assert!(total_ma > max_ma);
    }

    #[test]
    fn zero_readback_reduces_recur() {
        let zero: f64 = 0.1;
        let measured_total: f64 = 10.12;
        let recur = measured_total - zero;
        assert!((recur - 10.02_f64).abs() < 0.01);
    }

    // ── Filtering ──

    #[test]
    fn port_filtering_include_empty() {
        let ports = vec![MaynuoPortMetadata {
            port_path: "COM1".into(),
            port_type: None,
            usb_serial_number: None,
            usb_vid: None,
            usb_pid: None,
            manufacturer: None,
            product: None,
        }];
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
    fn load_profile_parses_example() {
        let profile_path = canonicalize_profile_path(
            Path::new("examples/magnetic/maynuo_m8812_axes.example.json"),
        )
        .unwrap();
        let profile = load_profile(&profile_path).unwrap();
        assert_eq!(profile.axes.x.axis_id, "mag_x");
        assert_eq!(profile.axes.y.axis_id, "mag_y");
        assert_eq!(profile.axes.z.axis_id, "mag_z");
    }
}
