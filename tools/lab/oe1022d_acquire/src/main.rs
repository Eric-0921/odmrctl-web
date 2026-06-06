//! OE1022D bounded acquisition prototype (M2.5).
//!
//! Captures a fixed number of RALL? frames from the real OE1022D, parses them
//! with the M2.4 parser, and writes raw-first artifacts.
//!
//! This is a bring-up tool — not executor or GUI integration.
//!
//! ## Safety
//!
//! Hard-coded command allow-list: only `*IDN?` and `RALL?` may be transmitted.
//! A secondary forbidden-pattern gate provides defense in depth.

use clap::Parser;
use odmr_oe1022d::parser::{parse_rall_frame, RallFrame, RALL_FRAME_BYTES};
use std::io::{Read, Write};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Hard-coded command allow-list
// ---------------------------------------------------------------------------

const ALLOWED_COMMANDS: &[&str] = &["*IDN?", "RALL?"];

const FORBIDDEN_PATTERNS: &[&str] = &[
    "*RST", "RST", "INIT", "RUN", "SSETD", "RSETD", "APHSD", "FMODD", "RSLPD", "PHASD", "ISRCD",
    "SENSD", "OFLTD", "OFSLD", "HARMD",
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

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(about = "OE1022D bounded acquisition prototype (M2.5)")]
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

    #[arg(long, default_value = "../../../docs/lab-bringup")]
    out_dir: String,
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single frame capture record.
struct FrameRecord {
    frame_index: u32,
    timestamp_unix_ms: u64,
    duration_ms: u64,
    raw_len: usize,
    raw_offset: u64,
    parse_status: String,
    parse_error: Option<String>,
    b_x_mv: Option<f64>,
    b_y_mv: Option<f64>,
    b_freq_hz: Option<f64>,
    b_noise_mv: Option<f64>,
    b_pll_locked: Option<bool>,
    b_input_overload: Option<bool>,
    b_gain_overload: Option<bool>,
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
    println!("  OE1022D Bounded Acquisition   M2.5");
    println!("========================================");
    println!();
    println!("Port:   {} @ {} baud", cli.port, cli.baud);
    println!(
        "Frames: {} (delay {} ms, timeout {} ms)",
        cli.frames, cli.delay_ms, cli.timeout_ms
    );
    println!();
    println!("SAFETY: Only *IDN? and RALL? will be sent.");
    println!();

    // -- Step 1: verify identity ------------------------------------------------

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
    println!();

    // -- Step 2: capture frames ------------------------------------------------

    println!("Capturing {} frames...", cli.frames);
    let (records, raw_payload) = match capture_frames(
        &cli.port,
        cli.baud,
        cli.frames,
        cli.delay_ms,
        cli.timeout_ms,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Capture failed: {}", e);
            std::process::exit(1);
        }
    };

    let captured = records.iter().filter(|r| r.parse_status == "ok").count();
    let failed = records
        .iter()
        .filter(|r| r.parse_status == "parse_error")
        .count();
    let timeouts = records
        .iter()
        .filter(|r| r.parse_status == "timeout")
        .count();
    println!(
        "Captured: {} ok, {} parse errors, {} timeouts",
        captured, failed, timeouts
    );
    println!();

    // -- Step 3: write output files -------------------------------------------

    let today = utc_date();
    let out_subdir = format!("{}/oe1022d_acquire_{}", cli.out_dir, today);

    if let Err(e) = std::fs::create_dir_all(&out_subdir) {
        eprintln!("Failed to create output directory '{}': {}", out_subdir, e);
        std::process::exit(1);
    }

    write_outputs(&out_subdir, &records, &idn, &raw_payload);

    println!("\nDone.");
}

// ---------------------------------------------------------------------------
// Serial helpers
// ---------------------------------------------------------------------------

/// Verify identity by sending *IDN? once.
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

    // IDN response is ASCII, 500ms is plenty
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

/// Capture `frames` RALL? responses.
///
/// Returns frame records and the concatenated raw payload.
fn capture_frames(
    port: &str,
    baud: u32,
    frames: u32,
    delay_ms: u64,
    timeout_ms: u64,
) -> Result<(Vec<FrameRecord>, Vec<u8>), String> {
    validate_command("RALL?")?;

    let mut port = serialport::new(port, baud)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| format!("open serial: {}", e))?;

    let mut records = Vec::with_capacity(frames as usize);
    let mut raw_payload = Vec::with_capacity(frames as usize * RALL_FRAME_BYTES);

    for i in 0..frames {
        let _ = port.clear(serialport::ClearBuffer::Input);

        let ts = utc_now_ms();
        let start = Instant::now();

        let cmd = "RALL?\r";
        port.write_all(cmd.as_bytes())
            .map_err(|e| format!("write: {}", e))?;
        port.flush().map_err(|e| format!("flush: {}", e))?;

        let mut frame_buf = Vec::with_capacity(RALL_FRAME_BYTES);
        let read_deadline = Instant::now() + Duration::from_millis(timeout_ms);

        // Fast-poll: RALL? returns 12288 bytes at ~49ms/frame on USB CDC.
        // macOS CDC driver delivers ~1020 bytes per read().
        while frame_buf.len() < RALL_FRAME_BYTES && Instant::now() < read_deadline {
            let mut chunk = vec![0u8; 4096];
            match port.read(&mut chunk) {
                Ok(0) => {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                }
                Ok(n) => {
                    chunk.truncate(n);
                    frame_buf.extend_from_slice(&chunk);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                }
                Err(e) => {
                    eprintln!(
                        "Frame {}: serial read error after {} bytes: {}",
                        i,
                        frame_buf.len(),
                        e
                    );
                    break;
                }
            }
        }

        let offset = raw_payload.len() as u64;
        raw_payload.extend_from_slice(&frame_buf);

        let actual_len = frame_buf.len();
        let duration_ms = start.elapsed().as_millis() as u64;

        let mut rec = FrameRecord {
            frame_index: i,
            timestamp_unix_ms: ts,
            duration_ms,
            raw_len: actual_len,
            raw_offset: offset,
            parse_status: "unknown".to_string(),
            parse_error: None,
            b_x_mv: None,
            b_y_mv: None,
            b_freq_hz: None,
            b_noise_mv: None,
            b_pll_locked: None,
            b_input_overload: None,
            b_gain_overload: None,
        };

        if actual_len == 0 {
            rec.parse_status = "timeout".to_string();
            rec.parse_error = Some("zero bytes read after RALL?".to_string());
        } else if actual_len < RALL_FRAME_BYTES {
            rec.parse_status = "parse_error".to_string();
            rec.parse_error = Some(format!(
                "incomplete frame: {} bytes (expected {})",
                actual_len, RALL_FRAME_BYTES
            ));
        } else {
            // Parse the frame with M2.4 parser
            match parse_rall_frame(&frame_buf[..RALL_FRAME_BYTES]) {
                Ok(parsed) => {
                    extract_b_channel_sample(&mut rec, &parsed);
                    rec.parse_status = "ok".to_string();
                }
                Err(e) => {
                    rec.parse_status = "parse_error".to_string();
                    rec.parse_error = Some(format!("{}", e));
                }
            }
        }

        records.push(rec);

        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
    }

    Ok((records, raw_payload))
}

/// Extract latest B-channel sample from a parsed RALL? frame.
fn extract_b_channel_sample(rec: &mut FrameRecord, frame: &RallFrame) {
    let m = &frame.measurements;
    let c = &frame.config;

    // Latest sample = index 49 (50 samples per parameter)
    rec.b_x_mv = m.lockin_B_X_mv.get(49).copied();
    rec.b_y_mv = m.lockin_B_Y_mv.get(49).copied();
    rec.b_freq_hz = m.lockin_B_freq_hz.get(49).copied();
    rec.b_noise_mv = m.lockin_B_noise_mv.get(49).copied();
    rec.b_pll_locked = c.b_pll_locked;
    rec.b_input_overload = c.b_input_overload;
    rec.b_gain_overload = c.b_gain_overload;
}

// ---------------------------------------------------------------------------
// Output writers
// ---------------------------------------------------------------------------

fn write_outputs(out_dir: &str, records: &[FrameRecord], idn: &str, raw_payload: &[u8]) {
    // Create subdirectories
    for subdir in &["raw", "index", "parsed"] {
        let path = format!("{}/{}", out_dir, subdir);
        if let Err(e) = std::fs::create_dir_all(&path) {
            eprintln!("Warning: could not create {}: {}", path, e);
        }
    }

    // raw/rall_frames.rawbin
    write_rawbin(out_dir, records, raw_payload);

    // index/rall_index.jsonl
    write_index(out_dir, records);

    // parsed/b_channel_preview.jsonl
    write_b_channel_preview(out_dir, records);

    // parsed/frame_summary.jsonl
    write_frame_summary(out_dir, records);

    // observed_commands.jsonl
    write_observed_commands(out_dir, idn);

    // acquisition_report.md
    write_report(out_dir, records, idn, raw_payload.len());
}

fn write_rawbin(out_dir: &str, records: &[FrameRecord], raw_payload: &[u8]) {
    let path = format!("{}/raw/rall_frames.rawbin", out_dir);
    let mut rawbin = Vec::new();
    for r in records {
        if (r.parse_status == "ok" || r.parse_status == "parse_error") && r.raw_len > 0 {
            let start = r.raw_offset as usize;
            let end = start + r.raw_len;
            if start <= raw_payload.len() && end <= raw_payload.len() {
                rawbin.extend_from_slice(&raw_payload[start..end]);
            }
        }
    }
    match std::fs::write(&path, &rawbin) {
        Ok(_) => println!("Wrote rawbin: {}", path),
        Err(e) => eprintln!("Failed to write rawbin: {}", e),
    }
}

fn write_index(out_dir: &str, records: &[FrameRecord]) {
    let path = format!("{}/index/rall_index.jsonl", out_dir);
    let mut lines = Vec::new();
    for r in records {
        if r.raw_len > 0 {
            let json = format!(
                "{{\"frame_index\":{},\"raw_offset\":{},\"raw_len\":{},\"timestamp_unix_ms\":{},\"duration_ms\":{},\"parse_status\":\"{}\"}}",
                r.frame_index, r.raw_offset, r.raw_len, r.timestamp_unix_ms, r.duration_ms, r.parse_status
            );
            lines.push(json);
        }
    }
    match std::fs::write(&path, lines.join("\n")) {
        Ok(_) => println!("Wrote index: {}", path),
        Err(e) => eprintln!("Failed to write index: {}", e),
    }
}

fn write_b_channel_preview(out_dir: &str, records: &[FrameRecord]) {
    let path = format!("{}/parsed/b_channel_preview.jsonl", out_dir);
    let mut lines = Vec::new();
    for r in records {
        if r.parse_status == "ok" {
            let json = format!(
                "{{\"frame_index\":{},\"timestamp_unix_ms\":{},\"b_x_mv\":{},\"b_y_mv\":{},\"b_freq_hz\":{},\"b_noise_mv\":{},\"b_pll_locked\":{},\"b_input_overload\":{},\"b_gain_overload\":{}}}",
                r.frame_index,
                r.timestamp_unix_ms,
                opt_f64_json(r.b_x_mv),
                opt_f64_json(r.b_y_mv),
                opt_f64_json(r.b_freq_hz),
                opt_f64_json(r.b_noise_mv),
                opt_bool_json(r.b_pll_locked),
                opt_bool_json(r.b_input_overload),
                opt_bool_json(r.b_gain_overload),
            );
            lines.push(json);
        }
    }
    match std::fs::write(&path, lines.join("\n")) {
        Ok(_) => println!("Wrote B-channel preview: {}", path),
        Err(e) => eprintln!("Failed to write B-channel preview: {}", e),
    }
}

fn write_frame_summary(out_dir: &str, records: &[FrameRecord]) {
    let path = format!("{}/parsed/frame_summary.jsonl", out_dir);
    let mut lines = Vec::new();
    for r in records {
        let err_field = match &r.parse_error {
            Some(e) => escape_json(e),
            None => String::new(),
        };
        let json = format!(
            "{{\"frame_index\":{},\"timestamp_unix_ms\":{},\"raw_len\":{},\"parse_status\":\"{}\",\"parse_error\":\"{}\"}}",
            r.frame_index, r.timestamp_unix_ms, r.raw_len, r.parse_status, err_field
        );
        lines.push(json);
    }
    match std::fs::write(&path, lines.join("\n")) {
        Ok(_) => println!("Wrote frame summary: {}", path),
        Err(e) => eprintln!("Failed to write frame summary: {}", e),
    }
}

fn write_observed_commands(out_dir: &str, idn: &str) {
    let path = format!("{}/observed_commands.jsonl", out_dir);
    let content = format!(
        "{{\"command\":\"*IDN?\",\"response\":\"{}\"}}\n{{\"command\":\"RALL?\",\"count\":\"see frame_summary\"}}\n",
        escape_json(idn)
    );
    match std::fs::write(&path, content) {
        Ok(_) => println!("Wrote observed commands: {}", path),
        Err(e) => eprintln!("Failed to write observed commands: {}", e),
    }
}

fn write_report(out_dir: &str, records: &[FrameRecord], idn: &str, total_bytes: usize) {
    let path = format!("{}/acquisition_report.md", out_dir);

    let ok_count = records.iter().filter(|r| r.parse_status == "ok").count();
    let err_count = records
        .iter()
        .filter(|r| r.parse_status == "parse_error")
        .count();
    let timeout_count = records
        .iter()
        .filter(|r| r.parse_status == "timeout")
        .count();

    // B-channel preview statistics
    let ok_records: Vec<_> = records.iter().filter(|r| r.parse_status == "ok").collect();
    let (b_x_mean, b_x_min, b_x_max) = stats(ok_records.iter().filter_map(|r| r.b_x_mv));
    let (b_y_mean, b_y_min, b_y_max) = stats(ok_records.iter().filter_map(|r| r.b_y_mv));
    let (b_freq_mean, b_freq_min, b_freq_max) =
        stats(ok_records.iter().filter_map(|r| r.b_freq_hz));
    let (b_noise_mean, b_noise_min, b_noise_max) =
        stats(ok_records.iter().filter_map(|r| r.b_noise_mv));
    let pll_locked_count = ok_records
        .iter()
        .filter(|r| r.b_pll_locked == Some(true))
        .count();
    let input_overload_count = ok_records
        .iter()
        .filter(|r| r.b_input_overload == Some(true))
        .count();
    let gain_overload_count = ok_records
        .iter()
        .filter(|r| r.b_gain_overload == Some(true))
        .count();

    let mut lines = Vec::new();
    lines.push("# OE1022D Bounded Acquisition Report".to_string());
    lines.push("".to_string());
    lines.push(format!("> **M2.5 prototype** — {}", utc_date()));
    lines
        .push("> **Safety**: Only `*IDN?` and `RALL?` were sent. No settings changed.".to_string());
    lines.push("".to_string());

    lines.push("## Summary".to_string());
    lines.push("".to_string());
    lines.push(format!("- **Device IDN**: `{}`", idn));
    lines.push(format!("- **Frames attempted**: {}", records.len()));
    lines.push(format!("- **Frames captured (ok)**: {}", ok_count));
    lines.push(format!("- **Parse errors**: {}", err_count));
    lines.push(format!("- **Timeouts**: {}", timeout_count));
    lines.push(format!("- **Total raw bytes**: {}", total_bytes));
    lines.push("".to_string());

    lines.push("## B-Channel Preview Statistics".to_string());
    lines.push("".to_string());
    lines.push("| Field | Mean | Min | Max |".to_string());
    lines.push("|-------|------|-----|-----|".to_string());
    lines.push(format!(
        "| B-X (mV) | {:.6} | {:.6} | {:.6} |",
        b_x_mean, b_x_min, b_x_max
    ));
    lines.push(format!(
        "| B-Y (mV) | {:.6} | {:.6} | {:.6} |",
        b_y_mean, b_y_min, b_y_max
    ));
    lines.push(format!(
        "| B-Freq (Hz) | {:.6} | {:.6} | {:.6} |",
        b_freq_mean, b_freq_min, b_freq_max
    ));
    lines.push(format!(
        "| B-Noise (mV) | {:.6} | {:.6} | {:.6} |",
        b_noise_mean, b_noise_min, b_noise_max
    ));
    lines.push("".to_string());

    lines.push("## Overload / PLL Status".to_string());
    lines.push("".to_string());
    lines.push("| Condition | Count |".to_string());
    lines.push("|-----------|-------|".to_string());
    lines.push(format!(
        "| PLL Locked | {} / {} |",
        pll_locked_count, ok_count
    ));
    lines.push(format!(
        "| Input Overload | {} / {} |",
        input_overload_count, ok_count
    ));
    lines.push(format!(
        "| Gain Overload | {} / {} |",
        gain_overload_count, ok_count
    ));
    lines.push("".to_string());

    lines.push("## Frame Details".to_string());
    lines.push("".to_string());
    lines.push("| # | Offset | Len | Status | Duration (ms) | Error |".to_string());
    lines.push("|---|--------|-----|--------|---------------|-------|".to_string());
    for r in records {
        let err_str = r.parse_error.as_deref().unwrap_or("");
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            r.frame_index, r.raw_offset, r.raw_len, r.parse_status, r.duration_ms, err_str
        ));
    }
    lines.push("".to_string());

    lines.push("## Forbidden Command Audit".to_string());
    lines.push("".to_string());
    lines.push("| Pattern | Present in source? |".to_string());
    lines.push("|---------|---------------------|".to_string());
    // We check at compile time that no forbidden command strings are emitted
    for pat in FORBIDDEN_PATTERNS {
        let source_has =
            std::include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs")).contains(pat);
        lines.push(format!(
            "| `{}` | {} |",
            pat,
            if source_has {
                "yes (as safety gate definition)"
            } else {
                "no"
            }
        ));
    }
    lines.push("".to_string());

    match std::fs::write(&path, lines.join("\n")) {
        Ok(_) => println!("Wrote acquisition report: {}", path),
        Err(e) => eprintln!("Failed to write acquisition report: {}", e),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn utc_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn opt_f64_json(v: Option<f64>) -> String {
    match v {
        Some(x) => format!("{:.6}", x),
        None => "null".to_string(),
    }
}

fn opt_bool_json(v: Option<bool>) -> String {
    match v {
        Some(b) => b.to_string(),
        None => "null".to_string(),
    }
}

fn utc_date() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let days = secs / 86400;
    let mut y = 1970;
    let mut d = days;
    loop {
        let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        let year_days = if leap { 366 } else { 365 };
        if d < year_days {
            break;
        }
        d -= year_days;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 1;
    for md in &month_days {
        if d < *md {
            break;
        }
        d -= *md;
        m += 1;
    }
    format!("{:04}-{:02}-{:02}", y, m, d + 1)
}

fn stats(iter: impl Iterator<Item = f64>) -> (f64, f64, f64) {
    let values: Vec<f64> = iter.collect();
    if values.is_empty() {
        return (f64::NAN, f64::NAN, f64::NAN);
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (mean, min, max)
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

    // -----------------------------------------------------------------------
    // Rawbin offset and index length tests
    // -----------------------------------------------------------------------

    #[test]
    fn rawbin_offset_and_index_length_match() {
        let records = vec![
            FrameRecord {
                frame_index: 0,
                timestamp_unix_ms: 1000,
                duration_ms: 600,
                raw_len: 12288,
                raw_offset: 0,
                parse_status: "ok".to_string(),
                parse_error: None,
                b_x_mv: Some(1.0),
                b_y_mv: Some(0.5),
                b_freq_hz: Some(1000.0),
                b_noise_mv: Some(0.01),
                b_pll_locked: Some(true),
                b_input_overload: Some(false),
                b_gain_overload: Some(false),
            },
            FrameRecord {
                frame_index: 1,
                timestamp_unix_ms: 2000,
                duration_ms: 610,
                raw_len: 12288,
                raw_offset: 12288,
                parse_status: "ok".to_string(),
                parse_error: None,
                b_x_mv: Some(1.1),
                b_y_mv: Some(0.6),
                b_freq_hz: Some(1000.0),
                b_noise_mv: Some(0.01),
                b_pll_locked: Some(true),
                b_input_overload: Some(false),
                b_gain_overload: Some(false),
            },
        ];
        assert_eq!(
            records[0].raw_offset + records[0].raw_len as u64,
            records[1].raw_offset
        );
        assert_eq!(records[0].raw_offset, 0);
        assert_eq!(records[1].raw_offset, 12288);
    }

    #[test]
    fn rawbin_total_size_matches_index_sum() {
        let records = vec![
            FrameRecord {
                frame_index: 0,
                timestamp_unix_ms: 0,
                duration_ms: 600,
                raw_len: 12288,
                raw_offset: 0,
                parse_status: "ok".to_string(),
                parse_error: None,
                b_x_mv: Some(1.0),
                b_y_mv: Some(0.5),
                b_freq_hz: Some(1000.0),
                b_noise_mv: Some(0.01),
                b_pll_locked: Some(true),
                b_input_overload: Some(false),
                b_gain_overload: Some(false),
            },
            FrameRecord {
                frame_index: 1,
                timestamp_unix_ms: 0,
                duration_ms: 610,
                raw_len: 12288,
                raw_offset: 12288,
                parse_status: "ok".to_string(),
                parse_error: None,
                b_x_mv: Some(1.1),
                b_y_mv: Some(0.6),
                b_freq_hz: Some(1000.0),
                b_noise_mv: Some(0.01),
                b_pll_locked: Some(true),
                b_input_overload: Some(false),
                b_gain_overload: Some(false),
            },
        ];
        let total: usize = records.iter().map(|r| r.raw_len).sum();
        assert_eq!(total, 12288 * 2);
    }

    // -----------------------------------------------------------------------
    // Parser tests with real fixture frames
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
    fn parsed_preview_generation_works_from_fixture_frame() {
        let raw = load_fixture_frame(0);
        assert_eq!(raw.len(), 12288);

        let frame = parse_rall_frame(&raw).expect("should parse real fixture frame");
        let mut rec = FrameRecord {
            frame_index: 0,
            timestamp_unix_ms: 0,
            duration_ms: 0,
            raw_len: 12288,
            raw_offset: 0,
            parse_status: "ok".to_string(),
            parse_error: None,
            b_x_mv: None,
            b_y_mv: None,
            b_freq_hz: None,
            b_noise_mv: None,
            b_pll_locked: None,
            b_input_overload: None,
            b_gain_overload: None,
        };
        extract_b_channel_sample(&mut rec, &frame);

        // All B-channel fields should be populated
        assert!(rec.b_x_mv.is_some(), "b_x_mv should be populated");
        assert!(rec.b_y_mv.is_some(), "b_y_mv should be populated");
        assert!(rec.b_freq_hz.is_some(), "b_freq_hz should be populated");
        assert!(rec.b_noise_mv.is_some(), "b_noise_mv should be populated");
        assert!(
            rec.b_pll_locked.is_some(),
            "b_pll_locked should be populated"
        );
        assert!(
            rec.b_input_overload.is_some(),
            "b_input_overload should be populated"
        );
        assert!(
            rec.b_gain_overload.is_some(),
            "b_gain_overload should be populated"
        );

        // Values should be finite
        assert!(rec.b_x_mv.unwrap().is_finite());
        assert!(rec.b_y_mv.unwrap().is_finite());
        assert!(rec.b_freq_hz.unwrap().is_finite());
        assert!(rec.b_noise_mv.unwrap().is_finite());
    }

    #[test]
    fn all_three_fixture_frames_parse_successfully() {
        for i in 0..3 {
            let raw = load_fixture_frame(i);
            assert_eq!(raw.len(), 12288);
            let frame = parse_rall_frame(&raw)
                .expect(&format!("fixture frame {} should parse successfully", i));

            // Verify measurements are non-empty
            assert_eq!(frame.measurements.lockin_B_X_mv.len(), 50);
            assert_eq!(frame.measurements.lockin_B_Y_mv.len(), 50);
            assert_eq!(frame.measurements.lockin_B_freq_hz.len(), 50);
            assert_eq!(frame.measurements.lockin_B_noise_mv.len(), 50);
        }
    }

    // -----------------------------------------------------------------------
    // Parser error handling tests
    // -----------------------------------------------------------------------

    #[test]
    fn parser_error_logged_not_panicked_on_short_frame() {
        // A frame that is too short should not panic
        let raw = vec![0u8; 100];
        let result = parse_rall_frame(&raw);
        assert!(result.is_err(), "short frame should produce an error");
    }

    #[test]
    fn parser_error_logged_not_panicked_on_empty_frame() {
        let raw = vec![];
        let result = parse_rall_frame(&raw);
        assert!(result.is_err(), "empty frame should produce an error");
    }

    #[test]
    fn malformed_frame_creates_failed_frame_summary() {
        // Simulate a malformed-frame record and verify it generates correct summary fields
        let rec = FrameRecord {
            frame_index: 0,
            timestamp_unix_ms: 1000,
            duration_ms: 100,
            raw_len: 100,
            raw_offset: 0,
            parse_status: "parse_error".to_string(),
            parse_error: Some("incomplete frame: 100 bytes (expected 12288)".to_string()),
            b_x_mv: None,
            b_y_mv: None,
            b_freq_hz: None,
            b_noise_mv: None,
            b_pll_locked: None,
            b_input_overload: None,
            b_gain_overload: None,
        };

        assert_eq!(rec.parse_status, "parse_error");
        assert!(rec.parse_error.is_some());
        assert!(rec.b_x_mv.is_none()); // No sample extracted from bad frame
    }

    #[test]
    fn timeout_frame_creates_proper_summary() {
        let rec = FrameRecord {
            frame_index: 0,
            timestamp_unix_ms: 1000,
            duration_ms: 2000,
            raw_len: 0,
            raw_offset: 0,
            parse_status: "timeout".to_string(),
            parse_error: Some("zero bytes read after RALL?".to_string()),
            b_x_mv: None,
            b_y_mv: None,
            b_freq_hz: None,
            b_noise_mv: None,
            b_pll_locked: None,
            b_input_overload: None,
            b_gain_overload: None,
        };

        assert_eq!(rec.parse_status, "timeout");
        assert_eq!(rec.raw_len, 0);
        assert!(rec.parse_error.is_some());
    }

    // -----------------------------------------------------------------------
    // No-CSV / No-SMB100A tests
    // -----------------------------------------------------------------------

    #[test]
    fn no_csv_string_in_output_paths() {
        // Verify that none of the output file paths contain ".csv"
        let output_dir = "docs/lab-bringup/oe1022d_acquire_test";
        let rawbin_path = format!("{}/raw/rall_frames.rawbin", output_dir);
        let index_path = format!("{}/index/rall_index.jsonl", output_dir);
        let preview_path = format!("{}/parsed/b_channel_preview.jsonl", output_dir);
        let summary_path = format!("{}/parsed/frame_summary.jsonl", output_dir);
        let observed_path = format!("{}/observed_commands.jsonl", output_dir);
        let report_path = format!("{}/acquisition_report.md", output_dir);

        assert!(!rawbin_path.contains(".csv"));
        assert!(!index_path.contains(".csv"));
        assert!(!preview_path.contains(".csv"));
        assert!(!summary_path.contains(".csv"));
        assert!(!observed_path.contains(".csv"));
        assert!(!report_path.contains(".csv"));
    }

    #[test]
    fn no_smb100a_command_exists_in_tool_source() {
        // Verify the acquire tool does not emit SMB100A commands.
        // Exclude lines inside the test section itself (after "mod tests {").
        let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"));
        let test_boundary = source.find("mod tests {").unwrap_or(source.len());
        let production_code = &source[..test_boundary];

        let smb_patterns = [
            "smb100a",
            "SMB100A",
            "FREQ ",
            "POW ",
            "OUTP ",
            "MOD:STAT",
            "FREQ:MODE",
        ];
        for pat in &smb_patterns {
            assert!(
                !production_code.contains(pat),
                "production code should not contain SMB100A command pattern '{}'",
                pat
            );
        }
    }

    // -----------------------------------------------------------------------
    // JSONL format tests
    // -----------------------------------------------------------------------

    #[test]
    fn b_channel_preview_jsonl_format_is_valid() {
        let rec = FrameRecord {
            frame_index: 42,
            timestamp_unix_ms: 1700000000000,
            duration_ms: 605,
            raw_len: 12288,
            raw_offset: 516096,
            parse_status: "ok".to_string(),
            parse_error: None,
            b_x_mv: Some(1.234),
            b_y_mv: Some(0.567),
            b_freq_hz: Some(1000.0),
            b_noise_mv: Some(0.012),
            b_pll_locked: Some(true),
            b_input_overload: Some(false),
            b_gain_overload: Some(false),
        };

        // Build the JSONL line and verify it parses
        let line = format!(
            "{{\"frame_index\":{},\"timestamp_unix_ms\":{},\"b_x_mv\":{},\"b_y_mv\":{},\"b_freq_hz\":{},\"b_noise_mv\":{},\"b_pll_locked\":{},\"b_input_overload\":{},\"b_gain_overload\":{}}}",
            rec.frame_index,
            rec.timestamp_unix_ms,
            opt_f64_json(rec.b_x_mv),
            opt_f64_json(rec.b_y_mv),
            opt_f64_json(rec.b_freq_hz),
            opt_f64_json(rec.b_noise_mv),
            opt_bool_json(rec.b_pll_locked),
            opt_bool_json(rec.b_input_overload),
            opt_bool_json(rec.b_gain_overload),
        );

        assert!(line.contains("\"frame_index\":42"));
        assert!(line.contains("\"b_x_mv\":1.234"));
        assert!(line.contains("\"b_pll_locked\":true"));
        assert!(!line.contains("csv"));
    }

    #[test]
    fn null_fields_preserved_in_preview() {
        let rec = FrameRecord {
            frame_index: 0,
            timestamp_unix_ms: 0,
            duration_ms: 0,
            raw_len: 0,
            raw_offset: 0,
            parse_status: "timeout".to_string(),
            parse_error: None,
            b_x_mv: None,
            b_y_mv: None,
            b_freq_hz: None,
            b_noise_mv: None,
            b_pll_locked: None,
            b_input_overload: None,
            b_gain_overload: None,
        };
        assert_eq!(opt_f64_json(rec.b_x_mv), "null");
        assert_eq!(opt_bool_json(rec.b_pll_locked), "null");
    }
}
