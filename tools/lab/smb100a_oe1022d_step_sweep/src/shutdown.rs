use crate::smb_transport::SmbTransport;
use crate::timeline::utc_now_ms;
use crate::types::EmergencyShutdownEvidence;
use std::time::Duration;

/// Attempt emergency shutdown: OUTP OFF → MOD:STAT OFF → FM:STAT OFF.
/// Returns evidence and post-shutdown query responses.
pub fn attempt_emergency_shutdown(
    transport: &mut SmbTransport,
    delay_ms: u64,
    trigger_reason: &str,
) -> EmergencyShutdownEvidence {
    let shutdown_ts = utc_now_ms();
    let mut outp_sent = false;
    let mut mod_sent = false;
    let mut fm_sent = false;
    let mut outp_after: Option<String> = None;
    let mut mod_after: Option<String> = None;

    // 1. OUTP OFF
    if transport.send_no_response("OUTP OFF").is_ok() {
        outp_sent = true;
        std::thread::sleep(Duration::from_millis(delay_ms));
    }

    // 2. MOD:STAT OFF
    if transport.send_no_response("MOD:STAT OFF").is_ok() {
        mod_sent = true;
        std::thread::sleep(Duration::from_millis(delay_ms));
    }

    // 3. FM:STAT OFF
    if transport.send_no_response("FM:STAT OFF").is_ok() {
        fm_sent = true;
        std::thread::sleep(Duration::from_millis(delay_ms));
    }

    // Drain and verify
    transport.drain_buffer();

    if let Ok(resp) = transport.query("OUTP?") {
        transport.drain_buffer();
        outp_after = Some(resp);
    }
    if let Ok(resp) = transport.query("MOD:STAT?") {
        transport.drain_buffer();
        mod_after = Some(resp);
    }

    EmergencyShutdownEvidence {
        shutdown_attempted: true,
        shutdown_timestamp_unix_ms: shutdown_ts,
        outp_command_sent: Some(outp_sent),
        mod_command_sent: Some(mod_sent),
        fm_command_sent: Some(fm_sent),
        outp_query_after_shutdown: outp_after,
        mod_query_after_shutdown: mod_after,
        trigger_reason: trigger_reason.into(),
    }
}
