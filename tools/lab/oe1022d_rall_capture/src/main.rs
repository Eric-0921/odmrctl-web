//! OE1022D RALL? Raw Capture CLI.
//!
//! Captures raw binary frames from OE1022D over serial, then runs an offline
//! parser probe to inspect candidate encodings.
//!
//! ## Usage
//!
//! ```bash
//! cargo run -- --port /dev/cu.usbmodem3361358734371 --baud 921600 --frames 100 --delay-ms 20
//! ```

use clap::Parser;
use oe1022d_rall_capture::{
    capture_report_to_markdown, probe_results_to_json, probe_results_to_markdown,
    records_to_jsonl, Oe1022dRallCapture,
};

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = "/dev/cu.usbmodem3361358734371")]
    port: String,
    #[arg(long, default_value = "921600")]
    baud: u32,
    #[arg(long, default_value = "10")]
    frames: u32,
    #[arg(long, default_value = "20")]
    delay_ms: u64,
    #[arg(long, default_value = "2000")]
    timeout_ms: u64,
    #[arg(long, default_value = "docs/lab-bringup")]
    out_dir: String,
}

fn main() {
    let cli = Cli::parse();

    // Validate CLI limits
    if cli.frames > 1000 {
        eprintln!("Error: frames must be <= 1000 (got {})", cli.frames);
        std::process::exit(1);
    }
    if cli.timeout_ms > 5000 {
        eprintln!("Error: timeout-ms must be <= 5000 (got {})", cli.timeout_ms);
        std::process::exit(1);
    }

    println!("========================================");
    println!("  OE1022D RALL? Raw Capture");
    println!("========================================");
    println!();
    println!("Port:   {} @ {} baud", cli.port, cli.baud);
    println!("Frames: {} (delay {} ms, timeout {} ms)", cli.frames, cli.delay_ms, cli.timeout_ms);
    println!();

    let capture = Oe1022dRallCapture::new(&cli.port, cli.baud);

    // Verify identity
    println!("Verifying identity...");
    let idn = match capture.verify_identity() {
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

    // Capture frames
    println!("Capturing {} frames...", cli.frames);
    let (records, raw_payload) = match capture.capture(cli.frames, cli.delay_ms) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Capture failed: {}", e);
            std::process::exit(1);
        }
    };

    let captured = records.iter().filter(|r| r.pass_fail == "pass").count();
    let timeouts = records.iter().filter(|r| r.pass_fail == "timeout").count();
    println!("Captured: {}, Timeouts: {}", captured, timeouts);
    println!();

    // Use first captured frame for parser probe
    let first_frame = records.iter().find(|r| r.pass_fail == "pass");
    let probe_results = if let Some(first) = first_frame {
        let start = first.offset_bytes as usize;
        let end = start + first.length_bytes;
        let frame_payload = &raw_payload[start..end.min(raw_payload.len())];
        println!("Running parser probe on frame {} ({} bytes)...", first.frame_index, frame_payload.len());
        oe1022d_rall_capture::probe_frame(frame_payload)
    } else {
        println!("No frames captured; skipping parser probe.");
        Vec::new()
    };

    // Write output files
    let today = utc_date();
    let out_subdir = format!("{}/rall_capture_{}", cli.out_dir, today);
    if let Err(e) = std::fs::create_dir_all(&out_subdir) {
        eprintln!("Warning: could not create out_dir: {}", e);
    }

    // rawbin: append each frame with length prefix
    let rawbin_path = format!("{}/rall_frames.rawbin", out_subdir);
    let mut rawbin = Vec::new();
    for r in &records {
        if r.pass_fail == "pass" {
            let start = r.offset_bytes as usize;
            let end = start + r.length_bytes;
            let payload = &raw_payload[start..end.min(raw_payload.len())];
            // Length-prefixed: u32_le len + payload
            rawbin.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            rawbin.extend_from_slice(payload);
        }
    }
    match std::fs::write(&rawbin_path, rawbin) {
        Ok(_) => println!("Wrote rawbin: {}", rawbin_path),
        Err(e) => eprintln!("Failed to write rawbin: {}", e),
    }

    // index.jsonl
    let index_path = format!("{}/rall_index.jsonl", out_subdir);
    let index_jsonl = records_to_jsonl(&records);
    match std::fs::write(&index_path, index_jsonl) {
        Ok(_) => println!("Wrote index: {}", index_path),
        Err(e) => eprintln!("Failed to write index: {}", e),
    }

    // observed_commands.jsonl
    let observed_path = format!("{}/observed_commands.jsonl", out_subdir);
    let observed = format!(
        "{{\"command\":\"*IDN?\",\"response\":\"{}\"}}\n",
        oe1022d_rall_capture::escape_json(&idn)
    );
    match std::fs::write(&observed_path, observed) {
        Ok(_) => println!("Wrote observed commands: {}", observed_path),
        Err(e) => eprintln!("Failed to write observed: {}", e),
    }

    // parser_probe_summary.json
    let probe_json_path = format!("{}/parser_probe_summary.json", out_subdir);
    let probe_json = probe_results_to_json(&probe_results);
    match std::fs::write(&probe_json_path, probe_json) {
        Ok(_) => println!("Wrote parser probe JSON: {}", probe_json_path),
        Err(e) => eprintln!("Failed to write parser probe JSON: {}", e),
    }

    // parser_probe_summary.md
    let probe_md_path = format!("{}/parser_probe_summary.md", out_subdir);
    let probe_md = probe_results_to_markdown(&probe_results);
    match std::fs::write(&probe_md_path, probe_md) {
        Ok(_) => println!("Wrote parser probe MD: {}", probe_md_path),
        Err(e) => eprintln!("Failed to write parser probe MD: {}", e),
    }

    // capture_report.md
    let report_path = format!("{}/capture_report.md", out_subdir);
    let report_md = capture_report_to_markdown(&records, &idn, raw_payload.len());
    match std::fs::write(&report_path, report_md) {
        Ok(_) => println!("Wrote capture report: {}", report_path),
        Err(e) => eprintln!("Failed to write capture report: {}", e),
    }

    println!("\nDone.");
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
