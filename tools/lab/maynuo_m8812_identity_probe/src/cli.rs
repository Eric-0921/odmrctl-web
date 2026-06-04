//! CLI argument parsing for the Maynuo M8812 identity probe tool.

use clap::Parser;
use std::path::PathBuf;

/// Identity-only Maynuo M8812 discovery tool.
///
/// Enumerates serial ports, sends only *IDN?, and maps observed SN values
/// to logical magnetic axes (X/Y/Z).  No current or output commands are sent.
#[derive(Parser, Debug, Clone)]
#[command(name = "maynuo-m8812-identity-probe")]
#[command(version = "0.1.0")]
#[command(about = "Identity-only Maynuo M8812 discovery — *IDN? probe, no current or output")]
pub struct CliArgs {
    /// Path to the Maynuo axes profile JSON.
    #[arg(long, default_value = "examples/magnetic/maynuo_m8812_axes.example.json")]
    pub profile: PathBuf,

    /// Output directory for artifact files.
    #[arg(long, default_value = "out/maynuo_identity_probe")]
    pub out_dir: PathBuf,

    /// Per-port read timeout in milliseconds.
    #[arg(long, default_value = "300")]
    pub timeout_ms: u64,

    /// Baud rate for serial communication.
    #[arg(long, default_value = "9600")]
    pub baudrate: u32,

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
}
