//! M3.4 recipe-shaped SMB100A/OE1022D two-device run with harness, replay, and real modes.

mod app;
mod artifacts;
mod cli;
mod command_audit_compare;
mod command_plan;
mod dry_run;
mod harness;
mod oe_bridge;
mod real_run;
mod recipe;
mod replay;
mod safety;
mod smb_bridge;
mod timeline;
mod types;

#[cfg(test)]
mod tests;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    if let Err(e) = app::run_app(&cli) {
        eprintln!("ERROR: {}", e);
        std::process::exit(1);
    }
}
