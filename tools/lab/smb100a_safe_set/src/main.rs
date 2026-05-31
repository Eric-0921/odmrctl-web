//! SMB100A Safe-Set Audit CLI.
//!
//! Human-in-the-loop tool that sends only pre-approved safe-set commands
//! to a real SMB100A over TCP/SCPI. Each setter requires operator
//! confirmation before transmission.
//!
//! ## Usage
//!
//! ```bash
//! cargo run -- --host 169.254.2.20 --port 5025
//! ```
//!
//! The tool will prompt for confirmation before each of the 12 safe-set steps.
//! Type `Y` (or Enter) to proceed, `n` to skip, or `abort` to stop and
//! send safe-disconnect commands.

use clap::Parser;
use smb100a_safe_set::{
    records_to_jsonl, records_to_markdown, SafeSetError, SafeSetStep, Smb100aSafeSet,
};
use std::io::{self, Write};

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = "169.254.2.20")]
    host: String,
    #[arg(long, default_value = "5025")]
    port: u16,
    #[arg(long, default_value = "docs/lab-bringup")]
    out_dir: String,
}

fn main() {
    let cli = Cli::parse();
    let runner = Smb100aSafeSet::new(&cli.host, cli.port);

    println!("========================================");
    println!("  SMB100A Safe-Set Audit");
    println!("========================================");
    println!();
    println!("Target: {}:{}", cli.host, cli.port);
    println!();
    println!("This tool will send ONLY pre-approved safe-set commands.");
    println!("RF output will remain OFF. Modulation will remain OFF.");
    println!();
    println!("Type Y  (or Enter) to confirm each step.");
    println!("Type n  to skip a step.");
    println!("Type abort to stop and disconnect safely.");
    println!();

    let safe_set_records = match runner.run(|i, step, resp_before| {
        prompt_step(i, step, resp_before)
    }) {
        Ok(recs) => recs,
        Err(SafeSetError::Aborted) => {
            println!("\nAborted by operator. Sending safe-disconnect...");
            // Attempt to ensure safe state even on abort.
            let _ = send_safe_disconnect(&cli.host, cli.port);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error during safe-set sequence: {}", e);
            std::process::exit(1);
        }
    };

    println!("\nSafe-set sequence complete. Running final validation...\n");

    let final_records = match runner.run_final_validation() {
        Ok(recs) => recs,
        Err(e) => {
            eprintln!("Error during final validation: {}", e);
            std::process::exit(1);
        }
    };

    // Print final validation results
    println!("Final Validation Results:");
    println!("-------------------------");
    for r in &final_records {
        println!(
            "  {} → {} ({})",
            r.query_before,
            r.response_before.as_deref().unwrap_or("TIMEOUT"),
            r.pass_fail
        );
    }
    println!();

    // Write output files
    let today = utc_date();
    let md_path = format!("{}/smb100a_safe_set_audit_{}.md", cli.out_dir, today);
    let jsonl_path = format!(
        "examples/verification/smb100a_safe_set_observed_{}.jsonl",
        today
    );

    let md = records_to_markdown(&safe_set_records, &final_records);
    let jsonl = records_to_jsonl(&safe_set_records);
    let final_jsonl = records_to_jsonl(&final_records);
    let combined_jsonl = format!("{}\n{}", jsonl, final_jsonl);

    if let Err(e) = std::fs::create_dir_all(&cli.out_dir) {
        eprintln!("Warning: could not create out_dir: {}", e);
    }
    if let Err(e) = std::fs::create_dir_all("examples/verification") {
        eprintln!("Warning: could not create examples/verification: {}", e);
    }

    match std::fs::write(&md_path, md) {
        Ok(_) => println!("Wrote Markdown report: {}", md_path),
        Err(e) => eprintln!("Failed to write {}: {}", md_path, e),
    }

    match std::fs::write(&jsonl_path, combined_jsonl) {
        Ok(_) => println!("Wrote JSONL observations: {}", jsonl_path),
        Err(e) => eprintln!("Failed to write {}: {}", jsonl_path, e),
    }

    println!("\nDone.");
}

fn prompt_step(
    i: usize,
    step: &SafeSetStep,
    resp_before: Option<&str>,
) -> Result<bool, SafeSetError> {
    println!(
        "Step {:2}/12: {}",
        i + 1,
        step.command
    );
    println!(
        "  Before: {} = {}",
        step.query_before,
        resp_before.unwrap_or("(timeout/no response)")
    );
    print!("  Confirm? [Y/n/abort] > ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return Err(SafeSetError::Aborted);
    }

    match input.trim() {
        "Y" | "y" | "" => {
            println!("  → Sending {} ...", step.command);
            Ok(true)
        }
        "n" | "N" => {
            println!("  → Skipped.");
            Ok(false)
        }
        "abort" => {
            println!("  → Aborting.");
            Err(SafeSetError::Aborted)
        }
        other => {
            println!("  Unrecognized '{}', treating as skip.", other);
            Ok(false)
        }
    }
}

fn send_safe_disconnect(host: &str, port: u16) -> Result<(), SafeSetError> {
    use std::io::Write;
    use std::net::TcpStream;
    use std::time::Duration;

    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| SafeSetError::IoError(format!("{}", e)))?,
        Duration::from_millis(2000),
    )
    .map_err(|e| SafeSetError::IoError(format!("{}", e)))?;

    let _ = stream.write_all(b"OUTP OFF\n");
    let _ = stream.write_all(b"MOD:STAT OFF\n");
    let _ = stream.flush();
    println!("Safe-disconnect sent: OUTP OFF, MOD:STAT OFF");
    Ok(())
}

fn utc_date() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Simple epoch-to-YYYY-MM-DD conversion
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
