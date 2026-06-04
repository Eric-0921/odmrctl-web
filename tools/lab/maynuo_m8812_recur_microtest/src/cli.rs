//! CLI argument definitions for maynuo-m8812-recur-microtest.

use clap::Parser;
use std::path::PathBuf;

/// Mag-M3 single-axis recurrent current / field micro-test.
#[derive(Parser, Debug, Clone)]
#[command(name = "maynuo-m8812-recur-microtest")]
#[command(version = "0.1.0")]
#[command(about = "Single-axis recurrent current / field micro-test for Maynuo M8812")]
pub struct CliArgs {
    /// Path to the Maynuo axes profile JSON.
    #[arg(long, default_value = "examples/magnetic/maynuo_m8812_axes.example.json")]
    pub profile: PathBuf,

    /// Output directory for artifact files.
    #[arg(long, default_value = "out/maynuo_recur_microtest")]
    pub out_dir: PathBuf,

    /// Per-port read timeout in milliseconds.
    #[arg(long, default_value = "300")]
    pub timeout_ms: u64,

    /// Baud rate for serial communication.
    #[arg(long, default_value = "9600")]
    pub baudrate: u32,

    /// Axis to test (required, e.g. "mag_x").
    #[arg(long)]
    pub axis_id: String,

    /// Recurrent current in mA (mutually exclusive with --target-field-nt).
    #[arg(long, default_value = "10.0")]
    pub recur_current_ma: f64,

    /// Target recurrent field in nT (mutually exclusive with --recur-current-ma).
    #[arg(long)]
    pub target_field_nt: Option<f64>,

    /// Settle time after CURR before first MEAS:CURR? (ms).
    #[arg(long, default_value = "2000")]
    pub settle_ms: u64,

    /// Number of repeated MEAS:CURR? queries per phase.
    #[arg(long, default_value = "5")]
    pub samples: u32,

    /// Delay between repeated MEAS:CURR? queries (ms).
    #[arg(long, default_value = "200")]
    pub sample_interval_ms: u64,

    /// Maximum allowed error between commanded and measured recur current (mA).
    #[arg(long, default_value = "2.0")]
    pub max_current_error_ma: f64,

    /// Maximum allowed standard deviation of recur readback samples (mA).
    #[arg(long, default_value = "0.5")]
    pub max_current_std_ma: f64,

    /// Dry-run: enumerate ports without opening them.
    #[arg(long)]
    pub dry_run: bool,

    /// Operator note recorded in the manifest.
    #[arg(long)]
    pub operator_note: Option<String>,

    /// Include only these port paths (repeatable).
    #[arg(long = "include-port")]
    pub include_port: Vec<String>,

    /// Exclude these port paths (repeatable).
    #[arg(long = "exclude-port")]
    pub exclude_port: Vec<String>,

    /// Maximum number of ports to probe.
    #[arg(long)]
    pub max_ports: Option<usize>,
}
