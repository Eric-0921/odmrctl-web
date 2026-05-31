//! Emergency shutdown path for the M3.1 FM/MOD micro-test tool.

use crate::timeline::utc_now_ms;
use crate::transport::SmbTransport;
use crate::types::EmergencyShutdownEvidence;
use std::time::Duration;

/// Attempt emergency shutdown sequence: OUTP OFF → MOD:STAT OFF → FM:STAT OFF.
/// Returns shutdown evidence and optional query responses for OUTP? and MOD:STAT?.
pub fn attempt_emergency_shutdown(
    transport: &mut SmbTransport,
    delay_ms: u64,
    trigger_reason: &str,
    warnings: &mut Vec<String>,
) -> (EmergencyShutdownEvidence, Option<String>, Option<String>) {
    let shutdown_ts = utc_now_ms();
    let mut outp_sent = false;
    let mut mod_sent = false;
    let mut fm_sent = false;
    let mut outp_after = None;
    let mut mod_after = None;

    if let Err(e) = transport.send_no_response("OUTP OFF") {
        warnings.push(format!("Emergency OUTP OFF transport error: {}", e));
    } else {
        outp_sent = true;
    }
    std::thread::sleep(Duration::from_millis(delay_ms));
    if let Err(e) = transport.send_no_response("MOD:STAT OFF") {
        warnings.push(format!("Emergency MOD:STAT OFF transport error: {}", e));
    } else {
        mod_sent = true;
    }
    std::thread::sleep(Duration::from_millis(delay_ms));
    if let Err(e) = transport.send_no_response("FM:STAT OFF") {
        warnings.push(format!("Emergency FM:STAT OFF transport error: {}", e));
    } else {
        fm_sent = true;
    }
    std::thread::sleep(Duration::from_millis(delay_ms));

    // Drain any stale ACKs before verification queries
    transport.drain_buffer();
    if let Ok(resp) = transport.query("OUTP?") {
        transport.drain_buffer();
        outp_after = Some(resp);
    }
    if let Ok(resp) = transport.query("MOD:STAT?") {
        transport.drain_buffer();
        mod_after = Some(resp);
    }

    let evidence = EmergencyShutdownEvidence {
        shutdown_attempted: true,
        shutdown_timestamp_unix_ms: shutdown_ts,
        outp_command_sent: Some(outp_sent),
        mod_command_sent: Some(mod_sent),
        fm_command_sent: Some(fm_sent),
        outp_query_after_shutdown: outp_after.clone(),
        mod_query_after_shutdown: mod_after.clone(),
        trigger_reason: trigger_reason.into(),
    };

    (evidence, outp_after, mod_after)
}
