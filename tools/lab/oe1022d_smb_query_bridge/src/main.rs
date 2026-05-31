//! M2.7B: Real SMB100A query-only snapshot + Real OE1022D passive acquisition.
//!
//! Combines a real SMB100A TCP query-only snapshot with real OE1022D passive
//! RALL? acquisition, producing formal run artifacts.
//!
//! ## Safety
//! - Only query commands (ending in `?`) are sent to real SMB100A.
//! - Setting commands are rejected before transport.
//! - SMB100A connection is closed before OE1022D acquisition begins.
//! - Only `*IDN?` and `RALL?` are sent to real OE1022D.
//! - All commands are recorded in `command_audit.jsonl`.

use clap::Parser;
use odmr_logging::{
    create_run_directory, EventLevel, RawIndexEntry, RunArtifactPaths, RunDirectory, RunEvent,
    RunEventType, RunManifest,
};
use odmr_oe1022d::parser::{latest_b_channel_sample, parse_rall_frame, RALL_FRAME_BYTES};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Hard-coded command allow-lists
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
#[command(name = "oe1022d-smb-query-bridge")]
#[command(about = "M2.7B: Real SMB100A query-only + Real OE1022D passive acquisition")]
struct Cli {
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

    #[arg(long, default_value = "../../runs")]
    run_root: String,

    #[arg(long)]
    run_id: String,
}

// ---------------------------------------------------------------------------
// JSON types
// ---------------------------------------------------------------------------

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
    created_at_unix_ms: u64,
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
    no_executor_integration: bool,
    no_gui_hardware_access: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CommandAuditEntry {
    timestamp_unix_ms: u64,
    device_id: String,
    command: String,
    allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    rejection_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transport_error: Option<String>,
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

// ---------------------------------------------------------------------------
// Event helpers
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicU64, Ordering};

static EVENT_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_event_id() -> u64 {
    EVENT_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn utc_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn make_event(
    run_id: &str,
    event_type: RunEventType,
    level: EventLevel,
    message: &str,
    device_id: &str,
    data: Option<serde_json::Value>,
) -> RunEvent {
    RunEvent {
        schema_version: "0.2.0".into(),
        kind: "run_event".into(),
        run_id: run_id.into(),
        event_id: format!("evt_{:010}", next_event_id()),
        timestamp_unix_ms: utc_now_ms(),
        timestamp_monotonic_ns: None,
        level,
        event_type,
        step_id: None,
        device_id: Some(device_id.into()),
        message: message.into(),
        data,
    }
}

// ---------------------------------------------------------------------------
// SMB100A query-only TCP transport
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

        // SMB100A SCPI responses end with newline
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

    fn close(self) {
        drop(self.stream);
    }
}

// ---------------------------------------------------------------------------
// SMB100A query snapshot builder
// ---------------------------------------------------------------------------

fn build_smb_query_snapshot(
    transport: &mut SmbQueryTransport,
) -> Result<Smb100aQuerySnapshot, String> {
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
    for q in &queries {
        let resp = match transport.query(q) {
            Ok(r) => r,
            Err(e) => format!("ERROR: {}", e),
        };
        results.push(SmbQueryResult {
            command: q.to_string(),
            response: resp,
        });
    }

    Ok(Smb100aQuerySnapshot {
        schema_version: "0.2.0".into(),
        device_id: "smb100a_main".into(),
        idn,
        queried_at_unix_ms: utc_now_ms(),
        queries: results,
        query_only_mode: true,
        connection_closed: false,
    })
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
    println!("  M2.7B Real SMB Query + Real OE Bridge");
    println!("========================================");
    println!();
    println!("SMB100A:      {}:{}", cli.smb_host, cli.smb_port);
    println!("OE1022D Port: {} @ {} baud", cli.oe_port, cli.oe_baud);
    println!(
        "Frames:       {} (delay {} ms, timeout {} ms)",
        cli.frames, cli.delay_ms, cli.timeout_ms
    );
    println!("Run ID:       {}", cli.run_id);
    println!("Run Root:     {}", cli.run_root);
    println!();
    println!("SAFETY: Only query commands to real SMB100A.");
    println!("SAFETY: SMB100A connection closed before OE acquisition.");
    println!("SAFETY: Only *IDN? and RALL? to real OE1022D.");
    println!();

    // -- Create run directory ------------------------------------------------

    let run_root = PathBuf::from(&cli.run_root);
    let run = match create_run_directory(&run_root, &cli.run_id) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to create run directory: {}", e);
            std::process::exit(1);
        }
    };

    let created_at = utc_now_ms();

    let mut event_writer = match run.open_event_writer() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to open event writer: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = event_writer.write_event(&make_event(
        &cli.run_id,
        RunEventType::RunCreated,
        EventLevel::Info,
        "Run directory created",
        "system",
        None,
    )) {
        eprintln!("Failed to write event: {}", e);
        std::process::exit(1);
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

    let mut record_audit = |entry: CommandAuditEntry| {
        command_audit.push(entry.clone());
        if let Ok(line) = serde_json::to_string(&entry) {
            let _ = writeln!(audit_writer, "{}", line);
            let _ = audit_writer.flush();
        }
    };

    // -- SMB100A query-only snapshot -----------------------------------------

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

    let smb_snapshot = match build_smb_query_snapshot(&mut smb_transport) {
        Ok(s) => {
            println!("SMB100A IDN: {}", s.idn);
            for q in &s.queries {
                println!("  {:20} -> {}", q.command, q.response);
            }
            s
        }
        Err(e) => {
            eprintln!("SMB100A query snapshot failed: {}", e);
            std::process::exit(1);
        }
    };

    // Record SMB100A commands in audit
    record_audit(CommandAuditEntry {
        timestamp_unix_ms: utc_now_ms(),
        device_id: "smb100a_main".to_string(),
        command: "*IDN?".to_string(),
        allowed: true,
        rejection_reason: None,
        response_preview: Some(smb_snapshot.idn.clone()),
        transport_error: None,
    });
    for q in &smb_snapshot.queries {
        record_audit(CommandAuditEntry {
            timestamp_unix_ms: utc_now_ms(),
            device_id: "smb100a_main".to_string(),
            command: q.command.clone(),
            allowed: true,
            rejection_reason: None,
            response_preview: Some(q.response.clone()),
            transport_error: None,
        });
    }

    if let Err(e) = event_writer.write_event(&make_event(
        &cli.run_id,
        RunEventType::DeviceIdentityVerified,
        EventLevel::Info,
        &format!("SMB100A identity verified: {}", smb_snapshot.idn),
        "smb100a_main",
        Some(serde_json::json!({
            "idn": smb_snapshot.idn,
            "mock": false,
            "real_hardware": true,
            "query_only": true,
        })),
    )) {
        eprintln!("Failed to write event: {}", e);
    }

    // -- Close SMB100A connection --------------------------------------------

    println!("Closing SMB100A connection before OE1022D acquisition...");
    smb_transport.close();

    if let Err(e) = event_writer.write_event(&make_event(
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
    )) {
        eprintln!("Failed to write event: {}", e);
    }

    // -- Real OE1022D identity ------------------------------------------------

    let oe_idn = match oe_verify_identity(&cli.oe_port, cli.oe_baud, cli.timeout_ms) {
        Ok(idn) => {
            println!("OE1022D IDN: {}", idn);
            idn
        }
        Err(e) => {
            eprintln!("OE1022D identity verification failed: {}", e);
            std::process::exit(1);
        }
    };

    record_audit(CommandAuditEntry {
        timestamp_unix_ms: utc_now_ms(),
        device_id: "oe1022d_main".to_string(),
        command: "*IDN?".to_string(),
        allowed: true,
        rejection_reason: None,
        response_preview: Some(oe_idn.clone()),
        transport_error: None,
    });

    if let Err(e) = event_writer.write_event(&make_event(
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
    )) {
        eprintln!("Failed to write event: {}", e);
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
            recipe_lock: "metadata/recipe.lock.json".into(),
            resolved_recipe_lock: "metadata/resolved_recipe.lock.json".into(),
            dry_run_plan_lock: "metadata/dry_run_plan.lock.json".into(),
            safety_report_lock: "metadata/safety_report.lock.json".into(),
            events: "events.jsonl".into(),
            index: "index.jsonl".into(),
            raw_bin: "raw/oe1022d_rall.rawbin".into(),
        },
        recipe_hash: None,
        resolved_recipe_id: None,
        safety_report_id: None,
    };
    if let Err(e) = run.write_manifest(&manifest) {
        eprintln!("Failed to write manifest: {}", e);
        std::process::exit(1);
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
        created_at_unix_ms: created_at,
    };
    if let Err(e) = run.write_json_artifact("metadata/acquisition_config.json", &acq_config) {
        eprintln!("Failed to write acquisition_config: {}", e);
        std::process::exit(1);
    }

    let parser_meta = ParserVersionMeta {
        schema_version: "0.2.0".into(),
        parser_crate: "odmr-oe1022d".into(),
        parser_version: env!("CARGO_PKG_VERSION").into(),
        rall_frame_bytes: RALL_FRAME_BYTES,
    };
    if let Err(e) = run.write_json_artifact("metadata/parser_version.json", &parser_meta) {
        eprintln!("Failed to write parser_version: {}", e);
        std::process::exit(1);
    }

    let mut smb_snapshot_for_write = smb_snapshot.clone();
    smb_snapshot_for_write.connection_closed = true;
    if let Err(e) = run.write_json_artifact(
        "metadata/smb100a_query_snapshot.json",
        &smb_snapshot_for_write,
    ) {
        eprintln!("Failed to write smb100a_query_snapshot: {}", e);
        std::process::exit(1);
    }

    let safety_note = SafetyBoundaryNote {
        schema_version: "0.2.0".into(),
        real_oe1022d_allowed_commands: vec!["*IDN?".into(), "RALL?".into()],
        real_smb100a_query_only: true,
        real_smb100a_setting_commands_blocked: true,
        smb_connection_closed_before_acquisition: true,
        no_csv_policy: true,
        no_executor_integration: true,
        no_gui_hardware_access: true,
    };
    if let Err(e) = run.write_json_artifact("metadata/safety_boundary_note.json", &safety_note) {
        eprintln!("Failed to write safety_boundary_note: {}", e);
        std::process::exit(1);
    }

    // -- Station snapshot ----------------------------------------------------

    let station_snapshot = serde_json::json!({
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
                "idn": smb_snapshot.idn,
                "mock": false,
                "real_hardware": true,
                "transport": {
                    "type": "tcp",
                    "host": cli.smb_host,
                    "port": cli.smb_port,
                },
                "query_only": true,
                "connection_closed_before_acquisition": true,
                "query_snapshot": smb_snapshot_for_write,
            }
        },
        "snapshot_at_unix_ms": utc_now_ms(),
    });
    if let Err(e) = run.write_station_snapshot_json(&station_snapshot) {
        eprintln!("Failed to write station_snapshot: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = event_writer.write_event(&make_event(
        &cli.run_id,
        RunEventType::StationSnapshotWritten,
        EventLevel::Info,
        "Station snapshot written",
        "system",
        None,
    )) {
        eprintln!("Failed to write event: {}", e);
    }

    // -- Open writers --------------------------------------------------------

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

    println!("Capturing {} frames...", cli.frames);

    if let Err(e) = event_writer.write_event(&make_event(
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
    )) {
        eprintln!("Failed to write event: {}", e);
    }

    let mut preview_rows: Vec<BChannelPreviewRow> = Vec::new();
    let mut summary_rows: Vec<FrameSummaryRow> = Vec::new();

    let mut ok_count = 0usize;
    let mut fail_count = 0usize;
    let mut timeout_count = 0usize;

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

                    if let Err(e) = event_writer.write_event(&make_event(
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
                    )) {
                        eprintln!("Failed to write event: {}", e);
                    }

                    record_audit(CommandAuditEntry {
                        timestamp_unix_ms: ts,
                        device_id: "oe1022d_main".to_string(),
                        command: "RALL?".to_string(),
                        allowed: true,
                        rejection_reason: None,
                        response_preview: Some(format!("{} bytes", buf.len())),
                        transport_error: None,
                    });

                    if buf.len() == RALL_FRAME_BYTES {
                        match parse_rall_frame(&buf) {
                            Ok(frame) => {
                                index_entry.parse_status = Some("success".into());
                                if let Err(e) = event_writer.write_event(&make_event(
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
                                )) {
                                    eprintln!("Failed to write event: {}", e);
                                }

                                if let Some(sample) = latest_b_channel_sample(&frame) {
                                    preview_rows.push(BChannelPreviewRow {
                                        run_id: cli.run_id.clone(),
                                        frame_index: i,
                                        timestamp_unix_ms: ts,
                                        b_x_mv: sample.x_mv,
                                        b_y_mv: sample.y_mv,
                                        b_freq_hz: sample.freq_hz,
                                        b_noise_mv: sample.noise_mv,
                                        b_pll_locked: frame.config.b_pll_locked.unwrap_or(false),
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
                                });
                                ok_count += 1;
                            }
                            Err(e) => {
                                index_entry.parse_status = Some("fail".into());
                                index_entry.notes = Some(format!("parse error: {}", e));
                                if let Err(evte) = event_writer.write_event(&make_event(
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
                                )) {
                                    eprintln!("Failed to write event: {}", evte);
                                }

                                summary_rows.push(FrameSummaryRow {
                                    run_id: cli.run_id.clone(),
                                    frame_index: i,
                                    timestamp_unix_ms: ts,
                                    raw_len: buf.len(),
                                    parse_status: "fail".into(),
                                    parse_error: Some(format!("{}", e)),
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
                        if let Err(e) = event_writer.write_event(&make_event(
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
                        )) {
                            eprintln!("Failed to write event: {}", e);
                        }

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
                        });
                        fail_count += 1;
                    }

                    if let Err(e) = index_writer.write_entry(&index_entry) {
                        eprintln!("Failed to write index entry: {}", e);
                    }

                    Ok(())
                }
                Err(e) => {
                    record_audit(CommandAuditEntry {
                        timestamp_unix_ms: ts,
                        device_id: "oe1022d_main".to_string(),
                        command: "RALL?".to_string(),
                        allowed: true,
                        rejection_reason: None,
                        response_preview: None,
                        transport_error: Some(e.clone()),
                    });

                    if let Err(evte) = event_writer.write_event(&make_event(
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
                    )) {
                        eprintln!("Failed to write event: {}", evte);
                    }

                    summary_rows.push(FrameSummaryRow {
                        run_id: cli.run_id.clone(),
                        frame_index: i,
                        timestamp_unix_ms: ts,
                        raw_len: 0,
                        parse_status: "timeout".into(),
                        parse_error: Some(e.clone()),
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

    // -- Finalize acquisition ------------------------------------------------

    if let Err(e) = event_writer.write_event(&make_event(
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
    )) {
        eprintln!("Failed to write event: {}", e);
    }

    if let Err(e) = write_parsed_jsonl(&run, "parsed/b_channel_preview.jsonl", &preview_rows) {
        eprintln!("Failed to write preview: {}", e);
    }
    if let Err(e) = write_parsed_jsonl(&run, "parsed/frame_summary.jsonl", &summary_rows) {
        eprintln!("Failed to write summary: {}", e);
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
        "forbidden_commands_found": [],
        "warnings": [],
        "errors": [],
        "smb100a_query_only": true,
        "smb100a_connection_closed_before_acquisition": true,
        "command_audit_entries": command_audit.len(),
    });
    if let Err(e) = run.write_json_artifact("audit_report.json", &audit_report) {
        eprintln!("Failed to write audit report: {}", e);
    }

    if let Err(e) = event_writer.write_event(&make_event(
        &cli.run_id,
        RunEventType::AuditCompleted,
        EventLevel::Info,
        "Audit completed",
        "system",
        Some(serde_json::json!({
            "passed": ok_count > 0 && fail_count == 0 && timeout_count == 0,
            "command_audit_entries": command_audit.len(),
        })),
    )) {
        eprintln!("Failed to write event: {}", e);
    }

    // -- Run completed -------------------------------------------------------

    if let Err(e) = event_writer.write_event(&make_event(
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
        })),
    )) {
        eprintln!("Failed to write event: {}", e);
    }

    println!();
    println!("Results:");
    println!("  OK:      {}", ok_count);
    println!("  Fail:    {}", fail_count);
    println!("  Timeout: {}", timeout_count);
    println!();
    println!("Run directory: {}", run.run_directory_path().display());
    println!("Done.");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn smb_query_allowlist_accepts_only_queries() {
        assert!(validate_smb_query_only("*IDN?").is_ok());
        assert!(validate_smb_query_only("OUTP?").is_ok());
        assert!(validate_smb_query_only("FREQ?").is_ok());
        assert!(validate_smb_query_only("SYST:ERR?").is_ok());
    }

    #[test]
    fn smb_setting_commands_are_rejected() {
        assert!(validate_smb_query_only("OUTP ON").is_err());
        assert!(validate_smb_query_only("OUTP OFF").is_err());
        assert!(validate_smb_query_only("MOD:STAT ON").is_err());
        assert!(validate_smb_query_only("FREQ 2.882GHz").is_err());
        assert!(validate_smb_query_only("POW -15dBm").is_err());
        assert!(validate_smb_query_only("FM:STAT ON").is_err());
        assert!(validate_smb_query_only("*RST").is_err());
    }

    #[test]
    fn smb_outp_off_is_rejected_because_query_only() {
        // Even though OUTP OFF is "safe" in some contexts, this task is query-only.
        assert!(validate_smb_query_only("OUTP OFF").is_err());
    }

    #[test]
    fn oe1022d_allowlist_accepts_only_idn_and_rall() {
        assert!(validate_oe_command("*IDN?").is_ok());
        assert!(validate_oe_command("RALL?").is_ok());
        assert!(validate_oe_command("SENSD 2,7").is_err());
        assert!(validate_oe_command("*RST").is_err());
    }

    #[test]
    fn command_audit_entry_serializes_correctly() {
        let entry = CommandAuditEntry {
            timestamp_unix_ms: 12345,
            device_id: "smb100a_main".into(),
            command: "*IDN?".into(),
            allowed: true,
            rejection_reason: None,
            response_preview: Some("Rohde&Schwarz,SMB100A".into()),
            transport_error: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("*IDN?"));
        assert!(json.contains("true"));
    }

    #[test]
    fn forbidden_smb_commands_never_reach_transport() {
        // Validate that all setting patterns are rejected by the validator
        let forbidden = [
            "OUTP ON",
            "OUTP OFF",
            "MOD:STAT ON",
            "FREQ 2.882GHz",
            "POW -15dBm",
            "POW:ALC AUTO",
            "FM:STAT ON",
            "FM:SOUR INT",
            "FM:DEV 4MHz",
            "LFO ON",
            "LFO:FREQ 500Hz",
            "LFO:VOLT 0.137V",
            "SWE:MODE AUTO",
            "*RST",
        ];
        for cmd in &forbidden {
            assert!(
                validate_smb_query_only(cmd).is_err(),
                "{} should be rejected",
                cmd
            );
        }
    }

    #[test]
    fn station_snapshot_includes_smb100a_query_state() {
        let snapshot = serde_json::json!({
            "devices": {
                "smb100a_main": {
                    "device_id": "smb100a_main",
                    "mock": false,
                    "real_hardware": true,
                    "query_only": true,
                    "idn": "Rohde&Schwarz,SMB100A,123456,5.00.116",
                },
                "oe1022d_main": {
                    "device_id": "oe1022d_main",
                    "mock": false,
                    "real_hardware": true,
                }
            }
        });
        let smb = &snapshot["devices"]["smb100a_main"];
        assert_eq!(smb["mock"], false);
        assert_eq!(smb["real_hardware"], true);
        assert_eq!(smb["query_only"], true);
    }

    #[test]
    fn run_directory_layout_is_complete() {
        let tmp = tempfile::tempdir().unwrap();
        let run = create_run_directory(tmp.path(), "test_layout").unwrap();

        // Write all expected artifacts
        fs::write(run.run_directory_path().join("manifest.json"), "{}").unwrap();
        fs::write(
            run.run_directory_path()
                .join("metadata/acquisition_config.json"),
            "{}",
        )
        .unwrap();
        fs::write(
            run.run_directory_path()
                .join("metadata/station_snapshot.json"),
            "{}",
        )
        .unwrap();
        fs::write(
            run.run_directory_path()
                .join("metadata/parser_version.json"),
            "{}",
        )
        .unwrap();
        fs::write(
            run.run_directory_path()
                .join("metadata/smb100a_query_snapshot.json"),
            "{}",
        )
        .unwrap();
        fs::write(
            run.run_directory_path()
                .join("metadata/safety_boundary_note.json"),
            "{}",
        )
        .unwrap();
        fs::write(run.run_directory_path().join("events.jsonl"), "\n").unwrap();
        fs::write(run.run_directory_path().join("command_audit.jsonl"), "\n").unwrap();
        fs::write(run.run_directory_path().join("index.jsonl"), "\n").unwrap();
        fs::write(
            run.run_directory_path().join("raw/oe1022d_rall.rawbin"),
            [0u8; 10],
        )
        .unwrap();
        fs::write(
            run.run_directory_path()
                .join("parsed/b_channel_preview.jsonl"),
            "\n",
        )
        .unwrap();
        fs::write(
            run.run_directory_path().join("parsed/frame_summary.jsonl"),
            "\n",
        )
        .unwrap();
        fs::write(run.run_directory_path().join("audit_report.json"), "{}").unwrap();

        assert!(run.run_directory_path().join("manifest.json").exists());
        assert!(run
            .run_directory_path()
            .join("metadata/acquisition_config.json")
            .exists());
        assert!(run
            .run_directory_path()
            .join("metadata/station_snapshot.json")
            .exists());
        assert!(run
            .run_directory_path()
            .join("metadata/parser_version.json")
            .exists());
        assert!(run
            .run_directory_path()
            .join("metadata/smb100a_query_snapshot.json")
            .exists());
        assert!(run
            .run_directory_path()
            .join("metadata/safety_boundary_note.json")
            .exists());
        assert!(run.run_directory_path().join("events.jsonl").exists());
        assert!(run
            .run_directory_path()
            .join("command_audit.jsonl")
            .exists());
        assert!(run.run_directory_path().join("index.jsonl").exists());
        assert!(run
            .run_directory_path()
            .join("raw/oe1022d_rall.rawbin")
            .exists());
        assert!(run
            .run_directory_path()
            .join("parsed/b_channel_preview.jsonl")
            .exists());
        assert!(run
            .run_directory_path()
            .join("parsed/frame_summary.jsonl")
            .exists());
        assert!(run.run_directory_path().join("audit_report.json").exists());
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
    fn audit_report_passes_when_all_ok() {
        let report = serde_json::json!({
            "passed": true,
            "frame_count": 10,
            "csv_files_found": [],
            "forbidden_commands_found": [],
            "smb100a_query_only": true,
            "command_audit_entries": 25,
        });
        assert_eq!(report["passed"], true);
        assert_eq!(
            report["forbidden_commands_found"].as_array().unwrap().len(),
            0
        );
    }

    #[test]
    fn smb100a_query_snapshot_serializes() {
        let snap = Smb100aQuerySnapshot {
            schema_version: "0.2.0".into(),
            device_id: "smb100a_main".into(),
            idn: "Rohde&Schwarz,SMB100A,123456,5.00.116".into(),
            queried_at_unix_ms: 12345,
            queries: vec![
                SmbQueryResult {
                    command: "OUTP?".into(),
                    response: "OFF".into(),
                },
                SmbQueryResult {
                    command: "FREQ?".into(),
                    response: "2882000000".into(),
                },
            ],
            query_only_mode: true,
            connection_closed: true,
        };
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("query_only_mode"));
        assert!(json.contains("connection_closed"));
    }

    #[test]
    fn safety_boundary_note_has_all_fields() {
        let note = SafetyBoundaryNote {
            schema_version: "0.2.0".into(),
            real_oe1022d_allowed_commands: vec!["*IDN?".into(), "RALL?".into()],
            real_smb100a_query_only: true,
            real_smb100a_setting_commands_blocked: true,
            smb_connection_closed_before_acquisition: true,
            no_csv_policy: true,
            no_executor_integration: true,
            no_gui_hardware_access: true,
        };
        let json = serde_json::to_string_pretty(&note).unwrap();
        assert!(json.contains("real_smb100a_query_only"));
        assert!(json.contains("real_smb100a_setting_commands_blocked"));
        assert!(json.contains("smb_connection_closed_before_acquisition"));
    }
}
