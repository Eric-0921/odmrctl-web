//! OE1022D buffer-sampling probe.
//!
//! This tool exists only for narrow bring-up validation of the OE1022D data
//! buffer path described in manual sections 5.2.8 and 5.2.9.
//!
//! Safety boundary:
//! - No RF / magnetic / laser commands
//! - No OE panel configuration beyond data-buffer commands
//! - No reset-to-default (`*RSTD`)
//! - Best-effort pause + buffer reset on exit

mod focused_test;
mod format_probe;
mod minimal_cmd_test;
mod precise_validation;
mod collector_test;
mod rall_bench;
mod rall_continuous;
mod rall_detailed;
mod rate_probe;
mod trcad_brute_force;
mod trcad_exact_format;
mod trcad_minimal;
mod trcad_probe;
mod trcad_raw_probe;

use clap::Parser;
use odmr_oe1022d::commands::{
    pause_sampling, query_sample_length, query_sample_run_mode, query_sample_step_time,
    query_sample_trigger_mode, query_standard_idn, query_stored_point_count, query_trace_data,
    query_sample_buffer_selector, reset_data_buffers, set_sample_buffer_selector, set_sample_length,
    set_sample_run_mode, set_sample_step_time_s, set_sample_trigger_mode, start_sampling,
};
use serde::Serialize;
use std::time::{Duration, Instant};

const ALLOWED_PREFIXES: &[&str] = &[
    "*IDN?",
    "SRATD ",
    "SRATD?",
    "SLEND ",
    "SLEND?",
    "SSLED ",
    "SSLED?",
    "STRGD ",
    "STRGD?",
    "SPRMD ",
    "SPRMD?",
    "STRDD ",
    "PAUSD ",
    "RESTD ",
    "SPTSD ?",
    "TRCAD ?",
];

const FORBIDDEN_PATTERNS: &[&str] = &[
    "*RST", "SSETD", "RSETD", "FMODD", "RSLPD", "FREQD", "PHASD", "ISRCD", "IGNDD", "ICPLD",
    "ILIND", "SENSD", "RMODD", "OFLTD", "OFSLD", "HARMD", "SYNCD", "SWVTD", "SLVLD", "SVLLD",
    "SVULD", "SVSLD", "SVSGD", "SVTMD", "SVRMD", "SVDCD", "FPOPD", "OEXPD", "SPEDD", "CAUXD",
    "SWTPD", "SLLMD", "SULMD", "SSLLD", "SSLGD", "STLMD", "SWRMD", "AGAND", "ARSVD", "APHSD",
    "ASCLD", "EQCDD", "EQCSD", "RALL?", "OUTPD?", "SNAPD?",
];

#[derive(Parser, Debug)]
#[command(about = "OE1022D narrow buffer-sampling probe for 1 kHz validation")]
struct Cli {
    #[arg(long, default_value = "/dev/cu.usbmodem3361358734371")]
    port: String,

    #[arg(long, default_value = "921600")]
    baud: u32,

    #[arg(long, default_value = "2")]
    channel: u8,

    #[arg(long, default_value = "1")]
    buffer: u8,

    #[arg(long, default_value = "1")]
    parameter: u8,

    #[arg(long, default_value = "0.001")]
    sample_step_s: f64,

    #[arg(long, default_value = "4096")]
    sample_length: u32,

    #[arg(long, default_value = "1")]
    sample_run_mode: u8,

    #[arg(long, default_value = "1200")]
    run_ms: u64,

    #[arg(long, default_value = "100")]
    poll_ms: u64,

    #[arg(long, default_value = "512")]
    read_points: u32,

    #[arg(long, default_value = "2000")]
    timeout_ms: u64,

    #[arg(long, default_value = "docs/lab-bringup")]
    out_dir: String,

    #[arg(long, help = "Run minimal command-by-command test instead of full probe")]
    minimal_test: bool,

    #[arg(long, help = "Run precise validation with fixed config and timing")]
    precise_validation: bool,

    #[arg(long, help = "Probe exact command format accepted by device")]
    format_probe: bool,

    #[arg(long, help = "Run focused test with fresh connection per command")]
    focused_test: bool,

    #[arg(long, help = "Measure actual buffer fill rate with high-frequency polling")]
    rate_probe: bool,

    #[arg(long, help = "Troubleshoot TRCAD? command")]
    trcad_probe: bool,

    #[arg(long, help = "Minimal TRCAD? format test")]
    trcad_minimal: bool,

    #[arg(long, help = "Raw byte-level TRCAD? response probe")]
    trcad_raw_probe: bool,

    #[arg(long, help = "Test TRCAD? with exact spacing from manual")]
    trcad_exact_format: bool,

    #[arg(long, help = "Brute-force probe for buffer-read command names")]
    trcad_brute_force: bool,

    #[arg(long, help = "Integration test for RallCollector on real hardware")]
    collector_test: bool,

    #[arg(long, help = "Continuous RALL? benchmark (correct frame alignment)")]
    rall_bench: bool,

    #[arg(long, help = "Test continuous RALL? polling behavior")]
    rall_continuous: bool,

    #[arg(long, help = "Detailed RALL? frame analysis")]
    rall_detailed: bool,
}

#[derive(Debug, Serialize)]
struct PollRecord {
    elapsed_ms: u64,
    response_elapsed_ms: u64,
    stored_points: u32,
}

#[derive(Debug, Serialize)]
struct ProbeReport {
    schema_version: u32,
    port: String,
    baud: u32,
    channel: u8,
    buffer: u8,
    parameter: u8,
    sample_step_s: f64,
    sample_length: u32,
    sample_run_mode: u8,
    run_ms: u64,
    poll_ms: u64,
    configured_step_reply: String,
    configured_length_reply: String,
    configured_buffer_reply: String,
    configured_trigger_reply: String,
    configured_run_mode_reply: String,
    device_idn: String,
    final_stored_points: u32,
    effective_hz: f64,
    trace_points_read: usize,
    trace_preview: Vec<f64>,
    polls: Vec<PollRecord>,
}

fn validate_command(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if !ALLOWED_PREFIXES.iter().any(|p| trimmed.starts_with(p)) {
        return Err(format!("command '{}' is not in allow-list", trimmed));
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

fn open_port(port_path: &str, baud: u32, timeout_ms: u64) -> Result<Box<dyn serialport::SerialPort>, String> {
    serialport::new(port_path, baud)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| format!("open serial: {}", e))
}

fn write_line(port: &mut dyn serialport::SerialPort, cmd: &str) -> Result<(), String> {
    validate_command(cmd)?;
    let line = format!("{cmd}\r");
    port.write_all(line.as_bytes())
        .map_err(|e| format!("write '{}': {}", cmd, e))?;
    port.flush()
        .map_err(|e| format!("flush '{}': {}", cmd, e))?;
    Ok(())
}

fn query_ascii(
    port: &mut dyn serialport::SerialPort,
    cmd: &str,
    settle_ms: u64,
    max_bytes: usize,
) -> Result<String, String> {
    let _ = port.clear(serialport::ClearBuffer::Input);
    write_line(port, cmd)?;
    let mut collected = Vec::new();

    for attempt in 0..5 {
        let wait_ms = if attempt == 0 { settle_ms } else { 200 };
        std::thread::sleep(Duration::from_millis(wait_ms));

        let remaining = max_bytes.saturating_sub(collected.len());
        if remaining == 0 {
            break;
        }

        let mut buf = vec![0u8; remaining.min(8192)];
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                buf.truncate(n);
                collected.extend_from_slice(&buf);

                loop {
                    let available = port.bytes_to_read().unwrap_or(0) as usize;
                    if available == 0 || collected.len() >= max_bytes {
                        break;
                    }
                    let chunk_len = available.min(max_bytes - collected.len()).min(8192);
                    let mut extra = vec![0u8; chunk_len];
                    match port.read(&mut extra) {
                        Ok(m) if m > 0 => {
                            extra.truncate(m);
                            collected.extend_from_slice(&extra);
                        }
                        _ => break,
                    }
                }
                break;
            }
            Ok(_) => continue,
            Err(_) => continue,
        }
    }

    Ok(String::from_utf8_lossy(&collected)
        .replace('\0', "")
        .trim()
        .to_string())
}

fn send_command_once(port_path: &str, baud: u32, timeout_ms: u64, cmd: &str) -> Result<(), String> {
    let mut port = open_port(port_path, baud, timeout_ms)?;
    let _ = port.clear(serialport::ClearBuffer::Input);
    write_line(port.as_mut(), cmd)
}

fn query_ascii_once(
    port_path: &str,
    baud: u32,
    timeout_ms: u64,
    cmd: &str,
    settle_ms: u64,
    max_bytes: usize,
) -> Result<String, String> {
    let mut port = open_port(port_path, baud, timeout_ms)?;
    query_ascii(port.as_mut(), cmd, settle_ms, max_bytes)
}

fn parse_point_count(text: &str) -> Result<u32, String> {
    text.trim()
        .parse::<u32>()
        .map_err(|e| format!("parse point count '{}': {}", text, e))
}

fn parse_trace(text: &str) -> Vec<f64> {
    text.split(',')
        .filter_map(|token| {
            let trimmed = token
                .trim()
                .trim_end_matches(['\r', '\n', '\0', '。', ';', ' ']);
            if trimmed.is_empty() {
                return None;
            }
            trimmed.parse::<f64>().ok()
        })
        .collect()
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
    let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
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

fn main() {
    let cli = Cli::parse();

    if cli.minimal_test {
        minimal_cmd_test::run(&cli.port, cli.baud);
        return;
    }
    if cli.precise_validation {
        precise_validation::run(&cli.port, cli.baud);
        return;
    }
    if cli.format_probe {
        format_probe::run(&cli.port, cli.baud);
        return;
    }
    if cli.focused_test {
        focused_test::run(&cli.port, cli.baud);
        return;
    }
    if cli.rate_probe {
        rate_probe::run(&cli.port, cli.baud);
        return;
    }
    if cli.trcad_probe {
        trcad_probe::run(&cli.port, cli.baud);
        return;
    }
    if cli.trcad_minimal {
        trcad_minimal::run(&cli.port, cli.baud);
        return;
    }
    if cli.trcad_raw_probe {
        trcad_raw_probe::run(&cli.port, cli.baud);
        return;
    }
    if cli.trcad_exact_format {
        trcad_exact_format::run(&cli.port, cli.baud);
        return;
    }
    if cli.trcad_brute_force {
        trcad_brute_force::run(&cli.port, cli.baud);
        return;
    }
    if cli.collector_test {
        collector_test::run(&cli.port, cli.baud);
        return;
    }
    if cli.rall_bench {
        rall_bench::run(&cli.port, cli.baud);
        return;
    }
    if cli.rall_continuous {
        rall_continuous::run(&cli.port, cli.baud);
        return;
    }
    if cli.rall_detailed {
        rall_detailed::run(&cli.port, cli.baud);
        return;
    }

    if cli.channel != 1 && cli.channel != 2 {
        eprintln!("channel must be 1 or 2");
        std::process::exit(1);
    }
    if !(1..=4).contains(&cli.buffer) {
        eprintln!("buffer must be 1..=4");
        std::process::exit(1);
    }
    if cli.sample_step_s < 0.001 {
        eprintln!("sample-step-s must be >= 0.001");
        std::process::exit(1);
    }
    if cli.sample_length == 0 || cli.sample_length > 16384 {
        eprintln!("sample-length must be 1..=16384");
        std::process::exit(1);
    }
    if cli.sample_run_mode > 1 {
        eprintln!("sample-run-mode must be 0 (single) or 1 (loop)");
        std::process::exit(1);
    }

    println!("=== OE1022D Buffer Probe ===");
    println!("Port: {} @ {}", cli.port, cli.baud);
    println!(
        "Channel: {}, Buffer: {}, Parameter: {}",
        cli.channel, cli.buffer, cli.parameter
    );
    println!(
        "Step: {} s, Length: {}, Run: {} ms",
        cli.sample_step_s, cli.sample_length, cli.run_ms
    );

    let idn = match query_ascii_once(
        &cli.port,
        cli.baud,
        cli.timeout_ms,
        query_standard_idn(),
        500,
        4096,
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("identity failed: {}", e);
            std::process::exit(1);
        }
    };
    println!("IDN: {}", idn);

    if let Err(e) = send_command_once(&cli.port, cli.baud, cli.timeout_ms, &pause_sampling(cli.channel)) {
        eprintln!("pause before configure failed: {}", e);
    }
    if let Err(e) = send_command_once(&cli.port, cli.baud, cli.timeout_ms, &reset_data_buffers(cli.channel)) {
        eprintln!("reset before configure failed: {}", e);
    }

    let setup_cmds = [
        set_sample_step_time_s(cli.channel, cli.sample_step_s),
        set_sample_length(cli.channel, cli.sample_length),
        set_sample_buffer_selector(cli.channel, cli.buffer, cli.parameter),
        set_sample_trigger_mode(cli.channel, 0),
        set_sample_run_mode(cli.channel, cli.sample_run_mode),
    ];

    for cmd in &setup_cmds {
        if let Err(e) = send_command_once(&cli.port, cli.baud, cli.timeout_ms, cmd) {
            eprintln!("setup failed on '{}': {}", cmd, e);
            std::process::exit(1);
        }
        std::thread::sleep(Duration::from_millis(150));
    }

    let configured_step_reply = query_ascii_once(
        &cli.port,
        cli.baud,
        cli.timeout_ms,
        &query_sample_step_time(cli.channel),
        800,
        1024,
    )
    .unwrap_or_else(|e| format!("ERR: {e}"));
    let configured_length_reply =
        query_ascii_once(&cli.port, cli.baud, cli.timeout_ms, &query_sample_length(cli.channel), 800, 1024)
            .unwrap_or_else(|e| format!("ERR: {e}"));
    let configured_buffer_reply =
        query_ascii_once(
            &cli.port,
            cli.baud,
            cli.timeout_ms,
            &query_sample_buffer_selector(cli.channel, cli.buffer),
            800,
            1024,
        )
            .unwrap_or_else(|e| format!("ERR: {e}"));
    let configured_trigger_reply =
        query_ascii_once(
            &cli.port,
            cli.baud,
            cli.timeout_ms,
            &query_sample_trigger_mode(cli.channel),
            800,
            1024,
        )
            .unwrap_or_else(|e| format!("ERR: {e}"));
    let configured_run_mode_reply =
        query_ascii_once(
            &cli.port,
            cli.baud,
            cli.timeout_ms,
            &query_sample_run_mode(cli.channel),
            800,
            1024,
        )
            .unwrap_or_else(|e| format!("ERR: {e}"));

    if let Err(e) = send_command_once(&cli.port, cli.baud, cli.timeout_ms, &reset_data_buffers(cli.channel)) {
        eprintln!("reset before start failed: {}", e);
        std::process::exit(1);
    }
    std::thread::sleep(Duration::from_millis(150));

    if let Err(e) = send_command_once(&cli.port, cli.baud, cli.timeout_ms, &start_sampling(cli.channel)) {
        eprintln!("start sampling failed: {}", e);
        std::process::exit(1);
    }

    let start = Instant::now();
    let mut polls = Vec::new();
    while start.elapsed() < Duration::from_millis(cli.run_ms) {
        std::thread::sleep(Duration::from_millis(cli.poll_ms));
        let elapsed_ms = start.elapsed().as_millis() as u64;
        match query_ascii_once(
            &cli.port,
            cli.baud,
            cli.timeout_ms,
            &query_stored_point_count(cli.channel),
            800,
            1024,
        ) {
            Ok(reply) => {
                let response_elapsed_ms = start.elapsed().as_millis() as u64;
                if let Ok(count) = parse_point_count(&reply) {
                    println!(
                        "t={} ms (response {} ms), stored_points={}",
                        elapsed_ms, response_elapsed_ms, count
                    );
                    polls.push(PollRecord {
                        elapsed_ms,
                        response_elapsed_ms,
                        stored_points: count,
                    });
                } else {
                    println!(
                        "t={} ms (response {} ms), stored_points=parse_error ({})",
                        elapsed_ms, response_elapsed_ms, reply
                    );
                }
            }
            Err(e) => {
                println!("t={} ms, poll_error={}", elapsed_ms, e);
            }
        }
    }

    let _ = send_command_once(&cli.port, cli.baud, cli.timeout_ms, &pause_sampling(cli.channel));
    std::thread::sleep(Duration::from_millis(50));

    let final_stored_points = polls.last().map(|p| p.stored_points).unwrap_or(0);
    let trace_points_to_read = final_stored_points.min(cli.read_points);
    let trace_reply = if trace_points_to_read > 0 {
        query_ascii_once(
            &cli.port,
            cli.baud,
            cli.timeout_ms,
            &query_trace_data(cli.channel, cli.buffer, 0, trace_points_to_read),
            800,
            512 * 1024,
        )
        .unwrap_or_else(|e| format!("ERR: {e}"))
    } else {
        String::new()
    };
    let trace = parse_trace(&trace_reply);
    let elapsed_s = cli.run_ms as f64 / 1000.0;
    let effective_hz = if elapsed_s > 0.0 {
        final_stored_points as f64 / elapsed_s
    } else {
        0.0
    };

    let report = ProbeReport {
        schema_version: 1,
        port: cli.port.clone(),
        baud: cli.baud,
        channel: cli.channel,
        buffer: cli.buffer,
        parameter: cli.parameter,
        sample_step_s: cli.sample_step_s,
        sample_length: cli.sample_length,
        sample_run_mode: cli.sample_run_mode,
        run_ms: cli.run_ms,
        poll_ms: cli.poll_ms,
        configured_step_reply,
        configured_length_reply,
        configured_buffer_reply,
        configured_trigger_reply,
        configured_run_mode_reply,
        device_idn: idn,
        final_stored_points,
        effective_hz,
        trace_points_read: trace.len(),
        trace_preview: trace.iter().take(16).copied().collect(),
        polls,
    };

    let out_subdir = format!("{}/oe1022d_buffer_probe_{}", cli.out_dir, utc_date());
    if let Err(e) = std::fs::create_dir_all(&out_subdir) {
        eprintln!("create out dir failed: {}", e);
        std::process::exit(1);
    }

    let report_json = serde_json::to_string_pretty(&report).expect("serialize report");
    let report_path = format!("{}/buffer_probe_report.json", out_subdir);
    if let Err(e) = std::fs::write(&report_path, report_json) {
        eprintln!("write report failed: {}", e);
        std::process::exit(1);
    }

    let trace_path = format!("{}/trace_reply.txt", out_subdir);
    if let Err(e) = std::fs::write(&trace_path, trace_reply) {
        eprintln!("write trace reply failed: {}", e);
        std::process::exit(1);
    }

    println!("Final stored points: {}", report.final_stored_points);
    println!("Effective Hz: {:.2}", report.effective_hz);
    println!("Trace points parsed: {}", report.trace_points_read);
    println!("Report: {}", report_path);
    println!("Trace reply: {}", trace_path);

    let _ = send_command_once(&cli.port, cli.baud, cli.timeout_ms, &pause_sampling(cli.channel));
    let _ = send_command_once(&cli.port, cli.baud, cli.timeout_ms, &reset_data_buffers(cli.channel));
}
