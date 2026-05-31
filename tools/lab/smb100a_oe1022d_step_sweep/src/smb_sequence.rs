//! SMB100A software-stepped sweep sequence — step-level primitives.
//!
//! Functions are composed by app.rs which interleaves OE1022D acquisition.
//! No internal SMB100A sweep commands are used.

use crate::cli::Cli;
use crate::smb_safety::validate_lf_shape;
use crate::smb_transport::{do_smb_sweep_query, do_smb_sweep_set, SmbTransport};
use crate::timeline::{utc_now_ms, TimelineTracker};
use crate::types::*;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Safety limits
// ---------------------------------------------------------------------------

const HARD_MAX_RF_POWER_DBM: f64 = -10.0;
const HARD_MAX_POINTS: u64 = 7;
const HARD_MAX_FRAMES_PER_STEP: u64 = 5;
const HARD_MAX_FM_DEVIATION_HZ: f64 = 5_000_000.0;

pub fn validate_safety_limits(cli: &Cli) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();

    if cli.max_rf_power_dbm > HARD_MAX_RF_POWER_DBM {
        errors.push(format!(
            "max_rf_power_dbm {} exceeds hard limit of {} dBm",
            cli.max_rf_power_dbm, HARD_MAX_RF_POWER_DBM
        ));
    }
    if cli.rf_power_dbm > cli.max_rf_power_dbm {
        errors.push(format!(
            "rf_power_dbm {} exceeds max_rf_power_dbm {}",
            cli.rf_power_dbm, cli.max_rf_power_dbm
        ));
    }
    if cli.rf_points > HARD_MAX_POINTS {
        errors.push(format!(
            "rf_points {} exceeds hard limit of {}",
            cli.rf_points, HARD_MAX_POINTS
        ));
    }
    if cli.frames_per_step > HARD_MAX_FRAMES_PER_STEP {
        errors.push(format!(
            "frames_per_step {} exceeds hard limit of {}",
            cli.frames_per_step, HARD_MAX_FRAMES_PER_STEP
        ));
    }
    if cli.max_fm_deviation_hz > HARD_MAX_FM_DEVIATION_HZ {
        errors.push(format!(
            "max_fm_deviation_hz {} exceeds hard limit of {} Hz",
            cli.max_fm_deviation_hz, HARD_MAX_FM_DEVIATION_HZ
        ));
    }
    if cli.fm_deviation_hz > cli.max_fm_deviation_hz {
        errors.push(format!(
            "fm_deviation_hz {} exceeds max_fm_deviation_hz {}",
            cli.fm_deviation_hz, cli.max_fm_deviation_hz
        ));
    }
    if cli.rf_stop_hz < cli.rf_start_hz {
        errors.push(format!(
            "rf_stop_hz {} < rf_start_hz {}",
            cli.rf_stop_hz, cli.rf_start_hz
        ));
    }
    if cli.rf_start_hz <= 0.0 || !cli.rf_start_hz.is_finite() {
        errors.push(format!("rf_start_hz {} is invalid", cli.rf_start_hz));
    }
    if cli.rf_stop_hz <= 0.0 || !cli.rf_stop_hz.is_finite() {
        errors.push(format!("rf_stop_hz {} is invalid", cli.rf_stop_hz));
    }

    if cli.set_internal_lf {
        if let Err(e) = validate_lf_shape(&cli.lf_shape) {
            errors.push(format!("LF shape validation: {}", e));
        }
    }

    if !errors.is_empty() {
        return Err(format!("Safety limit violation: {}", errors.join("; ")));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Step plan
// ---------------------------------------------------------------------------

pub fn compute_step_plan(cli: &Cli) -> StepPlan {
    let n = cli.rf_points;
    let mut freqs = Vec::with_capacity(n as usize);
    if n == 1 {
        freqs.push(cli.rf_start_hz);
    } else {
        for i in 0..n {
            let frac = (i as f64) / ((n - 1) as f64);
            freqs.push(cli.rf_start_hz + frac * (cli.rf_stop_hz - cli.rf_start_hz));
        }
    }

    StepPlan {
        schema_version: "0.2.0".into(),
        kind: "software_stepped_rf_plan".into(),
        rf_start_hz: cli.rf_start_hz,
        rf_stop_hz: cli.rf_stop_hz,
        rf_points: cli.rf_points,
        frequencies_hz: freqs,
        frames_per_step: cli.frames_per_step,
        rf_power_dbm: cli.rf_power_dbm,
        fm_deviation_hz: cli.fm_deviation_hz,
        software_stepped: true,
        smb_internal_sweep_used: false,
    }
}

// ---------------------------------------------------------------------------
// Preflight + configure
// ---------------------------------------------------------------------------

pub struct SmbPreflightResult {
    pub idn: String,
    pub snapshot_before: Smb100aSnapshot,
    pub syst_err_before: Vec<ErrorQueueObservation>,
    pub preflight: PreflightCheck,
}

/// Connect SMB100A, run preflight queries, evaluate safety preconditions.
pub fn run_preflight(
    transport: &mut SmbTransport,
    audit: &mut Vec<CommandAuditEntry>,
    forbidden_attempted: &mut Vec<String>,
    delay_ms: u64,
    cli: &Cli,
) -> Result<SmbPreflightResult, String> {
    // *IDN?
    let idn = do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "*IDN?")?;

    // Preflight queries
    let preflight_cmds = vec![
        "OUTP?",
        "MOD:STAT?",
        "FREQ?",
        "POW?",
        "POW:ALC?",
        "FM:STAT?",
        "FM:SOUR?",
        "FM:DEV?",
        "LFO?",
        "LFO:FREQ?",
        "LFO:VOLT?",
        "LFO:SHAP?",
    ];

    let mut before_results = Vec::new();
    for q in &preflight_cmds {
        let resp = do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, q)?;
        before_results.push(SmbQueryResult {
            command: q.to_string(),
            response: resp,
        });
    }

    let snapshot_before = Smb100aSnapshot {
        schema_version: "0.2.0".into(),
        device_id: "smb100a_main".into(),
        idn: idn.clone(),
        queried_at_unix_ms: utc_now_ms(),
        queries: before_results.clone(),
        connection_closed: false,
    };

    // SYST:ERR? x3
    let mut syst_err_before = Vec::new();
    for attempt in 1..=3 {
        let resp =
            do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "SYST:ERR?")?;
        let clean = resp.trim() == "0,\"No error\"" || resp.trim().starts_with("0,");
        syst_err_before.push(ErrorQueueObservation {
            timestamp_unix_ms: utc_now_ms(),
            attempt,
            command: "SYST:ERR?".into(),
            response: resp,
            clean,
        });
    }

    let mut errors = Vec::new();
    let mut outp_off = false;
    let mut mod_off = false;
    for r in &before_results {
        if r.command == "OUTP?" {
            if r.response.trim() == "0" || r.response.trim().eq_ignore_ascii_case("OFF") {
                outp_off = true;
            } else {
                errors.push(format!("OUTP? = '{}' (expected OFF/0)", r.response));
            }
        }
        if r.command == "MOD:STAT?" {
            if r.response.trim() == "0" || r.response.trim().eq_ignore_ascii_case("OFF") {
                mod_off = true;
            } else {
                errors.push(format!("MOD:STAT? = '{}' (expected OFF/0)", r.response));
            }
        }
    }

    let error_queue_clean = syst_err_before.iter().all(|o| o.clean);
    if !error_queue_clean {
        for o in &syst_err_before {
            if !o.clean {
                errors.push(format!("SYST:ERR? = '{}'", o.response));
            }
        }
    }

    let operator_approval_present = cli.operator_approves_step_sweep;
    let passed = outp_off && mod_off && error_queue_clean && operator_approval_present;

    let preflight = PreflightCheck {
        passed,
        outp_off_before: outp_off,
        mod_stat_off_before: mod_off,
        error_queue_clean_before: error_queue_clean,
        operator_approval_present,
        power_within_limit: cli.rf_power_dbm <= cli.max_rf_power_dbm,
        points_within_limit: cli.rf_points <= HARD_MAX_POINTS,
        fm_deviation_within_limit: cli.fm_deviation_hz <= cli.max_fm_deviation_hz,
        warnings: Vec::new(),
        errors,
    };

    Ok(SmbPreflightResult {
        idn,
        snapshot_before,
        syst_err_before,
        preflight,
    })
}

/// Configure SMB100A common parameters (POW, FM, MOD, optionally LF).
/// OUTP remains OFF throughout.
pub fn configure_smb_common(
    transport: &mut SmbTransport,
    audit: &mut Vec<CommandAuditEntry>,
    forbidden_attempted: &mut Vec<String>,
    delay_ms: u64,
    cli: &Cli,
    tracker: &mut TimelineTracker,
) -> Result<(), String> {
    let approval = cli.operator_approves_step_sweep;

    // Set POW
    let pow_cmd = format!("POW {:.2}", cli.rf_power_dbm);
    do_smb_sweep_set(
        transport,
        audit,
        forbidden_attempted,
        delay_ms,
        &pow_cmd,
        false,
        false,
    )?;
    // Verify POW
    let pow_resp = do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "POW?")?;
    let pow_readback: f64 = pow_resp.trim().parse().unwrap_or(0.0);
    if (pow_readback - cli.rf_power_dbm).abs() > 0.1 {
        return Err(format!(
            "POW? readback {} differs from requested {}",
            pow_readback, cli.rf_power_dbm
        ));
    }

    // POW:ALC AUTO
    do_smb_sweep_set(
        transport,
        audit,
        forbidden_attempted,
        delay_ms,
        "POW:ALC AUTO",
        false,
        false,
    )?;

    // Optional LF config
    if cli.set_internal_lf {
        let lf_freq_cmd = format!("LFO:FREQ {:.1}", cli.lf_frequency_hz);
        do_smb_sweep_set(
            transport,
            audit,
            forbidden_attempted,
            delay_ms,
            &lf_freq_cmd,
            false,
            false,
        )?;
        let lf_shape_cmd = format!("LFO:SHAP {}", cli.lf_shape);
        do_smb_sweep_set(
            transport,
            audit,
            forbidden_attempted,
            delay_ms,
            &lf_shape_cmd,
            false,
            false,
        )?;
        let lf_volt_cmd = format!("LFO:VOLT {:.3}", cli.lf_voltage_v);
        do_smb_sweep_set(
            transport,
            audit,
            forbidden_attempted,
            delay_ms,
            &lf_volt_cmd,
            false,
            false,
        )?;
        // Verify LF params
        let lf_freq_resp =
            do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "LFO:FREQ?")?;
        let lf_shape_resp =
            do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "LFO:SHAP?")?;
        let lf_volt_resp =
            do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "LFO:VOLT?")?;
        // Reject if LFO is ON
        let lfo_resp = do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "LFO?")?;
        if lfo_resp.trim() == "1" || lfo_resp.trim().eq_ignore_ascii_case("ON") {
            return Err("LFO output is ON; must remain OFF in M3.2".into());
        }
        let _ = (lf_freq_resp, lf_shape_resp, lf_volt_resp); // verification recorded
    }

    // FM:SOUR INT
    do_smb_sweep_set(
        transport,
        audit,
        forbidden_attempted,
        delay_ms,
        "FM:SOUR INT",
        false,
        false,
    )?;
    let fm_sour_resp =
        do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "FM:SOUR?")?;
    if !fm_sour_resp.trim().eq_ignore_ascii_case("INT") {
        return Err(format!("FM:SOUR? = '{}', expected INT", fm_sour_resp));
    }

    // FM:DEV
    let fm_dev_cmd = format!("FM:DEV {:.0}", cli.fm_deviation_hz);
    do_smb_sweep_set(
        transport,
        audit,
        forbidden_attempted,
        delay_ms,
        &fm_dev_cmd,
        false,
        false,
    )?;
    let fm_dev_resp =
        do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "FM:DEV?")?;
    let fm_dev_readback: f64 = fm_dev_resp.trim().parse().unwrap_or(0.0);
    if (fm_dev_readback - cli.fm_deviation_hz).abs() > 1000.0 {
        return Err(format!(
            "FM:DEV? readback {} differs from requested {}",
            fm_dev_readback, cli.fm_deviation_hz
        ));
    }

    // FM:STAT ON
    do_smb_sweep_set(
        transport,
        audit,
        forbidden_attempted,
        delay_ms,
        "FM:STAT ON",
        true,
        approval,
    )?;
    let fm_stat_resp =
        do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "FM:STAT?")?;
    if fm_stat_resp.trim() != "1" && !fm_stat_resp.trim().eq_ignore_ascii_case("ON") {
        return Err(format!(
            "FM:STAT? = '{}', expected ON after FM:STAT ON",
            fm_stat_resp
        ));
    }
    tracker.record("fm_enabled", "smb100a", None);

    // MOD:STAT ON
    do_smb_sweep_set(
        transport,
        audit,
        forbidden_attempted,
        delay_ms,
        "MOD:STAT ON",
        true,
        approval,
    )?;
    let mod_stat_resp =
        do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "MOD:STAT?")?;
    if mod_stat_resp.trim() != "1" && !mod_stat_resp.trim().eq_ignore_ascii_case("ON") {
        return Err(format!(
            "MOD:STAT? = '{}', expected ON after MOD:STAT ON",
            mod_stat_resp
        ));
    }
    tracker.record("mod_enabled", "smb100a", None);

    Ok(())
}

// ---------------------------------------------------------------------------
// Per-step operations
// ---------------------------------------------------------------------------

/// OUTP OFF → set FREQ → verify FREQ → OUTP ON → verify OUTP ON.
pub fn execute_step_rf_on(
    transport: &mut SmbTransport,
    audit: &mut Vec<CommandAuditEntry>,
    forbidden_attempted: &mut Vec<String>,
    delay_ms: u64,
    frequency_hz: f64,
    approval: bool,
    tracker: &mut TimelineTracker,
) -> Result<f64, String> {
    // Ensure OUTP OFF before changing frequency
    let _ = do_smb_sweep_set(
        transport,
        audit,
        forbidden_attempted,
        delay_ms,
        "OUTP OFF",
        false,
        false,
    );

    // Set FREQ
    let freq_cmd = format!("FREQ {:.0}", frequency_hz);
    do_smb_sweep_set(
        transport,
        audit,
        forbidden_attempted,
        delay_ms,
        &freq_cmd,
        false,
        false,
    )?;

    // Verify FREQ
    let freq_resp = do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "FREQ?")?;
    let freq_readback: f64 = freq_resp.trim().parse().unwrap_or(0.0);
    if (freq_readback - frequency_hz).abs() > 1.0 {
        return Err(format!(
            "FREQ? readback {} differs from requested {}",
            freq_readback, frequency_hz
        ));
    }

    // OUTP ON
    do_smb_sweep_set(
        transport,
        audit,
        forbidden_attempted,
        delay_ms,
        "OUTP ON",
        true,
        approval,
    )?;
    tracker.record("rf_step_rf_on", "smb100a", None);

    // Verify OUTP ON
    std::thread::sleep(Duration::from_millis(100));
    let outp_resp = do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "OUTP?")?;
    let rf_on = outp_resp.trim() == "1" || outp_resp.trim().eq_ignore_ascii_case("ON");
    if !rf_on {
        return Err(format!(
            "OUTP? = '{}' after OUTP ON (expected ON/1)",
            outp_resp
        ));
    }

    Ok(freq_readback)
}

/// OUTP OFF → verify OUTP OFF.
pub fn execute_step_rf_off(
    transport: &mut SmbTransport,
    audit: &mut Vec<CommandAuditEntry>,
    forbidden_attempted: &mut Vec<String>,
    delay_ms: u64,
    tracker: &mut TimelineTracker,
) -> Result<bool, String> {
    do_smb_sweep_set(
        transport,
        audit,
        forbidden_attempted,
        delay_ms,
        "OUTP OFF",
        false,
        false,
    )?;
    tracker.record("rf_step_rf_off", "smb100a", None);

    let outp_resp = do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "OUTP?")?;
    let rf_off = outp_resp.trim() == "0" || outp_resp.trim().eq_ignore_ascii_case("OFF");
    Ok(rf_off)
}

// ---------------------------------------------------------------------------
// Final shutdown
// ---------------------------------------------------------------------------

pub fn run_final_shutdown(
    transport: &mut SmbTransport,
    audit: &mut Vec<CommandAuditEntry>,
    forbidden_attempted: &mut Vec<String>,
    delay_ms: u64,
    leave_fm: bool,
    tracker: &mut TimelineTracker,
) -> Result<Smb100aSnapshot, String> {
    // OUTP OFF
    let _ = do_smb_sweep_set(
        transport,
        audit,
        forbidden_attempted,
        delay_ms,
        "OUTP OFF",
        false,
        false,
    );
    let _outp_after = do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "OUTP?")?;

    // MOD:STAT OFF
    do_smb_sweep_set(
        transport,
        audit,
        forbidden_attempted,
        delay_ms,
        "MOD:STAT OFF",
        false,
        false,
    )?;
    tracker.record("mod_disabled", "smb100a", None);
    let _mod_after =
        do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "MOD:STAT?")?;

    // FM:STAT OFF (unless leave_fm_config_enabled)
    if !leave_fm {
        do_smb_sweep_set(
            transport,
            audit,
            forbidden_attempted,
            delay_ms,
            "FM:STAT OFF",
            false,
            false,
        )?;
        tracker.record("fm_disabled", "smb100a", None);
        let _ = do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "FM:STAT?")?;
    }

    // SYST:ERR? x3
    for _attempt in 1..=3 {
        let _ = do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, "SYST:ERR?");
    }

    // Build after snapshot
    let after_queries = vec!["OUTP?", "MOD:STAT?", "FREQ?", "POW?", "FM:STAT?", "LFO?"];
    let mut after_results = Vec::new();
    for q in &after_queries {
        if let Ok(resp) = do_smb_sweep_query(transport, audit, forbidden_attempted, delay_ms, q) {
            after_results.push(SmbQueryResult {
                command: q.to_string(),
                response: resp,
            });
        }
    }

    Ok(Smb100aSnapshot {
        schema_version: "0.2.0".into(),
        device_id: "smb100a_main".into(),
        idn: "see snapshot_before".into(),
        queried_at_unix_ms: utc_now_ms(),
        queries: after_results,
        connection_closed: false,
    })
}

// ---------------------------------------------------------------------------
// Full sweep orchestration (called by app.rs)
// ---------------------------------------------------------------------------
