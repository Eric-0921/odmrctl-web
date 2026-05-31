//! CLI for M3.4 recipe-shaped two-device run. Supports harness-fake, replay, and real modes.

use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "recipe-two-device-run",
    about = "M3.4 recipe-shaped SMB100A/OE1022D two-device run"
)]
pub struct Cli {
    /// Run mode: harness-fake, replay, or real
    #[arg(long, default_value = "harness-fake")]
    pub mode: String,

    /// Path to recipe JSON file
    #[arg(
        long,
        default_value = "examples/recipes/m3_4_two_device_sweep.recipe.json"
    )]
    pub recipe: String,

    // ---- Run directory ----
    #[arg(long, default_value = "../../runs")]
    pub run_root: String,

    #[arg(long)]
    pub run_id: String,

    // ---- SMB100A connection (real mode) ----
    #[arg(long, default_value = "169.254.2.20")]
    pub smb_host: String,

    #[arg(long, default_value = "5025")]
    pub smb_port: u16,

    #[arg(long, default_value = "50")]
    pub smb_query_delay_ms: u64,

    #[arg(long, default_value = "3000")]
    pub smb_timeout_ms: u64,

    // ---- OE1022D connection (real mode) ----
    #[arg(long, default_value = "/dev/cu.usbmodem3361358734371")]
    pub oe_port: String,

    #[arg(long, default_value = "921600")]
    pub oe_baud: u32,

    #[arg(long, default_value = "8000")]
    pub oe_timeout_ms: u64,

    #[arg(long, default_value = "800")]
    pub oe_frame_delay_ms: u64,

    // ---- Run parameters ----
    #[arg(long)]
    pub operator_approves_real_run: bool,

    #[arg(long)]
    pub operator_approval_note: Option<String>,

    #[arg(long)]
    pub leave_fm_config_enabled: bool,

    // ---- Replay mode ----
    #[arg(long)]
    pub replay_run: Option<String>,

    #[arg(long)]
    pub replay_run_root: Option<String>,

    // ---- Harness options ----
    #[arg(long)]
    pub inject_parse_failures: bool,

    #[arg(long, default_value = "0.05")]
    pub parse_failure_rate: f64,
}
