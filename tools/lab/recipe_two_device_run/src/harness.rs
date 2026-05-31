//! Harness fake mode: fake SMB100A + fake OE1022D with deterministic output.

use crate::types::*;
use odmr_device::{DeviceResponse, FakeDevice};
use odmr_oe1022d::RALL_FRAME_BYTES;
use odmr_smb100a::fake::FakeSmb100a;
use std::collections::HashMap;

/// Create a fake SMB100A with default safe state.
pub fn create_fake_smb100a() -> FakeSmb100a {
    FakeSmb100a::new(odmr_types::DeviceId::new("smb100a_fake"))
}

/// Create a fake OE1022D.
pub fn create_fake_oe1022d() -> odmr_oe1022d::fake::FakeOe1022d {
    odmr_oe1022d::fake::FakeOe1022d::new(odmr_types::DeviceId::new("oe1022d_fake"))
}

/// Run a query against the fake SMB100A and record the audit entry.
pub fn fake_smb_query(
    smb: &mut FakeSmb100a,
    cmd: &str,
    ts: u64,
) -> (String, M3_4CommandAuditEntry) {
    let response = match smb.query(cmd) {
        Ok(DeviceResponse::Value(v)) => v,
        Ok(DeviceResponse::Ack) => "ACK".into(),
        Ok(DeviceResponse::Error(e)) => format!("ERR:{}", e),
        Err(e) => format!("ERR:{:?}", e),
    };

    let allowed = !cmd.contains(';');
    let audit = M3_4CommandAuditEntry {
        timestamp_unix_ms: ts,
        device_id: "smb100a".into(),
        command: cmd.to_string(),
        command_class: if cmd.ends_with('?') { "query" } else { "set" }.into(),
        allowed,
        sent_to_transport: true,
        rejection_reason: if allowed {
            None
        } else {
            Some("semicolon rejected".into())
        },
        response_preview: Some(response.clone()),
        transport_error: None,
        safety_relevant: matches!(
            cmd.trim(),
            "OUTP?"
                | "MOD:STAT?"
                | "SYST:ERR?"
                | "OUTP ON"
                | "OUTP OFF"
                | "MOD:STAT ON"
                | "MOD:STAT OFF"
                | "FM:STAT ON"
                | "FM:STAT OFF"
        ),
    };

    (response, audit)
}

/// Run a set command against the fake SMB100A and record the audit entry.
pub fn fake_smb_set(smb: &mut FakeSmb100a, cmd: &str, ts: u64) -> (String, M3_4CommandAuditEntry) {
    let response = match smb.send_command(cmd) {
        Ok(DeviceResponse::Ack) => "ACK".into(),
        Ok(DeviceResponse::Value(v)) => v,
        Ok(DeviceResponse::Error(e)) => format!("ERR:{}", e),
        Err(e) => format!("ERR:{:?}", e),
    };

    let allowed = !cmd.contains(';');
    let audit = M3_4CommandAuditEntry {
        timestamp_unix_ms: ts,
        device_id: "smb100a".into(),
        command: cmd.to_string(),
        command_class: "set".into(),
        allowed,
        sent_to_transport: true,
        rejection_reason: if allowed {
            None
        } else {
            Some("semicolon rejected".into())
        },
        response_preview: Some(response.clone()),
        transport_error: None,
        safety_relevant: matches!(
            cmd.trim(),
            "OUTP ON" | "OUTP OFF" | "MOD:STAT ON" | "MOD:STAT OFF" | "FM:STAT ON" | "FM:STAT OFF"
        ),
    };

    (response, audit)
}

/// Query the fake SMB100A state as a snapshot (for before/after artifacts).
pub fn fake_smb_snapshot(
    smb: &mut FakeSmb100a,
    ts: u64,
) -> (HashMap<String, String>, Vec<M3_4CommandAuditEntry>) {
    let queries = &[
        "*IDN?",
        "OUTP?",
        "FREQ?",
        "POW?",
        "POW:ALC?",
        "MOD:STAT?",
        "FM:STAT?",
        "FM:SOUR?",
        "FM:DEV?",
        "SYST:ERR?",
    ];
    let mut map = HashMap::new();
    let mut audit = Vec::new();
    for q in queries {
        let (resp, entry) = fake_smb_query(smb, q, ts);
        map.insert(q.to_string(), resp);
        audit.push(entry);
    }
    (map, audit)
}

/// Generate a deterministic 12288-byte RALL? frame for harness mode.
/// B-channel X and Y values are deterministic sine waves based on indices.
pub fn generate_deterministic_rall_frame(
    repeat_index: u64,
    step_index: u64,
    _frame_index: u64,
    seed: u64,
) -> Vec<u8> {
    let mut buf = vec![0u8; RALL_FRAME_BYTES];

    // Measurements section: 20 params × 50 samples × 4 bytes (f32 LE) = 4000 bytes
    // Param 0: B_X (50 samples)
    // Param 1: B_Y (50 samples)
    // Params 2-19: other (set to zero for determinism)

    let base_phase = (repeat_index * 1000 + step_index * 50 + seed) as f64 * 0.01;
    let b_x_amplitude = 10.0 + (step_index % 3) as f64;
    let b_y_amplitude = 8.0 + (repeat_index % 2) as f64;

    for sample in 0..50 {
        let phase = base_phase + sample as f64 * 0.125;
        let bx_val = (b_x_amplitude * phase.sin()) as f32;
        let by_val = (b_y_amplitude * phase.cos()) as f32;

        // Param 0 (B_X): offset 0-199
        let bx_offset = sample * 4; // Param 0 starts at offset 0
        buf[bx_offset..bx_offset + 4].copy_from_slice(&bx_val.to_le_bytes());

        // Param 1 (B_Y): offset 200-399
        let by_offset = 200 + sample * 4; // Param 1 starts at offset 200 (50*4)
        buf[by_offset..by_offset + 4].copy_from_slice(&by_val.to_le_bytes());
    }

    buf
}

type IndexedFrames = Vec<(usize, Vec<u8>)>;

/// Inject simulated parse failures into a set of frames.
/// Returns (good_frames, failed_frames) with indices.
pub fn inject_parse_failures(
    frames: &[Vec<u8>],
    failure_rate: f64,
) -> (IndexedFrames, IndexedFrames) {
    let mut good = Vec::new();
    let mut failed = Vec::new();

    for (i, frame) in frames.iter().enumerate() {
        // Use a deterministic pattern: every Nth frame fails based on rate
        let fail_this = (i as f64 * 1.73205).fract() < failure_rate;
        if fail_this && i > 0 {
            // Corrupt the frame by truncating it
            let mut corrupted = frame.clone();
            corrupted.truncate(frame.len() / 2);
            failed.push((i, corrupted));
        } else {
            good.push((i, frame.clone()));
        }
    }

    (good, failed)
}
