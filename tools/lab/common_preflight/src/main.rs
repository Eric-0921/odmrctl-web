//! common-preflight-cli — Station-level device preflight tool.
//!
//! Usage:
//!   common-preflight --station-profile station.json --out-dir preflight/
//!
//! Safety invariant: This tool only sends read-only queries and safe-state
//! verification commands. It never enables RF output, magnetic output, or
//! laser emission.

use clap::Parser;
use common_preflight::{run_station_preflight, StationProfile};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "common-preflight")]
struct Cli {
    /// Station profile JSON path
    #[arg(long)]
    station_profile: PathBuf,

    /// Output directory for preflight artifacts
    #[arg(long, default_value = "preflight")]
    out_dir: PathBuf,

    /// Only run preflight, do not proceed to any experiment
    #[arg(long)]
    preflight_only: bool,
}

fn main() {
    let cli = Cli::parse();

    println!("=== Common Preflight ===");
    println!("Profile: {}", cli.station_profile.display());
    println!();

    let profile = match StationProfile::load(cli.station_profile.to_str().unwrap()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: Failed to load station profile: {}", e);
            std::process::exit(1);
        }
    };

    println!("Station: {}", profile.name);
    println!("Devices: {}", profile.devices.len());
    for d in &profile.devices {
        println!("  - {} ({}) @ {}", d.device_id, d.kind, d.address);
    }
    println!();

    let report = match run_station_preflight(&profile) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ERROR: Preflight failed: {}", e);
            std::process::exit(1);
        }
    };

    // Write artifacts
    std::fs::create_dir_all(&cli.out_dir).unwrap_or_default();

    let json_path = cli.out_dir.join("station_preflight_report.json");
    if let Err(e) = common_preflight::station_report::write_json(&report, &json_path) {
        eprintln!("Warning: failed to write JSON report: {}", e);
    } else {
        println!("JSON report: {}", json_path.display());
    }

    let md_path = cli.out_dir.join("station_preflight_report.md");
    if let Err(e) = common_preflight::station_report::write_markdown(&report, &md_path) {
        eprintln!("Warning: failed to write Markdown report: {}", e);
    } else {
        println!("Markdown report: {}", md_path.display());
    }

    // Summary
    println!();
    println!("=== Preflight Result ===");
    println!("All reachable:          {}", if report.all_devices_reachable { "✅" } else { "❌" });
    println!("All identities verified: {}", if report.all_identities_verified { "✅" } else { "❌" });
    println!("All safe states OK:      {}", if report.all_safe_states_confirmed { "✅" } else { "❌" });
    println!("Elapsed: {} ms", report.elapsed_ms);
    println!();

    if report.passed() {
        println!("✅ PASSED — Station is ready for experiment.");
        if cli.preflight_only {
            println!("(--preflight-only: exiting without experiment)");
        }
    } else {
        println!("❌ FAILED — Fix issues before proceeding.");
        for d in &report.devices {
            if !d.warnings.is_empty() || !d.error_queue.is_empty() {
                println!("  [{}] {} warning(s), {} error(s)", d.device_id, d.warnings.len(), d.error_queue.len());
            }
        }
        std::process::exit(1);
    }
}
