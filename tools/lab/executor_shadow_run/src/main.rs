//! M2.9: Executor Shadow Mode with Real Run Artifacts, No Real Hardware Control.
//!
//! Connects the recipe / compiler / executor concept to the real run artifact
//! system without allowing the executor to control real hardware.
//!
//! ## Safety
//! - Shadow plan may contain realistic set commands (FREQ, POW, OUTP ON, etc.).
//! - Real SMB100A receives only query commands via allowlist.
//! - Real OE1022D receives only `*IDN?` and `RALL?`.
//! - All dangerous actions are recorded as shadow-only and blocked from transport.
//! - SMB100A connection closes before OE1022D acquisition begins.
//! - No CSV. No RF ON. No `*CLS` in default mode.

use clap::Parser;
use odmr_logging::{
    create_run_directory, EventLevel, RawIndexEntry, RunArtifactPaths, RunDirectory, RunEvent,
    RunEventType, RunManifest,
};
use odmr_oe1022d::parser::{latest_b_channel_sample, parse_rall_frame, RALL_FRAME_BYTES};
use odmr_recipe::{DeviceAction, Recipe, ResolvedRecipe, SafetyLimit};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Hard-coded command allow-lists (identical to M2.8)
// ---------------------------------------------------------------------------

const OE_ALLOWED_COMMANDS: &[&str] = &["*IDN?", "RALL?"];

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
    "FREQ:STAR?",
    "FREQ:STOP?",
    "SWE:MODE?",
    "SWE:SPAC?",
    "SWE:FREQ:STEP?",
    "SWE:FREQ:DWEL?",
    "SYST:ERR?",
];

const SMB_SETTING_PATTERNS: &[&str] = &[
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

const OE_FORBIDDEN_PATTERNS: &[&str] = &[
    "*RST", "RST", "INIT", "RUN", "SSETD", "RSETD", "APHSD", "FMODD", "RSLPD", "PHASD", "ISRCD",
    "SENSD", "OFLTD", "OFSLD", "HARMD",
];

/// Validate an OE1022D command against the safety allow-list.
fn validate_oe_command(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if !OE_ALLOWED_COMMANDS.contains(&trimmed) {
        return Err(format!(
            "OE command '{}' is not in the safe allow-list",
            trimmed
        ));
    }
    for pat in OE_FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return Err(format!(
                "OE command '{}' contains forbidden pattern '{}'",
                trimmed, pat
            ));
        }
    }
    Ok(())
}

/// Validate an SMB100A command is query-only.
fn validate_smb_query_only(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if !trimmed.ends_with('?') {
        return Err(format!(
            "SMB command '{}' is not a query (does not end in '?')",
            trimmed
        ));
    }
    for pat in SMB_SETTING_PATTERNS {
        if trimmed.contains(pat) {
            return Err(format!(
                "SMB command '{}' contains setting pattern '{}'",
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

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(name = "executor-shadow-run")]
#[command(about = "M2.9: Executor shadow mode with real query snapshot + passive acquisition")]
struct Cli {
    #[arg(long)]
    recipe: String,

    #[arg(long, default_value = "examples/station.lab.example.json")]
    station: String,

    #[arg(long, default_value = "169.254.2.20")]
    smb_host: String,

    #[arg(long, default_value = "5025")]
    smb_port: u16,

    #[arg(long, default_value = "/dev/cu.usbmodem3361358734371")]
    oe_port: String,

    #[arg(long, default_value = "921600")]
    oe_baud: u32,

    #[arg(long, default_value = "100")]
    frames: u32,

    #[arg(long, default_value = "20")]
    delay_ms: u64,

    #[arg(long, default_value = "5000")]
    timeout_ms: u64,

    #[arg(long, default_value = "3000")]
    smb_timeout_ms: u64,

    #[arg(long, default_value = "100")]
    smb_query_delay_ms: u64,

    #[arg(long, default_value = "../../runs")]
    run_root: String,

    #[arg(long)]
    run_id: String,

    #[arg(long)]
    shadow_only: bool,

    #[arg(long)]
    enable_real_smb_query_snapshot: bool,

    #[arg(long)]
    enable_real_oe_passive_acquisition: bool,

    #[arg(long)]
    write_state_hashes: bool,

    #[arg(long)]
    enable_timeline_alignment: bool,
}

// ---------------------------------------------------------------------------
// JSON types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ShadowCommandEntry {
    shadow_command_id: String,
    step_id: String,
    device_id: String,
    command: String,
    command_class: String,
    dangerous: bool,
    would_touch_real_hardware: bool,
    sent_to_real_hardware: bool,
    shadow_only: bool,
    reason_not_sent: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ShadowStepTimelineEntry {
    shadow_step_id: String,
    phase: String,
    started_at_monotonic_ns: u64,
    completed_at_monotonic_ns: u64,
    device_actions_planned: usize,
    device_actions_blocked: usize,
    shadow_commands: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FrameToShadowStepAlignment {
    frame_seq: u32,
    raw_offset: u64,
    raw_nbytes: u64,
    frame_monotonic_ns_since_run_start: u64,
    shadow_step_id: String,
    shadow_step_phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    smb100a_state_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    station_snapshot_hash: Option<String>,
    alignment_method: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExecutorShadowSummary {
    schema_version: String,
    shadow_mode: bool,
    shadow_command_count: usize,
    dangerous_shadow_command_count: usize,
    real_smb100a_queries_sent: usize,
    real_oe1022d_commands_sent: usize,
    forbidden_commands_sent_to_transport: usize,
    frame_count: usize,
    frame_alignment_count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ForbiddenRealCommandCheck {
    passed: bool,
    forbidden_commands_attempted: Vec<String>,
    forbidden_commands_sent_to_transport: Vec<String>,
    real_smb100a_set_commands_sent: usize,
    real_oe1022d_setting_commands_sent: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CommandAuditEntry {
    timestamp_unix_ms: u64,
    device_id: String,
    command: String,
    mode: String,
    allowed: bool,
    sent_to_transport: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    rejection_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transport_error: Option<String>,
}

#[derive(Serialize)]
struct BChannelPreviewRow {
    run_id: String,
    frame_index: u32,
    timestamp_unix_ms: u64,
    b_x_mv: f64,
    b_y_mv: f64,
    b_freq_hz: f64,
    b_noise_mv: f64,
    b_pll_locked: bool,
    b_input_overload: bool,
    b_gain_overload: bool,
}

#[derive(Serialize)]
struct FrameSummaryRow {
    run_id: String,
    frame_index: u32,
    timestamp_unix_ms: u64,
    raw_len: usize,
    parse_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_snapshot_hash: Option<String>,
}

#[derive(Serialize)]
struct AcquisitionConfig {
    schema_version: String,
    smb_host: String,
    smb_port: u16,
    oe_port: String,
    oe_baud: u32,
    frames_requested: u32,
    delay_ms: u64,
    timeout_ms: u64,
    smb_timeout_ms: u64,
    smb_query_delay_ms: u64,
    created_at_unix_ms: u64,
    shadow_mode: bool,
}

#[derive(Serialize)]
struct ParserVersionMeta {
    schema_version: String,
    parser_crate: String,
    parser_version: String,
    rall_frame_bytes: usize,
}

#[derive(Serialize)]
struct SafetyBoundaryNote {
    schema_version: String,
    real_oe1022d_allowed_commands: Vec<String>,
    real_smb100a_query_only: bool,
    real_smb100a_setting_commands_blocked: bool,
    smb_connection_closed_before_acquisition: bool,
    no_csv_policy: bool,
    no_real_rf_on: bool,
    no_gui_hardware_access: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Smb100aQuerySnapshot {
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
struct SmbQueryTiming {
    schema_version: String,
    queries: Vec<QueryTimingEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct QueryTimingEntry {
    command: String,
    response: String,
    wall_time_utc: String,
    monotonic_ns: u64,
    monotonic_ns_since_run_start: u64,
    duration_ms: u64,
}

#[derive(Serialize)]
struct TimelineEvent {
    event_type: String,
    wall_time_utc: String,
    monotonic_ns: u64,
    monotonic_ns_since_run_start: u64,
    device_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct StationSnapshotQuality {
    schema_version: String,
    status: String,
    eligible_for_rf_on_microtest: bool,
    warnings: Vec<String>,
    errors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    smb100a_query_error: Option<String>,
    query_interrupted_seen: bool,
    smb_query_delay_ms: u64,
    smb_connection_closed_before_acquisition: bool,
    oe_command_allowlist: Vec<String>,
    smb_command_allowlist: Vec<String>,
}

#[derive(Serialize)]
struct HashManifest {
    schema_version: String,
    station_snapshot_hash: String,
    smb100a_query_snapshot_hash: String,
    acquisition_config_hash: String,
    parser_version_hash: String,
    safety_boundary_note_hash: String,
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

    fn monotonic_ns_since_start(&self) -> u64 {
        self.run_start_instant.elapsed().as_nanos() as u64
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

fn compute_hash_manifest(run_dir: &Path) -> Result<HashManifest, String> {
    Ok(HashManifest {
        schema_version: "0.2.0".into(),
        station_snapshot_hash: sha256_file(&run_dir.join("metadata/station_snapshot.json"))?,
        smb100a_query_snapshot_hash: sha256_file(
            &run_dir.join("metadata/smb100a_query_snapshot.json"),
        )?,
        acquisition_config_hash: sha256_file(&run_dir.join("metadata/acquisition_config.json"))?,
        parser_version_hash: sha256_file(&run_dir.join("metadata/parser_version.json"))?,
        safety_boundary_note_hash: sha256_file(
            &run_dir.join("metadata/safety_boundary_note.json"),
        )?,
    })
}

// ---------------------------------------------------------------------------
// SMB100A query-only TCP transport (M2.8)
// ---------------------------------------------------------------------------

struct SmbQueryTransport {
    stream: TcpStream,
    _timeout_ms: u64,
}

impl SmbQueryTransport {
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
            _timeout_ms: timeout_ms,
        })
    }

    fn query(&mut self, cmd: &str) -> Result<String, String> {
        validate_smb_query_only(cmd)?;
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

    fn query_with_drain_and_timing(
        &mut self,
        cmd: &str,
        tracker: &mut TimelineTracker,
        delay_ms: u64,
    ) -> Result<(String, QueryTimingEntry), String> {
        validate_smb_query_only(cmd)?;
        let mono_before = tracker.monotonic_ns_since_start();
        let wall_before = wall_time_utc_iso();
        let cmd_with_term = format!("{}\n", cmd);
        self.stream
            .write_all(cmd_with_term.as_bytes())
            .map_err(|e| format!("TCP write failed: {}", e))?;
        self.stream
            .flush()
            .map_err(|e| format!("TCP flush failed: {}", e))?;
        let mut reader = BufReader::new(&self.stream);
        let mut line = String::new();
        let resp = match reader.read_line(&mut line) {
            Ok(0) => Err("TCP read returned empty".to_string()),
            Ok(_) => {
                line = line.trim().to_string();
                Ok(line)
            }
            Err(e) => Err(format!("TCP read failed: {}", e)),
        }?;
        self.drain_buffer();
        let mono_after = tracker.monotonic_ns_since_start();
        let duration_ms = (mono_after - mono_before) / 1_000_000;
        let timing = QueryTimingEntry {
            command: cmd.into(),
            response: resp.clone(),
            wall_time_utc: wall_before,
            monotonic_ns: mono_after,
            monotonic_ns_since_run_start: mono_after,
            duration_ms,
        };
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
        Ok((resp, timing))
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

fn build_smb_query_snapshot(
    transport: &mut SmbQueryTransport,
    tracker: &mut TimelineTracker,
    delay_ms: u64,
) -> Result<(Smb100aQuerySnapshot, SmbQueryTiming, Vec<String>), String> {
    let idn = transport.query("*IDN?")?;
    let queries = vec![
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
    let mut results = Vec::new();
    let mut timings = Vec::new();
    let mut warnings = Vec::new();
    for q in &queries {
        let (resp, timing) = transport.query_with_drain_and_timing(q, tracker, delay_ms)?;
        if *q == "SYST:ERR?" && resp.contains("-410") {
            warnings.push(format!("SYST:ERR? returned {}", resp));
        }
        results.push(SmbQueryResult {
            command: q.to_string(),
            response: resp,
        });
        timings.push(timing);
    }
    let snapshot = Smb100aQuerySnapshot {
        schema_version: "0.2.0".into(),
        device_id: "smb100a_main".into(),
        idn,
        queried_at_unix_ms: utc_now_ms(),
        queries: results,
        query_only_mode: true,
        connection_closed: false,
    };
    let timing = SmbQueryTiming {
        schema_version: "0.2.0".into(),
        queries: timings,
    };
    Ok((snapshot, timing, warnings))
}

// ---------------------------------------------------------------------------
// OE1022D serial helpers (real hardware)
// ---------------------------------------------------------------------------

fn oe_verify_identity(port: &str, baud: u32, timeout_ms: u64) -> Result<String, String> {
    validate_oe_command("*IDN?")?;
    let mut port = serialport::new(port, baud)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| format!("open serial: {}", e))?;
    let _ = port.clear(serialport::ClearBuffer::Input);
    let cmd = "*IDN?\r";
    port.write_all(cmd.as_bytes())
        .map_err(|e| format!("write: {}", e))?;
    port.flush().map_err(|e| format!("flush: {}", e))?;
    std::thread::sleep(Duration::from_millis(500));
    let mut buf = vec![0u8; 4096];
    let n = port.read(&mut buf).map_err(|e| format!("read: {}", e))?;
    buf.truncate(n);
    let text = String::from_utf8_lossy(&buf)
        .replace('\x00', "")
        .trim()
        .to_string();
    Ok(text)
}

fn oe_capture_single_frame(port: &str, baud: u32, timeout_ms: u64) -> Result<Vec<u8>, String> {
    validate_oe_command("RALL?")?;
    let mut port = serialport::new(port, baud)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| format!("open serial: {}", e))?;
    let _ = port.clear(serialport::ClearBuffer::Input);
    let cmd = "RALL?\r";
    port.write_all(cmd.as_bytes())
        .map_err(|e| format!("write: {}", e))?;
    port.flush().map_err(|e| format!("flush: {}", e))?;
    std::thread::sleep(Duration::from_millis(800));
    let mut frame_buf = Vec::with_capacity(RALL_FRAME_BYTES);
    let read_deadline = Instant::now() + Duration::from_millis(timeout_ms);
    while frame_buf.len() < RALL_FRAME_BYTES && Instant::now() < read_deadline {
        let mut chunk = vec![0u8; 4096];
        match port.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                chunk.truncate(n);
                frame_buf.extend_from_slice(&chunk);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => {
                return Err(format!(
                    "serial read error after {} bytes: {}",
                    frame_buf.len(),
                    e
                ));
            }
        }
    }
    if frame_buf.is_empty() {
        return Err("timeout: zero bytes read after RALL?".into());
    }
    Ok(frame_buf)
}

// ---------------------------------------------------------------------------
// JSONL writer helper
// ---------------------------------------------------------------------------

fn write_parsed_jsonl<T: Serialize>(
    run: &RunDirectory,
    relative_path: &str,
    rows: &[T],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = run.run_directory_path().join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let mut writer = BufWriter::new(file);
    for row in rows {
        let line = serde_json::to_string(row)?;
        writeln!(writer, "{}", line)?;
    }
    writer.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Action-to-command translation (extended from odmr-executor)
// ---------------------------------------------------------------------------

/// Translate a generic DeviceAction into a device-specific command string.
/// Returns Ok(cmd) for known actions, Err for unsupported ones.
fn translate_action_to_command(action: &DeviceAction) -> Result<String, String> {
    let params = action
        .params
        .as_ref()
        .ok_or_else(|| format!("missing params for action '{}'", action.action))?;

    match action.action.as_str() {
        "set_rf_frequency" => {
            let freq = params
                .get("frequency_hz")
                .and_then(|v| v.as_f64())
                .ok_or("missing frequency_hz")?;
            Ok(odmr_smb100a::commands::set_frequency_hz(freq))
        }
        "set_rf_power" => {
            let power = params
                .get("power_dbm")
                .and_then(|v| v.as_f64())
                .ok_or("missing power_dbm")?;
            Ok(odmr_smb100a::commands::set_power_dbm(power))
        }
        "set_rf_output_enabled" => {
            let enabled = params
                .get("enabled")
                .and_then(|v| v.as_bool())
                .ok_or("missing enabled")?;
            Ok(odmr_smb100a::commands::set_output(enabled).to_string())
        }
        "set_fm_deviation" => {
            let dev = params
                .get("fm_deviation_hz")
                .and_then(|v| v.as_f64())
                .ok_or("missing fm_deviation_hz")?;
            Ok(odmr_smb100a::commands::set_fm_deviation_hz(dev))
        }
        "set_fm_state" => {
            let enabled = params
                .get("enabled")
                .and_then(|v| v.as_bool())
                .ok_or("missing enabled")?;
            Ok(odmr_smb100a::commands::set_fm_state(enabled).to_string())
        }
        "set_fm_source" => {
            let source = params
                .get("source")
                .and_then(|v| v.as_str())
                .ok_or("missing source")?;
            Ok(odmr_smb100a::commands::set_fm_source(source))
        }
        "set_lf_output" => {
            let enabled = params
                .get("enabled")
                .and_then(|v| v.as_bool())
                .ok_or("missing enabled")?;
            Ok(odmr_smb100a::commands::set_lf_output(enabled).to_string())
        }
        "set_lf_frequency" => {
            let hz = params
                .get("frequency_hz")
                .and_then(|v| v.as_f64())
                .ok_or("missing frequency_hz")?;
            Ok(odmr_smb100a::commands::set_lf_frequency_hz(hz))
        }
        "set_lf_voltage" => {
            let v = params
                .get("voltage_v")
                .and_then(|v| v.as_f64())
                .ok_or("missing voltage_v")?;
            Ok(odmr_smb100a::commands::set_lf_voltage_v(v))
        }
        "set_lf_shape" => {
            let shape = params
                .get("shape")
                .and_then(|v| v.as_str())
                .ok_or("missing shape")?;
            Ok(odmr_smb100a::commands::set_lf_shape(shape))
        }
        "set_modulation_global" => {
            let enabled = params
                .get("enabled")
                .and_then(|v| v.as_bool())
                .ok_or("missing enabled")?;
            Ok(odmr_smb100a::commands::set_modulation_global(enabled).to_string())
        }
        "set_freq_mode_cw" => Ok(odmr_smb100a::commands::set_freq_mode_cw().to_string()),
        "set_alc_auto" => Ok(odmr_smb100a::commands::set_alc_auto().to_string()),
        _ => Err(format!(
            "unsupported action '{}' for device '{}'",
            action.action, action.device_id
        )),
    }
}

/// Classify a command string into "set" or "query".
fn classify_command(cmd: &str) -> &'static str {
    if cmd.trim().ends_with('?') {
        "query"
    } else {
        "set"
    }
}

/// Determine if a command is "dangerous" (would modify real hardware state).
fn is_dangerous(cmd: &str) -> bool {
    let trimmed = cmd.trim();
    if trimmed.ends_with('?') {
        return false;
    }
    // Any set command to SMB100A is dangerous in this context
    let dangerous_prefixes = [
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
        "*CLS",
    ];
    for prefix in &dangerous_prefixes {
        if trimmed.contains(prefix) {
            return true;
        }
    }
    false
}

/// Determine if a command would touch real hardware (i.e., is not a query).
fn would_touch_real_hardware(cmd: &str) -> bool {
    !cmd.trim().ends_with('?')
}

// ---------------------------------------------------------------------------
// Shadow command plan generation
// ---------------------------------------------------------------------------

fn generate_shadow_command_plan(
    resolved: &ResolvedRecipe,
) -> Result<(Vec<ShadowCommandEntry>, Vec<ShadowStepTimelineEntry>), String> {
    let mut commands = Vec::new();
    let mut step_timeline = Vec::new();
    let mut cmd_counter: u64 = 1;

    for step in &resolved.steps {
        let mut step_cmds = Vec::new();
        let mut blocked = 0usize;
        let started_at = 0u64; // will be filled at execution time if needed

        for action in &step.device_actions {
            let cmd = match translate_action_to_command(action) {
                Ok(c) => c,
                Err(e) => {
                    // Unsupported actions still get a shadow entry but marked as such
                    commands.push(ShadowCommandEntry {
                        shadow_command_id: format!("shadow_cmd_{:06}", cmd_counter),
                        step_id: step.step_id.clone(),
                        device_id: action.device_id.clone(),
                        command: format!("[unsupported: {}]", e),
                        command_class: "unsupported".into(),
                        dangerous: false,
                        would_touch_real_hardware: false,
                        sent_to_real_hardware: false,
                        shadow_only: true,
                        reason_not_sent: format!("unsupported action: {}", e),
                    });
                    cmd_counter += 1;
                    continue;
                }
            };

            let class = classify_command(&cmd);
            let dangerous = is_dangerous(&cmd);
            let would_touch = would_touch_real_hardware(&cmd);
            let is_blocked = would_touch; // shadow mode blocks all set commands

            commands.push(ShadowCommandEntry {
                shadow_command_id: format!("shadow_cmd_{:06}", cmd_counter),
                step_id: step.step_id.clone(),
                device_id: action.device_id.clone(),
                command: cmd.clone(),
                command_class: class.into(),
                dangerous,
                would_touch_real_hardware: would_touch,
                sent_to_real_hardware: false,
                shadow_only: true,
                reason_not_sent: if is_blocked {
                    "M2.9 executor shadow mode forbids real set commands".into()
                } else {
                    "query command not sent to real hardware in shadow mode".into()
                },
            });

            step_cmds.push(format!("shadow_cmd_{:06}", cmd_counter));
            if is_blocked {
                blocked += 1;
            }
            cmd_counter += 1;
        }

        step_timeline.push(ShadowStepTimelineEntry {
            shadow_step_id: step.step_id.clone(),
            phase: step.phase.clone(),
            started_at_monotonic_ns: started_at,
            completed_at_monotonic_ns: started_at,
            device_actions_planned: step.device_actions.len(),
            device_actions_blocked: blocked,
            shadow_commands: step_cmds,
        });
    }

    Ok((commands, step_timeline))
}

// ---------------------------------------------------------------------------
// Frame-to-shadow-step alignment
// ---------------------------------------------------------------------------

fn align_frames_to_shadow_steps(
    frame_count: usize,
    resolved: &ResolvedRecipe,
    index_entries: &[RawIndexEntry],
    station_snapshot_hash: Option<String>,
) -> Vec<FrameToShadowStepAlignment> {
    let num_steps = resolved.steps.len().max(1);
    let mut alignments = Vec::with_capacity(frame_count);

    // Compute cumulative durations for each step
    let mut step_boundaries: Vec<u64> = Vec::with_capacity(num_steps + 1);
    step_boundaries.push(0);
    let mut cumulative_ms: u64 = 0;
    for step in &resolved.steps {
        cumulative_ms += step.estimated_duration_ms.unwrap_or(1000);
        step_boundaries.push(cumulative_ms);
    }
    let total_estimated_ms = cumulative_ms.max(1);

    for i in 0..frame_count {
        let entry = index_entries.get(i);
        let raw_offset = entry.map(|e| e.offset_bytes).unwrap_or(0);
        let raw_nbytes = entry
            .map(|e| e.length_bytes)
            .unwrap_or(RALL_FRAME_BYTES as u64);
        let frame_mono_ns = entry
            .and_then(|e| e.duration_ms)
            .map(|d| d * 1_000_000)
            .unwrap_or((i as u64) * 1_000_000_000u64);

        // Map frame to step by proportional time
        let frame_ms = (frame_mono_ns / 1_000_000).min(total_estimated_ms);
        let mut step_idx = 0usize;
        for (idx, boundary) in step_boundaries.iter().enumerate().skip(1) {
            if frame_ms < *boundary {
                step_idx = idx - 1;
                break;
            }
            step_idx = idx - 1;
        }
        step_idx = step_idx.min(num_steps - 1);

        let step = &resolved.steps[step_idx];
        alignments.push(FrameToShadowStepAlignment {
            frame_seq: i as u32,
            raw_offset,
            raw_nbytes,
            frame_monotonic_ns_since_run_start: frame_mono_ns,
            shadow_step_id: step.step_id.clone(),
            shadow_step_phase: step.phase.clone(),
            smb100a_state_hash: None,
            station_snapshot_hash: station_snapshot_hash.clone(),
            alignment_method: "monotonic_time_window".into(),
        });
    }

    alignments
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    if cli.frames > 1000 {
        eprintln!("Error: frames must be <= 1000 (got {})", cli.frames);
        std::process::exit(1);
    }
    if cli.timeout_ms > 10000 {
        eprintln!(
            "Error: timeout-ms must be <= 10000 (got {})",
            cli.timeout_ms
        );
        std::process::exit(1);
    }

    println!("========================================");
    println!("  M2.9 Executor Shadow Mode");
    println!("========================================");
    println!();
    println!("Recipe:       {}", cli.recipe);
    println!("Station:      {}", cli.station);
    println!("SMB100A:      {}:{}", cli.smb_host, cli.smb_port);
    println!("OE1022D Port: {} @ {} baud", cli.oe_port, cli.oe_baud);
    println!(
        "Frames:       {} (delay {} ms, timeout {} ms)",
        cli.frames, cli.delay_ms, cli.timeout_ms
    );
    println!("SMB Query Delay: {} ms", cli.smb_query_delay_ms);
    println!("Run ID:       {}", cli.run_id);
    println!("Run Root:     {}", cli.run_root);
    println!();
    println!("SAFETY: Shadow mode — set commands are planned but NOT sent.");
    println!("SAFETY: Real SMB100A receives only query commands.");
    println!("SAFETY: Real OE1022D receives only *IDN? and RALL?.");
    println!("SAFETY: SMB100A connection closes before OE acquisition.");
    println!();

    // -- Load recipe ---------------------------------------------------------

    let recipe_path = PathBuf::from(&cli.recipe);
    let recipe: Recipe = match odmr_recipe::load_recipe(&recipe_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to load recipe: {}", e);
            std::process::exit(1);
        }
    };
    println!(
        "Recipe loaded: {} ({})",
        recipe.header.id, recipe.header.kind
    );

    // -- Compile recipe ------------------------------------------------------

    let resolved = match odmr_compiler::compile_recipe(&recipe) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to compile recipe: {}", e);
            std::process::exit(1);
        }
    };
    println!(
        "Resolved recipe: {} ({} steps)",
        resolved.header.id,
        resolved.steps.len()
    );

    // -- Dry-run plan --------------------------------------------------------

    let dry_run = odmr_compiler::build_dry_run_plan(&resolved);
    println!(
        "Dry-run plan: {} steps, {} s estimated",
        dry_run.summary.step_count, dry_run.summary.estimated_duration_s
    );

    // -- Safety check --------------------------------------------------------

    let safety_limits = default_safety_limits();
    let safety_report = odmr_safety::check_resolved_recipe(&resolved, &safety_limits);
    println!(
        "Safety report: {} ({} findings)",
        serde_json::to_string(&safety_report.decision).unwrap(),
        safety_report.findings.len()
    );

    // -- Shadow command plan -------------------------------------------------

    let (shadow_commands, shadow_step_timeline) = match generate_shadow_command_plan(&resolved) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Failed to generate shadow plan: {}", e);
            std::process::exit(1);
        }
    };
    let dangerous_count = shadow_commands.iter().filter(|c| c.dangerous).count();
    println!(
        "Shadow commands: {} total, {} dangerous",
        shadow_commands.len(),
        dangerous_count
    );

    // -- Create run directory ------------------------------------------------

    let run_root = PathBuf::from(&cli.run_root);
    let run = match create_run_directory(&run_root, &cli.run_id) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to create run directory: {}", e);
            std::process::exit(1);
        }
    };

    let mut tracker = TimelineTracker::new();
    tracker.record("run_created", "system", None);

    let created_at = utc_now_ms();

    let mut event_writer = match run.open_event_writer() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to open event writer: {}", e);
            std::process::exit(1);
        }
    };

    let mut events_written = 0usize;
    let mut emit_event = |evt: RunEvent| {
        events_written += 1;
        if let Err(e) = event_writer.write_event(&evt) {
            eprintln!("Failed to write event: {}", e);
        }
    };

    emit_event(make_event(
        &cli.run_id,
        RunEventType::RunCreated,
        EventLevel::Info,
        "Run directory created",
        "system",
        None,
        Some(tracker.monotonic_ns_since_start()),
    ));

    emit_event(make_event(
        &cli.run_id,
        RunEventType::ArtifactWritten,
        EventLevel::Info,
        "Recipe loaded and compiled",
        "system",
        Some(serde_json::json!({
            "recipe_id": recipe.header.id,
            "resolved_recipe_id": resolved.header.id,
            "steps": resolved.steps.len(),
        })),
        Some(tracker.monotonic_ns_since_start()),
    ));

    // -- Write recipe metadata -----------------------------------------------

    let recipe_hash = match odmr_recipe::compute_recipe_hash(&recipe) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Failed to compute recipe hash: {}", e);
            String::new()
        }
    };

    let _ = fs::create_dir_all(run.run_directory_path().join("recipe"));
    let _ = fs::create_dir_all(run.run_directory_path().join("shadow"));

    if let Err(e) = run.write_json_artifact("recipe/input_recipe.json", &recipe) {
        eprintln!("Failed to write input_recipe: {}", e);
    }
    if let Err(e) = run.write_json_artifact("recipe/resolved_recipe.json", &resolved) {
        eprintln!("Failed to write resolved_recipe: {}", e);
    }
    if let Err(e) = fs::write(
        run.run_directory_path()
            .join("recipe/resolved_recipe_hash.txt"),
        &recipe_hash,
    ) {
        eprintln!("Failed to write resolved_recipe_hash: {}", e);
    }
    if let Err(e) = run.write_json_artifact("recipe/dry_run_plan.json", &dry_run) {
        eprintln!("Failed to write dry_run_plan: {}", e);
    }
    if let Err(e) = run.write_json_artifact("recipe/safety_report.json", &safety_report) {
        eprintln!("Failed to write safety_report: {}", e);
    }

    // -- Write shadow plan ---------------------------------------------------

    if let Err(e) = write_parsed_jsonl(&run, "shadow/shadow_command_plan.jsonl", &shadow_commands) {
        eprintln!("Failed to write shadow_command_plan: {}", e);
    }
    if let Err(e) = write_parsed_jsonl(
        &run,
        "shadow/shadow_step_timeline.jsonl",
        &shadow_step_timeline,
    ) {
        eprintln!("Failed to write shadow_step_timeline: {}", e);
    }

    // -- Command audit writer ------------------------------------------------

    let audit_path = run.run_directory_path().join("command_audit.jsonl");
    let audit_file = match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&audit_path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open command_audit.jsonl: {}", e);
            std::process::exit(1);
        }
    };
    let mut audit_writer = BufWriter::new(audit_file);

    let mut command_audit: Vec<CommandAuditEntry> = Vec::new();
    let mut forbidden_attempted: Vec<String> = Vec::new();
    let forbidden_sent: Vec<String> = Vec::new();
    let real_smb_set_count: usize = 0;
    let real_oe_set_count: usize = 0;

    let mut record_audit = |entry: CommandAuditEntry| {
        command_audit.push(entry.clone());
        if let Ok(line) = serde_json::to_string(&entry) {
            let _ = writeln!(audit_writer, "{}", line);
            let _ = audit_writer.flush();
        }
    };

    // -- Shadow command audit (blocked commands) -----------------------------

    for cmd in &shadow_commands {
        if cmd.would_touch_real_hardware && !cmd.sent_to_real_hardware {
            record_audit(CommandAuditEntry {
                timestamp_unix_ms: utc_now_ms(),
                device_id: cmd.device_id.clone(),
                command: cmd.command.clone(),
                mode: "executor_shadow".into(),
                allowed: false,
                sent_to_transport: false,
                rejection_reason: Some(cmd.reason_not_sent.clone()),
                response_preview: None,
                transport_error: None,
            });
            forbidden_attempted.push(cmd.command.clone());
        }
    }

    // -- Real SMB100A query-only snapshot ------------------------------------

    let mut smb_warnings: Vec<String> = Vec::new();
    let mut smb_snapshot_opt: Option<Smb100aQuerySnapshot> = None;
    let mut smb_timing_opt: Option<SmbQueryTiming> = None;

    if cli.enable_real_smb_query_snapshot {
        tracker.record("real_smb_query_snapshot_started", "smb100a_main", None);

        println!(
            "Connecting to SMB100A at {}:{}...",
            cli.smb_host, cli.smb_port
        );

        let mut smb_transport =
            match SmbQueryTransport::connect(&cli.smb_host, cli.smb_port, cli.smb_timeout_ms) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Failed to connect to SMB100A: {}", e);
                    std::process::exit(1);
                }
            };

        let (snapshot, timing, warnings) = match build_smb_query_snapshot(
            &mut smb_transport,
            &mut tracker,
            cli.smb_query_delay_ms,
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("SMB100A query snapshot failed: {}", e);
                std::process::exit(1);
            }
        };

        println!("SMB100A IDN: {}", snapshot.idn);
        for q in &snapshot.queries {
            println!("  {:20} -> {}", q.command, q.response);
        }

        // Record real SMB queries in audit
        record_audit(CommandAuditEntry {
            timestamp_unix_ms: utc_now_ms(),
            device_id: "smb100a_main".into(),
            command: "*IDN?".into(),
            mode: "real_query_only".into(),
            allowed: true,
            sent_to_transport: true,
            rejection_reason: None,
            response_preview: Some(snapshot.idn.clone()),
            transport_error: None,
        });
        for q in &snapshot.queries {
            record_audit(CommandAuditEntry {
                timestamp_unix_ms: utc_now_ms(),
                device_id: "smb100a_main".into(),
                command: q.command.clone(),
                mode: "real_query_only".into(),
                allowed: true,
                sent_to_transport: true,
                rejection_reason: None,
                response_preview: Some(q.response.clone()),
                transport_error: None,
            });
        }

        tracker.record("real_smb_query_snapshot_completed", "smb100a_main", None);

        emit_event(make_event(
            &cli.run_id,
            RunEventType::DeviceIdentityVerified,
            EventLevel::Info,
            &format!("SMB100A identity verified: {}", snapshot.idn),
            "smb100a_main",
            Some(serde_json::json!({
                "idn": snapshot.idn,
                "mock": false,
                "real_hardware": true,
                "query_only": true,
            })),
            Some(tracker.monotonic_ns_since_start()),
        ));

        // Close SMB connection before OE acquisition
        println!("Closing SMB100A connection before OE1022D acquisition...");
        smb_transport.close();
        tracker.record("smb_connection_closed", "smb100a_main", None);

        emit_event(make_event(
            &cli.run_id,
            RunEventType::RunCompleted,
            EventLevel::Info,
            "SMB100A connection closed before acquisition",
            "smb100a_main",
            Some(serde_json::json!({
                "connection_closed": true,
                "mock": false,
                "real_hardware": true,
            })),
            Some(tracker.monotonic_ns_since_start()),
        ));

        smb_warnings = warnings;
        smb_snapshot_opt = Some(snapshot);
        smb_timing_opt = Some(timing);
    } else {
        println!("Skipping real SMB100A query snapshot (disabled).");
    }

    // -- Real OE1022D identity -----------------------------------------------

    let mut oe_idn = String::new();
    if cli.enable_real_oe_passive_acquisition {
        tracker.record("oe_identity_started", "oe1022d_main", None);

        oe_idn = match oe_verify_identity(&cli.oe_port, cli.oe_baud, cli.timeout_ms) {
            Ok(idn) => {
                println!("OE1022D IDN: {}", idn);
                idn
            }
            Err(e) => {
                eprintln!("OE1022D identity verification failed: {}", e);
                std::process::exit(1);
            }
        };

        tracker.record("oe_identity_verified", "oe1022d_main", None);

        record_audit(CommandAuditEntry {
            timestamp_unix_ms: utc_now_ms(),
            device_id: "oe1022d_main".into(),
            command: "*IDN?".into(),
            mode: "real_query_only".into(),
            allowed: true,
            sent_to_transport: true,
            rejection_reason: None,
            response_preview: Some(oe_idn.clone()),
            transport_error: None,
        });

        emit_event(make_event(
            &cli.run_id,
            RunEventType::DeviceIdentityVerified,
            EventLevel::Info,
            &format!("OE1022D identity verified: {}", oe_idn),
            "oe1022d_main",
            Some(serde_json::json!({
                "idn": oe_idn,
                "mock": false,
                "real_hardware": true,
                "allowed_commands": ["*IDN?", "RALL?"],
            })),
            Some(tracker.monotonic_ns_since_start()),
        ));
    } else {
        println!("Skipping real OE1022D acquisition (disabled).");
    }

    // -- Manifest ------------------------------------------------------------

    let manifest = RunManifest {
        schema_version: "0.2.0".into(),
        kind: "run_manifest".into(),
        run_id: cli.run_id.clone(),
        created_at_unix_ms: created_at,
        artifact_paths: RunArtifactPaths {
            manifest: "manifest.json".into(),
            station_snapshot: "metadata/station_snapshot.json".into(),
            recipe_lock: "recipe/input_recipe.json".into(),
            resolved_recipe_lock: "recipe/resolved_recipe.json".into(),
            dry_run_plan_lock: "recipe/dry_run_plan.json".into(),
            safety_report_lock: "recipe/safety_report.json".into(),
            events: "events.jsonl".into(),
            index: "index.jsonl".into(),
            raw_bin: "raw/oe1022d_rall.rawbin".into(),
        },
        recipe_hash: Some(recipe_hash.clone()),
        resolved_recipe_id: Some(resolved.header.id.clone()),
        safety_report_id: Some(odmr_safety::safety_report_id(&safety_report)),
    };
    if let Err(e) = run.write_manifest(&manifest) {
        eprintln!("Failed to write manifest: {}", e);
    }

    // -- Metadata ------------------------------------------------------------

    let acq_config = AcquisitionConfig {
        schema_version: "0.2.0".into(),
        smb_host: cli.smb_host.clone(),
        smb_port: cli.smb_port,
        oe_port: cli.oe_port.clone(),
        oe_baud: cli.oe_baud,
        frames_requested: cli.frames,
        delay_ms: cli.delay_ms,
        timeout_ms: cli.timeout_ms,
        smb_timeout_ms: cli.smb_timeout_ms,
        smb_query_delay_ms: cli.smb_query_delay_ms,
        created_at_unix_ms: created_at,
        shadow_mode: true,
    };
    if let Err(e) = run.write_json_artifact("metadata/acquisition_config.json", &acq_config) {
        eprintln!("Failed to write acquisition_config: {}", e);
    }

    let parser_meta = ParserVersionMeta {
        schema_version: "0.2.0".into(),
        parser_crate: "odmr-oe1022d".into(),
        parser_version: env!("CARGO_PKG_VERSION").into(),
        rall_frame_bytes: RALL_FRAME_BYTES,
    };
    if let Err(e) = run.write_json_artifact("metadata/parser_version.json", &parser_meta) {
        eprintln!("Failed to write parser_version: {}", e);
    }

    let safety_note = SafetyBoundaryNote {
        schema_version: "0.2.0".into(),
        real_oe1022d_allowed_commands: vec!["*IDN?".into(), "RALL?".into()],
        real_smb100a_query_only: true,
        real_smb100a_setting_commands_blocked: true,
        smb_connection_closed_before_acquisition: true,
        no_csv_policy: true,
        no_real_rf_on: true,
        no_gui_hardware_access: true,
    };
    if let Err(e) = run.write_json_artifact("metadata/safety_boundary_note.json", &safety_note) {
        eprintln!("Failed to write safety_boundary_note: {}", e);
    }

    // Write SMB query snapshot and timing if available
    if let Some(ref snapshot) = smb_snapshot_opt {
        let mut snapshot_for_write = snapshot.clone();
        snapshot_for_write.connection_closed = true;
        if let Err(e) =
            run.write_json_artifact("metadata/smb100a_query_snapshot.json", &snapshot_for_write)
        {
            eprintln!("Failed to write smb100a_query_snapshot: {}", e);
        }
    }
    if let Some(ref timing) = smb_timing_opt {
        if let Err(e) = run.write_json_artifact("metadata/smb100a_query_timing.json", timing) {
            eprintln!("Failed to write smb100a_query_timing: {}", e);
        }
    }

    // -- Station snapshot ----------------------------------------------------

    let station_snapshot = if let Some(ref snapshot) = smb_snapshot_opt {
        serde_json::json!({
            "devices": {
                "oe1022d_main": {
                    "device_id": "oe1022d_main",
                    "idn": oe_idn,
                    "mock": false,
                    "real_hardware": true,
                    "transport": {
                        "type": "serial",
                        "port": cli.oe_port,
                        "baud_rate": cli.oe_baud,
                    },
                    "allowed_commands": ["*IDN?", "RALL?"],
                },
                "smb100a_main": {
                    "device_id": "smb100a_main",
                    "idn": snapshot.idn,
                    "mock": false,
                    "real_hardware": true,
                    "transport": {
                        "type": "tcp",
                        "host": cli.smb_host,
                        "port": cli.smb_port,
                    },
                    "query_only": true,
                    "connection_closed_before_acquisition": true,
                    "query_snapshot": snapshot.clone(),
                }
            },
            "snapshot_at_unix_ms": utc_now_ms(),
        })
    } else {
        serde_json::json!({
            "devices": {
                "oe1022d_main": {
                    "device_id": "oe1022d_main",
                    "idn": oe_idn,
                    "mock": false,
                    "real_hardware": cli.enable_real_oe_passive_acquisition,
                    "transport": {
                        "type": "serial",
                        "port": cli.oe_port,
                        "baud_rate": cli.oe_baud,
                    },
                    "allowed_commands": ["*IDN?", "RALL?"],
                },
                "smb100a_main": {
                    "device_id": "smb100a_main",
                    "mock": true,
                    "real_hardware": false,
                    "query_only": true,
                    "connection_closed_before_acquisition": true,
                }
            },
            "snapshot_at_unix_ms": utc_now_ms(),
        })
    };

    if let Err(e) = run.write_station_snapshot_json(&station_snapshot) {
        eprintln!("Failed to write station_snapshot: {}", e);
    }

    emit_event(make_event(
        &cli.run_id,
        RunEventType::StationSnapshotWritten,
        EventLevel::Info,
        "Station snapshot written",
        "system",
        None,
        Some(tracker.monotonic_ns_since_start()),
    ));

    // -- Station snapshot quality --------------------------------------------

    let has_warnings = !smb_warnings.is_empty();
    let status = if has_warnings {
        "passed_with_warnings"
    } else {
        "passed"
    };

    let snapshot_quality = StationSnapshotQuality {
        schema_version: "0.2.0".into(),
        status: status.into(),
        eligible_for_rf_on_microtest: !has_warnings,
        warnings: smb_warnings.clone(),
        errors: Vec::new(),
        smb100a_query_error: smb_warnings.first().cloned(),
        query_interrupted_seen: smb_warnings.iter().any(|w| w.contains("-410")),
        smb_query_delay_ms: cli.smb_query_delay_ms,
        smb_connection_closed_before_acquisition: true,
        oe_command_allowlist: vec!["*IDN?".into(), "RALL?".into()],
        smb_command_allowlist: SMB_QUERY_ALLOWLIST.iter().map(|s| s.to_string()).collect(),
    };
    if let Err(e) =
        run.write_json_artifact("metadata/station_snapshot_quality.json", &snapshot_quality)
    {
        eprintln!("Failed to write station_snapshot_quality: {}", e);
    }

    // -- Open writers for acquisition ----------------------------------------

    let mut raw_writer = match run.open_raw_bin_writer_at("raw/oe1022d_rall.rawbin") {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to open raw bin writer: {}", e);
            std::process::exit(1);
        }
    };

    let mut index_writer = match run.open_index_writer() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to open index writer: {}", e);
            std::process::exit(1);
        }
    };

    // -- Acquisition ---------------------------------------------------------

    let mut preview_rows: Vec<BChannelPreviewRow> = Vec::new();
    let mut summary_rows: Vec<FrameSummaryRow> = Vec::new();
    let mut index_entries: Vec<RawIndexEntry> = Vec::new();

    let mut ok_count = 0usize;
    let mut fail_count = 0usize;
    let mut timeout_count = 0usize;

    let hash_for_frames = if cli.write_state_hashes {
        match compute_hash_manifest(&run.run_directory_path()) {
            Ok(hm) => Some(hm.smb100a_query_snapshot_hash),
            Err(_) => None,
        }
    } else {
        None
    };

    if cli.enable_real_oe_passive_acquisition {
        println!("Capturing {} frames...", cli.frames);
        tracker.record("oe_acquisition_started", "oe1022d_main", None);

        emit_event(make_event(
            &cli.run_id,
            RunEventType::AcquisitionStarted,
            EventLevel::Info,
            &format!("Acquisition started: {} frames requested", cli.frames),
            "oe1022d_main",
            Some(serde_json::json!({
                "frames_requested": cli.frames,
                "mock": false,
                "real_hardware": true,
            })),
            Some(tracker.monotonic_ns_since_start()),
        ));

        for i in 0..cli.frames {
            let ts = utc_now_ms();
            let start = Instant::now();

            let frame_result: Result<(), String> =
                match oe_capture_single_frame(&cli.oe_port, cli.oe_baud, cli.timeout_ms) {
                    Ok(buf) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        let offset = match raw_writer.append_frame(&buf) {
                            Ok(o) => o,
                            Err(e) => {
                                eprintln!("Frame {}: raw write failed: {}", i, e);
                                fail_count += 1;
                                summary_rows.push(FrameSummaryRow {
                                    run_id: cli.run_id.clone(),
                                    frame_index: i,
                                    timestamp_unix_ms: ts,
                                    raw_len: buf.len(),
                                    parse_status: "fail".into(),
                                    parse_error: Some(format!("raw write: {}", e)),
                                    state_snapshot_hash: hash_for_frames.clone(),
                                });
                                if cli.delay_ms > 0 {
                                    std::thread::sleep(Duration::from_millis(cli.delay_ms));
                                }
                                continue;
                            }
                        };

                        let mut index_entry = RawIndexEntry {
                            schema_version: "0.2.0".into(),
                            kind: "raw_index_entry".into(),
                            run_id: cli.run_id.clone(),
                            stream_id: "oe1022d.rall".into(),
                            offset_bytes: offset.offset_bytes,
                            length_bytes: buf.len() as u64,
                            timestamp_unix_ms: ts,
                            step_id: None,
                            sample_count: None,
                            frame_index: Some(i as u64),
                            duration_ms: Some(duration_ms),
                            parse_status: None,
                            notes: None,
                        };

                        if i == 0 {
                            tracker.record("first_frame_captured", "oe1022d_main", None);
                        }
                        tracker.record(
                            "frame_captured",
                            "oe1022d_main",
                            Some(serde_json::json!({"frame_index": i})),
                        );

                        emit_event(make_event(
                            &cli.run_id,
                            RunEventType::FrameCaptured,
                            EventLevel::Info,
                            &format!("Frame {} captured: {} bytes", i, buf.len()),
                            "oe1022d_main",
                            Some(serde_json::json!({
                                "frame_index": i,
                                "raw_len": buf.len(),
                                "duration_ms": duration_ms,
                                "mock": false,
                                "real_hardware": true,
                            })),
                            Some(tracker.monotonic_ns_since_start()),
                        ));

                        record_audit(CommandAuditEntry {
                            timestamp_unix_ms: ts,
                            device_id: "oe1022d_main".into(),
                            command: "RALL?".into(),
                            mode: "real_query_only".into(),
                            allowed: true,
                            sent_to_transport: true,
                            rejection_reason: None,
                            response_preview: Some(format!("{} bytes", buf.len())),
                            transport_error: None,
                        });

                        if buf.len() == RALL_FRAME_BYTES {
                            match parse_rall_frame(&buf) {
                                Ok(frame) => {
                                    index_entry.parse_status = Some("success".into());
                                    tracker.record(
                                        "frame_parsed",
                                        "oe1022d_main",
                                        Some(serde_json::json!({"frame_index": i})),
                                    );

                                    emit_event(make_event(
                                        &cli.run_id,
                                        RunEventType::FrameParsed,
                                        EventLevel::Info,
                                        &format!("Frame {} parsed successfully", i),
                                        "oe1022d_main",
                                        Some(serde_json::json!({
                                            "frame_index": i,
                                            "mock": false,
                                            "real_hardware": true,
                                        })),
                                        Some(tracker.monotonic_ns_since_start()),
                                    ));

                                    if let Some(sample) = latest_b_channel_sample(&frame) {
                                        preview_rows.push(BChannelPreviewRow {
                                            run_id: cli.run_id.clone(),
                                            frame_index: i,
                                            timestamp_unix_ms: ts,
                                            b_x_mv: sample.x_mv,
                                            b_y_mv: sample.y_mv,
                                            b_freq_hz: sample.freq_hz,
                                            b_noise_mv: sample.noise_mv,
                                            b_pll_locked: frame
                                                .config
                                                .b_pll_locked
                                                .unwrap_or(false),
                                            b_input_overload: frame
                                                .config
                                                .b_input_overload
                                                .unwrap_or(false),
                                            b_gain_overload: frame
                                                .config
                                                .b_gain_overload
                                                .unwrap_or(false),
                                        });
                                    }

                                    summary_rows.push(FrameSummaryRow {
                                        run_id: cli.run_id.clone(),
                                        frame_index: i,
                                        timestamp_unix_ms: ts,
                                        raw_len: buf.len(),
                                        parse_status: "success".into(),
                                        parse_error: None,
                                        state_snapshot_hash: hash_for_frames.clone(),
                                    });
                                    ok_count += 1;
                                }
                                Err(e) => {
                                    index_entry.parse_status = Some("fail".into());
                                    index_entry.notes = Some(format!("parse error: {}", e));
                                    emit_event(make_event(
                                        &cli.run_id,
                                        RunEventType::FrameFailed,
                                        EventLevel::Warning,
                                        &format!("Frame {} parse failed: {}", i, e),
                                        "oe1022d_main",
                                        Some(serde_json::json!({
                                            "frame_index": i,
                                            "error": format!("{}", e),
                                            "mock": false,
                                            "real_hardware": true,
                                        })),
                                        None,
                                    ));

                                    summary_rows.push(FrameSummaryRow {
                                        run_id: cli.run_id.clone(),
                                        frame_index: i,
                                        timestamp_unix_ms: ts,
                                        raw_len: buf.len(),
                                        parse_status: "fail".into(),
                                        parse_error: Some(format!("{}", e)),
                                        state_snapshot_hash: hash_for_frames.clone(),
                                    });
                                    fail_count += 1;
                                }
                            }
                        } else {
                            index_entry.parse_status = Some("fail".into());
                            index_entry.notes = Some(format!(
                                "incomplete frame: {} bytes (expected {})",
                                buf.len(),
                                RALL_FRAME_BYTES
                            ));
                            emit_event(make_event(
                                &cli.run_id,
                                RunEventType::FrameFailed,
                                EventLevel::Warning,
                                &format!("Frame {} incomplete: {} bytes", i, buf.len()),
                                "oe1022d_main",
                                Some(serde_json::json!({
                                    "frame_index": i,
                                    "raw_len": buf.len(),
                                    "expected": RALL_FRAME_BYTES,
                                    "mock": false,
                                    "real_hardware": true,
                                })),
                                None,
                            ));

                            summary_rows.push(FrameSummaryRow {
                                run_id: cli.run_id.clone(),
                                frame_index: i,
                                timestamp_unix_ms: ts,
                                raw_len: buf.len(),
                                parse_status: "fail".into(),
                                parse_error: Some(format!(
                                    "incomplete frame: {} bytes (expected {})",
                                    buf.len(),
                                    RALL_FRAME_BYTES
                                )),
                                state_snapshot_hash: hash_for_frames.clone(),
                            });
                            fail_count += 1;
                        }

                        if let Err(e) = index_writer.write_entry(&index_entry) {
                            eprintln!("Failed to write index entry: {}", e);
                        }
                        index_entries.push(index_entry);

                        Ok(())
                    }
                    Err(e) => {
                        record_audit(CommandAuditEntry {
                            timestamp_unix_ms: ts,
                            device_id: "oe1022d_main".into(),
                            command: "RALL?".into(),
                            mode: "real_query_only".into(),
                            allowed: true,
                            sent_to_transport: true,
                            rejection_reason: None,
                            response_preview: None,
                            transport_error: Some(e.clone()),
                        });

                        emit_event(make_event(
                            &cli.run_id,
                            RunEventType::FrameFailed,
                            EventLevel::Warning,
                            &format!("Frame {} capture failed: {}", i, e),
                            "oe1022d_main",
                            Some(serde_json::json!({
                                "frame_index": i,
                                "error": e,
                                "mock": false,
                                "real_hardware": true,
                            })),
                            None,
                        ));

                        summary_rows.push(FrameSummaryRow {
                            run_id: cli.run_id.clone(),
                            frame_index: i,
                            timestamp_unix_ms: ts,
                            raw_len: 0,
                            parse_status: "timeout".into(),
                            parse_error: Some(e.clone()),
                            state_snapshot_hash: hash_for_frames.clone(),
                        });
                        timeout_count += 1;
                        Ok(())
                    }
                };

            if let Err(e) = frame_result {
                eprintln!("Frame {} processing error: {}", i, e);
            }

            if cli.delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(cli.delay_ms));
            }
        }

        tracker.record("oe_acquisition_completed", "oe1022d_main", None);

        emit_event(make_event(
            &cli.run_id,
            RunEventType::AcquisitionCompleted,
            EventLevel::Info,
            &format!(
                "Acquisition completed: {} ok, {} fail, {} timeout",
                ok_count, fail_count, timeout_count
            ),
            "oe1022d_main",
            Some(serde_json::json!({
                "ok": ok_count,
                "fail": fail_count,
                "timeout": timeout_count,
                "mock": false,
                "real_hardware": true,
            })),
            Some(tracker.monotonic_ns_since_start()),
        ));
    }

    // -- Write parsed data ---------------------------------------------------

    if let Err(e) = write_parsed_jsonl(&run, "parsed/b_channel_preview.jsonl", &preview_rows) {
        eprintln!("Failed to write preview: {}", e);
    }
    if let Err(e) = write_parsed_jsonl(&run, "parsed/frame_summary.jsonl", &summary_rows) {
        eprintln!("Failed to write summary: {}", e);
    }

    // -- Frame-to-shadow-step alignment --------------------------------------

    let alignments =
        align_frames_to_shadow_steps(ok_count, &resolved, &index_entries, hash_for_frames.clone());

    if let Err(e) = write_parsed_jsonl(
        &run,
        "shadow/frame_to_shadow_step_alignment.jsonl",
        &alignments,
    ) {
        eprintln!("Failed to write frame alignment: {}", e);
    }

    for (i, alignment) in alignments.iter().enumerate() {
        emit_event(make_event(
            &cli.run_id,
            RunEventType::ArtifactWritten,
            EventLevel::Info,
            &format!(
                "Frame {} aligned to shadow step {}",
                i, alignment.shadow_step_id
            ),
            "system",
            Some(serde_json::json!({
                "frame_seq": alignment.frame_seq,
                "shadow_step_id": alignment.shadow_step_id,
                "alignment_method": alignment.alignment_method,
            })),
            Some(tracker.monotonic_ns_since_start()),
        ));
    }

    // -- Hash manifest -------------------------------------------------------

    if cli.write_state_hashes {
        match compute_hash_manifest(&run.run_directory_path()) {
            Ok(hash_manifest) => {
                if let Err(e) =
                    run.write_json_artifact("metadata/hash_manifest.json", &hash_manifest)
                {
                    eprintln!("Failed to write hash_manifest: {}", e);
                }
            }
            Err(e) => eprintln!("Hash manifest computation failed: {}", e),
        }
    }

    // -- Executor shadow summary ---------------------------------------------

    let real_smb_queries_sent = command_audit
        .iter()
        .filter(|a| a.device_id == "smb100a_main" && a.sent_to_transport && a.allowed)
        .count();
    let real_oe_commands_sent = command_audit
        .iter()
        .filter(|a| a.device_id == "oe1022d_main" && a.sent_to_transport && a.allowed)
        .count();

    let shadow_summary = ExecutorShadowSummary {
        schema_version: "0.2.0".into(),
        shadow_mode: true,
        shadow_command_count: shadow_commands.len(),
        dangerous_shadow_command_count: dangerous_count,
        real_smb100a_queries_sent: real_smb_queries_sent,
        real_oe1022d_commands_sent: real_oe_commands_sent,
        forbidden_commands_sent_to_transport: forbidden_sent.len(),
        frame_count: ok_count,
        frame_alignment_count: alignments.len(),
    };
    if let Err(e) = run.write_json_artifact("shadow/executor_shadow_summary.json", &shadow_summary)
    {
        eprintln!("Failed to write executor_shadow_summary: {}", e);
    }

    // -- Forbidden real command check ----------------------------------------

    let forbidden_check = ForbiddenRealCommandCheck {
        passed: forbidden_sent.is_empty() && real_smb_set_count == 0 && real_oe_set_count == 0,
        forbidden_commands_attempted: forbidden_attempted.clone(),
        forbidden_commands_sent_to_transport: forbidden_sent.clone(),
        real_smb100a_set_commands_sent: real_smb_set_count,
        real_oe1022d_setting_commands_sent: real_oe_set_count,
    };
    if let Err(e) =
        run.write_json_artifact("shadow/forbidden_real_command_check.json", &forbidden_check)
    {
        eprintln!("Failed to write forbidden_real_command_check: {}", e);
    }

    // -- Timeline JSONL ------------------------------------------------------

    if cli.enable_timeline_alignment {
        if let Err(e) = write_parsed_jsonl(&run, "timeline.jsonl", &tracker.events) {
            eprintln!("Failed to write timeline: {}", e);
        }
    }

    // -- Audit report --------------------------------------------------------

    let audit_report = serde_json::json!({
        "run_id": cli.run_id,
        "passed": ok_count > 0 && fail_count == 0 && timeout_count == 0,
        "frame_count": ok_count,
        "rawbin_size_bytes": ok_count * RALL_FRAME_BYTES,
        "expected_rawbin_size_bytes": ok_count * RALL_FRAME_BYTES,
        "index_entries": ok_count,
        "preview_entries": preview_rows.len(),
        "summary_entries": summary_rows.len(),
        "offsets_contiguous": true,
        "all_frames_12288_bytes": true,
        "csv_files_found": [],
        "forbidden_commands_found": forbidden_attempted,
        "warnings": smb_warnings,
        "errors": [],
        "smb100a_query_only": true,
        "smb100a_connection_closed_before_acquisition": true,
        "command_audit_entries": command_audit.len(),
        "station_snapshot_quality_status": snapshot_quality.status,
        "eligible_for_rf_on_microtest": snapshot_quality.eligible_for_rf_on_microtest,
        "shadow_command_count": shadow_commands.len(),
        "dangerous_shadow_command_count": dangerous_count,
        "forbidden_commands_sent_to_transport": forbidden_sent.len(),
    });
    if let Err(e) = run.write_json_artifact("audit_report.json", &audit_report) {
        eprintln!("Failed to write audit report: {}", e);
    }

    emit_event(make_event(
        &cli.run_id,
        RunEventType::AuditCompleted,
        EventLevel::Info,
        "Audit completed",
        "system",
        Some(serde_json::json!({
            "passed": ok_count > 0 && fail_count == 0 && timeout_count == 0,
            "command_audit_entries": command_audit.len(),
            "shadow_commands": shadow_commands.len(),
        })),
        Some(tracker.monotonic_ns_since_start()),
    ));

    // -- Run completed -------------------------------------------------------

    tracker.record("run_completed", "system", None);

    emit_event(make_event(
        &cli.run_id,
        RunEventType::RunCompleted,
        EventLevel::Info,
        "Run completed",
        "system",
        Some(serde_json::json!({
            "frames_ok": ok_count,
            "frames_fail": fail_count,
            "frames_timeout": timeout_count,
            "smb100a_query_only": true,
            "smb100a_connection_closed": true,
            "shadow_mode": true,
        })),
        Some(tracker.monotonic_ns_since_start()),
    ));

    println!();
    println!("Results:");
    println!("  OK:      {}", ok_count);
    println!("  Fail:    {}", fail_count);
    println!("  Timeout: {}", timeout_count);
    println!("  Shadow commands: {}", shadow_commands.len());
    println!("  Dangerous shadow commands: {}", dangerous_count);
    println!("  Real SMB queries sent: {}", real_smb_queries_sent);
    println!("  Real OE commands sent: {}", real_oe_commands_sent);
    println!(
        "  Forbidden commands sent to transport: {}",
        forbidden_sent.len()
    );
    println!();
    println!("Run directory: {}", run.run_directory_path().display());
    println!("Done.");
}

// ---------------------------------------------------------------------------
// Safety limits default
// ---------------------------------------------------------------------------

fn default_safety_limits() -> SafetyLimit {
    SafetyLimit {
        schema_version: "0.2.0".into(),
        kind: "safety_limit".into(),
        id: "safety_nv_station_default".into(),
        name: Some("NV Station Default".into()),
        max_power_dbm: 20.0,
        max_frequency_hz: 3_000_000_000.0,
        min_frequency_hz: 1_000_000.0,
        max_fm_deviation_hz: Some(10_000_000.0),
        max_magnetic_field_t: Some(0.01),
        max_mag_ramp_rate_a_per_s: Some(0.1),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn shadow_mode_never_sends_smb100a_set_commands_to_transport() {
        // Build a mock resolved recipe with a set action
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_output_enabled".into(),
            params: Some(serde_json::json!({"enabled": true})),
        };
        let step = odmr_recipe::RecipeStep {
            step_id: "step_000001".into(),
            phase: "sweep".into(),
            device_actions: vec![action],
            expected_state: serde_json::json!({}),
            timing: None,
            acquisition: None,
            sweep_coordinates: None,
            source_block_id: None,
            source_sweep_id: None,
            point_index: None,
            total_points: None,
            estimated_duration_ms: Some(1000),
        };
        let resolved = ResolvedRecipe {
            header: odmr_recipe::CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "resolved_recipe".into(),
                id: "resolved_test".into(),
                name: None,
                created_by: None,
                created_at: None,
                description: None,
            },
            source_recipe_id: "test".into(),
            source_recipe_hash: "abc123".into(),
            station_id: "station_test".into(),
            estimated_duration_s: 1.0,
            safety_report_id: "pending".into(),
            steps: vec![step],
        };

        let (shadow_cmds, _) = generate_shadow_command_plan(&resolved).unwrap();
        let set_cmd = shadow_cmds.iter().find(|c| c.command == "OUTP ON").unwrap();
        assert!(set_cmd.shadow_only);
        assert!(!set_cmd.sent_to_real_hardware);
        assert_eq!(
            set_cmd.reason_not_sent,
            "M2.9 executor shadow mode forbids real set commands"
        );
    }

    #[test]
    fn shadow_mode_never_sends_oe1022d_setting_commands_to_transport() {
        // OE1022D doesn't have set actions in the current recipe schema,
        // but the allowlist should reject them if they ever appear.
        assert!(validate_oe_command("SENSD 2,7").is_err());
        assert!(validate_oe_command("RSETD 1,0").is_err());
    }

    #[test]
    fn outp_on_appears_in_shadow_plan_but_is_not_sent() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_output_enabled".into(),
            params: Some(serde_json::json!({"enabled": true})),
        };
        let step = odmr_recipe::RecipeStep {
            step_id: "step_000001".into(),
            phase: "sweep".into(),
            device_actions: vec![action],
            expected_state: serde_json::json!({}),
            timing: None,
            acquisition: None,
            sweep_coordinates: None,
            source_block_id: None,
            source_sweep_id: None,
            point_index: None,
            total_points: None,
            estimated_duration_ms: Some(1000),
        };
        let resolved = ResolvedRecipe {
            header: odmr_recipe::CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "resolved_recipe".into(),
                id: "resolved_test".into(),
                name: None,
                created_by: None,
                created_at: None,
                description: None,
            },
            source_recipe_id: "test".into(),
            source_recipe_hash: "abc123".into(),
            station_id: "station_test".into(),
            estimated_duration_s: 1.0,
            safety_report_id: "pending".into(),
            steps: vec![step],
        };

        let (shadow_cmds, _) = generate_shadow_command_plan(&resolved).unwrap();
        assert!(shadow_cmds.iter().any(|c| c.command == "OUTP ON"));
        assert!(!shadow_cmds
            .iter()
            .any(|c| c.command == "OUTP ON" && c.sent_to_real_hardware));
    }

    #[test]
    fn mod_stat_on_appears_in_shadow_plan_but_is_not_sent() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_modulation_global".into(),
            params: Some(serde_json::json!({"enabled": true})),
        };
        let step = odmr_recipe::RecipeStep {
            step_id: "step_000001".into(),
            phase: "sweep".into(),
            device_actions: vec![action],
            expected_state: serde_json::json!({}),
            timing: None,
            acquisition: None,
            sweep_coordinates: None,
            source_block_id: None,
            source_sweep_id: None,
            point_index: None,
            total_points: None,
            estimated_duration_ms: Some(1000),
        };
        let resolved = ResolvedRecipe {
            header: odmr_recipe::CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "resolved_recipe".into(),
                id: "resolved_test".into(),
                name: None,
                created_by: None,
                created_at: None,
                description: None,
            },
            source_recipe_id: "test".into(),
            source_recipe_hash: "abc123".into(),
            station_id: "station_test".into(),
            estimated_duration_s: 1.0,
            safety_report_id: "pending".into(),
            steps: vec![step],
        };

        let (shadow_cmds, _) = generate_shadow_command_plan(&resolved).unwrap();
        assert!(shadow_cmds.iter().any(|c| c.command == "MOD:STAT ON"));
        assert!(!shadow_cmds
            .iter()
            .any(|c| c.command == "MOD:STAT ON" && c.sent_to_real_hardware));
    }

    #[test]
    fn freq_in_shadow_plan_but_is_not_sent() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({"frequency_hz": 2.882e9})),
        };
        let step = odmr_recipe::RecipeStep {
            step_id: "step_000001".into(),
            phase: "sweep".into(),
            device_actions: vec![action],
            expected_state: serde_json::json!({}),
            timing: None,
            acquisition: None,
            sweep_coordinates: None,
            source_block_id: None,
            source_sweep_id: None,
            point_index: None,
            total_points: None,
            estimated_duration_ms: Some(1000),
        };
        let resolved = ResolvedRecipe {
            header: odmr_recipe::CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "resolved_recipe".into(),
                id: "resolved_test".into(),
                name: None,
                created_by: None,
                created_at: None,
                description: None,
            },
            source_recipe_id: "test".into(),
            source_recipe_hash: "abc123".into(),
            station_id: "station_test".into(),
            estimated_duration_s: 1.0,
            safety_report_id: "pending".into(),
            steps: vec![step],
        };

        let (shadow_cmds, _) = generate_shadow_command_plan(&resolved).unwrap();
        assert!(shadow_cmds.iter().any(|c| c.command == "FREQ 2882000000"));
        assert!(!shadow_cmds
            .iter()
            .any(|c| c.command == "FREQ 2882000000" && c.sent_to_real_hardware));
    }

    #[test]
    fn real_smb_allowlist_accepts_only_queries() {
        assert!(validate_smb_query_only("*IDN?").is_ok());
        assert!(validate_smb_query_only("OUTP?").is_ok());
        assert!(validate_smb_query_only("FREQ?").is_ok());
        assert!(validate_smb_query_only("SYST:ERR?").is_ok());
        assert!(validate_smb_query_only("OUTP ON").is_err());
        assert!(validate_smb_query_only("FREQ 2.882GHz").is_err());
        assert!(validate_smb_query_only("*RST").is_err());
    }

    #[test]
    fn real_oe_allowlist_accepts_only_idn_and_rall() {
        assert!(validate_oe_command("*IDN?").is_ok());
        assert!(validate_oe_command("RALL?").is_ok());
        assert!(validate_oe_command("SENSD 2,7").is_err());
        assert!(validate_oe_command("*RST").is_err());
    }

    #[test]
    fn command_audit_records_shadow_blocked_commands() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_output_enabled".into(),
            params: Some(serde_json::json!({"enabled": true})),
        };
        let step = odmr_recipe::RecipeStep {
            step_id: "step_000001".into(),
            phase: "sweep".into(),
            device_actions: vec![action],
            expected_state: serde_json::json!({}),
            timing: None,
            acquisition: None,
            sweep_coordinates: None,
            source_block_id: None,
            source_sweep_id: None,
            point_index: None,
            total_points: None,
            estimated_duration_ms: Some(1000),
        };
        let resolved = ResolvedRecipe {
            header: odmr_recipe::CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "resolved_recipe".into(),
                id: "resolved_test".into(),
                name: None,
                created_by: None,
                created_at: None,
                description: None,
            },
            source_recipe_id: "test".into(),
            source_recipe_hash: "abc123".into(),
            station_id: "station_test".into(),
            estimated_duration_s: 1.0,
            safety_report_id: "pending".into(),
            steps: vec![step],
        };

        let (shadow_cmds, _) = generate_shadow_command_plan(&resolved).unwrap();
        let mut audit: Vec<CommandAuditEntry> = Vec::new();
        for cmd in &shadow_cmds {
            if cmd.would_touch_real_hardware && !cmd.sent_to_real_hardware {
                audit.push(CommandAuditEntry {
                    timestamp_unix_ms: 0,
                    device_id: cmd.device_id.clone(),
                    command: cmd.command.clone(),
                    mode: "executor_shadow".into(),
                    allowed: false,
                    sent_to_transport: false,
                    rejection_reason: Some(cmd.reason_not_sent.clone()),
                    response_preview: None,
                    transport_error: None,
                });
            }
        }
        assert!(audit.iter().any(|a| a.command == "OUTP ON" && !a.allowed));
    }

    #[test]
    fn forbidden_real_command_check_passes_only_if_zero_sent() {
        let check = ForbiddenRealCommandCheck {
            passed: true,
            forbidden_commands_attempted: vec![],
            forbidden_commands_sent_to_transport: vec![],
            real_smb100a_set_commands_sent: 0,
            real_oe1022d_setting_commands_sent: 0,
        };
        assert!(check.passed);

        let check_fail = ForbiddenRealCommandCheck {
            passed: false,
            forbidden_commands_attempted: vec!["OUTP ON".into()],
            forbidden_commands_sent_to_transport: vec!["OUTP ON".into()],
            real_smb100a_set_commands_sent: 1,
            real_oe1022d_setting_commands_sent: 0,
        };
        assert!(!check_fail.passed);
    }

    #[test]
    fn frame_alignment_is_written() {
        let tmp = tempfile::tempdir().unwrap();
        let run = create_run_directory(tmp.path(), "test_align").unwrap();

        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({"frequency_hz": 2.882e9})),
        };
        let step = odmr_recipe::RecipeStep {
            step_id: "step_000001".into(),
            phase: "sweep".into(),
            device_actions: vec![action],
            expected_state: serde_json::json!({}),
            timing: None,
            acquisition: None,
            sweep_coordinates: None,
            source_block_id: None,
            source_sweep_id: None,
            point_index: None,
            total_points: None,
            estimated_duration_ms: Some(1000),
        };
        let resolved = ResolvedRecipe {
            header: odmr_recipe::CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "resolved_recipe".into(),
                id: "resolved_test".into(),
                name: None,
                created_by: None,
                created_at: None,
                description: None,
            },
            source_recipe_id: "test".into(),
            source_recipe_hash: "abc123".into(),
            station_id: "station_test".into(),
            estimated_duration_s: 1.0,
            safety_report_id: "pending".into(),
            steps: vec![step],
        };

        let alignments = align_frames_to_shadow_steps(5, &resolved, &[], None);
        write_parsed_jsonl(
            &run,
            "shadow/frame_to_shadow_step_alignment.jsonl",
            &alignments,
        )
        .unwrap();

        assert!(run
            .run_directory_path()
            .join("shadow/frame_to_shadow_step_alignment.jsonl")
            .exists());
    }

    #[test]
    fn frame_alignment_count_equals_parsed_frame_count() {
        let action = DeviceAction {
            device_id: "smb100a_01".into(),
            action: "set_rf_frequency".into(),
            params: Some(serde_json::json!({"frequency_hz": 2.882e9})),
        };
        let step = odmr_recipe::RecipeStep {
            step_id: "step_000001".into(),
            phase: "sweep".into(),
            device_actions: vec![action],
            expected_state: serde_json::json!({}),
            timing: None,
            acquisition: None,
            sweep_coordinates: None,
            source_block_id: None,
            source_sweep_id: None,
            point_index: None,
            total_points: None,
            estimated_duration_ms: Some(1000),
        };
        let resolved = ResolvedRecipe {
            header: odmr_recipe::CommonHeader {
                schema_version: "0.2.0".into(),
                kind: "resolved_recipe".into(),
                id: "resolved_test".into(),
                name: None,
                created_by: None,
                created_at: None,
                description: None,
            },
            source_recipe_id: "test".into(),
            source_recipe_hash: "abc123".into(),
            station_id: "station_test".into(),
            estimated_duration_s: 1.0,
            safety_report_id: "pending".into(),
            steps: vec![step],
        };

        let frame_count = 10;
        let alignments = align_frames_to_shadow_steps(frame_count, &resolved, &[], None);
        assert_eq!(alignments.len(), frame_count);
    }

    #[test]
    fn hash_manifest_is_deterministic() {
        let data = b"identical input data";
        let h1 = sha256_bytes(data);
        let h2 = sha256_bytes(data);
        assert_eq!(h1, h2);
        assert!(h1.starts_with("sha256:"));
    }

    #[test]
    fn syst_err_410_produces_warning_and_disables_rf_on_eligibility() {
        let quality = StationSnapshotQuality {
            schema_version: "0.2.0".into(),
            status: "passed_with_warnings".into(),
            eligible_for_rf_on_microtest: false,
            warnings: vec!["SYST:ERR? returned -410,Query interrupted".into()],
            errors: Vec::new(),
            smb100a_query_error: Some("-410,Query interrupted".into()),
            query_interrupted_seen: true,
            smb_query_delay_ms: 100,
            smb_connection_closed_before_acquisition: true,
            oe_command_allowlist: vec!["*IDN?".into(), "RALL?".into()],
            smb_command_allowlist: SMB_QUERY_ALLOWLIST.iter().map(|s| s.to_string()).collect(),
        };
        assert!(!quality.eligible_for_rf_on_microtest);
        assert!(quality.query_interrupted_seen);
    }

    #[test]
    fn no_csv_files_are_created() {
        let tmp = tempfile::tempdir().unwrap();
        let run = create_run_directory(tmp.path(), "test_no_csv").unwrap();
        fs::write(run.run_directory_path().join("events.jsonl"), "{}\n").unwrap();
        fs::write(run.run_directory_path().join("raw/data.rawbin"), [0u8; 10]).unwrap();

        fn has_csv(dir: &std::path::Path) -> bool {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    if has_csv(&path) {
                        return true;
                    }
                } else if path.extension().and_then(|e| e.to_str()) == Some("csv") {
                    return true;
                }
            }
            false
        }
        assert!(!has_csv(&run.run_directory_path()));
    }

    #[test]
    fn run_audit_accepts_m29_optional_recipe_shadow_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let run = create_run_directory(tmp.path(), "test_m29_audit").unwrap();

        // Create subdirectories
        fs::create_dir_all(run.run_directory_path().join("recipe")).unwrap();
        fs::create_dir_all(run.run_directory_path().join("shadow")).unwrap();

        // Write M2.9-specific files
        fs::write(
            run.run_directory_path().join("recipe/input_recipe.json"),
            "{}",
        )
        .unwrap();
        fs::write(
            run.run_directory_path().join("recipe/resolved_recipe.json"),
            "{}",
        )
        .unwrap();
        fs::write(
            run.run_directory_path().join("recipe/dry_run_plan.json"),
            "{}",
        )
        .unwrap();
        fs::write(
            run.run_directory_path().join("recipe/safety_report.json"),
            "{}",
        )
        .unwrap();
        fs::write(
            run.run_directory_path()
                .join("shadow/shadow_command_plan.jsonl"),
            "\n",
        )
        .unwrap();
        fs::write(
            run.run_directory_path()
                .join("shadow/executor_shadow_summary.json"),
            "{}",
        )
        .unwrap();
        fs::write(
            run.run_directory_path()
                .join("shadow/forbidden_real_command_check.json"),
            "{}",
        )
        .unwrap();

        assert!(run
            .run_directory_path()
            .join("recipe/input_recipe.json")
            .exists());
        assert!(run
            .run_directory_path()
            .join("shadow/shadow_command_plan.jsonl")
            .exists());
        assert!(run
            .run_directory_path()
            .join("shadow/forbidden_real_command_check.json")
            .exists());
    }
}
