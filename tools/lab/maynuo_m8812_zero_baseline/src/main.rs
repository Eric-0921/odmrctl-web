//! maynuo-m8812-zero-baseline — Mag-M2B zero-mode output-on readback tool.
//!
//! Puts each Maynuo M8812 axis into output-on-zero-mode, measures the actual
//! zero baseline current via MEAS:CURR?, locks zero, and shuts down cleanly.
//!
//! Only allowed SCPI: *IDN?, SYST:REM, VOLT 75, CURR 0.00000, OUTP 1,
//! MEAS:CURR?, OUTP 0, SYST:LOC.

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
