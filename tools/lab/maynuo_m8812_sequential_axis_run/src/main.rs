//! maynuo-m8812-sequential-axis-run — Mag-M4 sequential multi-axis low-current
//! magnetic run.
//!
//! Sequential basis-vector testing: X 10mA → cleanup → Y 10mA → cleanup →
//! Z 10mA → cleanup. Only one axis is output-enabled at a time.
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
