//! M3.2: SMB100A Software-Stepped Mini Sweep + OE1022D Passive Acquisition.
//!
//! First two-device coordinated real-hardware test. SMB100A software-steps
//! through a frequency range while OE1022D passively acquires RALL? frames
//! at each step.
//!
//! This is NOT a full ODMR experiment. NOT an internal sweep. NOT GUI.
//!
//! ## Safety
//! - FM:STAT ON / MOD:STAT ON / OUTP ON require `--operator-approves-step-sweep`
//! - Pre-flight must confirm OUTP=OFF, MOD:STAT=OFF, SYST:ERR clean
//! - Power, FM deviation, points, and frames-per-step have hard limits
//! - Emergency shutdown sends OUTP OFF, MOD:STAT OFF, FM:STAT OFF
//! - No internal sweep commands. No CSV. No magnetic devices.

use clap::Parser;

mod alignment;
mod app;
mod artifacts;
mod cli;
mod oe_acquisition;
mod oe_transport;
mod shutdown;
mod smb_safety;
mod smb_sequence;
mod smb_transport;
mod timeline;
mod types;

pub use alignment::build_alignment_summary;
pub use app::run_app;
pub use artifacts::{sha256_bytes, sha256_file, write_jsonl};
pub use cli::Cli;
pub use oe_acquisition::{acquire_frames, build_alignment_for_step, OeFrameCapture};
pub use oe_transport::OeSerialTransport;
pub use shutdown::attempt_emergency_shutdown;
pub use smb_safety::{
    validate_lf_shape, validate_oe_command, validate_smb_sweep_query, validate_smb_sweep_set,
    LF_SHAPE_ALLOWLIST, OE_ALLOWLIST, SMB_FORBIDDEN_PATTERNS, SMB_SWEEP_QUERY_ALLOWLIST,
    SMB_SWEEP_SET_ALLOWLIST,
};
pub use smb_sequence::{
    compute_step_plan, configure_smb_common, execute_step_rf_off, execute_step_rf_on,
    run_final_shutdown, run_preflight, validate_safety_limits,
};
pub use smb_transport::{do_smb_sweep_query, do_smb_sweep_set, SmbTransport};
pub use timeline::{make_event, utc_now_ms, TimelineTracker};
pub use types::*;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = app::run_app(&cli) {
        eprintln!("M3.2 step sweep failed: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests;
