//! Core application logic: probe identity, enable zero-mode output, read back
//! baseline, lock zero, and shut down cleanly.

use crate::cli::CliArgs;
use crate::types::{
    AuditInvariants, AxisZeroBaseline, CommandAuditEntry, ZeroBaselineEvent, ZeroBaselineManifest,
    ZeroBaselineReport, ZeroBaselineSnapshot,
};
use odmr_mag::{
    expected_sn_from_idn, parse_maynuo_idn, MaynuoAxesProfile, MaynuoAxisProfile, MaynuoAxisRunner,
};
use odmr_maynuo_m8812::{
    MaynuoM8812Transport, MaynuoPortMetadata, MaynuoSerialPortConfig,
};
use odmr_types::DeviceId;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

// ── Identity probe result (local, not serialized) ──────────────────────

#[allow(dead_code)]
struct ProbeResult {
    port_path: String,
    idn_raw: Option<String>,
    sn: Option<String>,
    matched_axis_id: Option<String>,
    error: Option<String>,
}

// ── Public entry point ─────────────────────────────────────────────────

pub fn run(args: &CliArgs) -> Result<(), String> {
    let started_at = chrono_like_now();
    let profile_path = canonicalize_profile_path(&args.profile)?;
    let profile = load_profile(&profile_path)?;

    let mut events: Vec<ZeroBaselineEvent> = Vec::new();
    events.push(zt_event("run_started", None, None));

    // Phase 1: Enumerate and identity-probe candidate ports
    let all_ports = MaynuoM8812Transport::enumerate_ports()
        .map_err(|e| format!("enumerate ports: {e}"))?;
    let candidates = filter_ports(&all_ports, &args.include_port, &args.exclude_port, args.max_ports);
    events.push(zt_event("ports_scanned", None, Some(format!("candidates={}", candidates.len()))));

    let config = MaynuoSerialPortConfig {
        baudrate: args.baudrate,
        read_timeout_ms: args.timeout_ms,
        ..Default::default()
    };

    let id_results = probe_all_ports(&candidates, &config, args.dry_run, &mut events);

    // Phase 2: Map identity results to axes by SN
    let axis_port_map = map_ports_to_axes(&profile, &id_results)?;
    events.push(zt_event("ports_mapped", None, Some(format!("mapped={}", axis_port_map.len()))));

    // Phase 3: Process each mapped axis
    let mut axis_results: Vec<AxisZeroBaseline> = Vec::new();
    let mut global_audit: Vec<CommandAuditEntry> = Vec::new();
    let mut audit_seq: u64 = 0;

    for (axis_id, (port_path, idn)) in &axis_port_map {
        if let Some(ref target) = args.axis_id {
            if axis_id != target {
                continue;
            }
        }

        let axis_profile = match get_axis_profile(&profile, axis_id) {
            Ok(p) => p,
            Err(e) => {
                axis_results.push(AxisZeroBaseline {
                    axis_id: axis_id.clone(),
                    idn: idn.clone(),
                    port_path: port_path.clone(),
                    sn_tail: String::new(),
                    zero_set_current_ma: 0.0,
                    zero_readback_samples_ma: vec![],
                    zero_readback_current_ma: 0.0,
                    zero_readback_std_ma: 0.0,
                    zero_readback_current_a: 0.0,
                    coil_constant_nt_per_ma: 0.0,
                    lock_zero_applied: false,
                    output_was_on: false,
                    shutdown_succeeded: false,
                    errors: vec![e],
                });
                continue;
            }
        };

        let (baseline, mut audit) = process_axis(
            axis_profile,
            port_path,
            idn,
            &config,
            args,
            &mut audit_seq,
            &mut events,
        );
        global_audit.append(&mut audit);
        axis_results.push(baseline);
    }

    // Phase 4: Compute audit invariants
    let invariants = compute_audit_invariants(&global_audit, &axis_results);

    // Phase 5: Build report
    let all_invariants_pass = invariants.all_pass();
    let all_axes_ok = axis_results.iter().all(|a| a.errors.is_empty());
    let strict_fail = args.strict
        && id_results.iter().any(|r| r.idn_raw.is_some() && r.matched_axis_id.is_none() && r.sn.is_some());
    let passed = all_axes_ok && all_invariants_pass && !strict_fail;

    let report = ZeroBaselineReport {
        passed,
        axes_processed: axis_results.len(),
        axes_passed: axis_results.iter().filter(|a| a.errors.is_empty()).count(),
        axes_failed: axis_results.iter().filter(|a| !a.errors.is_empty()).count(),
        total_measurements: axis_results
            .iter()
            .map(|a| a.zero_readback_samples_ma.len() as u32)
            .sum(),
        audit_invariants: invariants.clone(),
        errors: axis_results
            .iter()
            .flat_map(|a| a.errors.clone())
            .collect(),
    };

    let completed_at = chrono_like_now();

    let manifest = ZeroBaselineManifest {
        schema_version: "0.1.0".into(),
        tool_name: "maynuo-m8812-zero-baseline".into(),
        tool_version: "0.1.0".into(),
        started_at_utc: started_at.clone(),
        completed_at_utc: completed_at.clone(),
        profile_path: profile_path.display().to_string(),
        passed,
        artifact_files: vec![
            "manifest.json".into(),
            "zero_baseline_snapshot.json".into(),
            "zero_baseline_report.json".into(),
            "zero_baseline_events.jsonl".into(),
            "command_audit.jsonl".into(),
        ],
        axes_processed: axis_results.iter().map(|a| a.axis_id.clone()).collect(),
        audit_invariants_met: all_invariants_pass,
        operator_note: args.operator_note.clone(),
        only_m2b_commands_sent: true,
    };

    events.push(zt_event(
        if passed { "run_passed" } else { "run_failed" },
        None,
        Some(format!("passed={passed}")),
    ));

    let snapshot = ZeroBaselineSnapshot {
        schema_version: "0.1.0".into(),
        axes: axis_results,
        timestamp_utc: completed_at,
    };

    crate::artifacts::write_artifacts(&args.out_dir, &manifest, &snapshot, &report, &events, &global_audit)?;

    eprintln!(
        "Zero baseline complete. passed={passed}. Artifacts written to {}",
        args.out_dir.display()
    );
    Ok(())
}

// ── Per-axis processing ────────────────────────────────────────────────

fn process_axis(
    profile: &MaynuoAxisProfile,
    port_path: &str,
    idn: &str,
    config: &MaynuoSerialPortConfig,
    args: &CliArgs,
    audit_seq: &mut u64,
    events: &mut Vec<ZeroBaselineEvent>,
) -> (AxisZeroBaseline, Vec<CommandAuditEntry>) {
    let mut baseline = AxisZeroBaseline {
        axis_id: profile.axis_id.clone(),
        idn: idn.to_string(),
        port_path: port_path.to_string(),
        sn_tail: profile.sn_tail.clone(),
        zero_set_current_ma: 0.0,
        zero_readback_samples_ma: Vec::new(),
        zero_readback_current_ma: 0.0,
        zero_readback_std_ma: 0.0,
        zero_readback_current_a: 0.0,
        coil_constant_nt_per_ma: profile.coil_constant_nt_per_ma,
        lock_zero_applied: false,
        output_was_on: false,
        shutdown_succeeded: false,
        errors: Vec::new(),
    };
    let mut audit: Vec<CommandAuditEntry> = Vec::new();

    // State machine runner for event recording
    let mut runner = MaynuoAxisRunner::new(profile.clone());

    // Apply identity states
    let _ = runner.apply_discovered(idn);
    let _ = runner.apply_axis_mapped();
    events.extend(runner_events(&runner));

    // Open transport
    let device_id = DeviceId::new(format!("zb-{}", &profile.axis_id));
    let mut transport = match MaynuoM8812Transport::open(device_id, port_path, config.clone()) {
        Ok(t) => t,
        Err(e) => {
            baseline.errors.push(format!("open port: {e}"));
            return (baseline, audit);
        }
    };

    // ── SYST:REM ──
    match transport.send_set_remote() {
        Ok(()) => {
            push_audit(audit_seq, &mut audit, &baseline.axis_id, "SYST:REM", "set_remote", false, None, None);
        }
        Err(e) => {
            baseline.errors.push(format!("SYST:REM: {e}"));
            attempt_cleanup(&mut transport, &mut baseline, audit_seq, &mut audit);
            return (baseline, audit);
        }
    }

    // ── VOLT 75 ──
    match transport.send_set_voltage(75) {
        Ok(()) => {
            push_audit(audit_seq, &mut audit, &baseline.axis_id, "VOLT 75", "set_voltage", false, None, None);
        }
        Err(e) => {
            baseline.errors.push(format!("VOLT 75: {e}"));
            attempt_cleanup(&mut transport, &mut baseline, audit_seq, &mut audit);
            return (baseline, audit);
        }
    }

    // ── CURR 0.00000 ──
    match transport.send_set_current(0.0) {
        Ok(()) => {
            push_audit(audit_seq, &mut audit, &baseline.axis_id, "CURR 0.00000", "set_current", false, None, None);
            let _ = runner.apply_initialized_output_off();
        }
        Err(e) => {
            baseline.errors.push(format!("CURR 0: {e}"));
            attempt_cleanup(&mut transport, &mut baseline, audit_seq, &mut audit);
            return (baseline, audit);
        }
    }

    // ── OUTP 1 ──
    match transport.send_set_output(true) {
        Ok(()) => {
            push_audit(audit_seq, &mut audit, &baseline.axis_id, "OUTP 1", "set_output", false, None, None);
            baseline.output_was_on = true;
            let _ = runner.apply_output_on_zero_mode();
            events.push(zt_event("mag_output_on_zero", Some(&baseline.axis_id), None));
        }
        Err(e) => {
            baseline.errors.push(format!("OUTP 1: {e}"));
            attempt_cleanup(&mut transport, &mut baseline, audit_seq, &mut audit);
            return (baseline, audit);
        }
    }

    // ── Settle ──
    sleep(Duration::from_millis(args.settle_ms));
    events.push(zt_event(
        "mag_settle_complete",
        Some(&baseline.axis_id),
        Some(format!("settle_ms={}", args.settle_ms)),
    ));

    // ── MEAS:CURR? × N ──
    let mut samples_a: Vec<f64> = Vec::new();
    for i in 0..args.zero_samples {
        if i > 0 {
            sleep(Duration::from_millis(args.sample_interval_ms));
        }
        match transport.query_meas_current() {
            Ok(current_a) => {
                samples_a.push(current_a);
                push_audit(
                    audit_seq, &mut audit, &baseline.axis_id,
                    "MEAS:CURR?", "query_current", true,
                    Some(format!("{:.6}", current_a)), None,
                );
            }
            Err(e) => {
                let err_str = e.to_string();
                baseline.errors.push(format!("MEAS:CURR? sample {i}: {err_str}"));
                push_audit(
                    audit_seq, &mut audit, &baseline.axis_id,
                    "MEAS:CURR?", "query_current", true,
                    None, Some(err_str),
                );
            }
        }
    }

    if samples_a.is_empty() {
        baseline.errors.push("no valid MEAS:CURR? samples".into());
        attempt_cleanup(&mut transport, &mut baseline, audit_seq, &mut audit);
        return (baseline, audit);
    }

    // Compute statistics
    let samples_ma: Vec<f64> = samples_a.iter().map(|a| a * 1000.0).collect();
    let mean_ma = samples_ma.iter().sum::<f64>() / samples_ma.len() as f64;
    let variance: f64 = samples_ma.iter().map(|s| (s - mean_ma).powi(2)).sum::<f64>() / samples_ma.len() as f64;
    let std_ma = variance.sqrt();

    let n_samples = samples_ma.len();
    baseline.zero_readback_samples_ma = samples_ma;
    baseline.zero_readback_current_ma = mean_ma;
    baseline.zero_readback_std_ma = std_ma;
    baseline.zero_readback_current_a = mean_ma / 1000.0;

    let _ = runner.apply_zero_measured(mean_ma);
    events.push(zt_event(
        "mag_zero_measured",
        Some(&baseline.axis_id),
        Some(format!("zero={:.3}_mA std={:.3}_mA n={}", mean_ma, std_ma, n_samples)),
    ));

    // ── Lock-zero (software event) ──
    let _ = runner.apply_lock_zero();
    baseline.lock_zero_applied = true;
    events.push(zt_event(
        "mag_lock_zero_applied",
        Some(&baseline.axis_id),
        Some(format!("zero={:.3}_mA", mean_ma)),
    ));

    // ── Cleanup: OUTP 0, CURR 0.00000, SYST:LOC ──
    attempt_cleanup(&mut transport, &mut baseline, audit_seq, &mut audit);

    (baseline, audit)
}

// ── Cleanup ────────────────────────────────────────────────────────────

fn attempt_cleanup(
    transport: &mut MaynuoM8812Transport,
    baseline: &mut AxisZeroBaseline,
    audit_seq: &mut u64,
    audit: &mut Vec<CommandAuditEntry>,
) {
    let mut all_ok = true;

    // CURR 0 before OUTP 0: ramp current down before disabling output
    match transport.send_set_current(0.0) {
        Ok(()) => {
            push_audit(audit_seq, audit, &baseline.axis_id, "CURR 0.00000", "set_current", false, None, None);
        }
        Err(e) => {
            all_ok = false;
            let err = e.to_string();
            baseline.errors.push(format!("shutdown CURR 0: {err}"));
            push_audit(audit_seq, audit, &baseline.axis_id, "CURR 0.00000", "set_current", false, None, Some(err));
        }
    }

    match transport.send_set_output(false) {
        Ok(()) => {
            push_audit(audit_seq, audit, &baseline.axis_id, "OUTP 0", "set_output", false, None, None);
        }
        Err(e) => {
            all_ok = false;
            let err = e.to_string();
            baseline.errors.push(format!("shutdown OUTP 0: {err}"));
            push_audit(audit_seq, audit, &baseline.axis_id, "OUTP 0", "set_output", false, None, Some(err));
        }
    }

    match transport.send_set_local() {
        Ok(()) => {
            push_audit(audit_seq, audit, &baseline.axis_id, "SYST:LOC", "set_local", false, None, None);
        }
        Err(e) => {
            all_ok = false;
            let err = e.to_string();
            baseline.errors.push(format!("shutdown SYST:LOC: {err}"));
            push_audit(audit_seq, audit, &baseline.axis_id, "SYST:LOC", "set_local", false, None, Some(err));
        }
    }

    baseline.shutdown_succeeded = all_ok;
}

// ── Identity probing ───────────────────────────────────────────────────

fn probe_all_ports(
    candidates: &[MaynuoPortMetadata],
    config: &MaynuoSerialPortConfig,
    dry_run: bool,
    events: &mut Vec<ZeroBaselineEvent>,
) -> Vec<ProbeResult> {
    let mut results = Vec::new();
    for meta in candidates {
        if dry_run {
            results.push(ProbeResult {
                port_path: meta.port_path.clone(),
                idn_raw: None,
                sn: None,
                matched_axis_id: None,
                error: Some("dry-run: port not opened".into()),
            });
            events.push(zt_event("port_skipped_dry_run", None, Some(meta.port_path.clone())));
            continue;
        }

        let device_id = DeviceId::new(format!("probe-{}", sanitize_id(&meta.port_path)));
        let mut transport = match MaynuoM8812Transport::open(device_id, &meta.port_path, config.clone()) {
            Ok(t) => t,
            Err(e) => {
                results.push(ProbeResult {
                    port_path: meta.port_path.clone(),
                    idn_raw: None,
                    sn: None,
                    matched_axis_id: None,
                    error: Some(e.to_string()),
                });
                events.push(zt_event("port_open_failed", None, Some(meta.port_path.clone())));
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
                    matched_axis_id: None,
                    error: None,
                });
                events.push(zt_event("port_probed", None, Some(meta.port_path.clone())));
            }
            Err(e) => {
                results.push(ProbeResult {
                    port_path: meta.port_path.clone(),
                    idn_raw: None,
                    sn: None,
                    matched_axis_id: None,
                    error: Some(e.to_string()),
                });
                events.push(zt_event("port_probe_failed", None, Some(meta.port_path.clone())));
            }
        }
    }
    results
}

fn map_ports_to_axes(
    profile: &MaynuoAxesProfile,
    results: &[ProbeResult],
) -> Result<BTreeMap<String, (String, String)>, String> {
    let axes = [&profile.axes.x, &profile.axes.y, &profile.axes.z];
    let mut mapping: BTreeMap<String, (String, String)> = BTreeMap::new();

    for result in results {
        let sn = match &result.sn {
            Some(s) => s,
            None => continue,
        };
        for axis in &axes {
            let expected_sn = match expected_sn_from_idn(&axis.expected_idn) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if &expected_sn == sn {
                if mapping.contains_key(&axis.axis_id) {
                    return Err(format!("duplicate SN {sn} for axis {}", axis.axis_id));
                }
                mapping.insert(
                    axis.axis_id.clone(),
                    (result.port_path.clone(), result.idn_raw.clone().unwrap_or_default()),
                );
                break;
            }
        }
    }
    Ok(mapping)
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
        nonzero_current_attempted: false,
    });
}

fn compute_audit_invariants(audit: &[CommandAuditEntry], axes: &[AxisZeroBaseline]) -> AuditInvariants {
    use crate::types::PerAxisInvariants;
    use std::collections::BTreeMap;

    let nonzero = audit.iter().any(|e| e.nonzero_current_attempted);

    let outp_on = audit.iter().any(|e| e.command == "OUTP 1");

    // Global OUTP-after-CURR check
    let mut curr_zero_seen = false;
    let mut outp_after_curr = false;
    for e in audit {
        if e.command == "CURR 0.00000" {
            curr_zero_seen = true;
        }
        if e.command == "OUTP 1" && curr_zero_seen {
            outp_after_curr = true;
        }
    }

    let meas_count = audit
        .iter()
        .filter(|e| e.command == "MEAS:CURR?")
        .count() as u32;

    let readback_recorded = axes.iter().any(|a| !a.zero_readback_samples_ma.is_empty());
    let lock_recorded = axes.iter().any(|a| a.lock_zero_applied);
    let final_outp_off = audit.iter().any(|e| e.command == "OUTP 0");
    let final_curr_zero = audit.iter().any(|e| e.command == "CURR 0.00000");
    let final_local = audit.iter().any(|e| e.command == "SYST:LOC");

    // Per-axis invariants
    let mut per_axis: BTreeMap<String, PerAxisInvariants> = BTreeMap::new();
    for axis in axes {
        let axis_audit: Vec<&CommandAuditEntry> = audit
            .iter()
            .filter(|e| e.axis_id == axis.axis_id)
            .collect();

        let pa_outp_on = axis_audit.iter().any(|e| e.command == "OUTP 1");

        let mut pa_curr_zero_seen = false;
        let mut pa_outp_after_curr = false;
        for e in &axis_audit {
            if e.command == "CURR 0.00000" {
                pa_curr_zero_seen = true;
            }
            if e.command == "OUTP 1" && pa_curr_zero_seen {
                pa_outp_after_curr = true;
            }
        }

        let pa_meas_count = axis_audit
            .iter()
            .filter(|e| e.command == "MEAS:CURR?")
            .count() as u32;

        let pa_readback = !axis.zero_readback_samples_ma.is_empty();
        let pa_lock = axis.lock_zero_applied;
        let pa_outp_off = axis_audit.iter().any(|e| e.command == "OUTP 0");
        let pa_curr_zero = axis_audit.iter().any(|e| e.command == "CURR 0.00000");
        let pa_local = axis_audit.iter().any(|e| e.command == "SYST:LOC");

        let mut pa = PerAxisInvariants {
            outp_on_sent: pa_outp_on,
            outp_on_only_after_curr_zero: pa_outp_after_curr,
            measured_current_queries_sent: pa_meas_count,
            zero_readback_current_ma_recorded: pa_readback,
            lock_zero_event_recorded: pa_lock,
            final_output_off: pa_outp_off,
            final_current_zero_command_sent: pa_curr_zero,
            final_local_mode_requested: pa_local,
            all_pass: false,
        };
        pa.all_pass = pa.check();
        per_axis.insert(axis.axis_id.clone(), pa);
    }

    AuditInvariants {
        nonzero_current_sent: nonzero,
        outp_on_sent: outp_on,
        outp_on_only_after_curr_zero: outp_after_curr,
        measured_current_queries_sent: meas_count,
        zero_set_current_ma: 0.0,
        zero_readback_current_ma_recorded: readback_recorded,
        lock_zero_event_recorded: lock_recorded,
        recurrent_current_sent: false,
        recurrent_field_sent: false,
        final_output_off: final_outp_off,
        final_current_zero_command_sent: final_curr_zero,
        final_local_mode_requested: final_local,
        per_axis,
    }
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

fn zt_event(event_type: &str, axis_id: Option<&str>, detail: Option<String>) -> ZeroBaselineEvent {
    ZeroBaselineEvent {
        event_type: event_type.into(),
        timestamp: chrono_like_now(),
        axis_id: axis_id.map(|s| s.to_string()),
        detail,
    }
}

fn runner_events(runner: &MaynuoAxisRunner) -> Vec<ZeroBaselineEvent> {
    runner
        .events
        .iter()
        .map(|e| ZeroBaselineEvent {
            event_type: e.event_type.clone(),
            timestamp: e.timestamp.clone(),
            axis_id: Some(e.axis_id.clone()),
            detail: e.detail.clone(),
        })
        .collect()
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
    use crate::types::CommandAuditEntry;
    use odmr_mag::{MaynuoAxes, MaynuoSerialSettings};

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

    fn make_audit_entry(seq: u64, axis_id: &str, command: &str, class: &str, expects_response: bool) -> CommandAuditEntry {
        CommandAuditEntry {
            seq,
            timestamp: "2026-06-01T00:00:00Z".into(),
            axis_id: axis_id.into(),
            command: command.into(),
            command_class: class.into(),
            expects_response,
            response_preview: None,
            transport_error: None,
            allowed: true,
            nonzero_current_attempted: false,
        }
    }

    fn make_axis_baseline(axis_id: &str, has_readback: bool, has_lock: bool) -> AxisZeroBaseline {
        AxisZeroBaseline {
            axis_id: axis_id.into(),
            idn: String::new(),
            port_path: String::new(),
            sn_tail: String::new(),
            zero_set_current_ma: 0.0,
            zero_readback_samples_ma: if has_readback { vec![0.0, 0.1] } else { vec![] },
            zero_readback_current_ma: if has_readback { 0.05 } else { 0.0 },
            zero_readback_std_ma: if has_readback { 0.05 } else { 0.0 },
            zero_readback_current_a: 0.0,
            coil_constant_nt_per_ma: 143.26,
            lock_zero_applied: has_lock,
            output_was_on: true,
            shutdown_succeeded: true,
            errors: vec![],
        }
    }

    // ── Invariant tests ──

    #[test]
    fn invariants_pass_all_conditions_met() {
        let audit = vec![
            make_audit_entry(1, "mag_x", "SYST:REM", "set_remote", false),
            make_audit_entry(2, "mag_x", "VOLT 75", "set_voltage", false),
            make_audit_entry(3, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(4, "mag_x", "OUTP 1", "set_output", false),
            make_audit_entry(5, "mag_x", "MEAS:CURR?", "query_current", true),
            make_audit_entry(6, "mag_x", "OUTP 0", "set_output", false),
            make_audit_entry(7, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(8, "mag_x", "SYST:LOC", "set_local", false),
        ];
        let axes = vec![make_axis_baseline("mag_x", true, true)];
        let inv = compute_audit_invariants(&audit, &axes);
        assert!(inv.all_pass());
    }

    #[test]
    fn invariants_fail_outp_before_curr_zero() {
        let audit = vec![
            make_audit_entry(1, "mag_x", "OUTP 1", "set_output", false),
            make_audit_entry(2, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(3, "mag_x", "MEAS:CURR?", "query_current", true),
            make_audit_entry(4, "mag_x", "OUTP 0", "set_output", false),
            make_audit_entry(5, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(6, "mag_x", "SYST:LOC", "set_local", false),
        ];
        let axes = vec![make_axis_baseline("mag_x", true, true)];
        let inv = compute_audit_invariants(&audit, &axes);
        assert!(!inv.all_pass());
        assert!(!inv.outp_on_only_after_curr_zero);
    }

    #[test]
    fn invariants_fail_no_meas_curr() {
        let audit = vec![
            make_audit_entry(1, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(2, "mag_x", "OUTP 1", "set_output", false),
            make_audit_entry(3, "mag_x", "OUTP 0", "set_output", false),
            make_audit_entry(4, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(5, "mag_x", "SYST:LOC", "set_local", false),
        ];
        let axes = vec![make_axis_baseline("mag_x", true, true)];
        let inv = compute_audit_invariants(&audit, &axes);
        assert!(!inv.all_pass());
        assert_eq!(inv.measured_current_queries_sent, 0);
    }

    #[test]
    fn invariants_fail_no_outp_off() {
        let audit = vec![
            make_audit_entry(1, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(2, "mag_x", "OUTP 1", "set_output", false),
            make_audit_entry(3, "mag_x", "MEAS:CURR?", "query_current", true),
            make_audit_entry(4, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(5, "mag_x", "SYST:LOC", "set_local", false),
        ];
        let axes = vec![make_axis_baseline("mag_x", true, true)];
        let inv = compute_audit_invariants(&audit, &axes);
        assert!(!inv.all_pass());
        assert!(!inv.final_output_off);
    }

    #[test]
    fn invariants_fail_no_syst_loc() {
        let audit = vec![
            make_audit_entry(1, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(2, "mag_x", "OUTP 1", "set_output", false),
            make_audit_entry(3, "mag_x", "MEAS:CURR?", "query_current", true),
            make_audit_entry(4, "mag_x", "OUTP 0", "set_output", false),
            make_audit_entry(5, "mag_x", "CURR 0.00000", "set_current", false),
        ];
        let axes = vec![make_axis_baseline("mag_x", true, true)];
        let inv = compute_audit_invariants(&audit, &axes);
        assert!(!inv.all_pass());
        assert!(!inv.final_local_mode_requested);
    }

    #[test]
    fn invariants_fail_nonzero_current() {
        let mut bad_entry = make_audit_entry(4, "mag_x", "CURR 0.00001", "set_current", false);
        bad_entry.nonzero_current_attempted = true;
        let audit = vec![
            make_audit_entry(1, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(2, "mag_x", "OUTP 1", "set_output", false),
            make_audit_entry(3, "mag_x", "MEAS:CURR?", "query_current", true),
            bad_entry,
            make_audit_entry(5, "mag_x", "OUTP 0", "set_output", false),
            make_audit_entry(6, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(7, "mag_x", "SYST:LOC", "set_local", false),
        ];
        let axes = vec![make_axis_baseline("mag_x", true, true)];
        let inv = compute_audit_invariants(&audit, &axes);
        assert!(!inv.all_pass());
        assert!(inv.nonzero_current_sent);
    }

    #[test]
    fn invariants_fail_no_readback() {
        let audit = vec![
            make_audit_entry(1, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(2, "mag_x", "OUTP 1", "set_output", false),
            make_audit_entry(3, "mag_x", "MEAS:CURR?", "query_current", true),
            make_audit_entry(4, "mag_x", "OUTP 0", "set_output", false),
            make_audit_entry(5, "mag_x", "CURR 0.00000", "set_current", false),
            make_audit_entry(6, "mag_x", "SYST:LOC", "set_local", false),
        ];
        let axes = vec![make_axis_baseline("mag_x", false, false)]; // no readback, no lock
        let inv = compute_audit_invariants(&audit, &axes);
        assert!(!inv.all_pass());
        assert!(!inv.zero_readback_current_ma_recorded);
        assert!(!inv.lock_zero_event_recorded);
    }

    // ── Map ports to axes ──

    #[test]
    fn map_ports_x_axis_matched() {
        let profile = example_profile();
        let results = vec![ProbeResult {
            port_path: "COM4".into(),
            idn_raw: Some("MAYNUO,M8812,080020960220402020,V2.7".into()),
            sn: Some("080020960220402020".into()),
            matched_axis_id: None,
            error: None,
        }];
        let mapping = map_ports_to_axes(&profile, &results).unwrap();
        assert_eq!(mapping.len(), 1);
        assert_eq!(mapping.get("mag_x").unwrap().0, "COM4");
    }

    #[test]
    fn map_ports_unknown_sn_not_matched() {
        let profile = example_profile();
        let results = vec![ProbeResult {
            port_path: "COM9".into(),
            idn_raw: Some("MAYNUO,M8812,999999999999999999,V2.7".into()),
            sn: Some("999999999999999999".into()),
            matched_axis_id: None,
            error: None,
        }];
        let mapping = map_ports_to_axes(&profile, &results).unwrap();
        assert!(mapping.is_empty());
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
