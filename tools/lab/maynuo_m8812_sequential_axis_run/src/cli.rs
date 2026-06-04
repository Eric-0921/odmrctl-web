//! CLI argument definitions for maynuo-m8812-sequential-axis-run.

use clap::Parser;
use std::path::PathBuf;

/// Mag-M4 sequential multi-axis low-current magnetic run.
#[derive(Parser, Debug, Clone)]
#[command(name = "maynuo-m8812-sequential-axis-run")]
#[command(version = "0.1.0")]
#[command(about = "Sequential multi-axis low-current magnetic run for Maynuo M8812")]
pub struct CliArgs {
    /// Path to the Maynuo axes profile JSON.
    #[arg(long, default_value = "examples/magnetic/maynuo_m8812_axes.example.json")]
    pub profile: PathBuf,

    /// Output directory for artifact files.
    #[arg(long, default_value = "out/maynuo_sequential_axis_run")]
    pub out_dir: PathBuf,

    /// Per-port read timeout in milliseconds.
    #[arg(long, default_value = "300")]
    pub timeout_ms: u64,

    /// Baud rate for serial communication.
    #[arg(long, default_value = "9600")]
    pub baudrate: u32,

    /// Recurrent current in mA for all axes.
    #[arg(long, default_value = "10.0")]
    pub recur_current_ma: f64,

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

    /// Axes to process (default: all three in order mag_x, mag_y, mag_z).
    #[arg(long = "axis-id", default_values_t = vec![String::from("mag_x"), String::from("mag_y"), String::from("mag_z")])]
    pub axis_ids: Vec<String>,
}
