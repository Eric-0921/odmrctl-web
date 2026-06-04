//! CLI for Mag-M5A minimal combined run.

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "rf-mag-oe-minimal-run",
    about = "Mag-M5A: minimal RF + single-axis magnetic + OE combined run"
)]
pub struct Cli {
    // ---- SMB100A ----
    #[arg(long, default_value = "169.254.2.20")]
    pub smb_host: String,

    #[arg(long, default_value = "5025")]
    pub smb_port: u16,

    #[arg(long, default_value = "50")]
    pub smb_query_delay_ms: u64,

    #[arg(long, default_value = "3000")]
    pub smb_timeout_ms: u64,

    // ---- OE1022D ----
    #[arg(long, default_value = "/dev/cu.usbmodem3361358734371")]
    pub oe_port: String,

    #[arg(long, default_value = "921600")]
    pub oe_baud: u32,

    #[arg(long, default_value = "5000")]
    pub oe_timeout_ms: u64,

    #[arg(long, default_value = "800")]
    pub oe_frame_delay_ms: u64,

    // ---- Magnetic ----
    #[arg(
        long,
        default_value = "../../../examples/magnetic/maynuo_m8812_axes.example.json"
    )]
    pub mag_profile: PathBuf,

    #[arg(long, default_value = "mag_x")]
    pub mag_axis_id: String,

    #[arg(long, default_value = "10.0")]
    pub mag_recur_current_ma: f64,

    #[arg(long, default_value = "5")]
    pub mag_samples: u64,

    // ---- RF settings ----
    #[arg(long, default_value = "2882000000")]
    pub rf_frequency_hz: u64,

    #[arg(long, default_value = "-30.0")]
    pub rf_power_dbm: f64,

    // ---- Acquisition ----
    #[arg(long, default_value = "10")]
    pub frames: u64,

    #[arg(long, default_value = "2000")]
    pub settle_ms: u64,

    // ---- Output ----
    #[arg(long, default_value = "out/rf_mag_oe_minimal_run")]
    pub out_dir: PathBuf,

    // ---- Operator ----
    #[arg(long)]
    pub operator_approve: bool,

    #[arg(long)]
    pub operator_note: Option<String>,

    // ---- Preflight ----
    #[arg(long)]
    pub station_profile: Option<PathBuf>,

    #[arg(long)]
    pub preflight_only: bool,

    // ---- Dry-run ----
    #[arg(long)]
    pub dry_run: bool,
}
