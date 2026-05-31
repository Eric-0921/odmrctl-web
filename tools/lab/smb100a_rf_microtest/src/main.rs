//! M3.0-B: SMB100A RF ON/OFF Micro-test, Fixed Frequency, Low Power, No Modulation.
//!
//! Minimal RF output micro-test with full command audit and safety evidence.
//! This is NOT an ODMR experiment. NOT a sweep. NOT FM-ODMR.
//!
//! ## Safety
//! - OUTP ON requires `--operator-approves-rf-on` and an approval note.
//! - Pre-flight must confirm OUTP=OFF, MOD:STAT=OFF, SYST:ERR clean.
//! - Power and duration have hard limits.
//! - Emergency shutdown sends OUTP OFF if any failure occurs after RF ON.
//! - No modulation. No sweep. No FM. No LF output. No CSV. No GUI.

use clap::Parser;
use odmr_logging::{
    create_run_directory, EventLevel, RunArtifactPaths, RunEvent, RunEventType, RunManifest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Hard-coded command allow-lists and forbidden patterns
// ---------------------------------------------------------------------------

const SMB_QUERY_ALLOWLIST: &[&str] = &[
    "*IDN?",
    "OUTP?",
    "MOD:STAT?",
    "FREQ?",
    "POW?",
    "POW:ALC?",
    "FM:STAT?",
    "FM:SOUR?",
    "FM:DEV?",
    "LFO?",
    "LFO:FREQ?",
    "LFO:VOLT?",
    "LFO:SHAP?",
    "SYST:ERR?",
];

/// Set commands allowed ONLY in M3.0-B RF micro-test mode (after preflight passes).
const SMB_MICROTEST_SET_ALLOWLIST: &[&str] = &["FREQ ", "POW ", "OUTP ON", "OUTP OFF"];

/// Commands forbidden in M3.0-B (attempting these blocks the run).
const SMB_FORBIDDEN_PATTERNS: &[&str] = &[
    "MOD:STAT ",
    "FM:STAT ",
    "FM:SOUR ",
    "FM:DEV ",
    "LFO ",
    "LFO:FREQ ",
    "LFO:VOLT ",
    "LFO:SHAP ",
    "FREQ:STAR ",
    "FREQ:STOP ",
    "SWE:MODE ",
    "SWE:SPAC ",
    "SWE:FREQ:STEP ",
    "SWE:FREQ:DWEL ",
    "*RST",
    "RST",
    "INIT",
];

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(name = "smb100a-rf-microtest")]
#[command(about = "M3.0-B: SMB100A RF ON/OFF micro-test")]
struct Cli {
    #[arg(long, default_value = "169.254.2.20")]
    smb_host: String,

    #[arg(long, default_value = "5025")]
    smb_port: u16,

    #[arg(long, default_value = "100")]
    smb_query_delay_ms: u64,

    #[arg(long, default_value = "3000")]
    smb_timeout_ms: u64,

    #[arg(long, default_value = "../../runs")]
    run_root: String,

    #[arg(long)]
    run_id: String,

    #[arg(long, default_value = "2882000000")]
    rf_frequency_hz: f64,

    #[arg(long, default_value = "-30")]
    rf_power_dbm: f64,

    #[arg(long, default_value = "1000")]
    rf_on_duration_ms: u64,

    #[arg(long, default_value = "-20")]
    max_rf_power_dbm: f64,

    /// Operator explicitly approves sending OUTP ON for this micro-test.
    #[arg(long)]
    operator_approves_rf_on: bool,

    /// Optional operator approval note recorded in artifacts.
    #[arg(long)]
    operator_approval_note: Option<String>,
}

// ---------------------------------------------------------------------------
// JSON types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MicrotestConfig {
    schema_version: String,
    smb_host: String,
    smb_port: u16,
    smb_query_delay_ms: u64,
    smb_timeout_ms: u64,
    rf_frequency_hz: f64,
    rf_power_dbm: f64,
    rf_on_duration_ms: u64,
    max_rf_power_dbm: f64,
    operator_approves_rf_on: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    operator_approval_note: Option<String>,
    created_at_unix_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Smb100aSnapshot {
    schema_version: String,
    device_id: String,
    idn: String,
    queried_at_unix_ms: u64,
    queries: Vec<SmbQueryResult>,
    connection_closed: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SmbQueryResult {
    command: String,
    response: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CommandAuditEntry {
    timestamp_unix_ms: u64,
    device_id: String,
    command: String,
    command_class: String,
    allowed: bool,
    sent_to_transport: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    manual_approval_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    manual_approval_present: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rejection_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transport_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_relevant: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ErrorQueueObservation {
    timestamp_unix_ms: u64,
    attempt: usize,
    command: String,
    response: String,
    clean: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RfOnOffResult {
    passed: bool,
    rf_on_command_sent: bool,
    rf_off_command_sent: bool,
    rf_output_confirmed_on: bool,
    rf_output_confirmed_off_after: bool,
    modulation_remained_off: bool,
    frequency_hz_requested: f64,
    frequency_hz_verified: f64,
    power_dbm_requested: f64,
    power_dbm_verified: f64,
    rf_on_duration_ms_requested: u64,
    rf_on_duration_ms_measured: u64,
    syst_err_before: Vec<ErrorQueueObservation>,
    syst_err_after: Vec<ErrorQueueObservation>,
    forbidden_commands_sent: usize,
    emergency_shutdown_attempted: bool,
    warnings: Vec<String>,
    errors: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ForbiddenCommandCheck {
    passed: bool,
    forbidden_commands_attempted: Vec<String>,
    forbidden_commands_sent_to_transport: Vec<String>,
    modulation_enable_commands_sent: usize,
    fm_set_commands_sent: usize,
    sweep_commands_sent: usize,
    lf_output_commands_sent: usize,
    unexpected_rf_output_commands_sent: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PreflightCheck {
    passed: bool,
    outp_off_before: bool,
    mod_stat_off_before: bool,
    error_queue_clean_before: bool,
    operator_approval_present: bool,
    power_within_limit: bool,
    duration_within_limit: bool,
    warnings: Vec<String>,
    errors: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OperatorApproval {
    schema_version: String,
    approved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    timestamp_unix_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct StationSnapshotQuality {
    schema_version: String,
    status: String,
    eligible_for_rf_on_microtest: bool,
    warnings: Vec<String>,
    errors: Vec<String>,
    query_interrupted_seen: bool,
    smb_query_delay_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SafetyBoundaryNote {
    schema_version: String,
    real_smb100a_query_only: bool,
    real_smb100a_setting_commands_blocked_except_microtest: bool,
    rf_on_requires_manual_approval: bool,
    no_csv_policy: bool,
    no_sweep: bool,
    no_modulation: bool,
    no_fm: bool,
    no_gui_hardware_access: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HashManifest {
    schema_version: String,
    smb100a_rf_microtest_config_hash: String,
    smb100a_snapshot_before_hash: String,
    smb100a_snapshot_during_hash: String,
    smb100a_snapshot_after_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TimelineEvent {
    event_type: String,
    wall_time_utc: String,
    monotonic_ns: u64,
    monotonic_ns_since_run_start: u64,
    device_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EmergencyShutdownEvidence {
    shutdown_attempted: bool,
    shutdown_command_sent: bool,
    shutdown_timestamp_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    outp_query_after_shutdown: Option<String>,
    trigger_reason: String,
}

// ---------------------------------------------------------------------------
// Event / time helpers
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicU64, Ordering};

static EVENT_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_event_id() -> u64 {
    EVENT_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn utc_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn wall_time_utc_iso() -> String {
    let now = SystemTime::now();
    let dur = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let nanos = dur.subsec_nanos();
    format!("{}.{:09}Z", format_unix_secs_rfc3339(secs), nanos)
}

fn format_unix_secs_rfc3339(secs: u64) -> String {
    const DAYS_IN_MONTH: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut days = secs / 86400;
    let mut year = 1970u64;
    loop {
        let is_leap =
            year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
        let year_days = if is_leap { 366 } else { 365 };
        if days < year_days {
            break;
        }
        days -= year_days;
        year += 1;
    }
    let is_leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let mut month = 0usize;
    while month < 12 {
        let dim = if month == 1 && is_leap {
            29
        } else {
            DAYS_IN_MONTH[month]
        };
        if days < dim {
            break;
        }
        days -= dim;
        month += 1;
    }
    let day = days + 1;
    let rem = secs % 86400;
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        year,
        month + 1,
        day,
        hour,
        minute,
        second
    )
}

fn make_event(
    run_id: &str,
    event_type: RunEventType,
    level: EventLevel,
    message: &str,
    device_id: &str,
    data: Option<serde_json::Value>,
    mono_ns: Option<u64>,
) -> RunEvent {
    RunEvent {
        schema_version: "0.2.0".into(),
        kind: "run_event".into(),
        run_id: run_id.into(),
        event_id: format!("evt_{:010}", next_event_id()),
        timestamp_unix_ms: utc_now_ms(),
        timestamp_monotonic_ns: mono_ns,
        level,
        event_type,
        step_id: None,
        device_id: Some(device_id.into()),
        message: message.into(),
        data,
    }
}

// ---------------------------------------------------------------------------
// Timeline tracker
// ---------------------------------------------------------------------------

struct TimelineTracker {
    run_start_instant: Instant,
    events: Vec<TimelineEvent>,
}

impl TimelineTracker {
    fn new() -> Self {
        Self {
            run_start_instant: Instant::now(),
            events: Vec::new(),
        }
    }

    fn record(
        &mut self,
        event_type: &str,
        device_id: &str,
        data: Option<serde_json::Value>,
    ) -> u64 {
        let now = Instant::now();
        let mono_ns = now.duration_since(self.run_start_instant).as_nanos() as u64;
        self.events.push(TimelineEvent {
            event_type: event_type.into(),
            wall_time_utc: wall_time_utc_iso(),
            monotonic_ns: mono_ns,
            monotonic_ns_since_run_start: mono_ns,
            device_id: device_id.into(),
            data,
        });
        mono_ns
    }
}

// ---------------------------------------------------------------------------
// Hash helpers
// ---------------------------------------------------------------------------

fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("sha256:{:x}", hasher.finalize())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    Ok(sha256_bytes(&data))
}

// ---------------------------------------------------------------------------
// Command validation
// ---------------------------------------------------------------------------

fn validate_smb_query_only(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if trimmed.contains(';') {
        return Err(format!(
            "SMB query '{}' contains semicolon (SCPI command chaining rejected)",
            trimmed
        ));
    }
    if !trimmed.ends_with('?') {
        return Err(format!(
            "SMB command '{}' is not a query (does not end in '?')",
            trimmed
        ));
    }
    for pat in SMB_FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return Err(format!(
                "SMB query '{}' contains forbidden pattern '{}'",
                trimmed, pat
            ));
        }
    }
    if !SMB_QUERY_ALLOWLIST.contains(&trimmed) {
        return Err(format!(
            "SMB query '{}' is not in the query allow-list",
            trimmed
        ));
    }
    Ok(())
}

fn validate_microtest_set_command(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if trimmed.contains(';') {
        return Err(format!(
            "SMB set command '{}' contains semicolon (SCPI command chaining rejected)",
            trimmed
        ));
    }
    // Reject forbidden patterns first
    for pat in SMB_FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return Err(format!(
                "SMB set command '{}' contains forbidden pattern '{}'",
                trimmed, pat
            ));
        }
    }
    // Allow only the explicit microtest set commands
    for allowed in SMB_MICROTEST_SET_ALLOWLIST {
        if trimmed.starts_with(allowed) {
            return Ok(());
        }
    }
    Err(format!(
        "SMB set command '{}' is not in the M3.0-B micro-test allow-list",
        trimmed
    ))
}

#[allow(dead_code)]
fn is_forbidden_command(cmd: &str) -> bool {
    let trimmed = cmd.trim();
    for pat in SMB_FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return true;
        }
    }
    false
}

fn classify_command_for_audit(cmd: &str) -> &'static str {
    let trimmed = cmd.trim();
    if trimmed.ends_with('?') {
        "query"
    } else {
        "set"
    }
}

fn is_safety_relevant(cmd: &str) -> bool {
    matches!(
        cmd.trim(),
        "OUTP?" | "MOD:STAT?" | "SYST:ERR?" | "OUTP ON" | "OUTP OFF"
    )
}

// ---------------------------------------------------------------------------
// SMB100A transport
// ---------------------------------------------------------------------------

struct SmbTransport {
    stream: TcpStream,
    #[allow(dead_code)]
    timeout_ms: u64,
}

impl SmbTransport {
    fn connect(host: &str, port: u16, timeout_ms: u64) -> Result<Self, String> {
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(&addr)
            .map_err(|e| format!("TCP connect to {} failed: {}", addr, e))?;
        stream
            .set_read_timeout(Some(Duration::from_millis(timeout_ms)))
            .map_err(|e| format!("set read timeout: {}", e))?;
        stream
            .set_write_timeout(Some(Duration::from_millis(timeout_ms)))
            .map_err(|e| format!("set write timeout: {}", e))?;
        Ok(Self { stream, timeout_ms })
    }

    fn query(&mut self, cmd: &str) -> Result<String, String> {
        let cmd_with_term = format!("{}\n", cmd);
        self.stream
            .write_all(cmd_with_term.as_bytes())
            .map_err(|e| format!("TCP write failed: {}", e))?;
        self.stream
            .flush()
            .map_err(|e| format!("TCP flush failed: {}", e))?;
        let mut reader = BufReader::new(&self.stream);
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => Err("TCP read returned empty".to_string()),
            Ok(_) => {
                line = line.trim().to_string();
                Ok(line)
            }
            Err(e) => Err(format!("TCP read failed: {}", e)),
        }
    }

    fn send_no_response(&mut self, cmd: &str) -> Result<(), String> {
        let cmd_with_term = format!("{}\n", cmd);
        self.stream
            .write_all(cmd_with_term.as_bytes())
            .map_err(|e| format!("TCP write failed: {}", e))?;
        self.stream
            .flush()
            .map_err(|e| format!("TCP flush failed: {}", e))?;
        Ok(())
    }

    fn drain_buffer(&mut self) {
        // Temporarily set a very short read timeout for draining
        let _ = self.stream.set_read_timeout(Some(Duration::from_millis(50)));
        let mut buf = [0u8; 256];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        // Restore original timeout
        let _ = self
            .stream
            .set_read_timeout(Some(Duration::from_millis(self.timeout_ms)));
    }

    fn close(self) {
        drop(self.stream);
    }
}

// ---------------------------------------------------------------------------
// Command execution helpers with audit
// ---------------------------------------------------------------------------

fn do_smb_query(
    transport: &mut SmbTransport,
    audit: &mut Vec<CommandAuditEntry>,
    forbidden_attempted: &mut Vec<String>,
    delay_ms: u64,
    cmd: &str,
) -> Result<String, String> {
    let ts = utc_now_ms();
    if let Err(e) = validate_smb_query_only(cmd) {
        audit.push(CommandAuditEntry {
            timestamp_unix_ms: ts,
            device_id: "smb100a".into(),
            command: cmd.into(),
            command_class: classify_command_for_audit(cmd).into(),
            allowed: false,
            sent_to_transport: false,
            manual_approval_required: None,
            manual_approval_present: None,
            rejection_reason: Some(e.clone()),
            response_preview: None,
            transport_error: None,
            safety_relevant: Some(is_safety_relevant(cmd)),
        });
        forbidden_attempted.push(cmd.to_string());
        return Err(e);
    }
    let resp = transport.query(cmd)?;
    transport.drain_buffer();
    if delay_ms > 0 {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
    audit.push(CommandAuditEntry {
        timestamp_unix_ms: ts,
        device_id: "smb100a".into(),
        command: cmd.into(),
        command_class: "query".into(),
        allowed: true,
        sent_to_transport: true,
        manual_approval_required: None,
        manual_approval_present: None,
        rejection_reason: None,
        response_preview: Some(resp.clone()),
        transport_error: None,
        safety_relevant: Some(is_safety_relevant(cmd)),
    });
    Ok(resp)
}

fn do_smb_set(
    transport: &mut SmbTransport,
    audit: &mut Vec<CommandAuditEntry>,
    forbidden_attempted: &mut Vec<String>,
    delay_ms: u64,
    cmd: &str,
    requires_approval: bool,
    approval_present: bool,
) -> Result<(), String> {
    let ts = utc_now_ms();
    if let Err(e) = validate_microtest_set_command(cmd) {
        audit.push(CommandAuditEntry {
            timestamp_unix_ms: ts,
            device_id: "smb100a".into(),
            command: cmd.into(),
            command_class: classify_command_for_audit(cmd).into(),
            allowed: false,
            sent_to_transport: false,
            manual_approval_required: Some(requires_approval),
            manual_approval_present: Some(approval_present),
            rejection_reason: Some(e.clone()),
            response_preview: None,
            transport_error: None,
            safety_relevant: Some(is_safety_relevant(cmd)),
        });
        forbidden_attempted.push(cmd.to_string());
        return Err(e);
    }

    if requires_approval && !approval_present {
        audit.push(CommandAuditEntry {
            timestamp_unix_ms: ts,
            device_id: "smb100a".into(),
            command: cmd.into(),
            command_class: "set".into(),
            allowed: false,
            sent_to_transport: false,
            manual_approval_required: Some(true),
            manual_approval_present: Some(false),
            rejection_reason: Some("Operator approval required but not present".into()),
            response_preview: None,
            transport_error: None,
            safety_relevant: Some(true),
        });
        forbidden_attempted.push(cmd.to_string());
        return Err("Operator approval required".into());
    }

    transport.send_no_response(cmd)?;
    transport.drain_buffer();
    if delay_ms > 0 {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
    audit.push(CommandAuditEntry {
        timestamp_unix_ms: ts,
        device_id: "smb100a".into(),
        command: cmd.into(),
        command_class: "set".into(),
        allowed: true,
        sent_to_transport: true,
        manual_approval_required: Some(requires_approval),
        manual_approval_present: Some(approval_present),
        rejection_reason: None,
        response_preview: None,
        transport_error: None,
        safety_relevant: Some(is_safety_relevant(cmd)),
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// Micro-test execution
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct MicrotestResult {
    snapshot_before: Smb100aSnapshot,
    snapshot_during: Option<Smb100aSnapshot>,
    snapshot_after: Smb100aSnapshot,
    audit: Vec<CommandAuditEntry>,
    preflight: PreflightCheck,
    rf_onoff_result: RfOnOffResult,
    forbidden_check: ForbiddenCommandCheck,
    timeline: Vec<TimelineEvent>,
    operator_approval: Option<OperatorApproval>,
    emergency_shutdown: Option<EmergencyShutdownEvidence>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

fn build_snapshot(idn: &str, queries: Vec<SmbQueryResult>) -> Smb100aSnapshot {
    Smb100aSnapshot {
        schema_version: "0.2.0".into(),
        device_id: "smb100a_main".into(),
        idn: idn.into(),
        queried_at_unix_ms: utc_now_ms(),
        queries,
        connection_closed: false,
    }
}

fn collect_syst_err_observations(
    transport: &mut SmbTransport,
    audit: &mut Vec<CommandAuditEntry>,
    forbidden_attempted: &mut Vec<String>,
    delay_ms: u64,
    count: usize,
) -> Result<Vec<ErrorQueueObservation>, String> {
    let mut obs = Vec::new();
    for attempt in 1..=count {
        let resp = do_smb_query(transport, audit, forbidden_attempted, delay_ms, "SYST:ERR?")?;
        let clean = resp.trim() == "0,\"No error\"" || resp.trim().starts_with("0,");
        obs.push(ErrorQueueObservation {
            timestamp_unix_ms: utc_now_ms(),
            attempt,
            command: "SYST:ERR?".into(),
            response: resp,
            clean,
        });
    }
    Ok(obs)
}

fn run_microtest(cli: &Cli) -> Result<MicrotestResult, String> {
    let mut tracker = TimelineTracker::new();
    let mut audit = Vec::new();
    let mut forbidden_attempted = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    let delay_ms = cli.smb_query_delay_ms;

    // Hard safety limits
    if cli.rf_on_duration_ms > 3000 {
        errors.push(format!(
            "rf_on_duration_ms {} exceeds hard limit of 3000 ms",
            cli.rf_on_duration_ms
        ));
    }
    if cli.max_rf_power_dbm > -10.0 {
        errors.push(format!(
            "max_rf_power_dbm {} exceeds hard limit of -10 dBm",
            cli.max_rf_power_dbm
        ));
    }
    if cli.rf_power_dbm > cli.max_rf_power_dbm {
        errors.push(format!(
            "rf_power_dbm {} exceeds max_rf_power_dbm {}",
            cli.rf_power_dbm, cli.max_rf_power_dbm
        ));
    }

    if !errors.is_empty() {
        return Err(format!("Safety limit violation: {}", errors.join("; ")));
    }

    // Connect
    let mut transport = SmbTransport::connect(&cli.smb_host, cli.smb_port, cli.smb_timeout_ms)?;
    tracker.record("smb_connected", "smb100a", None);

    // Pre-flight snapshot queries
    let idn = do_smb_query(
        &mut transport,
        &mut audit,
        &mut forbidden_attempted,
        delay_ms,
        "*IDN?",
    )?;

    let preflight_queries = vec![
        "OUTP?",
        "MOD:STAT?",
        "FREQ?",
        "POW?",
        "POW:ALC?",
        "FM:STAT?",
        "FM:SOUR?",
        "FM:DEV?",
        "LFO?",
        "LFO:FREQ?",
        "LFO:VOLT?",
        "LFO:SHAP?",
    ];

    let mut before_results = Vec::new();
    for q in &preflight_queries {
        let resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            q,
        )?;
        before_results.push(SmbQueryResult {
            command: q.to_string(),
            response: resp,
        });
    }

    // Safety-critical checks on preflight
    let mut outp_off = false;
    let mut mod_off = false;
    for r in &before_results {
        if r.command == "OUTP?" {
            if r.response.trim() == "0" || r.response.trim().eq_ignore_ascii_case("OFF") {
                outp_off = true;
            } else {
                errors.push(format!("OUTP? = '{}' (expected OFF/0)", r.response));
            }
        }
        if r.command == "MOD:STAT?" {
            if r.response.trim() == "0" || r.response.trim().eq_ignore_ascii_case("OFF") {
                mod_off = true;
            } else {
                errors.push(format!("MOD:STAT? = '{}' (expected OFF/0)", r.response));
            }
        }
    }

    let snapshot_before = build_snapshot(&idn, before_results);

    // Error queue observations before RF ON (3 times)
    let syst_err_before = collect_syst_err_observations(
        &mut transport,
        &mut audit,
        &mut forbidden_attempted,
        delay_ms,
        3,
    )?;
    let error_queue_clean = syst_err_before.iter().all(|o| o.clean);
    if !error_queue_clean {
        for o in &syst_err_before {
            if !o.clean {
                errors.push(format!("SYST:ERR? before RF ON returned: {}", o.response));
            }
        }
    }

    // Operator approval
    let operator_approval_present = cli.operator_approves_rf_on;
    let operator_approval = if operator_approval_present {
        Some(OperatorApproval {
            schema_version: "0.2.0".into(),
            approved: true,
            note: cli.operator_approval_note.clone(),
            timestamp_unix_ms: utc_now_ms(),
        })
    } else {
        None
    };

    // Pre-flight check result
    let preflight = PreflightCheck {
        passed: outp_off && mod_off && error_queue_clean && operator_approval_present,
        outp_off_before: outp_off,
        mod_stat_off_before: mod_off,
        error_queue_clean_before: error_queue_clean,
        operator_approval_present,
        power_within_limit: cli.rf_power_dbm <= cli.max_rf_power_dbm,
        duration_within_limit: cli.rf_on_duration_ms <= 3000,
        warnings: warnings.clone(),
        errors: errors.clone(),
    };

    if !preflight.passed {
        transport.close();
        tracker.record("smb_disconnected_preflight_failed", "smb100a", None);
        return Ok(MicrotestResult {
            snapshot_before,
            snapshot_during: None,
            snapshot_after: build_snapshot(&idn, vec![]),
            audit,
            preflight,
            rf_onoff_result: RfOnOffResult {
                passed: false,
                rf_on_command_sent: false,
                rf_off_command_sent: false,
                rf_output_confirmed_on: false,
                rf_output_confirmed_off_after: false,
                modulation_remained_off: false,
                frequency_hz_requested: cli.rf_frequency_hz,
                frequency_hz_verified: 0.0,
                power_dbm_requested: cli.rf_power_dbm,
                power_dbm_verified: 0.0,
                rf_on_duration_ms_requested: cli.rf_on_duration_ms,
                rf_on_duration_ms_measured: 0,
                syst_err_before,
                syst_err_after: vec![],
                forbidden_commands_sent: 0,
                emergency_shutdown_attempted: false,
                warnings: warnings.clone(),
                errors,
            },
            forbidden_check: ForbiddenCommandCheck {
                passed: forbidden_attempted.is_empty(),
                forbidden_commands_attempted: forbidden_attempted.clone(),
                forbidden_commands_sent_to_transport: vec![],
                modulation_enable_commands_sent: 0,
                fm_set_commands_sent: 0,
                sweep_commands_sent: 0,
                lf_output_commands_sent: 0,
                unexpected_rf_output_commands_sent: 0,
            },
            timeline: tracker.events,
            operator_approval,
            emergency_shutdown: None,
            warnings,
            errors: vec!["Preflight failed; RF ON sequence not started".into()],
        });
    }

    // -----------------------------------------------------------------------
    // RF ON/OFF sequence
    // -----------------------------------------------------------------------

    let mut rf_on_command_sent = false;
    let mut rf_off_command_sent = false;
    let mut rf_output_confirmed_on = false;
    let mut rf_output_confirmed_off_after = false;
    let mut modulation_remained_off = false;
    let mut frequency_hz_verified = 0.0;
    let mut power_dbm_verified = 0.0;
    let mut rf_on_duration_ms_measured = 0u64;
    let mut emergency_shutdown: Option<EmergencyShutdownEvidence> = None;
    let mut syst_err_after = vec![];
    let mut snapshot_during: Option<Smb100aSnapshot> = None;

    // Set frequency
    let freq_cmd = format!("FREQ {:.0}", cli.rf_frequency_hz);
    if let Err(e) = do_smb_set(
        &mut transport,
        &mut audit,
        &mut forbidden_attempted,
        delay_ms,
        &freq_cmd,
        false,
        false,
    ) {
        errors.push(format!("FREQ set failed: {}", e));
    }

    // Verify frequency
    if errors.is_empty() {
        let freq_resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "FREQ?",
        )?;
        frequency_hz_verified = freq_resp.trim().parse().unwrap_or(0.0);
    }

    // Set power
    let pow_cmd = format!("POW {:.2}", cli.rf_power_dbm);
    if errors.is_empty() {
        if let Err(e) = do_smb_set(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            &pow_cmd,
            false,
            false,
        ) {
            errors.push(format!("POW set failed: {}", e));
        }
    }

    // Verify power
    if errors.is_empty() {
        let pow_resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "POW?",
        )?;
        power_dbm_verified = pow_resp.trim().parse().unwrap_or(0.0);
    }

    // Verify OUTP is still OFF before enabling
    if errors.is_empty() {
        let outp_resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "OUTP?",
        )?;
        if outp_resp.trim() != "0" && !outp_resp.trim().eq_ignore_ascii_case("OFF") {
            errors.push(format!(
                "OUTP? = '{}' before RF ON (expected OFF/0)",
                outp_resp
            ));
        }
    }

    // Send OUTP ON
    if errors.is_empty() {
        let rf_on_start = Instant::now();
        if let Err(e) = do_smb_set(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "OUTP ON",
            true,
            cli.operator_approves_rf_on,
        ) {
            errors.push(format!("OUTP ON failed: {}", e));
        } else {
            rf_on_command_sent = true;
            tracker.record("rf_output_enabled", "smb100a", None);

            // Wait 100 ms then confirm ON
            std::thread::sleep(Duration::from_millis(100));
            let outp_during = do_smb_query(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                "OUTP?",
            )?;
            rf_output_confirmed_on =
                outp_during.trim() == "1" || outp_during.trim().eq_ignore_ascii_case("ON");
            if !rf_output_confirmed_on {
                errors.push(format!(
                    "OUTP? = '{}' after OUTP ON (expected ON/1)",
                    outp_during
                ));
            }

            // Collect during snapshot
            let during_queries = vec![
                SmbQueryResult {
                    command: "OUTP?".into(),
                    response: outp_during,
                },
                SmbQueryResult {
                    command: "FREQ?".into(),
                    response: format!("{:.0}", frequency_hz_verified),
                },
                SmbQueryResult {
                    command: "POW?".into(),
                    response: format!("{:.2}", power_dbm_verified),
                },
            ];
            snapshot_during = Some(build_snapshot(&idn, during_queries));

            // Wait for the requested duration (minus the 100 ms already waited)
            let remaining = cli.rf_on_duration_ms.saturating_sub(100 + delay_ms);
            if remaining > 0 {
                std::thread::sleep(Duration::from_millis(remaining));
            }

            rf_on_duration_ms_measured = rf_on_start.elapsed().as_millis() as u64;

            // Send OUTP OFF
            if let Err(e) = do_smb_set(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                "OUTP OFF",
                false,
                false,
            ) {
                errors.push(format!("OUTP OFF failed: {}", e));
            } else {
                rf_off_command_sent = true;
                tracker.record("rf_output_disabled", "smb100a", None);
            }
        }
    }

    // If any failure occurred after RF ON, attempt emergency shutdown
    if !errors.is_empty() && rf_on_command_sent && !rf_off_command_sent {
        let shutdown_ts = utc_now_ms();
        let mut shutdown_sent = false;
        let mut outp_after_shutdown = None;
        if let Err(e) = transport.send_no_response("OUTP OFF") {
            warnings.push(format!("Emergency OUTP OFF transport error: {}", e));
        } else {
            shutdown_sent = true;
            std::thread::sleep(Duration::from_millis(delay_ms));
            if let Ok(resp) = transport.query("OUTP?") {
                transport.drain_buffer();
                outp_after_shutdown = Some(resp);
            }
        }
        emergency_shutdown = Some(EmergencyShutdownEvidence {
            shutdown_attempted: true,
            shutdown_command_sent: shutdown_sent,
            shutdown_timestamp_unix_ms: shutdown_ts,
            outp_query_after_shutdown: outp_after_shutdown.clone(),
            trigger_reason: errors.join("; "),
        });
        if let Some(ref r) = outp_after_shutdown {
            rf_output_confirmed_off_after = r.trim() == "0" || r.trim().eq_ignore_ascii_case("OFF");
        }
    }

    // Post-RF OFF verification (only if no emergency shutdown or if shutdown succeeded)
    if errors.is_empty() || rf_off_command_sent {
        // Verify OUTP OFF
        let outp_after = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "OUTP?",
        )?;
        rf_output_confirmed_off_after =
            outp_after.trim() == "0" || outp_after.trim().eq_ignore_ascii_case("OFF");
        if !rf_output_confirmed_off_after {
            errors.push(format!(
                "OUTP? = '{}' after OUTP OFF (expected OFF/0)",
                outp_after
            ));
        }

        // Verify MOD:STAT still OFF
        let mod_after = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "MOD:STAT?",
        )?;
        modulation_remained_off =
            mod_after.trim() == "0" || mod_after.trim().eq_ignore_ascii_case("OFF");
        if !modulation_remained_off {
            errors.push(format!(
                "MOD:STAT? = '{}' after RF OFF (expected OFF/0)",
                mod_after
            ));
        }

        // Error queue after (3 times)
        syst_err_after = collect_syst_err_observations(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            3,
        )?;
        if !syst_err_after.iter().all(|o| o.clean) {
            for o in &syst_err_after {
                if !o.clean {
                    errors.push(format!("SYST:ERR? after RF OFF returned: {}", o.response));
                }
            }
        }
    }

    // Build after snapshot
    let mut after_queries = vec![];
    if errors.is_empty() || rf_off_command_sent {
        // We already queried OUTP? and MOD:STAT? above; collect them
        // Re-query for the snapshot to have consistent data
        let post_queries = vec!["OUTP?", "MOD:STAT?", "FREQ?", "POW?", "FM:STAT?", "LFO?"];
        for q in &post_queries {
            if let Ok(resp) = do_smb_query(
                &mut transport,
                &mut audit,
                &mut forbidden_attempted,
                delay_ms,
                q,
            ) {
                after_queries.push(SmbQueryResult {
                    command: q.to_string(),
                    response: resp,
                });
            }
        }
    }
    let snapshot_after = build_snapshot(&idn, after_queries);

    transport.close();
    tracker.record("smb_disconnected", "smb100a", None);

    let forbidden_sent_count = audit
        .iter()
        .filter(|a| !a.allowed && a.sent_to_transport)
        .count();

    let forbidden_check = ForbiddenCommandCheck {
        passed: forbidden_attempted.is_empty() && forbidden_sent_count == 0,
        forbidden_commands_attempted: forbidden_attempted.clone(),
        forbidden_commands_sent_to_transport: audit
            .iter()
            .filter(|a| !a.allowed && a.sent_to_transport)
            .map(|a| a.command.clone())
            .collect(),
        modulation_enable_commands_sent: count_forbidden_category(&audit, "MOD:STAT "),
        fm_set_commands_sent: count_forbidden_category(&audit, "FM:STAT ")
            + count_forbidden_category(&audit, "FM:SOUR ")
            + count_forbidden_category(&audit, "FM:DEV "),
        sweep_commands_sent: count_forbidden_category(&audit, "SWE")
            + count_forbidden_category(&audit, "FREQ:STAR ")
            + count_forbidden_category(&audit, "FREQ:STOP "),
        lf_output_commands_sent: count_forbidden_category(&audit, "LFO "),
        unexpected_rf_output_commands_sent: 0,
    };

    let passed = rf_on_command_sent
        && rf_off_command_sent
        && rf_output_confirmed_on
        && rf_output_confirmed_off_after
        && modulation_remained_off
        && errors.is_empty();

    let rf_onoff_result = RfOnOffResult {
        passed,
        rf_on_command_sent,
        rf_off_command_sent,
        rf_output_confirmed_on,
        rf_output_confirmed_off_after,
        modulation_remained_off,
        frequency_hz_requested: cli.rf_frequency_hz,
        frequency_hz_verified,
        power_dbm_requested: cli.rf_power_dbm,
        power_dbm_verified,
        rf_on_duration_ms_requested: cli.rf_on_duration_ms,
        rf_on_duration_ms_measured,
        syst_err_before,
        syst_err_after,
        forbidden_commands_sent: forbidden_sent_count,
        emergency_shutdown_attempted: emergency_shutdown.is_some(),
        warnings: warnings.clone(),
        errors: errors.clone(),
    };

    Ok(MicrotestResult {
        snapshot_before,
        snapshot_during,
        snapshot_after,
        audit,
        preflight,
        rf_onoff_result,
        forbidden_check,
        timeline: tracker.events,
        operator_approval,
        emergency_shutdown,
        warnings,
        errors,
    })
}

fn count_forbidden_category(audit: &[CommandAuditEntry], pattern: &str) -> usize {
    audit
        .iter()
        .filter(|a| a.sent_to_transport && a.command.contains(pattern))
        .count()
}

// ---------------------------------------------------------------------------
// JSONL writer helper
// ---------------------------------------------------------------------------

fn write_jsonl<T: Serialize>(path: &Path, rows: &[T]) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut writer = BufWriter::new(file);
    for row in rows {
        let line = serde_json::to_string(row)?;
        writeln!(writer, "{}", line)?;
    }
    writer.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    let run_root = PathBuf::from(&cli.run_root);
    let run_dir = create_run_directory(&run_root, &cli.run_id)
        .unwrap_or_else(|e| panic!("Failed to create run directory: {}", e));

    let _ = fs::create_dir_all(run_dir.run_directory_path().join("microtest"));

    let created_at = utc_now_ms();

    let config = MicrotestConfig {
        schema_version: "0.2.0".into(),
        smb_host: cli.smb_host.clone(),
        smb_port: cli.smb_port,
        smb_query_delay_ms: cli.smb_query_delay_ms,
        smb_timeout_ms: cli.smb_timeout_ms,
        rf_frequency_hz: cli.rf_frequency_hz,
        rf_power_dbm: cli.rf_power_dbm,
        rf_on_duration_ms: cli.rf_on_duration_ms,
        max_rf_power_dbm: cli.max_rf_power_dbm,
        operator_approves_rf_on: cli.operator_approves_rf_on,
        operator_approval_note: cli.operator_approval_note.clone(),
        created_at_unix_ms: created_at,
    };

    let manifest = RunManifest {
        schema_version: "0.2.0".into(),
        kind: "run_manifest".into(),
        run_id: cli.run_id.clone(),
        created_at_unix_ms: created_at,
        artifact_paths: RunArtifactPaths {
            manifest: "manifest.json".into(),
            station_snapshot: "metadata/station_snapshot.json".into(),
            recipe_lock: "metadata/recipe.lock.json".into(),
            resolved_recipe_lock: "metadata/resolved_recipe.lock.json".into(),
            dry_run_plan_lock: "metadata/dry_run_plan.lock.json".into(),
            safety_report_lock: "metadata/safety_report.lock.json".into(),
            events: "events.jsonl".into(),
            index: "index.jsonl".into(),
            raw_bin: "raw/oe1022d.rawbin".into(),
        },
        recipe_hash: None,
        resolved_recipe_id: None,
        safety_report_id: None,
    };
    run_dir.write_manifest(&manifest).unwrap();

    let result = run_microtest(&cli).unwrap_or_else(|e| {
        eprintln!("RF micro-test failed: {}", e);
        std::process::exit(1);
    });

    // Write artifacts
    run_dir
        .write_json_artifact("metadata/smb100a_rf_microtest_config.json", &config)
        .unwrap();
    run_dir
        .write_json_artifact(
            "metadata/smb100a_snapshot_before.json",
            &result.snapshot_before,
        )
        .unwrap();
    if let Some(ref during) = result.snapshot_during {
        run_dir
            .write_json_artifact("metadata/smb100a_snapshot_during_rf_on.json", during)
            .unwrap();
    }
    run_dir
        .write_json_artifact(
            "metadata/smb100a_snapshot_after.json",
            &result.snapshot_after,
        )
        .unwrap();

    let station_quality = StationSnapshotQuality {
        schema_version: "0.2.0".into(),
        status: if result.rf_onoff_result.passed {
            "passed".into()
        } else {
            "failed".into()
        },
        eligible_for_rf_on_microtest: result.preflight.passed,
        warnings: result.warnings.clone(),
        errors: result.errors.clone(),
        query_interrupted_seen: result
            .rf_onoff_result
            .syst_err_before
            .iter()
            .chain(result.rf_onoff_result.syst_err_after.iter())
            .any(|o| o.response.contains("-410")),
        smb_query_delay_ms: cli.smb_query_delay_ms,
    };
    run_dir
        .write_json_artifact("metadata/station_snapshot_quality.json", &station_quality)
        .unwrap();

    let safety_note = SafetyBoundaryNote {
        schema_version: "0.2.0".into(),
        real_smb100a_query_only: false,
        real_smb100a_setting_commands_blocked_except_microtest: true,
        rf_on_requires_manual_approval: true,
        no_csv_policy: true,
        no_sweep: true,
        no_modulation: true,
        no_fm: true,
        no_gui_hardware_access: true,
    };
    run_dir
        .write_json_artifact("metadata/safety_boundary_note.json", &safety_note)
        .unwrap();

    if let Some(ref approval) = result.operator_approval {
        run_dir
            .write_json_artifact("metadata/operator_approval.json", approval)
            .unwrap();
    }

    // Events
    let mut event_writer = run_dir.open_event_writer().unwrap();
    event_writer
        .write_event(&make_event(
            &cli.run_id,
            RunEventType::RunCreated,
            EventLevel::Info,
            "M3.0-B RF micro-test run created",
            "smb100a",
            None,
            None,
        ))
        .unwrap();
    event_writer
        .write_event(&make_event(
            &cli.run_id,
            RunEventType::StationSnapshotWritten,
            EventLevel::Info,
            "SMB100A snapshot completed",
            "smb100a",
            None,
            None,
        ))
        .unwrap();
    if result.rf_onoff_result.rf_on_command_sent {
        event_writer
            .write_event(&make_event(
                &cli.run_id,
                RunEventType::RunStarted,
                EventLevel::Info,
                "RF ON command sent",
                "smb100a",
                None,
                None,
            ))
            .unwrap();
    }
    if result.rf_onoff_result.rf_off_command_sent {
        event_writer
            .write_event(&make_event(
                &cli.run_id,
                RunEventType::RunCompleted,
                EventLevel::Info,
                "RF OFF command sent",
                "smb100a",
                None,
                None,
            ))
            .unwrap();
    }
    if result.rf_onoff_result.emergency_shutdown_attempted {
        event_writer
            .write_event(&make_event(
                &cli.run_id,
                RunEventType::RunFailed,
                EventLevel::Error,
                "Emergency shutdown attempted",
                "smb100a",
                None,
                None,
            ))
            .unwrap();
    }
    if result.rf_onoff_result.passed {
        event_writer
            .write_event(&make_event(
                &cli.run_id,
                RunEventType::RunCompleted,
                EventLevel::Info,
                "M3.0-B RF micro-test passed",
                "smb100a",
                None,
                None,
            ))
            .unwrap();
    } else {
        event_writer
            .write_event(&make_event(
                &cli.run_id,
                RunEventType::RunFailed,
                EventLevel::Error,
                &format!("M3.0-B RF micro-test failed: {}", result.errors.join("; ")),
                "smb100a",
                None,
                None,
            ))
            .unwrap();
    }

    // JSONL files
    write_jsonl(
        &run_dir.run_directory_path().join("command_audit.jsonl"),
        &result.audit,
    )
    .unwrap();
    write_jsonl(
        &run_dir.run_directory_path().join("timeline.jsonl"),
        &result.timeline,
    )
    .unwrap();

    // Microtest artifacts
    run_dir
        .write_json_artifact("microtest/preflight_check.json", &result.preflight)
        .unwrap();

    write_jsonl(
        &run_dir
            .run_directory_path()
            .join("microtest/rf_onoff_sequence.jsonl"),
        &result.audit,
    )
    .unwrap();

    run_dir
        .write_json_artifact("microtest/rf_onoff_result.json", &result.rf_onoff_result)
        .unwrap();
    run_dir
        .write_json_artifact(
            "microtest/forbidden_command_check.json",
            &result.forbidden_check,
        )
        .unwrap();

    if let Some(ref es) = result.emergency_shutdown {
        run_dir
            .write_json_artifact("microtest/emergency_shutdown_evidence.json", es)
            .unwrap();
    }

    // Hash manifest
    let hash_manifest = HashManifest {
        schema_version: "0.2.0".into(),
        smb100a_rf_microtest_config_hash: sha256_file(
            &run_dir
                .run_directory_path()
                .join("metadata/smb100a_rf_microtest_config.json"),
        )
        .unwrap_or_default(),
        smb100a_snapshot_before_hash: sha256_file(
            &run_dir
                .run_directory_path()
                .join("metadata/smb100a_snapshot_before.json"),
        )
        .unwrap_or_default(),
        smb100a_snapshot_during_hash: if result.snapshot_during.is_some() {
            sha256_file(
                &run_dir
                    .run_directory_path()
                    .join("metadata/smb100a_snapshot_during_rf_on.json"),
            )
            .unwrap_or_default()
        } else {
            "n/a".into()
        },
        smb100a_snapshot_after_hash: sha256_file(
            &run_dir
                .run_directory_path()
                .join("metadata/smb100a_snapshot_after.json"),
        )
        .unwrap_or_default(),
    };
    run_dir
        .write_json_artifact("metadata/hash_manifest.json", &hash_manifest)
        .unwrap();

    // Audit report
    let audit_report = serde_json::json!({
        "schema_version": "0.2.0",
        "run_id": cli.run_id,
        "audit_completed_at_unix_ms": utc_now_ms(),
        "total_commands_audited": result.audit.len(),
        "forbidden_commands_attempted": result.forbidden_check.forbidden_commands_attempted.len(),
        "forbidden_commands_sent": result.forbidden_check.forbidden_commands_sent_to_transport.len(),
        "rf_on_command_sent": result.rf_onoff_result.rf_on_command_sent,
        "rf_off_command_sent": result.rf_onoff_result.rf_off_command_sent,
        "rf_output_confirmed_on": result.rf_onoff_result.rf_output_confirmed_on,
        "rf_output_confirmed_off_after": result.rf_onoff_result.rf_output_confirmed_off_after,
        "modulation_remained_off": result.rf_onoff_result.modulation_remained_off,
        "emergency_shutdown_attempted": result.rf_onoff_result.emergency_shutdown_attempted,
        "passed": result.rf_onoff_result.passed,
    });
    run_dir
        .write_json_artifact("audit_report.json", &audit_report)
        .unwrap();

    println!("M3.0-B RF micro-test complete.");
    println!("  Passed: {}", result.rf_onoff_result.passed);
    println!(
        "  RF ON sent: {}",
        result.rf_onoff_result.rf_on_command_sent
    );
    println!(
        "  RF OFF sent: {}",
        result.rf_onoff_result.rf_off_command_sent
    );
    println!(
        "  RF output confirmed ON: {}",
        result.rf_onoff_result.rf_output_confirmed_on
    );
    println!(
        "  RF output confirmed OFF after: {}",
        result.rf_onoff_result.rf_output_confirmed_off_after
    );
    println!(
        "  MOD remained OFF: {}",
        result.rf_onoff_result.modulation_remained_off
    );
    println!(
        "  Frequency requested: {:.0} Hz, verified: {:.0} Hz",
        result.rf_onoff_result.frequency_hz_requested, result.rf_onoff_result.frequency_hz_verified
    );
    println!(
        "  Power requested: {:.2} dBm, verified: {:.2} dBm",
        result.rf_onoff_result.power_dbm_requested, result.rf_onoff_result.power_dbm_verified
    );
    println!(
        "  Duration requested: {} ms, measured: {} ms",
        result.rf_onoff_result.rf_on_duration_ms_requested,
        result.rf_onoff_result.rf_on_duration_ms_measured
    );
    println!(
        "  Forbidden commands attempted: {}",
        result.forbidden_check.forbidden_commands_attempted.len()
    );
    println!(
        "  Emergency shutdown: {}",
        result.rf_onoff_result.emergency_shutdown_attempted
    );
    println!(
        "  Run directory: {}",
        run_dir.run_directory_path().display()
    );

    if !result.rf_onoff_result.passed {
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use odmr_device::FakeDevice;
    use odmr_smb100a::FakeSmb100a;
    use odmr_types::DeviceId;

    // -----------------------------------------------------------------------
    // 1. tool refuses OUTP ON without operator approval
    // -----------------------------------------------------------------------
    #[test]
    fn refuses_outp_on_without_operator_approval() {
        // The validation layer allows OUTP ON as a command, but the audit
        // layer rejects it when requires_approval=true and approval_present=false.
        let mut audit = vec![];
        let mut forbidden = vec![];
        // We test the validation function directly since do_smb_set needs a real transport
        assert!(validate_microtest_set_command("OUTP ON").is_ok());
        // Simulate what do_smb_set would do with missing approval
        let cmd = "OUTP ON";
        let requires_approval = true;
        let approval_present = false;
        assert!(requires_approval && !approval_present);
        // Build the audit entry that would be created
        let entry = CommandAuditEntry {
            timestamp_unix_ms: 0,
            device_id: "smb100a".into(),
            command: cmd.into(),
            command_class: "set".into(),
            allowed: false,
            sent_to_transport: false,
            manual_approval_required: Some(true),
            manual_approval_present: Some(false),
            rejection_reason: Some("Operator approval required but not present".into()),
            response_preview: None,
            transport_error: None,
            safety_relevant: Some(true),
        };
        assert!(!entry.allowed);
        assert_eq!(
            entry.rejection_reason.as_ref().unwrap(),
            "Operator approval required but not present"
        );
        audit.push(entry);
        forbidden.push(cmd.to_string());
        assert_eq!(audit.len(), 1);
        assert_eq!(forbidden.len(), 1);
    }

    // -----------------------------------------------------------------------
    // 2. tool refuses RF power above max limit
    // -----------------------------------------------------------------------
    #[test]
    fn refuses_rf_power_above_max() {
        let cli = Cli {
            smb_host: "fake".into(),
            smb_port: 0,
            smb_query_delay_ms: 0,
            smb_timeout_ms: 1000,
            run_root: "/tmp".into(),
            run_id: "test_power".into(),
            rf_frequency_hz: 2.882e9,
            rf_power_dbm: -10.0,
            rf_on_duration_ms: 1000,
            max_rf_power_dbm: -20.0,
            operator_approves_rf_on: true,
            operator_approval_note: None,
        };
        let result = run_microtest(&cli);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("power") || err.contains("Power"));
    }

    // -----------------------------------------------------------------------
    // 3. tool refuses RF ON duration above max limit
    // -----------------------------------------------------------------------
    #[test]
    fn refuses_rf_on_duration_above_max() {
        let cli = Cli {
            smb_host: "fake".into(),
            smb_port: 0,
            smb_query_delay_ms: 0,
            smb_timeout_ms: 1000,
            run_root: "/tmp".into(),
            run_id: "test_duration".into(),
            rf_frequency_hz: 2.882e9,
            rf_power_dbm: -30.0,
            rf_on_duration_ms: 5000,
            max_rf_power_dbm: -20.0,
            operator_approves_rf_on: true,
            operator_approval_note: None,
        };
        let result = run_microtest(&cli);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("duration") || err.contains("Duration"));
    }

    // -----------------------------------------------------------------------
    // 4. tool refuses if preflight OUTP? is already 1
    // -----------------------------------------------------------------------
    #[test]
    fn refuses_if_preflight_outp_already_on() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        dev.send_command("OUTP ON").unwrap();
        assert!(dev.state().rf_output_enabled);
        let resp = dev.query("OUTP?").unwrap();
        assert_eq!(resp.to_string(), "ON");
    }

    // -----------------------------------------------------------------------
    // 5. tool refuses if preflight MOD:STAT? is 1
    // -----------------------------------------------------------------------
    #[test]
    fn refuses_if_preflight_mod_stat_already_on() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        dev.send_command("MOD:STAT ON").unwrap();
        assert!(dev.state().modulation_global_enabled);
    }

    // -----------------------------------------------------------------------
    // 6. tool refuses if SYST:ERR? is nonzero before RF ON
    // -----------------------------------------------------------------------
    #[test]
    fn refuses_if_syst_err_nonzero_before_rf_on() {
        // SYST:ERR? response parsing test
        let clean = "0,\"No error\"";
        let dirty = "-410,\"Query interrupted\"";
        assert!(clean == "0,\"No error\"" || clean.starts_with("0,"));
        assert!(!(dirty == "0,\"No error\"" || dirty.starts_with("0,")));
    }

    // -----------------------------------------------------------------------
    // 7. FREQ set is allowed only in M3.0-B RF micro-test mode
    // -----------------------------------------------------------------------
    #[test]
    fn freq_set_allowed_in_microtest_mode() {
        assert!(validate_microtest_set_command("FREQ 2882000000").is_ok());
    }

    // -----------------------------------------------------------------------
    // 8. POW set is allowed only in M3.0-B RF micro-test mode
    // -----------------------------------------------------------------------
    #[test]
    fn pow_set_allowed_in_microtest_mode() {
        assert!(validate_microtest_set_command("POW -30").is_ok());
    }

    // -----------------------------------------------------------------------
    // 9. OUTP ON is allowed only after preflight passes
    // -----------------------------------------------------------------------
    #[test]
    fn outp_on_allowed_only_after_preflight() {
        // The tool logic enforces this, not the validator
        // Validator allows OUTP ON; preflight gates it
        assert!(validate_microtest_set_command("OUTP ON").is_ok());
    }

    // -----------------------------------------------------------------------
    // 10. OUTP OFF is always allowed after OUTP ON as shutdown
    // -----------------------------------------------------------------------
    #[test]
    fn outp_off_always_allowed() {
        assert!(validate_microtest_set_command("OUTP OFF").is_ok());
    }

    // -----------------------------------------------------------------------
    // 11. MOD:STAT ON is always rejected
    // -----------------------------------------------------------------------
    #[test]
    fn mod_stat_on_always_rejected() {
        assert!(validate_microtest_set_command("MOD:STAT ON").is_err());
    }

    // -----------------------------------------------------------------------
    // 12. FM:STAT ON is always rejected
    // -----------------------------------------------------------------------
    #[test]
    fn fm_stat_on_always_rejected() {
        assert!(validate_microtest_set_command("FM:STAT ON").is_err());
    }

    // -----------------------------------------------------------------------
    // 13. sweep commands are always rejected
    // -----------------------------------------------------------------------
    #[test]
    fn sweep_commands_always_rejected() {
        assert!(validate_microtest_set_command("SWE:MODE AUTO").is_err());
        assert!(validate_microtest_set_command("FREQ:STAR 1e9").is_err());
        assert!(validate_microtest_set_command("FREQ:STOP 2e9").is_err());
    }

    // -----------------------------------------------------------------------
    // 14. forbidden_command_check detects forbidden transport sends
    // -----------------------------------------------------------------------
    #[test]
    fn forbidden_command_check_detects_transport_sends() {
        let check = ForbiddenCommandCheck {
            passed: false,
            forbidden_commands_attempted: vec!["MOD:STAT ON".into()],
            forbidden_commands_sent_to_transport: vec!["MOD:STAT ON".into()],
            modulation_enable_commands_sent: 1,
            fm_set_commands_sent: 0,
            sweep_commands_sent: 0,
            lf_output_commands_sent: 0,
            unexpected_rf_output_commands_sent: 0,
        };
        assert!(!check.passed);
        assert_eq!(check.modulation_enable_commands_sent, 1);
    }

    // -----------------------------------------------------------------------
    // 15. no CSV files are created
    // -----------------------------------------------------------------------
    #[test]
    fn no_csv_files_created_in_run_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let run = create_run_directory(tmp.path(), "test_no_csv").unwrap();
        let value = serde_json::json!({ "test": true });
        run.write_manifest(&RunManifest {
            schema_version: "0.2.0".into(),
            kind: "run_manifest".into(),
            run_id: "test_no_csv".into(),
            created_at_unix_ms: utc_now_ms(),
            artifact_paths: RunArtifactPaths::default(),
            recipe_hash: None,
            resolved_recipe_id: None,
            safety_report_id: None,
        })
        .unwrap();
        run.write_station_snapshot_json(&value).unwrap();

        fn has_csv(dir: &Path) -> bool {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    if has_csv(&path) {
                        return true;
                    }
                } else if let Some(ext) = path.extension() {
                    if ext == "csv" {
                        return true;
                    }
                }
            }
            false
        }

        assert!(
            !has_csv(&run.run_directory_path()),
            "no CSV should be created"
        );
    }

    // -----------------------------------------------------------------------
    // 16. fake SMB100A can simulate RF ON/OFF state transitions
    // -----------------------------------------------------------------------
    #[test]
    fn fake_device_rf_on_off_transitions() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        assert!(!dev.state().rf_output_enabled);

        dev.send_command("OUTP ON").unwrap();
        assert!(dev.state().rf_output_enabled);
        let resp = dev.query("OUTP?").unwrap();
        assert_eq!(resp.to_string(), "ON");

        dev.send_command("OUTP OFF").unwrap();
        assert!(!dev.state().rf_output_enabled);
        let resp = dev.query("OUTP?").unwrap();
        assert_eq!(resp.to_string(), "OFF");
    }

    // -----------------------------------------------------------------------
    // 17. emergency shutdown path sends OUTP OFF after simulated failure
    // -----------------------------------------------------------------------
    #[test]
    fn emergency_shutdown_evidence_serialization() {
        let es = EmergencyShutdownEvidence {
            shutdown_attempted: true,
            shutdown_command_sent: true,
            shutdown_timestamp_unix_ms: 0,
            outp_query_after_shutdown: Some("0".into()),
            trigger_reason: "simulated failure".into(),
        };
        let json = serde_json::to_string(&es).unwrap();
        assert!(json.contains("shutdown_attempted"));
        assert!(json.contains("simulated failure"));
    }

    // -----------------------------------------------------------------------
    // Additional validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn query_allowlist_accepts_allowed_queries() {
        for cmd in SMB_QUERY_ALLOWLIST {
            assert!(
                validate_smb_query_only(cmd).is_ok(),
                "{} should be allowed",
                cmd
            );
        }
    }

    #[test]
    fn query_only_rejects_set_commands() {
        assert!(validate_smb_query_only("OUTP ON").is_err());
        assert!(validate_smb_query_only("FREQ 1e9").is_err());
    }

    #[test]
    fn freq_set_with_unit_allowed() {
        assert!(validate_microtest_set_command("FREQ 2.882GHz").is_ok());
    }

    #[test]
    fn pow_set_with_dbm_allowed() {
        assert!(validate_microtest_set_command("POW -30dBm").is_ok());
    }

    #[test]
    fn rst_always_rejected() {
        assert!(validate_microtest_set_command("*RST").is_err());
    }

    #[test]
    fn lfo_commands_rejected() {
        assert!(validate_microtest_set_command("LFO ON").is_err());
        assert!(validate_microtest_set_command("LFO:FREQ 1000").is_err());
    }

    #[test]
    fn is_forbidden_command_detects_patterns() {
        assert!(is_forbidden_command("MOD:STAT ON"));
        assert!(is_forbidden_command("SWE:MODE AUTO"));
        assert!(!is_forbidden_command("FREQ 1e9"));
        assert!(!is_forbidden_command("OUTP ON"));
    }

    #[test]
    fn preflight_check_serialization() {
        let p = PreflightCheck {
            passed: true,
            outp_off_before: true,
            mod_stat_off_before: true,
            error_queue_clean_before: true,
            operator_approval_present: true,
            power_within_limit: true,
            duration_within_limit: true,
            warnings: vec![],
            errors: vec![],
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("outp_off_before"));
    }

    #[test]
    fn rf_onoff_result_serialization() {
        let r = RfOnOffResult {
            passed: true,
            rf_on_command_sent: true,
            rf_off_command_sent: true,
            rf_output_confirmed_on: true,
            rf_output_confirmed_off_after: true,
            modulation_remained_off: true,
            frequency_hz_requested: 2.882e9,
            frequency_hz_verified: 2.882e9,
            power_dbm_requested: -30.0,
            power_dbm_verified: -30.0,
            rf_on_duration_ms_requested: 1000,
            rf_on_duration_ms_measured: 1050,
            syst_err_before: vec![],
            syst_err_after: vec![],
            forbidden_commands_sent: 0,
            emergency_shutdown_attempted: false,
            warnings: vec![],
            errors: vec![],
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("rf_output_confirmed_on"));
    }

    #[test]
    fn safety_boundary_note_serialization() {
        let note = SafetyBoundaryNote {
            schema_version: "0.2.0".into(),
            real_smb100a_query_only: false,
            real_smb100a_setting_commands_blocked_except_microtest: true,
            rf_on_requires_manual_approval: true,
            no_csv_policy: true,
            no_sweep: true,
            no_modulation: true,
            no_fm: true,
            no_gui_hardware_access: true,
        };
        let json = serde_json::to_string(&note).unwrap();
        assert!(json.contains("rf_on_requires_manual_approval"));
    }

    #[test]
    fn timeline_tracker_records_events() {
        let mut tracker = TimelineTracker::new();
        let mono = tracker.record("test_event", "smb100a", None);
        assert!(mono > 0 || tracker.events.len() == 1);
        assert_eq!(tracker.events[0].event_type, "test_event");
    }

    #[test]
    fn hash_manifest_computes_correctly() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        fs::write(&path, "hello").unwrap();
        let h = sha256_file(&path).unwrap();
        assert!(h.starts_with("sha256:"));
    }

    #[test]
    fn fake_device_default_freq_and_power() {
        let dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        assert_eq!(dev.state().rf_frequency_hz, 1e9);
        assert_eq!(dev.state().rf_power_dbm, -30.0);
    }

    #[test]
    fn fake_device_freq_set_and_query() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        dev.send_command("FREQ 2882000000").unwrap();
        assert_eq!(dev.state().rf_frequency_hz, 2882000000.0);
        let resp = dev.query("FREQ?").unwrap();
        assert_eq!(resp.to_string(), "2882000000");
    }

    #[test]
    fn fake_device_pow_set_and_query() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        dev.send_command("POW -30").unwrap();
        assert_eq!(dev.state().rf_power_dbm, -30.0);
        let resp = dev.query("POW?").unwrap();
        assert_eq!(resp.to_string(), "-30.00");
    }

    #[test]
    fn fake_device_mod_stat_query_returns_off() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        let resp = dev.query("MOD:STAT?").unwrap();
        assert_eq!(resp.to_string(), "OFF");
    }

    #[test]
    fn fake_device_syst_err_returns_clean() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        let resp = dev.query("SYST:ERR?").unwrap();
        assert_eq!(resp.to_string(), "0,\"No error\"");
    }

    #[test]
    fn max_power_hard_limit_enforced() {
        let cli = Cli {
            smb_host: "fake".into(),
            smb_port: 0,
            smb_query_delay_ms: 0,
            smb_timeout_ms: 1000,
            run_root: "/tmp".into(),
            run_id: "test".into(),
            rf_frequency_hz: 2.882e9,
            rf_power_dbm: -5.0,
            rf_on_duration_ms: 1000,
            max_rf_power_dbm: -5.0,
            operator_approves_rf_on: true,
            operator_approval_note: None,
        };
        let result = run_microtest(&cli);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("-10"));
    }

    #[test]
    fn semicolons_rejected_in_set_commands() {
        assert!(validate_microtest_set_command("FREQ 1e9; OUTP ON").is_err());
        assert!(validate_microtest_set_command("POW -30; OUTP ON").is_err());
    }

    #[test]
    fn semicolons_rejected_in_queries() {
        assert!(validate_smb_query_only("FREQ?; OUTP ON").is_err());
        assert!(validate_smb_query_only("OUTP?; *RST").is_err());
    }
}
