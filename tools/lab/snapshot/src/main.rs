//! lab-snapshot — Read-only real-device station snapshot CLI.
//!
//! **Safety invariant**: this tool only sends pre-defined read-only queries.
//! There is no generic `send(cmd)` API.
//!
//! Usage:
//!   lab-snapshot \
//!     --smb100a-host 169.254.2.20 --smb100a-port 5025 \
//!     --oe1022d-port /dev/cu.usbmodem3361358734371 --oe1022d-baud 115200 \
//!     --out-dir docs/lab-bringup/

use clap::Parser;
use lab_snapshot::{records_to_jsonl, records_to_markdown, Smb100aSnapshot, Oe1022dSnapshot};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lab-snapshot")]
#[command(about = "Read-only real-device station snapshot for ODMR lab bring-up")]
struct Cli {
    /// SMB100A host IP
    #[arg(long, default_value = "169.254.2.20")]
    smb100a_host: String,

    /// SMB100A TCP port
    #[arg(long, default_value = "5025")]
    smb100a_port: u16,

    /// OE1022D serial port
    #[arg(long, default_value = "/dev/cu.usbmodem3361358734371")]
    oe1022d_port: String,

    /// OE1022D baud rate
    #[arg(long, default_value = "115200")]
    oe1022d_baud: u32,

    /// Output directory for markdown and jsonl files
    #[arg(long, default_value = "docs/lab-bringup")]
    out_dir: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    println!("=== ODMR Lab Snapshot (read-only) ===");
    println!("SMB100A: {}:{}", cli.smb100a_host, cli.smb100a_port);
    println!("OE1022D: {} @ {} baud", cli.oe1022d_port, cli.oe1022d_baud);
    println!("Output: {}", cli.out_dir.display());
    println!();

    let smb100a = Smb100aSnapshot::new(&cli.smb100a_host, cli.smb100a_port);
    let oe1022d = Oe1022dSnapshot::new(&cli.oe1022d_port, cli.oe1022d_baud);

    let smb100a_records = match smb100a.run() {
        Ok(recs) => {
            println!("SMB100A: {} queries completed", recs.len());
            recs
        }
        Err(e) => {
            eprintln!("SMB100A snapshot failed: {}", e);
            std::process::exit(1);
        }
    };

    let oe1022d_records = match oe1022d.run() {
        Ok(recs) => {
            println!("OE1022D: {} queries completed", recs.len());
            recs
        }
        Err(e) => {
            eprintln!("OE1022D snapshot failed: {}", e);
            std::process::exit(1);
        }
    };

    std::fs::create_dir_all(&cli.out_dir).expect("create output directory");

    let date = chrono_like_date();
    let md_path = cli.out_dir.join(format!("real_station_snapshot_{}.md", date));
    let smb_jsonl_path = cli
        .out_dir
        .join(format!("smb100a_readonly_observed_{}.jsonl", date));
    let oe_jsonl_path = cli
        .out_dir
        .join(format!("oe1022d_readonly_observed_{}.jsonl", date));

    let md = records_to_markdown(&smb100a_records, &oe1022d_records);
    std::fs::write(&md_path, md).expect("write markdown");
    println!("Markdown written to {}", md_path.display());

    let smb_jsonl = records_to_jsonl(&smb100a_records);
    std::fs::write(&smb_jsonl_path, smb_jsonl).expect("write smb100a jsonl");
    println!("SMB100A JSONL written to {}", smb_jsonl_path.display());

    let oe_jsonl = records_to_jsonl(&oe1022d_records);
    std::fs::write(&oe_jsonl_path, oe_jsonl).expect("write oe1022d jsonl");
    println!("OE1022D JSONL written to {}", oe_jsonl_path.display());

    println!();
    println!("=== Snapshot complete ===");
}

fn chrono_like_date() -> String {
    // Best-effort YYYY-MM-DD using system time (no chrono dependency)
    // This is intentionally simple; filenames only need to be unique per day.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Approximate conversion for display filenames (not calendar-accurate)
    let days = secs / 86400;
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}
