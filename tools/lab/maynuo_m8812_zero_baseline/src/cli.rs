//! CLI argument definitions for maynuo-m8812-zero-baseline.

use clap::Parser;
use std::path::PathBuf;

/// Mag-M2B zero-mode output-on readback + lock-zero baseline capture.
#[derive(Parser, Debug, Clone)]
#[command(name = "maynuo-m8812-zero-baseline")]
#[command(version = "0.1.0")]
#[command(about = "Zero-mode output-on readback + lock-zero baseline for Maynuo M8812 axes")]
pub struct CliArgs {
    /// Path to the Maynuo axes profile JSON.
    #[arg(long, default_value = "examples/magnetic/maynuo_m8812_axes.example.json")]
    pub profile: PathBuf,

    /// Output directory for artifact files.
    #[arg(long, default_value = "out/maynuo_zero_baseline")]
    pub out_dir: PathBuf,

    /// Per-port read timeout in milliseconds.
    #[arg(long, default_value = "300")]
    pub timeout_ms: u64,

    /// Baud rate for serial communication.
    #[arg(long, default_value = "9600")]
    pub baudrate: u32,

    /// Settle time after OUTP 1 before first MEAS:CURR? (ms).
    #[arg(long, default_value = "2000")]
    pub settle_ms: u64,

    /// Number of repeated MEAS:CURR? queries for zero baseline averaging.
    #[arg(long, default_value = "5")]
    pub zero_samples: u32,

    /// Delay between repeated MEAS:CURR? queries (ms).
    #[arg(long, default_value = "200")]
    pub sample_interval_ms: u64,

    /// Dry-run: enumerate ports without opening them.
    #[arg(long)]
    pub dry_run: bool,

    /// Operator note recorded in the manifest.
    #[arg(long)]
    pub operator_note: Option<String>,

    /// Strict mode: unknown Maynuo SN causes overall failure.
    #[arg(long)]
    pub strict: bool,

    /// Include only these port paths (repeatable).
    #[arg(long = "include-port")]
    pub include_port: Vec<String>,

    /// Exclude these port paths (repeatable).
    #[arg(long = "exclude-port")]
    pub exclude_port: Vec<String>,

    /// Maximum number of ports to probe.
    #[arg(long)]
    pub max_ports: Option<usize>,

    /// Process only this axis_id (e.g. "mag_x"). Skips other axes.
    #[arg(long)]
    pub axis_id: Option<String>,
}
