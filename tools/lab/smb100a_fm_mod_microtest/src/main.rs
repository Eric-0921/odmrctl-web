//! M3.1: SMB100A Fixed-Frequency FM/MOD ON/OFF Micro-test.
//!
//! Controlled RF/FM/MOD micro-test with full command audit and safety evidence.
//! This is NOT an ODMR experiment. NOT a sweep. NOT GUI-controlled.
//!
//! ## Safety
//! - FM:STAT ON / MOD:STAT ON / OUTP ON require `--operator-approves-fm-mod-on`.
//! - Pre-flight must confirm OUTP=OFF, MOD:STAT=OFF, SYST:ERR clean.
//! - Power, FM deviation, and duration have hard limits.
//! - Emergency shutdown sends OUTP OFF, MOD:STAT OFF, FM:STAT OFF.
//! - No sweep. No LF output enable. No CSV. No GUI. No magnetic devices.

use clap::Parser;

mod app;
mod artifacts;
mod cli;
mod safety;
mod sequence;
mod shutdown;
mod timeline;
mod transport;
mod types;

pub use artifacts::{sha256_bytes, sha256_file, write_jsonl};
pub use cli::Cli;
pub use safety::{
    is_safety_relevant, validate_lf_shape, validate_microtest_set_command,
    validate_smb_query_only, LF_SHAPE_ALLOWLIST, SMB_FORBIDDEN_PATTERNS,
    SMB_MICROTEST_SET_ALLOWLIST, SMB_QUERY_ALLOWLIST,
};
pub use sequence::run_microtest;
pub use timeline::{make_event, utc_now_ms};
pub use types::*;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = app::run_app(&cli) {
        eprintln!("FM/MOD micro-test failed: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests;
