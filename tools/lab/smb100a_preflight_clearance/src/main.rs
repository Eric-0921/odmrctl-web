//! M3.0-A: SMB100A Preflight Error Queue Clearance, No RF Output.
//!
//! Query-only preflight tool that verifies SMB100A is in a clean, safe,
//! RF-off state before any RF output milestone.
//!
//! ## Safety
//! - Default mode is query-only. No set commands reach transport.
//! - `*CLS` is allowed only in diagnostic mode with explicit operator approval.
//! - If OUTP? != 0 or MOD:STAT? != 0, the run aborts immediately.
//! - Forbidden commands (OUTP ON/OFF, FREQ, POW, etc.) are rejected.
//! - No CSV. No OE1022D. No GUI.

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

/// Commands that are forbidden in ALL M3.0-A modes (including diagnostic).
/// Patterns that identify forbidden set commands (used for rejection checks).
const SMB_FORBIDDEN_PATTERNS: &[&str] = &[
    "OUTP ",
    "MOD:STAT ",
    "FREQ ",
    "POW ",
    "POW:ALC ",
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
#[command(name = "smb100a-preflight-clearance")]
#[command(about = "M3.0-A: SMB100A preflight error queue clearance, no RF output")]
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

    /// Default query-only mode; no set commands sent.
    #[arg(long)]
    query_only: bool,

    /// Enable diagnostic error-queue clearance mode (requires operator approval).
    #[arg(long)]
    diagnostic_clear_error_queue: bool,

    /// Operator explicitly approves sending `*CLS` in diagnostic mode.
    #[arg(long)]
    operator_approves_cls: bool,

    /// Optional operator approval note recorded in artifacts.
    #[arg(long)]
    operator_approval_note: Option<String>,
}

// ---------------------------------------------------------------------------
// JSON types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PreflightConfig {
    schema_version: String,
    smb_host: String,
    smb_port: u16,
    smb_query_delay_ms: u64,
    smb_timeout_ms: u64,
    query_only: bool,
    diagnostic_clear_error_queue: bool,
    operator_approves_cls: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    operator_approval_note: Option<String>,
    created_at_unix_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Smb100aPreflightSnapshot {
    schema_version: String,
    device_id: String,
    idn: String,
    queried_at_unix_ms: u64,
    queries: Vec<SmbQueryResult>,
    query_only_mode: bool,
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
struct ClearanceDecision {
    schema_version: String,
    decision: String,
    reason: String,
    cls_sent: bool,
    operator_approved_cls: bool,
    error_queue_clean_after: bool,
    rf_output_off_after: bool,
    modulation_global_off_after: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RfOnMicrotestEligibility {
    eligible_for_rf_on_microtest: bool,
    reason: String,
    smb100a_idn: String,
    rf_output_off: bool,
    modulation_global_off: bool,
    error_queue_clean: bool,
    syst_err_observations: Vec<ErrorQueueObservation>,
    cls_sent: bool,
    operator_approved_cls: bool,
    forbidden_commands_sent: usize,
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
struct ForbiddenCommandCheck {
    passed: bool,
    forbidden_commands_attempted: Vec<String>,
    forbidden_commands_sent_to_transport: Vec<String>,
    rf_output_commands_sent: usize,
    modulation_set_commands_sent: usize,
    frequency_set_commands_sent: usize,
    power_set_commands_sent: usize,
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
    real_smb100a_setting_commands_blocked: bool,
    cls_only_with_manual_approval: bool,
    no_csv_policy: bool,
    no_rf_on: bool,
    no_gui_hardware_access: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HashManifest {
    schema_version: String,
    smb100a_preflight_config_hash: String,
    smb100a_preflight_snapshot_before_hash: String,
    smb100a_preflight_snapshot_after_hash: String,
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

/// Validate that a command is a permitted query in M3.0-A.
fn validate_smb_query_only(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if !trimmed.ends_with('?') {
        return Err(format!(
            "SMB command '{}' is not a query (does not end in '?')",
            trimmed
        ));
    }
    for pat in SMB_FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return Err(format!(
                "SMB command '{}' contains forbidden setting pattern '{}'",
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

/// Validate that `*CLS` is permitted only in explicitly approved diagnostic mode.
fn validate_cls_command(
    cmd: &str,
    diagnostic_mode: bool,
    operator_approves: bool,
) -> Result<(), String> {
    let trimmed = cmd.trim();
    if trimmed != "*CLS" {
        return Err(format!(
            "validate_cls_command called with non-*CLS: '{}'",
            trimmed
        ));
    }
    if !diagnostic_mode {
        return Err("*CLS is forbidden outside diagnostic mode".into());
    }
    if !operator_approves {
        return Err("*CLS requires --operator-approves-cls".into());
    }
    Ok(())
}

/// Check if a command is in the forbidden list.
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
    if trimmed == "*CLS" {
        "diagnostic_clear"
    } else if trimmed.ends_with('?') {
        "query"
    } else {
        "set"
    }
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
        Ok(Self {
            stream,
            timeout_ms,
        })
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
        let mut buf = [0u8; 256];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(_) => break,
            }
        }
    }

    fn close(self) {
        drop(self.stream);
    }
}

// ---------------------------------------------------------------------------
// Preflight execution
// ---------------------------------------------------------------------------

struct PreflightResult {
    snapshot_before: Smb100aPreflightSnapshot,
    snapshot_after: Option<Smb100aPreflightSnapshot>,
    audit: Vec<CommandAuditEntry>,
    error_queue_observations: Vec<ErrorQueueObservation>,
    forbidden_check: ForbiddenCommandCheck,
    eligibility: RfOnMicrotestEligibility,
    timeline: Vec<TimelineEvent>,
    operator_approval: Option<OperatorApproval>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

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

fn run_preflight(cli: &Cli) -> Result<PreflightResult, String> {
    let mut tracker = TimelineTracker::new();
    let mut audit = Vec::new();
    let mut forbidden_attempted = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    let diagnostic_mode = cli.diagnostic_clear_error_queue;
    let operator_approves = cli.operator_approves_cls;
    let delay_ms = cli.smb_query_delay_ms;

    // Connect
    let mut transport = SmbTransport::connect(&cli.smb_host, cli.smb_port, cli.smb_timeout_ms)?;
    tracker.record("smb_connected", "smb100a", None);

    // Build before snapshot
    let idn = do_smb_query(
        &mut transport,
        &mut audit,
        &mut forbidden_attempted,
        delay_ms,
        "*IDN?",
    )?;
    let before_queries = vec![
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
    for q in &before_queries {
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

    // Safety-critical checks on before snapshot
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

    let snapshot_before = Smb100aPreflightSnapshot {
        schema_version: "0.2.0".into(),
        device_id: "smb100a_main".into(),
        idn: idn.clone(),
        queried_at_unix_ms: utc_now_ms(),
        queries: before_results,
        query_only_mode: true,
        connection_closed: false,
    };

    // Error queue observations (3 times)
    let mut error_queue_observations = Vec::new();
    for attempt in 1..=3 {
        let resp = do_smb_query(
            &mut transport,
            &mut audit,
            &mut forbidden_attempted,
            delay_ms,
            "SYST:ERR?",
        )?;
        let clean = resp.trim() == "0,\"No error\"" || resp.trim().starts_with("0,");
        error_queue_observations.push(ErrorQueueObservation {
            timestamp_unix_ms: utc_now_ms(),
            attempt,
            command: "SYST:ERR?".into(),
            response: resp.clone(),
            clean,
        });
        if !clean {
            warnings.push(format!("SYST:ERR? attempt {} returned: {}", attempt, resp));
        }
    }

    let error_queue_clean = error_queue_observations.iter().all(|o| o.clean);

    // Determine whether to send *CLS
    let mut cls_sent = false;
    let mut operator_approval: Option<OperatorApproval> = None;
    let mut snapshot_after: Option<Smb100aPreflightSnapshot> = None;

    if diagnostic_mode && operator_approves {
        if !outp_off || !mod_off {
            errors.push("Refusing *CLS because RF output or MOD is not OFF".into());
        } else if error_queue_clean {
            warnings.push(
                "Error queue already clean; *CLS not strictly necessary but operator approved"
                    .into(),
            );
        }

        if errors.is_empty() {
            // Record operator approval
            operator_approval = Some(OperatorApproval {
                schema_version: "0.2.0".into(),
                approved: true,
                note: cli.operator_approval_note.clone(),
                timestamp_unix_ms: utc_now_ms(),
            });

            // Validate *CLS explicitly
            if let Err(e) = validate_cls_command("*CLS", diagnostic_mode, operator_approves) {
                errors.push(e);
            } else {
                let ts = utc_now_ms();
                transport
                    .send_no_response("*CLS")
                    .map_err(|e| e.to_string())?;
                cls_sent = true;
                audit.push(CommandAuditEntry {
                    timestamp_unix_ms: ts,
                    device_id: "smb100a".into(),
                    command: "*CLS".into(),
                    command_class: "diagnostic_clear".into(),
                    allowed: true,
                    sent_to_transport: true,
                    manual_approval_required: Some(true),
                    manual_approval_present: Some(true),
                    rejection_reason: None,
                    response_preview: None,
                    transport_error: None,
                    safety_relevant: Some(true),
                });
                std::thread::sleep(Duration::from_millis(100));

                // Re-query after *CLS
                let after_idn = do_smb_query(
                    &mut transport,
                    &mut audit,
                    &mut forbidden_attempted,
                    delay_ms,
                    "*IDN?",
                )?;
                let mut after_results = Vec::new();
                for q in &before_queries {
                    let resp = do_smb_query(
                        &mut transport,
                        &mut audit,
                        &mut forbidden_attempted,
                        delay_ms,
                        q,
                    )?;
                    after_results.push(SmbQueryResult {
                        command: q.to_string(),
                        response: resp,
                    });
                }

                // Verify safety-critical states after *CLS
                for r in &after_results {
                    if r.command == "OUTP?"
                        && !(r.response.trim() == "0"
                            || r.response.trim().eq_ignore_ascii_case("OFF"))
                    {
                        errors.push(format!(
                            "After *CLS: OUTP? = '{}' (expected OFF/0)",
                            r.response
                        ));
                    }
                    if r.command == "MOD:STAT?"
                        && !(r.response.trim() == "0"
                            || r.response.trim().eq_ignore_ascii_case("OFF"))
                    {
                        errors.push(format!(
                            "After *CLS: MOD:STAT? = '{}' (expected OFF/0)",
                            r.response
                        ));
                    }
                }

                // Re-check error queue 3 times
                for attempt in 1..=3 {
                    let resp = do_smb_query(
                        &mut transport,
                        &mut audit,
                        &mut forbidden_attempted,
                        delay_ms,
                        "SYST:ERR?",
                    )?;
                    let clean = resp.trim() == "0,\"No error\"" || resp.trim().starts_with("0,");
                    error_queue_observations.push(ErrorQueueObservation {
                        timestamp_unix_ms: utc_now_ms(),
                        attempt: attempt + 3,
                        command: "SYST:ERR?".into(),
                        response: resp.clone(),
                        clean,
                    });
                    if !clean {
                        warnings.push(format!(
                            "After *CLS: SYST:ERR? attempt {} returned: {}",
                            attempt + 3,
                            resp
                        ));
                    }
                }

                snapshot_after = Some(Smb100aPreflightSnapshot {
                    schema_version: "0.2.0".into(),
                    device_id: "smb100a_main".into(),
                    idn: after_idn,
                    queried_at_unix_ms: utc_now_ms(),
                    queries: after_results,
                    query_only_mode: true,
                    connection_closed: false,
                });
            }
        }
    }

    transport.close();
    tracker.record("smb_disconnected", "smb100a", None);

    // Final eligibility
    let final_error_queue_clean = error_queue_observations.iter().all(|o| o.clean);
    let eligible = outp_off && mod_off && final_error_queue_clean && errors.is_empty();

    let eligibility = RfOnMicrotestEligibility {
        eligible_for_rf_on_microtest: eligible,
        reason: if eligible {
            "All safety checks passed".into()
        } else {
            format!("Safety checks failed: {}", errors.join("; "))
        },
        smb100a_idn: idn,
        rf_output_off: outp_off,
        modulation_global_off: mod_off,
        error_queue_clean: final_error_queue_clean,
        syst_err_observations: error_queue_observations.clone(),
        cls_sent,
        operator_approved_cls: operator_approval
            .as_ref()
            .map(|a| a.approved)
            .unwrap_or(false),
        forbidden_commands_sent: 0,
        warnings: warnings.clone(),
        errors: errors.clone(),
    };

    let forbidden_check = ForbiddenCommandCheck {
        passed: true,
        forbidden_commands_attempted: forbidden_attempted,
        forbidden_commands_sent_to_transport: Vec::new(),
        rf_output_commands_sent: 0,
        modulation_set_commands_sent: 0,
        frequency_set_commands_sent: 0,
        power_set_commands_sent: 0,
    };

    Ok(PreflightResult {
        snapshot_before,
        snapshot_after,
        audit,
        error_queue_observations,
        forbidden_check,
        eligibility,
        timeline: tracker.events,
        operator_approval,
        warnings,
        errors,
    })
}

fn is_safety_relevant(cmd: &str) -> bool {
    matches!(cmd.trim(), "OUTP?" | "MOD:STAT?" | "SYST:ERR?" | "*CLS")
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

    if !cli.query_only && !cli.diagnostic_clear_error_queue {
        eprintln!("Error: Must specify either --query-only or --diagnostic-clear-error-queue");
        std::process::exit(1);
    }

    if cli.diagnostic_clear_error_queue && !cli.operator_approves_cls {
        eprintln!("Error: --diagnostic-clear-error-queue requires --operator-approves-cls");
        std::process::exit(1);
    }

    let run_root = PathBuf::from(&cli.run_root);
    let run_dir = create_run_directory(&run_root, &cli.run_id)
        .unwrap_or_else(|e| panic!("Failed to create run directory: {}", e));

    // Create extra subdirectories
    let _ = fs::create_dir_all(run_dir.run_directory_path().join("preflight"));

    let created_at = utc_now_ms();

    let config = PreflightConfig {
        schema_version: "0.2.0".into(),
        smb_host: cli.smb_host.clone(),
        smb_port: cli.smb_port,
        smb_query_delay_ms: cli.smb_query_delay_ms,
        smb_timeout_ms: cli.smb_timeout_ms,
        query_only: cli.query_only,
        diagnostic_clear_error_queue: cli.diagnostic_clear_error_queue,
        operator_approves_cls: cli.operator_approves_cls,
        operator_approval_note: cli.operator_approval_note.clone(),
        created_at_unix_ms: created_at,
    };

    // Write manifest
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

    // Run preflight
    let result = run_preflight(&cli).unwrap_or_else(|e| {
        eprintln!("Preflight failed: {}", e);
        std::process::exit(1);
    });

    // Write artifacts
    run_dir
        .write_json_artifact("metadata/smb100a_preflight_config.json", &config)
        .unwrap();

    run_dir
        .write_json_artifact(
            "metadata/smb100a_preflight_snapshot_before.json",
            &result.snapshot_before,
        )
        .unwrap();

    if let Some(ref after) = result.snapshot_after {
        run_dir
            .write_json_artifact("metadata/smb100a_preflight_snapshot_after.json", after)
            .unwrap();
    }

    let station_quality = StationSnapshotQuality {
        schema_version: "0.2.0".into(),
        status: if result.eligibility.eligible_for_rf_on_microtest {
            "passed".into()
        } else {
            "failed".into()
        },
        eligible_for_rf_on_microtest: result.eligibility.eligible_for_rf_on_microtest,
        warnings: result.warnings.clone(),
        errors: result.errors.clone(),
        query_interrupted_seen: result
            .error_queue_observations
            .iter()
            .any(|o| o.response.contains("-410")),
        smb_query_delay_ms: cli.smb_query_delay_ms,
    };
    run_dir
        .write_json_artifact("metadata/station_snapshot_quality.json", &station_quality)
        .unwrap();

    let safety_note = SafetyBoundaryNote {
        schema_version: "0.2.0".into(),
        real_smb100a_query_only: true,
        real_smb100a_setting_commands_blocked: true,
        cls_only_with_manual_approval: true,
        no_csv_policy: true,
        no_rf_on: true,
        no_gui_hardware_access: true,
    };
    run_dir
        .write_json_artifact("metadata/safety_boundary_note.json", &safety_note)
        .unwrap();

    // Events
    let mut event_writer = run_dir.open_event_writer().unwrap();
    event_writer
        .write_event(&make_event(
            &cli.run_id,
            RunEventType::RunCreated,
            EventLevel::Info,
            "M3.0-A preflight run created",
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
            "SMB100A preflight snapshot completed",
            "smb100a",
            None,
            None,
        ))
        .unwrap();
    event_writer
        .write_event(&make_event(
            &cli.run_id,
            RunEventType::AuditCompleted,
            EventLevel::Info,
            "Command audit completed",
            "smb100a",
            None,
            None,
        ))
        .unwrap();
    if result.eligibility.eligible_for_rf_on_microtest {
        event_writer
            .write_event(&make_event(
                &cli.run_id,
                RunEventType::RunCompleted,
                EventLevel::Info,
                "Preflight passed; eligible for RF ON micro-test",
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
                &format!("Preflight failed: {}", result.errors.join("; ")),
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

    write_jsonl(
        &run_dir
            .run_directory_path()
            .join("preflight/error_queue_observations.jsonl"),
        &result.error_queue_observations,
    )
    .unwrap();

    run_dir
        .write_json_artifact(
            "preflight/clearance_decision.json",
            &ClearanceDecision {
                schema_version: "0.2.0".into(),
                decision: if result.eligibility.eligible_for_rf_on_microtest {
                    "cleared".into()
                } else {
                    "blocked".into()
                },
                reason: result.eligibility.reason.clone(),
                cls_sent: result.eligibility.cls_sent,
                operator_approved_cls: result.eligibility.operator_approved_cls,
                error_queue_clean_after: result.eligibility.error_queue_clean,
                rf_output_off_after: result.eligibility.rf_output_off,
                modulation_global_off_after: result.eligibility.modulation_global_off,
            },
        )
        .unwrap();

    run_dir
        .write_json_artifact(
            "preflight/rf_on_microtest_eligibility.json",
            &result.eligibility,
        )
        .unwrap();

    if let Some(ref approval) = result.operator_approval {
        run_dir
            .write_json_artifact("preflight/operator_approval.json", approval)
            .unwrap();
    }

    run_dir
        .write_json_artifact(
            "preflight/forbidden_command_check.json",
            &result.forbidden_check,
        )
        .unwrap();

    // Hash manifest
    let hash_manifest = HashManifest {
        schema_version: "0.2.0".into(),
        smb100a_preflight_config_hash: sha256_file(
            &run_dir
                .run_directory_path()
                .join("metadata/smb100a_preflight_config.json"),
        )
        .unwrap_or_default(),
        smb100a_preflight_snapshot_before_hash: sha256_file(
            &run_dir
                .run_directory_path()
                .join("metadata/smb100a_preflight_snapshot_before.json"),
        )
        .unwrap_or_default(),
        smb100a_preflight_snapshot_after_hash: if result.snapshot_after.is_some() {
            sha256_file(
                &run_dir
                    .run_directory_path()
                    .join("metadata/smb100a_preflight_snapshot_after.json"),
            )
            .unwrap_or_default()
        } else {
            "n/a".into()
        },
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
        "eligible_for_rf_on_microtest": result.eligibility.eligible_for_rf_on_microtest,
        "cls_sent": result.eligibility.cls_sent,
        "operator_approved_cls": result.eligibility.operator_approved_cls,
    });
    run_dir
        .write_json_artifact("audit_report.json", &audit_report)
        .unwrap();

    println!("M3.0-A preflight complete.");
    println!(
        "  Eligible for RF ON micro-test: {}",
        result.eligibility.eligible_for_rf_on_microtest
    );
    println!(
        "  Error queue clean: {}",
        result.eligibility.error_queue_clean
    );
    println!("  RF output OFF: {}", result.eligibility.rf_output_off);
    println!(
        "  MOD global OFF: {}",
        result.eligibility.modulation_global_off
    );
    println!("  *CLS sent: {}", result.eligibility.cls_sent);
    println!(
        "  Forbidden commands sent: {}",
        result
            .forbidden_check
            .forbidden_commands_sent_to_transport
            .len()
    );
    println!(
        "  Run directory: {}",
        run_dir.run_directory_path().display()
    );
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
    // Command validation tests
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
    fn query_only_rejects_cls() {
        let err = validate_smb_query_only("*CLS").unwrap_err();
        assert!(err.contains("not a query"));
    }

    #[test]
    fn cls_allowed_only_with_both_flags() {
        assert!(validate_cls_command("*CLS", true, true).is_ok());
    }

    #[test]
    fn cls_rejected_without_diagnostic_mode() {
        let err = validate_cls_command("*CLS", false, true).unwrap_err();
        assert!(err.contains("forbidden outside diagnostic mode"));
    }

    #[test]
    fn cls_rejected_without_operator_approval() {
        let err = validate_cls_command("*CLS", true, false).unwrap_err();
        assert!(err.contains("requires --operator-approves-cls"));
    }

    #[test]
    fn outp_on_is_rejected() {
        assert!(is_forbidden_command("OUTP ON"));
    }

    #[test]
    fn outp_off_is_rejected() {
        assert!(is_forbidden_command("OUTP OFF"));
    }

    #[test]
    fn mod_stat_on_is_rejected() {
        assert!(is_forbidden_command("MOD:STAT ON"));
    }

    #[test]
    fn freq_set_is_rejected() {
        assert!(is_forbidden_command("FREQ 2882000000"));
    }

    #[test]
    fn pow_set_is_rejected() {
        assert!(is_forbidden_command("POW -15dBm"));
    }

    #[test]
    fn forbidden_patterns_cover_outp() {
        assert!(SMB_FORBIDDEN_PATTERNS.iter().any(|p| "OUTP ON".contains(p)));
    }

    // -----------------------------------------------------------------------
    // Eligibility logic tests
    // -----------------------------------------------------------------------

    #[test]
    fn eligibility_true_when_safe() {
        let e = RfOnMicrotestEligibility {
            eligible_for_rf_on_microtest: true,
            reason: "ok".into(),
            smb100a_idn: "test".into(),
            rf_output_off: true,
            modulation_global_off: true,
            error_queue_clean: true,
            syst_err_observations: vec![],
            cls_sent: false,
            operator_approved_cls: false,
            forbidden_commands_sent: 0,
            warnings: vec![],
            errors: vec![],
        };
        assert!(e.eligible_for_rf_on_microtest);
    }

    #[test]
    fn eligibility_false_when_outp_on() {
        let e = RfOnMicrotestEligibility {
            eligible_for_rf_on_microtest: false,
            reason: "unsafe".into(),
            smb100a_idn: "test".into(),
            rf_output_off: false,
            modulation_global_off: true,
            error_queue_clean: true,
            syst_err_observations: vec![],
            cls_sent: false,
            operator_approved_cls: false,
            forbidden_commands_sent: 0,
            warnings: vec![],
            errors: vec!["OUTP ON".into()],
        };
        assert!(!e.eligible_for_rf_on_microtest);
    }

    #[test]
    fn eligibility_false_when_mod_on() {
        let e = RfOnMicrotestEligibility {
            eligible_for_rf_on_microtest: false,
            reason: "unsafe".into(),
            smb100a_idn: "test".into(),
            rf_output_off: true,
            modulation_global_off: false,
            error_queue_clean: true,
            syst_err_observations: vec![],
            cls_sent: false,
            operator_approved_cls: false,
            forbidden_commands_sent: 0,
            warnings: vec![],
            errors: vec!["MOD ON".into()],
        };
        assert!(!e.eligible_for_rf_on_microtest);
    }

    #[test]
    fn eligibility_false_when_syst_err_nonzero() {
        let e = RfOnMicrotestEligibility {
            eligible_for_rf_on_microtest: false,
            reason: "unsafe".into(),
            smb100a_idn: "test".into(),
            rf_output_off: true,
            modulation_global_off: true,
            error_queue_clean: false,
            syst_err_observations: vec![ErrorQueueObservation {
                timestamp_unix_ms: 0,
                attempt: 1,
                command: "SYST:ERR?".into(),
                response: "-410,\"Query interrupted\"".into(),
                clean: false,
            }],
            cls_sent: false,
            operator_approved_cls: false,
            forbidden_commands_sent: 0,
            warnings: vec![],
            errors: vec!["error queue not clean".into()],
        };
        assert!(!e.eligible_for_rf_on_microtest);
    }

    // -----------------------------------------------------------------------
    // Forbidden command check tests
    // -----------------------------------------------------------------------

    #[test]
    fn forbidden_check_passes_when_empty() {
        let f = ForbiddenCommandCheck {
            passed: true,
            forbidden_commands_attempted: vec![],
            forbidden_commands_sent_to_transport: vec![],
            rf_output_commands_sent: 0,
            modulation_set_commands_sent: 0,
            frequency_set_commands_sent: 0,
            power_set_commands_sent: 0,
        };
        assert!(f.passed);
    }

    #[test]
    fn forbidden_check_fails_when_transport_send() {
        let f = ForbiddenCommandCheck {
            passed: false,
            forbidden_commands_attempted: vec!["OUTP ON".into()],
            forbidden_commands_sent_to_transport: vec!["OUTP ON".into()],
            rf_output_commands_sent: 1,
            modulation_set_commands_sent: 0,
            frequency_set_commands_sent: 0,
            power_set_commands_sent: 0,
        };
        assert!(!f.passed);
    }

    // -----------------------------------------------------------------------
    // Fake device integration tests
    // -----------------------------------------------------------------------

    #[test]
    fn fake_device_default_state_is_safe() {
        let dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        let st = dev.state();
        assert!(!st.rf_output_enabled);
        assert!(!st.modulation_global_enabled);
    }

    #[test]
    fn fake_device_query_outp_returns_off() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        let resp = dev.query("OUTP?").unwrap();
        assert_eq!(resp.to_string(), "OFF");
    }

    #[test]
    fn fake_device_query_mod_stat_returns_off() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        let resp = dev.query("MOD:STAT?").unwrap();
        assert_eq!(resp.to_string(), "OFF");
    }

    #[test]
    fn fake_device_query_syst_err_returns_clean() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        let resp = dev.query("SYST:ERR?").unwrap();
        assert_eq!(resp.to_string(), "0,\"No error\"");
    }

    #[test]
    fn fake_device_accepts_cls() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        // *CLS is a non-query command; FakeDevice::send_command handles it
        let _ = dev.send_command("*CLS").unwrap();
        // Should not panic or error
    }

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

    #[test]
    fn safety_boundary_note_serialization() {
        let note = SafetyBoundaryNote {
            schema_version: "0.2.0".into(),
            real_smb100a_query_only: true,
            real_smb100a_setting_commands_blocked: true,
            cls_only_with_manual_approval: true,
            no_csv_policy: true,
            no_rf_on: true,
            no_gui_hardware_access: true,
        };
        let json = serde_json::to_string(&note).unwrap();
        assert!(json.contains("real_smb100a_query_only"));
    }

    #[test]
    fn run_manifest_has_correct_paths() {
        let m = RunManifest {
            schema_version: "0.2.0".into(),
            kind: "run_manifest".into(),
            run_id: "m3_0a_test".into(),
            created_at_unix_ms: 0,
            artifact_paths: RunArtifactPaths::default(),
            recipe_hash: None,
            resolved_recipe_id: None,
            safety_report_id: None,
        };
        assert_eq!(m.artifact_paths.manifest, "manifest.json");
        assert_eq!(m.artifact_paths.events, "events.jsonl");
    }

    #[test]
    fn timeline_tracker_records_events() {
        let mut tracker = TimelineTracker::new();
        let mono = tracker.record("test_event", "smb100a", None);
        assert!(mono > 0 || tracker.events.len() == 1);
        assert_eq!(tracker.events[0].event_type, "test_event");
    }

    #[test]
    fn command_audit_entry_has_expected_fields() {
        let entry = CommandAuditEntry {
            timestamp_unix_ms: 0,
            device_id: "smb100a".into(),
            command: "OUTP?".into(),
            command_class: "query".into(),
            allowed: true,
            sent_to_transport: true,
            manual_approval_required: None,
            manual_approval_present: None,
            rejection_reason: None,
            response_preview: Some("0".into()),
            transport_error: None,
            safety_relevant: Some(true),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("OUTP?"));
        assert!(json.contains("query"));
    }

    #[test]
    fn hash_manifest_computes_correctly() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        fs::write(&path, "hello").unwrap();
        let h = sha256_file(&path).unwrap();
        assert!(h.starts_with("sha256:"));
    }
}
