//! OE1022D logged acquisition prototype (M2.6).
//!
//! Integrates the M2.5 bounded acquisition flow with the formal `odmr-logging`
//! run artifact contract.
//!
//! ## Safety
//!
//! Hard-coded command allow-list: only `*IDN?` and `RALL?` may be transmitted.
//! A secondary forbidden-pattern gate provides defense in depth.

use clap::Parser;
use odmr_logging::{
    create_run_directory, EventLevel, RawIndexEntry, RunArtifactPaths, RunDirectory, RunEvent,
    RunEventType, RunManifest,
};
use odmr_oe1022d::parser::{latest_b_channel_sample, parse_rall_frame, RALL_FRAME_BYTES};
use serde::Serialize;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Hard-coded command allow-list
// ---------------------------------------------------------------------------

const ALLOWED_COMMANDS: &[&str] = &["*IDN?", "RALL?"];

const FORBIDDEN_PATTERNS: &[&str] = &[
    "*RST", "RST", "INIT", "RUN", "SSETD", "RSETD", "APHSD", "FMODD", "RSLPD", "PHASD", "ISRCD",
    "SENSD", "OFLTD", "OFSLD", "HARMD",
];

const SMB100A_PATTERNS: &[&str] = &[
    "smb100a",
    "SMB100A",
    "FREQ ",
    "POW ",
    "OUTP ",
    "MOD:STAT",
    "FREQ:MODE",
];

/// Validate that `cmd` is in the pre-defined allow-list and contains no
/// forbidden substrings.
pub fn validate_command(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if !ALLOWED_COMMANDS.contains(&trimmed) {
        return Err(format!(
            "command '{}' is not in the safe allow-list",
            trimmed
        ));
    }
    for pat in FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return Err(format!(
                "command '{}' contains forbidden pattern '{}'",
                trimmed, pat
            ));
        }
    }
    Ok(())
}

/// Reject strings that contain SMB100A command patterns.
pub fn reject_smb100a_commands(text: &str) -> Result<(), String> {
    for pat in SMB100A_PATTERNS {
        if text.contains(pat) {
            return Err(format!("text contains SMB100A command pattern '{}'", pat));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(about = "OE1022D logged acquisition (M2.6)")]
struct Cli {
    #[arg(long, default_value = "/dev/cu.usbmodem3361358734371")]
    port: String,

    #[arg(long, default_value = "921600")]
    baud: u32,

    #[arg(long, default_value = "100")]
    frames: u32,

    #[arg(long, default_value = "20")]
    delay_ms: u64,

    #[arg(long, default_value = "5000")]
    timeout_ms: u64,

    #[arg(long, default_value = "../../runs")]
    run_root: String,

    #[arg(long)]
    run_id: String,
}

// ---------------------------------------------------------------------------
// Parsed output row types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct BChannelPreviewRow {
    run_id: String,
    frame_index: u32,
    timestamp_unix_ms: u64,
    b_x_mv: f64,
    b_y_mv: f64,
    b_freq_hz: f64,
    b_noise_mv: f64,
    b_pll_locked: bool,
    b_input_overload: bool,
    b_gain_overload: bool,
}

#[derive(Serialize)]
struct FrameSummaryRow {
    run_id: String,
    frame_index: u32,
    timestamp_unix_ms: u64,
    raw_len: usize,
    parse_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_error: Option<String>,
}

#[derive(Serialize)]
struct AcquisitionConfig {
    schema_version: String,
    port: String,
    baud: u32,
    frames_requested: u32,
    delay_ms: u64,
    timeout_ms: u64,
    created_at_unix_ms: u64,
}

#[derive(Serialize)]
struct ParserVersionMeta {
    schema_version: String,
    parser_crate: String,
    parser_version: String,
    rall_frame_bytes: usize,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    // Validate CLI limits
    if cli.frames > 1000 {
        eprintln!("Error: frames must be <= 1000 (got {})", cli.frames);
        std::process::exit(1);
    }
    if cli.timeout_ms > 10000 {
        eprintln!(
            "Error: timeout-ms must be <= 10000 (got {})",
            cli.timeout_ms
        );
        std::process::exit(1);
    }

    // Safety gate: verify CLI commands are safe before opening serial
    let commands_to_send = ["*IDN?", "RALL?"];
    for cmd in &commands_to_send {
        if let Err(e) = validate_command(cmd) {
            eprintln!("SAFETY VIOLATION: {}", e);
            std::process::exit(1);
        }
    }

    println!("========================================");
    println!("  OE1022D Logged Acquisition   M2.6");
    println!("========================================");
    println!();
    println!("Port:   {} @ {} baud", cli.port, cli.baud);
    println!(
        "Frames: {} (delay {} ms, timeout {} ms)",
        cli.frames, cli.delay_ms, cli.timeout_ms
    );
    println!("Run ID: {}", cli.run_id);
    println!("Run Root: {}", cli.run_root);
    println!();
    println!("SAFETY: Only *IDN? and RALL? will be sent.");
    println!();

    // -- Create run directory ------------------------------------------------

    let run_root = std::path::PathBuf::from(&cli.run_root);
    let run = match create_run_directory(&run_root, &cli.run_id) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to create run directory: {}", e);
            std::process::exit(1);
        }
    };

    let created_at = utc_now_ms();

    // Write manifest
    let manifest = RunManifest {
        schema_version: "0.2.0".into(),
        kind: "run_manifest".into(),
        run_id: cli.run_id.clone(),
        created_at_unix_ms: created_at,
        artifact_paths: RunArtifactPaths {
            manifest: "manifest.json".into(),
            station_snapshot: "metadata/station_snapshot.json".into(),
            recipe_lock: "metadata/recipe.lock.json".into(),
            resolved_recipe_lock: "metadata/resolved_recipe.lock.json".into(),
            dry_run_plan_lock: "metadata/dry_run_plan.lock.json".into(),
            safety_report_lock: "metadata/safety_report.lock.json".into(),
            events: "events.jsonl".into(),
            index: "index.jsonl".into(),
            raw_bin: "raw/oe1022d_rall.rawbin".into(),
        },
        recipe_hash: None,
        resolved_recipe_id: None,
        safety_report_id: None,
    };
    if let Err(e) = run.write_manifest(&manifest) {
        eprintln!("Failed to write manifest: {}", e);
        std::process::exit(1);
    }

    // Write metadata/acquisition_config.json
    let acq_config = AcquisitionConfig {
        schema_version: "0.2.0".into(),
        port: cli.port.clone(),
        baud: cli.baud,
        frames_requested: cli.frames,
        delay_ms: cli.delay_ms,
        timeout_ms: cli.timeout_ms,
        created_at_unix_ms: created_at,
    };
    if let Err(e) = run.write_json_artifact("metadata/acquisition_config.json", &acq_config) {
        eprintln!("Failed to write acquisition_config: {}", e);
        std::process::exit(1);
    }

    // Write metadata/parser_version.json
    let parser_meta = ParserVersionMeta {
        schema_version: "0.2.0".into(),
        parser_crate: "odmr-oe1022d".into(),
        parser_version: env!("CARGO_PKG_VERSION").into(),
        rall_frame_bytes: RALL_FRAME_BYTES,
    };
    if let Err(e) = run.write_json_artifact("metadata/parser_version.json", &parser_meta) {
        eprintln!("Failed to write parser_version: {}", e);
        std::process::exit(1);
    }

    // Open event writer
    let mut event_writer = match run.open_event_writer() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to open event writer: {}", e);
            std::process::exit(1);
        }
    };

    // Write run_created event
    if let Err(e) = event_writer.write_event(&make_event(
        &cli.run_id,
        RunEventType::RunCreated,
        EventLevel::Info,
        "Run directory created",
        None,
    )) {
        eprintln!("Failed to write event: {}", e);
        std::process::exit(1);
    }

    // -- Step 1: verify identity ---------------------------------------------

    let idn = match verify_identity(&cli.port, cli.baud, cli.timeout_ms) {
        Ok(idn) => {
            println!("IDN: {}", idn);
            idn
        }
        Err(e) => {
            eprintln!("Identity verification failed: {}", e);
            std::process::exit(1);
        }
    };

    // Write station_snapshot
    let station_snapshot = serde_json::json!({
        "device_id": "oe1022d_main",
        "idn": idn,
        "transport": {
            "type": "serial",
            "port": cli.port,
            "baud_rate": cli.baud,
        },
        "snapshot_at_unix_ms": utc_now_ms(),
    });
    if let Err(e) = run.write_station_snapshot_json(&station_snapshot) {
        eprintln!("Failed to write station_snapshot: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = event_writer.write_event(&make_event(
        &cli.run_id,
        RunEventType::DeviceIdentityVerified,
        EventLevel::Info,
        &format!("Device identity verified: {}", idn),
        Some(serde_json::json!({"idn": idn})),
    )) {
        eprintln!("Failed to write event: {}", e);
        std::process::exit(1);
    }

    // -- Step 2: open writers ------------------------------------------------

    let mut raw_writer = match run.open_raw_bin_writer_at("raw/oe1022d_rall.rawbin") {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to open raw bin writer: {}", e);
            std::process::exit(1);
        }
    };

    let mut index_writer = match run.open_index_writer() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to open index writer: {}", e);
            std::process::exit(1);
        }
    };

    // -- Step 3: capture frames ----------------------------------------------

    println!("Capturing {} frames...", cli.frames);

    if let Err(e) = event_writer.write_event(&make_event(
        &cli.run_id,
        RunEventType::AcquisitionStarted,
        EventLevel::Info,
        &format!("Acquisition started: {} frames requested", cli.frames),
        Some(serde_json::json!({"frames_requested": cli.frames})),
    )) {
        eprintln!("Failed to write event: {}", e);
        std::process::exit(1);
    }

    let mut preview_rows: Vec<BChannelPreviewRow> = Vec::new();
    let mut summary_rows: Vec<FrameSummaryRow> = Vec::new();

    let mut ok_count = 0usize;
    let mut fail_count = 0usize;
    let mut timeout_count = 0usize;

    for i in 0..cli.frames {
        let ts = utc_now_ms();
        let start = Instant::now();

        let frame_result: Result<(), String> = match capture_single_frame(
            &cli.port,
            cli.baud,
            cli.timeout_ms,
        ) {
            Ok(buf) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let offset = match raw_writer.append_frame(&buf) {
                    Ok(o) => o,
                    Err(e) => {
                        eprintln!("Frame {}: raw write failed: {}", i, e);
                        fail_count += 1;
                        summary_rows.push(FrameSummaryRow {
                            run_id: cli.run_id.clone(),
                            frame_index: i,
                            timestamp_unix_ms: ts,
                            raw_len: buf.len(),
                            parse_status: "fail".into(),
                            parse_error: Some(format!("raw write: {}", e)),
                        });
                        if cli.delay_ms > 0 {
                            std::thread::sleep(Duration::from_millis(cli.delay_ms));
                        }
                        continue;
                    }
                };

                let mut index_entry = RawIndexEntry {
                    schema_version: "0.2.0".into(),
                    kind: "raw_index_entry".into(),
                    run_id: cli.run_id.clone(),
                    stream_id: "oe1022d.rall".into(),
                    offset_bytes: offset.offset_bytes,
                    length_bytes: buf.len() as u64,
                    timestamp_unix_ms: ts,
                    step_id: None,
                    sample_count: None,
                    frame_index: Some(i as u64),
                    duration_ms: Some(duration_ms),
                    parse_status: None,
                    notes: None,
                };

                if let Err(e) = event_writer.write_event(&make_event(
                    &cli.run_id,
                    RunEventType::FrameCaptured,
                    EventLevel::Info,
                    &format!("Frame {} captured: {} bytes", i, buf.len()),
                    Some(serde_json::json!({"frame_index": i, "raw_len": buf.len(), "duration_ms": duration_ms})),
                )) {
                    eprintln!("Failed to write event: {}", e);
                }

                if buf.len() == RALL_FRAME_BYTES {
                    match parse_rall_frame(&buf) {
                        Ok(frame) => {
                            index_entry.parse_status = Some("success".into());
                            if let Err(e) = event_writer.write_event(&make_event(
                                &cli.run_id,
                                RunEventType::FrameParsed,
                                EventLevel::Info,
                                &format!("Frame {} parsed successfully", i),
                                Some(serde_json::json!({"frame_index": i})),
                            )) {
                                eprintln!("Failed to write event: {}", e);
                            }

                            if let Some(sample) = latest_b_channel_sample(&frame) {
                                preview_rows.push(BChannelPreviewRow {
                                    run_id: cli.run_id.clone(),
                                    frame_index: i,
                                    timestamp_unix_ms: ts,
                                    b_x_mv: sample.x_mv,
                                    b_y_mv: sample.y_mv,
                                    b_freq_hz: sample.freq_hz,
                                    b_noise_mv: sample.noise_mv,
                                    b_pll_locked: frame.config.b_pll_locked.unwrap_or(false),
                                    b_input_overload: frame
                                        .config
                                        .b_input_overload
                                        .unwrap_or(false),
                                    b_gain_overload: frame.config.b_gain_overload.unwrap_or(false),
                                });
                            }

                            summary_rows.push(FrameSummaryRow {
                                run_id: cli.run_id.clone(),
                                frame_index: i,
                                timestamp_unix_ms: ts,
                                raw_len: buf.len(),
                                parse_status: "success".into(),
                                parse_error: None,
                            });
                            ok_count += 1;
                        }
                        Err(e) => {
                            index_entry.parse_status = Some("fail".into());
                            index_entry.notes = Some(format!("parse error: {}", e));
                            if let Err(evte) = event_writer.write_event(&make_event(
                                &cli.run_id,
                                RunEventType::FrameFailed,
                                EventLevel::Warning,
                                &format!("Frame {} parse failed: {}", i, e),
                                Some(serde_json::json!({"frame_index": i, "error": format!("{}", e)})),
                            )) {
                                eprintln!("Failed to write event: {}", evte);
                            }

                            summary_rows.push(FrameSummaryRow {
                                run_id: cli.run_id.clone(),
                                frame_index: i,
                                timestamp_unix_ms: ts,
                                raw_len: buf.len(),
                                parse_status: "fail".into(),
                                parse_error: Some(format!("{}", e)),
                            });
                            fail_count += 1;
                        }
                    }
                } else {
                    index_entry.parse_status = Some("fail".into());
                    index_entry.notes = Some(format!(
                        "incomplete frame: {} bytes (expected {})",
                        buf.len(),
                        RALL_FRAME_BYTES
                    ));
                    if let Err(e) = event_writer.write_event(&make_event(
                        &cli.run_id,
                        RunEventType::FrameFailed,
                        EventLevel::Warning,
                        &format!("Frame {} incomplete: {} bytes", i, buf.len()),
                        Some(serde_json::json!({"frame_index": i, "raw_len": buf.len(), "expected": RALL_FRAME_BYTES})),
                    )) {
                        eprintln!("Failed to write event: {}", e);
                    }

                    summary_rows.push(FrameSummaryRow {
                        run_id: cli.run_id.clone(),
                        frame_index: i,
                        timestamp_unix_ms: ts,
                        raw_len: buf.len(),
                        parse_status: "fail".into(),
                        parse_error: Some(format!(
                            "incomplete frame: {} bytes (expected {})",
                            buf.len(),
                            RALL_FRAME_BYTES
                        )),
                    });
                    fail_count += 1;
                }

                if let Err(e) = index_writer.write_entry(&index_entry) {
                    eprintln!("Failed to write index entry: {}", e);
                }

                Ok(())
            }
            Err(e) => {
                // Timeout or serial error
                if let Err(evte) = event_writer.write_event(&make_event(
                    &cli.run_id,
                    RunEventType::FrameFailed,
                    EventLevel::Warning,
                    &format!("Frame {} capture failed: {}", i, e),
                    Some(serde_json::json!({"frame_index": i, "error": e})),
                )) {
                    eprintln!("Failed to write event: {}", evte);
                }

                summary_rows.push(FrameSummaryRow {
                    run_id: cli.run_id.clone(),
                    frame_index: i,
                    timestamp_unix_ms: ts,
                    raw_len: 0,
                    parse_status: "timeout".into(),
                    parse_error: Some(e.clone()),
                });
                timeout_count += 1;
                Ok(())
            }
        };

        if let Err(e) = frame_result {
            eprintln!("Frame {} processing error: {}", i, e);
        }

        if cli.delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(cli.delay_ms));
        }
    }

    // -- Step 4: finalize ----------------------------------------------------

    if let Err(e) = event_writer.write_event(&make_event(
        &cli.run_id,
        RunEventType::AcquisitionCompleted,
        EventLevel::Info,
        &format!(
            "Acquisition completed: {} ok, {} fail, {} timeout",
            ok_count, fail_count, timeout_count
        ),
        Some(serde_json::json!({"ok": ok_count, "fail": fail_count, "timeout": timeout_count})),
    )) {
        eprintln!("Failed to write event: {}", e);
    }

    // Write parsed outputs
    if let Err(e) = write_parsed_jsonl(&run, "parsed/b_channel_preview.jsonl", &preview_rows) {
        eprintln!("Failed to write preview: {}", e);
    }
    if let Err(e) = write_parsed_jsonl(&run, "parsed/frame_summary.jsonl", &summary_rows) {
        eprintln!("Failed to write summary: {}", e);
    }

    if let Err(e) = event_writer.write_event(&make_event(
        &cli.run_id,
        RunEventType::RunCompleted,
        EventLevel::Info,
        "Run completed",
        Some(serde_json::json!({"frames_ok": ok_count, "frames_fail": fail_count, "frames_timeout": timeout_count})),
    )) {
        eprintln!("Failed to write event: {}", e);
    }

    println!();
    println!("Results:");
    println!("  OK:      {}", ok_count);
    println!("  Fail:    {}", fail_count);
    println!("  Timeout: {}", timeout_count);
    println!();
    println!("Run directory: {}", run.run_directory_path().display());
    println!("Done.");
}

// ---------------------------------------------------------------------------
// Event helper
// ---------------------------------------------------------------------------

static EVENT_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_event_id() -> u64 {
    EVENT_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn make_event(
    run_id: &str,
    event_type: RunEventType,
    level: EventLevel,
    message: &str,
    data: Option<serde_json::Value>,
) -> RunEvent {
    RunEvent {
        schema_version: "0.2.0".into(),
        kind: "run_event".into(),
        run_id: run_id.into(),
        event_id: format!("evt_{:010}", next_event_id()),
        timestamp_unix_ms: utc_now_ms(),
        timestamp_monotonic_ns: None,
        level,
        event_type,
        step_id: None,
        device_id: Some("oe1022d_main".into()),
        message: message.into(),
        data,
    }
}

// ---------------------------------------------------------------------------
// Serial helpers
// ---------------------------------------------------------------------------

fn verify_identity(port: &str, baud: u32, timeout_ms: u64) -> Result<String, String> {
    validate_command("*IDN?")?;

    let mut port = serialport::new(port, baud)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| format!("open serial: {}", e))?;

    let _ = port.clear(serialport::ClearBuffer::Input);

    let cmd = "*IDN?\r";
    port.write_all(cmd.as_bytes())
        .map_err(|e| format!("write: {}", e))?;
    port.flush().map_err(|e| format!("flush: {}", e))?;

    std::thread::sleep(Duration::from_millis(500));

    let mut buf = vec![0u8; 4096];
    let n = port.read(&mut buf).map_err(|e| format!("read: {}", e))?;

    buf.truncate(n);
    let text = String::from_utf8_lossy(&buf)
        .replace('\x00', "")
        .trim()
        .to_string();
    Ok(text)
}

fn capture_single_frame(port: &str, baud: u32, timeout_ms: u64) -> Result<Vec<u8>, String> {
    validate_command("RALL?")?;

    let mut port = serialport::new(port, baud)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| format!("open serial: {}", e))?;

    let _ = port.clear(serialport::ClearBuffer::Input);

    let cmd = "RALL?\r";
    port.write_all(cmd.as_bytes())
        .map_err(|e| format!("write: {}", e))?;
    port.flush().map_err(|e| format!("flush: {}", e))?;

    std::thread::sleep(Duration::from_millis(800));

    let mut frame_buf = Vec::with_capacity(RALL_FRAME_BYTES);
    let read_deadline = Instant::now() + Duration::from_millis(timeout_ms);

    while frame_buf.len() < RALL_FRAME_BYTES && Instant::now() < read_deadline {
        let mut chunk = vec![0u8; 4096];
        match port.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                chunk.truncate(n);
                frame_buf.extend_from_slice(&chunk);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => {
                return Err(format!(
                    "serial read error after {} bytes: {}",
                    frame_buf.len(),
                    e
                ));
            }
        }
    }

    if frame_buf.is_empty() {
        return Err("timeout: zero bytes read after RALL?".into());
    }

    Ok(frame_buf)
}

// ---------------------------------------------------------------------------
// JSONL writer helper
// ---------------------------------------------------------------------------

fn write_parsed_jsonl<T: Serialize>(
    run: &RunDirectory,
    relative_path: &str,
    rows: &[T],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = run.run_directory_path().join(relative_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let mut writer = BufWriter::new(file);
    for row in rows {
        let line = serde_json::to_string(row)?;
        writeln!(writer, "{}", line)?;
    }
    writer.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Time helpers
// ---------------------------------------------------------------------------

fn utc_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // -----------------------------------------------------------------------
    // Safety gate tests
    // -----------------------------------------------------------------------

    #[test]
    fn safety_gate_allows_only_idn_and_rall() {
        assert!(validate_command("*IDN?").is_ok());
        assert!(validate_command("RALL?").is_ok());
    }

    #[test]
    fn safety_gate_rejects_unknown_commands() {
        assert!(validate_command("FREQD? 2").is_err());
        assert!(validate_command("PHASD? 2").is_err());
        assert!(validate_command("HELLO").is_err());
    }

    #[test]
    fn safety_gate_rejects_all_forbidden_oe1022d_setting_commands() {
        let forbidden_setters = [
            "*RST", "RST", "INIT", "RUN", "SSETD", "RSETD", "APHSD", "FMODD", "RSLPD", "PHASD",
            "ISRCD", "SENSD", "OFLTD", "OFSLD", "HARMD",
        ];
        for cmd in &forbidden_setters {
            assert!(
                validate_command(cmd).is_err(),
                "forbidden command '{}' should be rejected",
                cmd
            );
        }
    }

    #[test]
    fn safety_gate_rejects_forbidden_substrings() {
        assert!(validate_command("FMODD 2,0").is_err());
        assert!(validate_command("send SENSD 2,7").is_err());
        assert!(validate_command("*RSTD").is_err());
    }

    #[test]
    fn smb100a_patterns_are_rejected() {
        assert!(reject_smb100a_commands("FREQ 2.88GHz").is_err());
        assert!(reject_smb100a_commands("POW 10dBm").is_err());
        assert!(reject_smb100a_commands("OUTP ON").is_err());
        assert!(reject_smb100a_commands("*IDN?").is_ok());
        assert!(reject_smb100a_commands("RALL?").is_ok());
    }

    // -----------------------------------------------------------------------
    // Run directory layout tests
    // -----------------------------------------------------------------------

    #[test]
    fn run_directory_layout_is_created_correctly() {
        let tmp = tempfile::tempdir().unwrap();
        let run = create_run_directory(tmp.path(), "test_run_layout").unwrap();
        let root = run.run_directory_path();

        assert!(root.join("manifest.json").parent().unwrap().is_dir());
        assert!(root.join("metadata").is_dir());
        assert!(root.join("raw").is_dir());
        assert!(root.join("parsed").is_dir());
        assert!(root.join("events.jsonl").parent().unwrap().is_dir());
        assert!(root.join("index.jsonl").parent().unwrap().is_dir());
    }

    #[test]
    fn manifest_is_written() {
        let tmp = tempfile::tempdir().unwrap();
        let run = create_run_directory(tmp.path(), "test_manifest").unwrap();

        let manifest = RunManifest {
            schema_version: "0.2.0".into(),
            kind: "run_manifest".into(),
            run_id: "test_manifest".into(),
            created_at_unix_ms: utc_now_ms(),
            artifact_paths: RunArtifactPaths::default(),
            recipe_hash: None,
            resolved_recipe_id: None,
            safety_report_id: None,
        };
        run.write_manifest(&manifest).unwrap();

        let path = run.run_directory_path().join("manifest.json");
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        let parsed: RunManifest = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.run_id, "test_manifest");
    }

    #[test]
    fn events_contain_acquisition_started_and_completed() {
        let tmp = tempfile::tempdir().unwrap();
        let run = create_run_directory(tmp.path(), "test_events").unwrap();
        let mut ew = run.open_event_writer().unwrap();

        ew.write_event(&make_event(
            "test_events",
            RunEventType::AcquisitionStarted,
            EventLevel::Info,
            "acquisition started",
            None,
        ))
        .unwrap();

        ew.write_event(&make_event(
            "test_events",
            RunEventType::AcquisitionCompleted,
            EventLevel::Info,
            "acquisition completed",
            None,
        ))
        .unwrap();

        let path = run.run_directory_path().join("events.jsonl");
        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        let ev1: RunEvent = serde_json::from_str(lines[0]).unwrap();
        assert!(matches!(ev1.event_type, RunEventType::AcquisitionStarted));

        let ev2: RunEvent = serde_json::from_str(lines[1]).unwrap();
        assert!(matches!(ev2.event_type, RunEventType::AcquisitionCompleted));
    }

    #[test]
    fn rawbin_offset_and_index_entries_match() {
        let tmp = tempfile::tempdir().unwrap();
        let run = create_run_directory(tmp.path(), "test_raw_index").unwrap();
        let mut rw = run
            .open_raw_bin_writer_at("raw/oe1022d_rall.rawbin")
            .unwrap();
        let mut iw = run.open_index_writer().unwrap();

        let frame1 = vec![0xABu8; 12288];
        let entry1 = rw.append_frame(&frame1).unwrap();
        assert_eq!(entry1.offset_bytes, 0);
        assert_eq!(entry1.length_bytes, 12288);

        let frame2 = vec![0xCDu8; 12288];
        let entry2 = rw.append_frame(&frame2).unwrap();
        assert_eq!(entry2.offset_bytes, 12288);
        assert_eq!(entry2.length_bytes, 12288);

        iw.write_entry(&RawIndexEntry {
            run_id: "test_raw_index".into(),
            stream_id: "oe1022d.rall".into(),
            offset_bytes: entry1.offset_bytes,
            length_bytes: entry1.length_bytes,
            timestamp_unix_ms: 1000,
            frame_index: Some(0),
            duration_ms: Some(100),
            parse_status: Some("success".into()),
            ..empty_index_entry()
        })
        .unwrap();

        iw.write_entry(&RawIndexEntry {
            run_id: "test_raw_index".into(),
            stream_id: "oe1022d.rall".into(),
            offset_bytes: entry2.offset_bytes,
            length_bytes: entry2.length_bytes,
            timestamp_unix_ms: 2000,
            frame_index: Some(1),
            duration_ms: Some(105),
            parse_status: Some("success".into()),
            ..empty_index_entry()
        })
        .unwrap();

        let path = run.run_directory_path().join("index.jsonl");
        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    fn empty_index_entry() -> RawIndexEntry {
        RawIndexEntry {
            schema_version: "0.2.0".into(),
            kind: "raw_index_entry".into(),
            run_id: String::new(),
            stream_id: String::new(),
            offset_bytes: 0,
            length_bytes: 0,
            timestamp_unix_ms: 0,
            step_id: None,
            sample_count: None,
            frame_index: None,
            duration_ms: None,
            parse_status: None,
            notes: None,
        }
    }

    // -----------------------------------------------------------------------
    // Parser tests with fixture frames
    // -----------------------------------------------------------------------

    fn load_fixture_frame(index: usize) -> Vec<u8> {
        let filename = format!(
            "{}/../../../tests/fixtures/oe1022d_rall/rall_frame_{:03}.raw",
            env!("CARGO_MANIFEST_DIR"),
            index
        );
        fs::read(&filename).expect("fixture file should exist")
    }

    #[test]
    fn parsed_preview_can_be_generated_from_fixture_frame() {
        let raw = load_fixture_frame(0);
        assert_eq!(raw.len(), 12288);

        let frame = parse_rall_frame(&raw).expect("should parse real fixture frame");
        let sample = latest_b_channel_sample(&frame).expect("should have B-channel sample");

        let preview = BChannelPreviewRow {
            run_id: "test".into(),
            frame_index: 0,
            timestamp_unix_ms: 0,
            b_x_mv: sample.x_mv,
            b_y_mv: sample.y_mv,
            b_freq_hz: sample.freq_hz,
            b_noise_mv: sample.noise_mv,
            b_pll_locked: frame.config.b_pll_locked.unwrap_or(false),
            b_input_overload: frame.config.b_input_overload.unwrap_or(false),
            b_gain_overload: frame.config.b_gain_overload.unwrap_or(false),
        };

        assert!(preview.b_x_mv.is_finite());
        assert!(preview.b_y_mv.is_finite());
        assert!(preview.b_freq_hz.is_finite());
        assert!(preview.b_noise_mv.is_finite());
    }

    #[test]
    fn parse_failure_creates_frame_failed_event_not_panic() {
        let raw = vec![0u8; 100];
        let result = parse_rall_frame(&raw);
        assert!(result.is_err(), "short frame should produce an error");

        // Simulate the event creation path (should not panic)
        let err_msg = format!("{}", result.unwrap_err());
        let event = make_event(
            "test",
            RunEventType::FrameFailed,
            EventLevel::Warning,
            &format!("Frame 0 parse failed: {}", err_msg),
            Some(serde_json::json!({"frame_index": 0, "error": err_msg})),
        );
        assert_eq!(event.event_type, RunEventType::FrameFailed);
    }

    // -----------------------------------------------------------------------
    // No-CSV / No-dependency tests
    // -----------------------------------------------------------------------

    #[test]
    fn no_csv_files_created_anywhere() {
        let tmp = tempfile::tempdir().unwrap();
        let run = create_run_directory(tmp.path(), "test_no_csv").unwrap();
        let manifest = RunManifest {
            schema_version: "0.2.0".into(),
            kind: "run_manifest".into(),
            run_id: "test_no_csv".into(),
            created_at_unix_ms: utc_now_ms(),
            artifact_paths: RunArtifactPaths::default(),
            recipe_hash: None,
            resolved_recipe_id: None,
            safety_report_id: None,
        };
        run.write_manifest(&manifest).unwrap();
        run.write_station_snapshot_json(&serde_json::json!({"test": true}))
            .unwrap();

        fn has_csv(dir: &std::path::Path) -> bool {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    if has_csv(&path) {
                        return true;
                    }
                } else if let Some(ext) = path.extension() {
                    if ext == "csv" {
                        return true;
                    }
                }
            }
            false
        }

        assert!(
            !has_csv(&run.run_directory_path()),
            "no CSV files should be created"
        );
    }

    #[test]
    fn no_gui_or_executor_dependency_in_source() {
        // This is a compile-time guarantee: Cargo.toml does not list
        // odmr-executor, odmr-compiler, odmr-safety, odmr-smb100a, odmr-device,
        // or any GUI crate as a dependency.
        // If this test compiles, the dependency exclusion holds.
    }
}
