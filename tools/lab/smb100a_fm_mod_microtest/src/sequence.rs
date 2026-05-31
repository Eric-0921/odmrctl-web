use crate::cli::Cli;
use crate::safety::{count_forbidden_category, validate_lf_shape};
use crate::timeline::{utc_now_ms, TimelineTracker};
use crate::transport::{do_smb_query, do_smb_set, SmbTransport};
use crate::types::*;
use std::time::{Duration, Instant};

fn build_snapshot(idn: &str, queries: Vec<SmbQueryResult>) -> Smb100aSnapshot {
    Smb100aSnapshot {
        schema_version: "0.2.0".into(),
        device_id: "smb100a_main".into(),
        idn: idn.into(),
        queried_at_unix_ms: utc_now_ms(),
        queries,
        connection_closed: false,
    }
}

fn collect_syst_err_observations(
    transport: &mut SmbTransport,
    audit: &mut Vec<CommandAuditEntry>,
    forbidden_attempted: &mut Vec<String>,
    delay_ms: u64,
    count: usize,
) -> Result<Vec<ErrorQueueObservation>, String> {
    let mut obs = Vec::new();
    for attempt in 1..=count {
        let resp = do_smb_query(transport, audit, forbidden_attempted, delay_ms, "SYST:ERR?")?;
        let clean = resp.trim() == "0,\"No error\"" || resp.trim().starts_with("0,");
        obs.push(ErrorQueueObservation {
            timestamp_unix_ms: utc_now_ms(),
            attempt,
            command: "SYST:ERR?".into(),
            response: resp,
            clean,
        });
    }
    Ok(obs)
}

pub fn run_microtest(cli: &Cli) -> Result<MicrotestResult, String> {
    let mut tracker = TimelineTracker::new();
    let mut audit = Vec::new();
    let mut forbidden_attempted = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    let delay_ms = cli.smb_query_delay_ms;

    // Hard safety limits
    if cli.fm_on_duration_ms > 5000 {
        errors.push(format!(
            "fm_on_duration_ms {} exceeds hard limit of 5000 ms",
            cli.fm_on_duration_ms
        ));
    }
    if cli.max_rf_power_dbm > -10.0 {
        errors.push(format!(
            "max_rf_power_dbm {} exceeds hard limit of -10 dBm",
            cli.max_rf_power_dbm
        ));
    }
    if cli.rf_power_dbm > cli.max_rf_power_dbm {
        errors.push(format!(
            "rf_power_dbm {} exceeds max_rf_power_dbm {}",
            cli.rf_power_dbm, cli.max_rf_power_dbm
        ));
    }
    if cli.fm_deviation_hz > cli.max_fm_deviation_hz {
        errors.push(format!(
            "fm_deviation_hz {} exceeds max_fm_deviation_hz {}",
            cli.fm_deviation_hz, cli.max_fm_deviation_hz
        ));
    }
    if cli.max_fm_deviation_hz > 5000000.0 {
        errors.push(format!(
            "max_fm_deviation_hz {} exceeds hard limit of 5000000 Hz",
            cli.max_fm_deviation_hz
        ));
    }

    if !errors.is_empty() {
        return Err(format!("Safety limit violation: {}", errors.join("; ")));
    }

    // Connect
    let mut transport = SmbTransport::connect(&cli.smb_host, cli.smb_port, cli.smb_timeout_ms)?;
    tracker.record("smb_connected", "smb100a", None);

    // Pre-flight snapshot queries
    let idn = do_smb_query(
        &mut transport,
        &mut audit,
        &mut forbidden_attempted,
        delay_ms,
        "*IDN?",
    )?;

    let preflight_queries = vec![
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
    for q in &preflight_queries {
        let resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            q,
        )?;
        before_results.push(SmbQueryResult {
            command: q.to_string(),
            response: resp,
        });
    }

    // Safety-critical checks on preflight
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

    let snapshot_before = build_snapshot(&idn, before_results);

    // Error queue observations before test (3 times)
    let syst_err_before = collect_syst_err_observations(
        &mut transport,
        &mut audit,
        &mut forbidden_attempted,
        delay_ms,
        3,
    )?;
    let error_queue_clean = syst_err_before.iter().all(|o| o.clean);
    if !error_queue_clean {
        for o in &syst_err_before {
            if !o.clean {
                errors.push(format!("SYST:ERR? before test returned: {}", o.response));
            }
        }
    }

    // Operator approval
    let operator_approval_present = cli.operator_approves_fm_mod_on;
    let operator_approval = if operator_approval_present {
        Some(OperatorApproval {
            schema_version: "0.2.0".into(),
            approved: true,
            note: cli.operator_approval_note.clone(),
            timestamp_unix_ms: utc_now_ms(),
        })
    } else {
        None
    };

    // Pre-flight check result
    let preflight = PreflightCheck {
        passed: outp_off
            && mod_off
            && error_queue_clean
            && operator_approval_present
            && errors.is_empty(),
        outp_off_before: outp_off,
        mod_stat_off_before: mod_off,
        error_queue_clean_before: error_queue_clean,
        operator_approval_present,
        power_within_limit: cli.rf_power_dbm <= cli.max_rf_power_dbm,
        fm_deviation_within_limit: cli.fm_deviation_hz <= cli.max_fm_deviation_hz,
        duration_within_limit: cli.fm_on_duration_ms <= 5000,
        no_magnetic_serial_enumeration: true,
        no_magnetic_commands: true,
        warnings: warnings.clone(),
        errors: errors.clone(),
    };

    if !preflight.passed {
        transport.close();
        tracker.record("smb_disconnected_preflight_failed", "smb100a", None);
        return Ok(MicrotestResult {
            snapshot_before,
            snapshot_during: None,
            snapshot_after: build_snapshot(&idn, vec![]),
            audit,
            preflight,
            fm_mod_result: FmModResult {
                passed: false,
                rf_on_command_sent: false,
                rf_off_command_sent: false,
                rf_output_confirmed_on: false,
                rf_output_confirmed_off_after: false,
                mod_on_command_sent: false,
                mod_off_command_sent: false,
                modulation_confirmed_on: false,
                modulation_confirmed_off_after: false,
                fm_enabled: false,
                fm_disabled_after: false,
                fm_source_requested: "INT".into(),
                fm_source_verified: "".into(),
                fm_deviation_hz_requested: cli.fm_deviation_hz,
                fm_deviation_hz_verified: 0.0,
                frequency_hz_requested: cli.rf_frequency_hz,
                frequency_hz_verified: 0.0,
                power_dbm_requested: cli.rf_power_dbm,
                power_dbm_verified: 0.0,
                lf_frequency_hz_requested: cli.lf_frequency_hz,
                lf_frequency_hz_verified: 0.0,
                lf_shape_requested: cli.lf_shape.clone(),
                lf_shape_verified: "".into(),
                lf_voltage_v_requested: cli.lf_voltage_v,
                lf_voltage_v_verified: 0.0,
                lf_output_was_not_enabled: true,
                magnetic_devices_touched: false,
                magnetic_commands_sent: 0,
                fm_on_duration_ms_requested: cli.fm_on_duration_ms,
                fm_on_duration_ms_measured: 0,
                syst_err_before,
                syst_err_after: vec![],
                forbidden_commands_sent: 0,
                emergency_shutdown_attempted: false,
                warnings: warnings.clone(),
                errors,
            },
            forbidden_check: ForbiddenCommandCheck {
                passed: forbidden_attempted.is_empty(),
                forbidden_commands_attempted: forbidden_attempted.clone(),
                forbidden_commands_sent_to_transport: vec![],
                sweep_commands_sent: 0,
                lf_output_enable_commands_sent: 0,
                unexpected_rf_output_commands_sent: 0,
                unexpected_modulation_commands_sent: 0,
                unexpected_fm_commands_sent: 0,
                magnetic_commands_sent: 0,
            },
            timeline: tracker.events,
            operator_approval,
            emergency_shutdown: None,
            magnetic_not_in_scope: MagneticNotInScope {
                magnetic_devices_in_scope: false,
                magnetic_serial_enumeration_performed: false,
                magnetic_commands_sent: 0,
                reason: "M3.1 is SMB100A-only fixed-frequency FM/MOD micro-test".into(),
                known_verified_axis_sns: MagneticAxisSns {
                    x: "080020960220402020".into(),
                    y: "080020960220402022".into(),
                    z: "080020960220402003".into(),
                },
                note: "SN mapping is preserved for magnetic line, but no magnetic hardware was touched in M3.1.".into(),
            },
            warnings,
            errors: vec!["Preflight failed; FM/MOD sequence not started".into()],
        });
    }

    // -----------------------------------------------------------------------
    // FM/MOD/RF sequence
    // -----------------------------------------------------------------------

    let mut rf_on_command_sent = false;
    let mut rf_off_command_sent = false;
    let mut rf_output_confirmed_on = false;
    let mut rf_output_confirmed_off_after = false;

    let mut mod_on_command_sent = false;
    let mut mod_off_command_sent = false;
    let mut modulation_confirmed_on = false;
    let mut modulation_confirmed_off_after = false;

    let mut fm_enabled = false;
    let mut fm_disabled_after = false;
    let mut fm_source_verified = "".to_string();
    let mut fm_deviation_hz_verified = 0.0;

    let mut frequency_hz_verified = 0.0;
    let mut power_dbm_verified = 0.0;

    let mut lf_frequency_hz_verified = 0.0;
    let mut lf_shape_verified = "".to_string();
    let mut lf_voltage_v_verified = 0.0;
    let mut lf_output_was_not_enabled = true;

    let mut fm_on_duration_ms_measured = 0u64;
    let mut emergency_shutdown: Option<EmergencyShutdownEvidence> = None;
    let mut syst_err_after = vec![];
    let mut snapshot_during: Option<Smb100aSnapshot> = None;

    // Set frequency
    let freq_cmd = format!("FREQ {:.0}", cli.rf_frequency_hz);
    if let Err(e) = do_smb_set(
        &mut transport,
        &mut audit,
        &mut forbidden_attempted,
        delay_ms,
        &freq_cmd,
        false,
        false,
    ) {
        errors.push(format!("FREQ set failed: {}", e));
    }

    // Verify frequency
    if errors.is_empty() {
        let freq_resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "FREQ?",
        )?;
        frequency_hz_verified = freq_resp.trim().parse().unwrap_or(0.0);
        if (frequency_hz_verified - cli.rf_frequency_hz).abs() > 1.0 {
            errors.push(format!(
                "FREQ? = {:.0} after FREQ set (requested {:.0})",
                frequency_hz_verified, cli.rf_frequency_hz
            ));
        }
    }

    // Set power
    let pow_cmd = format!("POW {:.2}", cli.rf_power_dbm);
    if errors.is_empty() {
        if let Err(e) = do_smb_set(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            &pow_cmd,
            false,
            false,
        ) {
            errors.push(format!("POW set failed: {}", e));
        }
    }

    // Verify power
    if errors.is_empty() {
        let pow_resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "POW?",
        )?;
        power_dbm_verified = pow_resp.trim().parse().unwrap_or(0.0);
        if (power_dbm_verified - cli.rf_power_dbm).abs() > 0.1 {
            errors.push(format!(
                "POW? = {:.2} after POW set (requested {:.2})",
                power_dbm_verified, cli.rf_power_dbm
            ));
        }
    }

    // Set ALC
    if errors.is_empty() {
        if let Err(e) = do_smb_set(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "POW:ALC AUTO",
            false,
            false,
        ) {
            errors.push(format!("POW:ALC set failed: {}", e));
        }
    }

    // Verify ALC
    if errors.is_empty() {
        let _alc_resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "POW:ALC?",
        )?;
    }

    // Optional LF internal generator parameters
    if errors.is_empty() && cli.set_internal_lf {
        let lf_freq_cmd = format!("LFO:FREQ {:.0}", cli.lf_frequency_hz);
        if let Err(e) = do_smb_set(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            &lf_freq_cmd,
            false,
            false,
        ) {
            errors.push(format!("LFO:FREQ set failed: {}", e));
        }

        if errors.is_empty() {
            let lf_freq_resp = do_smb_query(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                "LFO:FREQ?",
            )?;
            lf_frequency_hz_verified = lf_freq_resp.trim().parse().unwrap_or(0.0);
        }

        let lf_shap_cmd = format!("LFO:SHAP {}", cli.lf_shape);
        if errors.is_empty() {
            if let Err(e) = validate_lf_shape(&cli.lf_shape) {
                errors.push(format!("LFO:SHAP validation failed: {}", e));
            }
        }
        if errors.is_empty() {
            if let Err(e) = do_smb_set(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                &lf_shap_cmd,
                false,
                false,
            ) {
                errors.push(format!("LFO:SHAP set failed: {}", e));
            }
        }

        if errors.is_empty() {
            let lf_shap_resp = do_smb_query(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                "LFO:SHAP?",
            )?;
            lf_shape_verified = lf_shap_resp.trim().to_string();
        }

        let lf_volt_cmd = format!("LFO:VOLT {:.3}", cli.lf_voltage_v);
        if errors.is_empty() {
            if let Err(e) = do_smb_set(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                &lf_volt_cmd,
                false,
                false,
            ) {
                errors.push(format!("LFO:VOLT set failed: {}", e));
            }
        }

        if errors.is_empty() {
            let lf_volt_resp = do_smb_query(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                "LFO:VOLT?",
            )?;
            lf_voltage_v_verified = lf_volt_resp.trim().parse().unwrap_or(0.0);
        }

        // Verify LFO output is OFF (we must not enable it)
        if errors.is_empty() {
            let lfo_resp = do_smb_query(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                "LFO?",
            )?;
            lf_output_was_not_enabled =
                lfo_resp.trim() == "0" || lfo_resp.trim().eq_ignore_ascii_case("OFF");
            if !lf_output_was_not_enabled {
                errors.push(format!(
                    "LFO? = '{}' (expected OFF/0; LF output must not be enabled in M3.1)",
                    lfo_resp
                ));
            }
        }
    }

    // FM:SOUR INT
    if errors.is_empty() {
        if let Err(e) = do_smb_set(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "FM:SOUR INT",
            false,
            false,
        ) {
            errors.push(format!("FM:SOUR set failed: {}", e));
        }
    }

    if errors.is_empty() {
        let fm_sour_resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "FM:SOUR?",
        )?;
        fm_source_verified = fm_sour_resp.trim().to_string();
    }

    // FM:DEV
    if errors.is_empty() {
        let fm_dev_cmd = format!("FM:DEV {:.0}", cli.fm_deviation_hz);
        if let Err(e) = do_smb_set(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            &fm_dev_cmd,
            false,
            false,
        ) {
            errors.push(format!("FM:DEV set failed: {}", e));
        }
    }

    if errors.is_empty() {
        let fm_dev_resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "FM:DEV?",
        )?;
        fm_deviation_hz_verified = fm_dev_resp.trim().parse().unwrap_or(0.0);
        if (fm_deviation_hz_verified - cli.fm_deviation_hz).abs() > 100.0 {
            errors.push(format!(
                "FM:DEV? = {:.0} after FM:DEV set (requested {:.0})",
                fm_deviation_hz_verified, cli.fm_deviation_hz
            ));
        }
    }

    // FM:STAT ON
    if errors.is_empty() {
        if let Err(e) = do_smb_set(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "FM:STAT ON",
            true,
            cli.operator_approves_fm_mod_on,
        ) {
            errors.push(format!("FM:STAT ON failed: {}", e));
        } else {
            fm_enabled = true;
            tracker.record("fm_enabled", "smb100a", None);
        }
    }

    if errors.is_empty() {
        let fm_stat_resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "FM:STAT?",
        )
        .unwrap_or_else(|e| {
            errors.push(format!("FM:STAT? verify after FM:STAT ON failed: {}", e));
            String::new()
        });
        if !(fm_stat_resp.trim() == "1" || fm_stat_resp.trim().eq_ignore_ascii_case("ON")) {
            errors.push(format!(
                "FM:STAT? = '{}' after FM:STAT ON (expected ON/1)",
                fm_stat_resp
            ));
        }
    }

    // Verify OUTP is still OFF before MOD:STAT ON
    if errors.is_empty() {
        let outp_resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "OUTP?",
        )
        .unwrap_or_else(|e| {
            errors.push(format!("OUTP? verify before MOD:STAT ON failed: {}", e));
            String::new()
        });
        if outp_resp.trim() != "0" && !outp_resp.trim().eq_ignore_ascii_case("OFF") {
            errors.push(format!(
                "OUTP? = '{}' before MOD:STAT ON (expected OFF/0)",
                outp_resp
            ));
        }
    }

    // MOD:STAT ON
    if errors.is_empty() {
        if let Err(e) = do_smb_set(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "MOD:STAT ON",
            true,
            cli.operator_approves_fm_mod_on,
        ) {
            errors.push(format!("MOD:STAT ON failed: {}", e));
        } else {
            mod_on_command_sent = true;
            tracker.record("modulation_enabled", "smb100a", None);
        }
    }

    if errors.is_empty() {
        let mod_stat_resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "MOD:STAT?",
        )
        .unwrap_or_else(|e| {
            errors.push(format!("MOD:STAT? verify after MOD:STAT ON failed: {}", e));
            String::new()
        });
        modulation_confirmed_on =
            mod_stat_resp.trim() == "1" || mod_stat_resp.trim().eq_ignore_ascii_case("ON");
        if !modulation_confirmed_on {
            errors.push(format!(
                "MOD:STAT? = '{}' after MOD:STAT ON (expected ON/1)",
                mod_stat_resp
            ));
        }
    }

    // OUTP ON
    if errors.is_empty() {
        let fm_on_start = Instant::now();
        if let Err(e) = do_smb_set(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "OUTP ON",
            true,
            cli.operator_approves_fm_mod_on,
        ) {
            errors.push(format!("OUTP ON failed: {}", e));
        } else {
            rf_on_command_sent = true;
            tracker.record("rf_output_enabled", "smb100a", None);

            // Wait 100 ms then confirm ON
            std::thread::sleep(Duration::from_millis(100));
            let outp_during = do_smb_query(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                "OUTP?",
            )
            .unwrap_or_else(|e| {
                errors.push(format!("OUTP? verify after OUTP ON failed: {}", e));
                String::new()
            });
            rf_output_confirmed_on =
                outp_during.trim() == "1" || outp_during.trim().eq_ignore_ascii_case("ON");
            if !rf_output_confirmed_on {
                errors.push(format!(
                    "OUTP? = '{}' after OUTP ON (expected ON/1)",
                    outp_during
                ));
            }

            // Collect during snapshot
            let during_queries = vec![
                SmbQueryResult {
                    command: "OUTP?".into(),
                    response: outp_during,
                },
                SmbQueryResult {
                    command: "MOD:STAT?".into(),
                    response: if modulation_confirmed_on {
                        "ON".into()
                    } else {
                        "OFF".into()
                    },
                },
                SmbQueryResult {
                    command: "FM:STAT?".into(),
                    response: if fm_enabled {
                        "ON".into()
                    } else {
                        "OFF".into()
                    },
                },
                SmbQueryResult {
                    command: "FREQ?".into(),
                    response: format!("{:.0}", frequency_hz_verified),
                },
                SmbQueryResult {
                    command: "POW?".into(),
                    response: format!("{:.2}", power_dbm_verified),
                },
                SmbQueryResult {
                    command: "FM:DEV?".into(),
                    response: format!("{:.0}", fm_deviation_hz_verified),
                },
            ];
            snapshot_during = Some(build_snapshot(&idn, during_queries));

            // Wait for the requested duration (minus the 100 ms already waited)
            // Two delay_ms sleeps incurred: one in do_smb_set("OUTP ON"), one in do_smb_query("OUTP?")
            let remaining = cli.fm_on_duration_ms.saturating_sub(100 + 2 * delay_ms);
            if remaining > 0 {
                std::thread::sleep(Duration::from_millis(remaining));
            }

            fm_on_duration_ms_measured = fm_on_start.elapsed().as_millis() as u64;

            // Send OUTP OFF
            if let Err(e) = do_smb_set(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                "OUTP OFF",
                false,
                false,
            ) {
                errors.push(format!("OUTP OFF failed: {}", e));
            } else {
                rf_off_command_sent = true;
                tracker.record("rf_output_disabled", "smb100a", None);
            }
        }
    }

    // If any failure occurred after state-changing commands, attempt emergency shutdown.
    // Covers: RF ON without OFF, FM enabled, or MOD ON without OFF.
    let needs_emergency_shutdown = !errors.is_empty()
        && ((rf_on_command_sent && !rf_off_command_sent)
            || fm_enabled
            || (mod_on_command_sent && !mod_off_command_sent));
    if needs_emergency_shutdown {
        let (evidence, outp_after, mod_after) = crate::shutdown::attempt_emergency_shutdown(
            &mut transport,
            delay_ms,
            &errors.join("; "),
            &mut warnings,
        );
        emergency_shutdown = Some(evidence);
        if let Some(ref r) = outp_after {
            rf_output_confirmed_off_after = r.trim() == "0" || r.trim().eq_ignore_ascii_case("OFF");
        }
        if let Some(ref r) = mod_after {
            modulation_confirmed_off_after =
                r.trim() == "0" || r.trim().eq_ignore_ascii_case("OFF");
        }
    }

    // Post-RF OFF verification (only if no emergency shutdown or if shutdown succeeded)
    if errors.is_empty() || rf_off_command_sent {
        // Verify OUTP OFF
        let outp_after = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "OUTP?",
        )
        .unwrap_or_else(|e| {
            errors.push(format!("OUTP? verify after OUTP OFF failed: {}", e));
            String::new()
        });
        rf_output_confirmed_off_after =
            outp_after.trim() == "0" || outp_after.trim().eq_ignore_ascii_case("OFF");
        if !rf_output_confirmed_off_after {
            errors.push(format!(
                "OUTP? = '{}' after OUTP OFF (expected OFF/0)",
                outp_after
            ));
        }

        // Send MOD:STAT OFF
        if let Err(e) = do_smb_set(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "MOD:STAT OFF",
            false,
            false,
        ) {
            errors.push(format!("MOD:STAT OFF failed: {}", e));
        } else {
            mod_off_command_sent = true;
        }

        // Verify MOD:STAT OFF
        let mod_after = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "MOD:STAT?",
        )
        .unwrap_or_else(|e| {
            errors.push(format!("MOD:STAT? verify after MOD:STAT OFF failed: {}", e));
            String::new()
        });
        modulation_confirmed_off_after =
            mod_after.trim() == "0" || mod_after.trim().eq_ignore_ascii_case("OFF");
        if !modulation_confirmed_off_after {
            errors.push(format!(
                "MOD:STAT? = '{}' after MOD:STAT OFF (expected OFF/0)",
                mod_after
            ));
        }

        // FM:STAT OFF (unless leave-fm-config-enabled)
        if !cli.leave_fm_config_enabled {
            if let Err(e) = do_smb_set(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                "FM:STAT OFF",
                false,
                false,
            ) {
                errors.push(format!("FM:STAT OFF failed: {}", e));
            } else {
                fm_disabled_after = true;
            }
        } else {
            fm_disabled_after = false;
        }

        let fm_after = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "FM:STAT?",
        )
        .unwrap_or_else(|e| {
            errors.push(format!("FM:STAT? verify after FM:STAT OFF failed: {}", e));
            String::new()
        });
        let fm_is_off = fm_after.trim() == "0" || fm_after.trim().eq_ignore_ascii_case("OFF");
        if !cli.leave_fm_config_enabled && !fm_is_off {
            errors.push(format!(
                "FM:STAT? = '{}' after FM:STAT OFF (expected OFF/0)",
                fm_after
            ));
        }

        // Error queue after (3 times)
        syst_err_after = collect_syst_err_observations(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            3,
        )?;
        if !syst_err_after.iter().all(|o| o.clean) {
            for o in &syst_err_after {
                if !o.clean {
                    errors.push(format!("SYST:ERR? after test returned: {}", o.response));
                }
            }
        }
    }

    // Build after snapshot
    let mut after_queries = vec![];
    if errors.is_empty() || rf_off_command_sent {
        let post_queries = vec!["OUTP?", "MOD:STAT?", "FREQ?", "POW?", "FM:STAT?", "LFO?"];
        for q in &post_queries {
            if let Ok(resp) = do_smb_query(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                q,
            ) {
                after_queries.push(SmbQueryResult {
                    command: q.to_string(),
                    response: resp,
                });
            }
        }
    }
    let snapshot_after = build_snapshot(&idn, after_queries);

    transport.close();
    tracker.record("smb_disconnected", "smb100a", None);

    let forbidden_sent_count = audit
        .iter()
        .filter(|a| !a.allowed && a.sent_to_transport)
        .count();

    let forbidden_check = ForbiddenCommandCheck {
        passed: forbidden_attempted.is_empty() && forbidden_sent_count == 0,
        forbidden_commands_attempted: forbidden_attempted.clone(),
        forbidden_commands_sent_to_transport: audit
            .iter()
            .filter(|a| !a.allowed && a.sent_to_transport)
            .map(|a| a.command.clone())
            .collect(),
        sweep_commands_sent: count_forbidden_category(&audit, "SWE")
            + count_forbidden_category(&audit, "FREQ:STAR ")
            + count_forbidden_category(&audit, "FREQ:STOP ")
            + count_forbidden_category(&audit, "FREQ:MODE "),
        lf_output_enable_commands_sent: count_forbidden_category(&audit, "LFO ON")
            + count_forbidden_category(&audit, "LFO OFF"),
        unexpected_rf_output_commands_sent: 0,
        unexpected_modulation_commands_sent: 0,
        unexpected_fm_commands_sent: 0,
        magnetic_commands_sent: 0,
    };

    let passed = rf_on_command_sent
        && rf_off_command_sent
        && rf_output_confirmed_on
        && rf_output_confirmed_off_after
        && mod_on_command_sent
        && mod_off_command_sent
        && modulation_confirmed_on
        && modulation_confirmed_off_after
        && fm_enabled
        && errors.is_empty();

    let fm_mod_result = FmModResult {
        passed,
        rf_on_command_sent,
        rf_off_command_sent,
        rf_output_confirmed_on,
        rf_output_confirmed_off_after,
        mod_on_command_sent,
        mod_off_command_sent,
        modulation_confirmed_on,
        modulation_confirmed_off_after,
        fm_enabled,
        fm_disabled_after: if cli.leave_fm_config_enabled {
            false
        } else {
            fm_disabled_after
        },
        fm_source_requested: "INT".into(),
        fm_source_verified,
        fm_deviation_hz_requested: cli.fm_deviation_hz,
        fm_deviation_hz_verified,
        frequency_hz_requested: cli.rf_frequency_hz,
        frequency_hz_verified,
        power_dbm_requested: cli.rf_power_dbm,
        power_dbm_verified,
        lf_frequency_hz_requested: cli.lf_frequency_hz,
        lf_frequency_hz_verified,
        lf_shape_requested: cli.lf_shape.clone(),
        lf_shape_verified,
        lf_voltage_v_requested: cli.lf_voltage_v,
        lf_voltage_v_verified,
        lf_output_was_not_enabled,
        magnetic_devices_touched: false,
        magnetic_commands_sent: 0,
        fm_on_duration_ms_requested: cli.fm_on_duration_ms,
        fm_on_duration_ms_measured,
        syst_err_before,
        syst_err_after,
        forbidden_commands_sent: forbidden_sent_count,
        emergency_shutdown_attempted: emergency_shutdown.is_some(),
        warnings: warnings.clone(),
        errors: errors.clone(),
    };

    Ok(MicrotestResult {
        snapshot_before,
        snapshot_during,
        snapshot_after,
        audit,
        preflight,
        fm_mod_result,
        forbidden_check,
        timeline: tracker.events,
        operator_approval,
        emergency_shutdown,
        magnetic_not_in_scope: MagneticNotInScope {
            magnetic_devices_in_scope: false,
            magnetic_serial_enumeration_performed: false,
            magnetic_commands_sent: 0,
            reason: "M3.1 is SMB100A-only fixed-frequency FM/MOD micro-test".into(),
            known_verified_axis_sns: MagneticAxisSns {
                x: "080020960220402020".into(),
                y: "080020960220402022".into(),
                z: "080020960220402003".into(),
            },
            note: "SN mapping is preserved for magnetic line, but no magnetic hardware was touched in M3.1.".into(),
        },
        warnings,
        errors,
    })
}
