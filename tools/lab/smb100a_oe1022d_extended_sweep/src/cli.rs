use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "smb100a-oe1022d-extended-sweep")]
#[command(about = "M3.3: Extended SMB100A software-stepped RF sweep + OE1022D passive acquisition with repeat")]
pub struct Cli {
    // --- SMB100A ---
    #[arg(long, default_value = "169.254.2.20")]
    pub smb_host: String,

    #[arg(long, default_value = "5025")]
    pub smb_port: u16,

    #[arg(long, default_value = "50")]
    pub smb_query_delay_ms: u64,

    #[arg(long, default_value = "3000")]
    pub smb_timeout_ms: u64,

    // --- OE1022D ---
    #[arg(long, default_value = "/dev/cu.usbmodem3361358734371")]
    pub oe_port: String,

    #[arg(long, default_value = "921600")]
    pub oe_baud: u32,

    #[arg(long, default_value = "8000")]
    pub oe_timeout_ms: u64,

    // --- Run ---
    #[arg(long, default_value = "../../runs")]
    pub run_root: String,

    #[arg(long)]
    pub run_id: String,

    // --- RF sweep ---
    #[arg(long, default_value = "2878000000")]
    pub rf_start_hz: f64,

    #[arg(long, default_value = "2886000000")]
    pub rf_stop_hz: f64,

    #[arg(long, default_value = "11")]
    pub rf_points: u64,

    #[arg(long, default_value = "-30")]
    pub rf_power_dbm: f64,

    #[arg(long, default_value = "-20")]
    pub max_rf_power_dbm: f64,

    // --- Repeat ---
    #[arg(long, default_value = "2")]
    pub repeat_count: u64,

    // --- FM ---
    #[arg(long, default_value = "4000000")]
    pub fm_deviation_hz: f64,

    #[arg(long, default_value = "5000000")]
    pub max_fm_deviation_hz: f64,

    // --- LF ---
    #[arg(long)]
    pub set_internal_lf: bool,

    #[arg(long, default_value = "500")]
    pub lf_frequency_hz: f64,

    #[arg(long, default_value = "SQU")]
    pub lf_shape: String,

    #[arg(long, default_value = "0.137")]
    pub lf_voltage_v: f64,

    // --- OE acquisition ---
    #[arg(long, default_value = "5")]
    pub frames_per_step: u64,

    #[arg(long, default_value = "20")]
    pub inter_frame_delay_ms: u64,

    #[arg(long, default_value = "800")]
    pub oe_frame_delay_ms: u64,

    // --- Safety ---
    /// Operator explicitly approves the extended sweep with FM/MOD/RF output.
    #[arg(long)]
    pub operator_approves_extended_sweep: bool,

    /// Optional operator approval note recorded in artifacts.
    #[arg(long)]
    pub operator_approval_note: Option<String>,

    /// Leave FM configuration enabled after test (skip FM:STAT OFF in shutdown).
    #[arg(long)]
    pub leave_fm_config_enabled: bool,
}
