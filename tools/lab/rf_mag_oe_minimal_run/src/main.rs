//! Mag-M5A: minimal RF + single-axis magnetic + OE combined run tool.
//!
//! Execution order:
//!   1. Create run directory
//!   2. Load magnetic axis profile
//!   3. SMB preflight (identity, state, error queue)
//!   4. OE preflight (identity, acquisition ready)
//!   5. Maynuo identity (enumerate ports, SN match)
//!   6. Maynuo zero-baseline (remote → volt → curr 0 → outp 1 → measure → lock-zero)
//!   7. Maynuo recurrent current (curr nonzero → settle → measure → reconstruct)
//!   8. SMB RF ON (freq → pow → outp on → verify)
//!   9. OE acquisition (N frames, raw-first)
//!  10. Cleanup (RF OFF → mag curr 0 → outp 0 → local → final verify)

mod app;
mod artifacts;
mod cli;
mod mag_bridge;
mod oe_bridge;
mod smb_bridge;
mod types;

#[cfg(test)]
mod tests;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    if let Err(e) = app::run(&cli) {
        eprintln!("ERROR: {}", e);
        std::process::exit(1);
    }
}
