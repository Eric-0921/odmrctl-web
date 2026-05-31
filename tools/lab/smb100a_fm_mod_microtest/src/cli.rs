//! CLI arguments for the M3.1 SMB100A FM/MOD micro-test tool.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "smb100a-fm-mod-microtest")]
#[command(about = "M3.1: SMB100A fixed-frequency FM/MOD ON/OFF micro-test")]
pub struct Cli {
    #[arg(long, default_value = "169.254.2.20")]
    pub smb_host: String,

    #[arg(long, default_value = "5025")]
    pub smb_port: u16,

    #[arg(long, default_value = "50")]
    pub smb_query_delay_ms: u64,

    #[arg(long, default_value = "3000")]
    pub smb_timeout_ms: u64,

    #[arg(long, default_value = "../../runs")]
    pub run_root: String,

    #[arg(long)]
    pub run_id: String,

    #[arg(long, default_value = "2882000000")]
    pub rf_frequency_hz: f64,

    #[arg(long, default_value = "-30")]
    pub rf_power_dbm: f64,

    #[arg(long, default_value = "-20")]
    pub max_rf_power_dbm: f64,

    #[arg(long, default_value = "4000000")]
    pub fm_deviation_hz: f64,

    #[arg(long, default_value = "5000000")]
    pub max_fm_deviation_hz: f64,

    #[arg(long, default_value = "1000")]
    pub fm_on_duration_ms: u64,

    #[arg(long)]
    pub set_internal_lf: bool,

    #[arg(long, default_value = "500")]
    pub lf_frequency_hz: f64,

    #[arg(long, default_value = "SQU")]
    pub lf_shape: String,

    #[arg(long, default_value = "0.137")]
    pub lf_voltage_v: f64,

    /// Operator explicitly approves sending FM:STAT ON / MOD:STAT ON / OUTP ON.
    #[arg(long)]
    pub operator_approves_fm_mod_on: bool,

    /// Optional operator approval note recorded in artifacts.
    #[arg(long)]
    pub operator_approval_note: Option<String>,

    /// Leave FM configuration enabled after test (do not send FM:STAT OFF).
    #[arg(long)]
    pub leave_fm_config_enabled: bool,
}
