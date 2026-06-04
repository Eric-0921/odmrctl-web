//! maynuo-m8812-recur-microtest — Mag-M3 single-axis recurrent current micro-test.
//!
//! Single-axis only. Verifies: zero baseline → lock-zero → CURR nonzero →
//! readback reconstruction of recur current / field → cleanup.
//!
//! SCPI: *IDN?, SYST:REM, VOLT 75, CURR 0, OUTP 1, MEAS:CURR?,
//! CURR {nonzero}, OUTP 0, SYST:LOC.

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
