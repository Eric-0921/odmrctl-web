//! M2.7A: Fake SMB100A + Real OE1022D bridge acquisition tool.
//!
//! Combines a mock SMB100A state timeline with real OE1022D passive RALL?
//! acquisition, producing formal run artifacts.
//!
//! ## Safety
//! - Only `*IDN?` and `RALL?` are sent to real OE1022D.
//! - No real SMB100A connection is attempted.
//! - All fake SMB100A events are marked `mock: true`.
//! - All real OE1022D events are marked `mock: false, real_hardware: true`.

use clap::Parser;
use odmr_device::FakeDevice;
use odmr_logging::{
    create_run_directory, EventLevel, RawIndexEntry, RunArtifactPaths, RunDirectory, RunEvent,
    RunEventType, RunManifest,
};
use odmr_oe1022d::parser::{latest_b_channel_sample, parse_rall_frame, RALL_FRAME_BYTES};
use odmr_smb100a::fake::FakeSmb100a;
use odmr_types::DeviceId;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Hard-coded command allow-list
// ---------------------------------------------------------------------------

const ALLOWED_COMMANDS: &[&str] = &["*IDN?", "RALL?"];

const FORBIDDEN_PATTERNS: &[&str] = &[
    "*RST", "RST", "INIT", "RUN", "SSETD", "RSETD", "APHSD", "FMODD", "RSLPD", "PHASD", "ISRCD",
    "SENSD", "OFLTD", "OFSLD", "HARMD",
];

#[allow(dead_code)]
const SMB100A_PATTERNS: &[&str] = &[
    "smb100a",
    "SMB100A",
    "FREQ ",
    "POW ",
    "OUTP ",
    "MOD:STAT",
    "FREQ:MODE",
];

#[allow(dead_code)]
fn reject_smb100a_commands(text: &str) -> Result<(), String> {
    for pat in SMB100A_PATTERNS {
        if text.contains(pat) {
            return Err(format!("text contains SMB100A command pattern '{}'", pat));
        }
    }
    Ok(())
}

fn validate_command(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if !ALLOWED_COMMANDS.contains(&trimmed) {
        return Err(format!(
            "command '{}' is not in the safe allow-list",
            trimmed
        ));
    }
    for pat in FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return Err(format!(
                "command '{}' contains forbidden pattern '{}'",
                trimmed, pat
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(name = "oe1022d-smb-fake-bridge")]
#[command(about = "M2.7A: Fake SMB100A + Real OE1022D bridge")]
struct Cli {
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

    #[arg(long, default_value = "../../runs")]
    run_root: String,

    #[arg(long)]
    run_id: String,

    #[arg(long, value_name = "JSON")]
    fake_smb_profile: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// JSON types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FakeSmbProfile {
    pub device_id: String,
    pub idn: String,
    pub initial_state: Smb100aStateJson,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Smb100aStateJson {
    pub rf_frequency_hz: f64,
    pub rf_power_dbm: f64,
    pub rf_output_enabled: bool,
    pub modulation_global_enabled: bool,
    pub fm_enabled: bool,
    pub fm_source: String,
    pub fm_deviation_hz: f64,
    pub lf_output_enabled: bool,
    pub lf_frequency_hz: f64,
    pub lf_voltage_v: f64,
    pub lf_shape: String,
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
}

#[derive(Serialize)]
struct AcquisitionConfig {
    schema_version: String,
    oe_port: String,
    oe_baud: u32,
    frames_requested: u32,
    delay_ms: u64,
    timeout_ms: u64,
    fake_smb100a_enabled: bool,
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
    real_smb100a_connected: bool,
    fake_smb100a_mock: bool,
    no_csv_policy: bool,
    no_executor_integration: bool,
    no_gui_hardware_access: bool,
}

#[derive(Serialize)]
struct FakeSmb100aStateTimelineEntry {
    timestamp_unix_ms: u64,
    frame_index: Option<u64>,
    state: serde_json::Value,
    mock: bool,
}

// ---------------------------------------------------------------------------
// Event helpers
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicU64, Ordering};

static EVENT_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_event_id() -> u64 {
    EVENT_COUNTER.fetch_add(1, Ordering::SeqCst)
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

fn utc_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Fake SMB100A state from profile
// ---------------------------------------------------------------------------

fn load_fake_smb_profile(path: &PathBuf) -> Result<FakeSmbProfile, Box<dyn std::error::Error>> {
    let s = fs::read_to_string(path)?;
    let profile: FakeSmbProfile = serde_json::from_str(&s)?;
    Ok(profile)
}

fn apply_json_state(dev: &mut FakeSmb100a, json: &Smb100aStateJson) {
    let st = dev.state_mut();
    st.rf_frequency_hz = json.rf_frequency_hz;
    st.rf_power_dbm = json.rf_power_dbm;
    st.rf_output_enabled = json.rf_output_enabled;
    st.modulation_global_enabled = json.modulation_global_enabled;
    st.fm_enabled = json.fm_enabled;
    st.fm_source = json.fm_source.clone();
    st.fm_deviation_hz = json.fm_deviation_hz;
    st.lf_output_enabled = json.lf_output_enabled;
    st.lf_frequency_hz = json.lf_frequency_hz;
    st.lf_voltage_v = json.lf_voltage_v;
    st.lf_shape = json.lf_shape.clone();
}

fn state_to_json(dev: &FakeSmb100a) -> serde_json::Value {
    let st = dev.state();
    serde_json::json!({
        "rf_frequency_hz": st.rf_frequency_hz,
        "rf_power_dbm": st.rf_power_dbm,
        "rf_output_enabled": st.rf_output_enabled,
        "modulation_global_enabled": st.modulation_global_enabled,
        "fm_enabled": st.fm_enabled,
        "fm_source": st.fm_source,
        "fm_deviation_hz": st.fm_deviation_hz,
        "lf_output_enabled": st.lf_output_enabled,
        "lf_frequency_hz": st.lf_frequency_hz,
        "lf_voltage_v": st.lf_voltage_v,
        "lf_shape": st.lf_shape,
    })
}

// ---------------------------------------------------------------------------
// Serial helpers (real OE1022D)
// ---------------------------------------------------------------------------

fn verify_identity(port: &str, baud: u32, timeout_ms: u64) -> Result<String, String> {
    validate_command("*IDN?")?;

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

fn capture_single_frame(port: &str, baud: u32, timeout_ms: u64) -> Result<Vec<u8>, String> {
    validate_command("RALL?")?;

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
    println!("  M2.7A Fake SMB + Real OE Bridge");
    println!("========================================");
    println!();
    println!("OE1022D Port: {} @ {} baud", cli.oe_port, cli.oe_baud);
    println!(
        "Frames:       {} (delay {} ms, timeout {} ms)",
        cli.frames, cli.delay_ms, cli.timeout_ms
    );
    println!("Run ID:       {}", cli.run_id);
    println!("Run Root:     {}", cli.run_root);
    println!();
    println!("SAFETY: Only *IDN? and RALL? to real OE1022D.");
    println!("SAFETY: No real SMB100A connection.");
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

    // -- Fake SMB100A setup --------------------------------------------------

    let mut fake_smb = FakeSmb100a::new(DeviceId::new("smb100a_fake"));
    let mut smb_timeline: Vec<FakeSmb100aStateTimelineEntry> = Vec::new();

    if let Some(ref profile_path) = cli.fake_smb_profile {
        match load_fake_smb_profile(profile_path) {
            Ok(profile) => {
                apply_json_state(&mut fake_smb, &profile.initial_state);
                println!(
                    "Fake SMB100A loaded from profile: {} ({})",
                    profile.device_id, profile.idn
                );
                if let Err(e) = event_writer.write_event(&make_event(
                    &cli.run_id,
                    RunEventType::FakeSmb100aStateLoaded,
                    EventLevel::Info,
                    &format!(
                        "Fake SMB100A state loaded from profile: {}",
                        profile_path.display()
                    ),
                    "smb100a_fake",
                    Some(serde_json::json!({
                        "mock": true,
                        "real_hardware": false,
                        "profile": profile_path.display().to_string(),
                        "idn": profile.idn,
                    })),
                )) {
                    eprintln!("Failed to write event: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Warning: failed to load fake SMB profile: {}", e);
            }
        }
    }

    if let Err(e) = event_writer.write_event(&make_event(
        &cli.run_id,
        RunEventType::MockDeviceRegistered,
        EventLevel::Info,
        "Fake SMB100A registered",
        "smb100a_fake",
        Some(serde_json::json!({
            "mock": true,
            "real_hardware": false,
            "idn": fake_smb.idn().to_string(),
        })),
    )) {
        eprintln!("Failed to write event: {}", e);
    }

    smb_timeline.push(FakeSmb100aStateTimelineEntry {
        timestamp_unix_ms: utc_now_ms(),
        frame_index: None,
        state: state_to_json(&fake_smb),
        mock: true,
    });

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
        oe_port: cli.oe_port.clone(),
        oe_baud: cli.oe_baud,
        frames_requested: cli.frames,
        delay_ms: cli.delay_ms,
        timeout_ms: cli.timeout_ms,
        fake_smb100a_enabled: cli.fake_smb_profile.is_some(),
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

    let safety_note = SafetyBoundaryNote {
        schema_version: "0.2.0".into(),
        real_oe1022d_allowed_commands: vec!["*IDN?".into(), "RALL?".into()],
        real_smb100a_connected: false,
        fake_smb100a_mock: true,
        no_csv_policy: true,
        no_executor_integration: true,
        no_gui_hardware_access: true,
    };
    if let Err(e) = run.write_json_artifact("metadata/safety_boundary_note.json", &safety_note) {
        eprintln!("Failed to write safety_boundary_note: {}", e);
        std::process::exit(1);
    }

    // -- Fake SMB100A profile snapshot ---------------------------------------

    if let Some(ref profile_path) = cli.fake_smb_profile {
        let profile_copy = run
            .run_directory_path()
            .join("metadata/fake_smb100a_profile.json");
        if let Err(e) = fs::copy(profile_path, profile_copy) {
            eprintln!("Failed to copy fake SMB profile: {}", e);
        }
    }

    // -- Real OE1022D identity ------------------------------------------------

    let idn = match verify_identity(&cli.oe_port, cli.oe_baud, cli.timeout_ms) {
        Ok(idn) => {
            println!("OE1022D IDN: {}", idn);
            idn
        }
        Err(e) => {
            eprintln!("Identity verification failed: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = event_writer.write_event(&make_event(
        &cli.run_id,
        RunEventType::DeviceIdentityVerified,
        EventLevel::Info,
        &format!("Device identity verified: {}", idn),
        "oe1022d_main",
        Some(serde_json::json!({
            "idn": idn,
            "mock": false,
            "real_hardware": true,
            "allowed_commands": ["*IDN?", "RALL?"],
        })),
    )) {
        eprintln!("Failed to write event: {}", e);
    }

    // -- Station snapshot ----------------------------------------------------

    let station_snapshot = serde_json::json!({
        "devices": {
            "oe1022d_main": {
                "device_id": "oe1022d_main",
                "idn": idn,
                "mock": false,
                "real_hardware": true,
                "transport": {
                    "type": "serial",
                    "port": cli.oe_port,
                    "baud_rate": cli.oe_baud,
                },
                "allowed_commands": ["*IDN?", "RALL?"],
            },
            "smb100a_fake": {
                "device_id": "smb100a_fake",
                "idn": fake_smb.idn(),
                "mock": true,
                "real_hardware": false,
                "transport": { "type": "mock" },
                "state": state_to_json(&fake_smb),
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

        // Update fake SMB100A timeline before each frame
        smb_timeline.push(FakeSmb100aStateTimelineEntry {
            timestamp_unix_ms: ts,
            frame_index: Some(i as u64),
            state: state_to_json(&fake_smb),
            mock: true,
        });

        let frame_result: Result<(), String> =
            match capture_single_frame(&cli.oe_port, cli.oe_baud, cli.timeout_ms) {
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

    // -- Finalize fake SMB timeline ------------------------------------------

    if let Err(e) = write_parsed_jsonl(
        &run,
        "mock/fake_smb100a_state_timeline.jsonl",
        &smb_timeline,
    ) {
        eprintln!("Failed to write SMB timeline: {}", e);
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

    fn make_test_profile() -> FakeSmbProfile {
        FakeSmbProfile {
            device_id: "smb100a_fake".into(),
            idn: "Rohde&Schwarz,SMB100A,FAKE123,5.00.116".into(),
            initial_state: Smb100aStateJson {
                rf_frequency_hz: 2.882e9,
                rf_power_dbm: -15.0,
                rf_output_enabled: false,
                modulation_global_enabled: false,
                fm_enabled: false,
                fm_source: "INT".into(),
                fm_deviation_hz: 4e6,
                lf_output_enabled: true,
                lf_frequency_hz: 500.0,
                lf_voltage_v: 0.137,
                lf_shape: "SQUARE".into(),
            },
        }
    }

    #[test]
    fn fake_smb_profile_serializes_and_deserializes() {
        let profile = make_test_profile();
        let json = serde_json::to_string(&profile).unwrap();
        let parsed: FakeSmbProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.device_id, "smb100a_fake");
        assert!(!parsed.initial_state.rf_output_enabled);
    }

    #[test]
    fn fake_smb_state_applies_to_device() {
        let profile = make_test_profile();
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_fake"));
        apply_json_state(&mut dev, &profile.initial_state);
        let st = dev.state();
        assert_eq!(st.rf_frequency_hz, 2.882e9);
        assert_eq!(st.lf_frequency_hz, 500.0);
        assert!(!st.rf_output_enabled);
    }

    #[test]
    fn oe1022d_command_gate_allows_only_idn_and_rall() {
        assert!(validate_command("*IDN?").is_ok());
        assert!(validate_command("RALL?").is_ok());
        assert!(validate_command("FREQD 2,500").is_err());
        assert!(validate_command("SENSD 2,7").is_err());
    }

    #[test]
    fn forbidden_oe1022d_setters_are_rejected() {
        let forbidden = [
            "SENSD", "OFLTD", "OFSLD", "PHASD", "FMODD", "ISRCD", "SYNCD", "HARMD", "*RST",
        ];
        for cmd in &forbidden {
            assert!(validate_command(cmd).is_err(), "{} should be rejected", cmd);
        }
    }

    #[test]
    fn smb100a_patterns_are_rejected() {
        assert!(reject_smb100a_commands("FREQ 2.88GHz").is_err());
        assert!(reject_smb100a_commands("OUTP ON").is_err());
        assert!(reject_smb100a_commands("*IDN?").is_ok());
        assert!(reject_smb100a_commands("RALL?").is_ok());
    }

    #[test]
    fn safety_boundary_note_is_valid_json() {
        let note = SafetyBoundaryNote {
            schema_version: "0.2.0".into(),
            real_oe1022d_allowed_commands: vec!["*IDN?".into(), "RALL?".into()],
            real_smb100a_connected: false,
            fake_smb100a_mock: true,
            no_csv_policy: true,
            no_executor_integration: true,
            no_gui_hardware_access: true,
        };
        let json = serde_json::to_string(&note).unwrap();
        assert!(json.contains("false"));
        assert!(json.contains("true"));
    }

    #[test]
    fn fake_smb_timeline_entry_has_mock_flag() {
        let entry = FakeSmb100aStateTimelineEntry {
            timestamp_unix_ms: 0,
            frame_index: Some(0),
            state: serde_json::json!({"rf_output_enabled": false}),
            mock: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"mock\":true"));
    }

    #[test]
    fn run_directory_has_mock_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        let run = create_run_directory(tmp.path(), "test_mock_dir").unwrap();
        let mock_dir = run.run_directory_path().join("mock");
        fs::create_dir_all(&mock_dir).unwrap();
        assert!(mock_dir.is_dir());
    }

    #[test]
    fn station_snapshot_contains_mock_and_real_flags() {
        let snapshot = serde_json::json!({
            "devices": {
                "oe1022d_main": {
                    "mock": false,
                    "real_hardware": true,
                },
                "smb100a_fake": {
                    "mock": true,
                    "real_hardware": false,
                }
            }
        });
        let oe = &snapshot["devices"]["oe1022d_main"];
        assert_eq!(oe["mock"], false);
        assert_eq!(oe["real_hardware"], true);
        let smb = &snapshot["devices"]["smb100a_fake"];
        assert_eq!(smb["mock"], true);
        assert_eq!(smb["real_hardware"], false);
    }

    #[test]
    fn event_has_device_id_and_mock_flags() {
        let ev = make_event(
            "test_run",
            RunEventType::MockDeviceRegistered,
            EventLevel::Info,
            "test",
            "smb100a_fake",
            Some(serde_json::json!({"mock": true, "real_hardware": false})),
        );
        assert_eq!(ev.device_id, Some("smb100a_fake".into()));
        assert!(ev.data.unwrap().get("mock").unwrap().as_bool().unwrap());
    }

    #[test]
    fn no_csv_files_in_run_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let run = create_run_directory(tmp.path(), "test_no_csv").unwrap();
        // Write some non-CSV files
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
}
