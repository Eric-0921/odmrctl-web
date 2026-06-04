//! maynuo-m8812-identity-probe — Identity-only Maynuo M8812 discovery tool.
//!
//! Enumerates serial ports, sends only `*IDN?`, maps observed serial numbers
//! to logical magnetic axes (X/Y/Z).  No current or output commands are sent.
//!
//! ## Safety
//!
//! This tool is identity-only.  It does NOT send SYST:REM, VOLT, CURR, OUTP,
//! MEAS:CURR?, or any other state-changing command.  The only SCPI command
//! emitted is `*IDN?`.

mod app;
mod artifacts;
mod cli;
mod types;

use clap::Parser;

fn main() {
    let args = cli::CliArgs::parse();
    if let Err(e) = app::run(&args) {
        eprintln!("ERROR: {e}");
        std::process::exit(1);
    }
}
