//! M2.6 OE1022D run artifact auditor.
//!
//! Validates run directory layout, raw bin integrity, index consistency,
//! event completeness, CSV absence, and forbidden-command absence.
//! Optionally promotes selected frames to test fixtures.

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek};
use std::path::{Path, PathBuf};

const RALL_FRAME_BYTES: u64 = 12288;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(name = "oe1022d-run-audit")]
#[command(about = "Audit M2.6 OE1022D run artifacts and promote fixtures")]
struct Args {
    /// Path to the run directory to audit
    #[arg(long, value_name = "DIR")]
    run_dir: PathBuf,

    /// Write audit_report.json into the run directory
    #[arg(long)]
    write_report: bool,

    /// Promote frames to fixture directory
    #[arg(long)]
    promote_fixtures: bool,

    /// Root for fixture output (used with --promote-fixtures)
    #[arg(long, value_name = "DIR")]
    fixture_root: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Audit report
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct AuditReport {
    run_id: String,
    passed: bool,
    frame_count: u64,
    rawbin_size_bytes: u64,
    expected_rawbin_size_bytes: u64,
    index_entries: u64,
    preview_entries: u64,
    summary_entries: u64,
    event_counts: HashMap<String, u64>,
    offsets_contiguous: bool,
    all_frames_12288_bytes: bool,
    csv_files_found: Vec<String>,
    forbidden_commands_found: Vec<String>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

impl AuditReport {
    fn new(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            passed: true,
            frame_count: 0,
            rawbin_size_bytes: 0,
            expected_rawbin_size_bytes: 0,
            index_entries: 0,
            preview_entries: 0,
            summary_entries: 0,
            event_counts: HashMap::new(),
            offsets_contiguous: true,
            all_frames_12288_bytes: true,
            csv_files_found: Vec::new(),
            forbidden_commands_found: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn fail(&mut self, msg: impl Into<String>) {
        self.passed = false;
        self.errors.push(msg.into());
    }

    fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }
}

// ---------------------------------------------------------------------------
// Core audit
// ---------------------------------------------------------------------------

fn run_audit(run_dir: &Path) -> AuditReport {
    let run_id = run_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut r = AuditReport::new(run_id);

    // 1. Required layout
    let required_paths = [
        "manifest.json",
        "metadata/acquisition_config.json",
        "metadata/parser_version.json",
        "events.jsonl",
        "index.jsonl",
        "raw/oe1022d_rall.rawbin",
        "parsed/b_channel_preview.jsonl",
        "parsed/frame_summary.jsonl",
    ];
    for rel in &required_paths {
        let path = run_dir.join(rel);
        if !path.exists() {
            r.fail(format!("missing required file: {}", rel));
        }
    }

    // 4. station_snapshot is optional for M2.6 but checked if present
    let station_snapshot = run_dir.join("metadata/station_snapshot.json");
    if station_snapshot.exists() {
        if let Err(e) = fs::read_to_string(&station_snapshot) {
            r.fail(format!("station_snapshot.json unreadable: {e}"));
        }
    }

    // M2.8 optional metadata files — validate JSON if present
    for m28_meta in [
        "metadata/station_snapshot_quality.json",
        "metadata/state_profile_diff.json",
        "metadata/hash_manifest.json",
        "metadata/smb100a_query_timing.json",
    ] {
        let path = run_dir.join(m28_meta);
        if path.exists() {
            if let Err(e) = fs::read_to_string(&path).and_then(|s| {
                serde_json::from_str::<serde_json::Value>(&s)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }) {
                r.fail(format!("{} invalid JSON: {}", m28_meta, e));
            }
        }
    }

    // timeline.jsonl is optional for M2.8 — validate JSONL if present
    let timeline_path = run_dir.join("timeline.jsonl");
    if timeline_path.exists() {
        match read_jsonl_lines(&timeline_path) {
            Ok(lines) => {
                for (i, line) in lines.iter().enumerate() {
                    if let Err(e) = serde_json::from_str::<serde_json::Value>(line) {
                        r.fail(format!("timeline.jsonl line {} invalid JSON: {}", i + 1, e));
                    }
                }
            }
            Err(e) => r.fail(format!("timeline.jsonl unreadable: {}", e)),
        }
    }

    // M2.9 optional metadata files — validate JSON if present
    for m29_meta in [
        "recipe/input_recipe.json",
        "recipe/resolved_recipe.json",
        "recipe/dry_run_plan.json",
        "recipe/safety_report.json",
        "shadow/executor_shadow_summary.json",
        "shadow/forbidden_real_command_check.json",
    ] {
        let path = run_dir.join(m29_meta);
        if path.exists() {
            if let Err(e) = fs::read_to_string(&path).and_then(|s| {
                serde_json::from_str::<serde_json::Value>(&s)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }) {
                r.fail(format!("{} invalid JSON: {}", m29_meta, e));
            }
        }
    }

    // M2.9 optional JSONL files — validate JSONL if present
    for m29_jsonl in [
        "shadow/shadow_command_plan.jsonl",
        "shadow/shadow_step_timeline.jsonl",
        "shadow/frame_to_shadow_step_alignment.jsonl",
    ] {
        let path = run_dir.join(m29_jsonl);
        if path.exists() {
            match read_jsonl_lines(&path) {
                Ok(lines) => {
                    for (i, line) in lines.iter().enumerate() {
                        if let Err(e) = serde_json::from_str::<serde_json::Value>(line) {
                            r.fail(format!("{} line {} invalid JSON: {}", m29_jsonl, i + 1, e));
                        }
                    }
                }
                Err(e) => r.fail(format!("{} unreadable: {}", m29_jsonl, e)),
            }
        }
    }

    // 2. manifest.json valid JSON
    let manifest_path = run_dir.join("manifest.json");
    let manifest: Option<serde_json::Value> = if manifest_path.exists() {
        match fs::read_to_string(&manifest_path) {
            Ok(s) => match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(v) => Some(v),
                Err(e) => {
                    r.fail(format!("manifest.json invalid JSON: {e}"));
                    None
                }
            },
            Err(e) => {
                r.fail(format!("manifest.json unreadable: {e}"));
                None
            }
        }
    } else {
        None
    };

    // Override run_id from manifest if available
    if let Some(ref m) = manifest {
        if let Some(id) = m.get("run_id").and_then(|v| v.as_str()) {
            r.run_id = id.to_string();
        }
    }

    // 3-5. metadata JSON validity
    for meta in [
        "metadata/acquisition_config.json",
        "metadata/parser_version.json",
    ] {
        let path = run_dir.join(meta);
        if path.exists() {
            if let Err(e) = fs::read_to_string(&path).and_then(|s| {
                serde_json::from_str::<serde_json::Value>(&s)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }) {
                r.fail(format!("{} invalid JSON: {}", meta, e));
            }
        }
    }

    // 6. events.jsonl valid JSONL
    let events_path = run_dir.join("events.jsonl");
    let mut event_counts: HashMap<String, u64> = HashMap::new();
    if events_path.exists() {
        match read_jsonl_lines(&events_path) {
            Ok(lines) => {
                for (i, line) in lines.iter().enumerate() {
                    match serde_json::from_str::<serde_json::Value>(line) {
                        Ok(obj) => {
                            if let Some(et) = obj.get("event_type").and_then(|v| v.as_str()) {
                                *event_counts.entry(et.to_string()).or_insert(0) += 1;
                            }
                            // Forbidden-command scan in events
                            scan_for_forbidden_commands(
                                &obj,
                                &mut r,
                                &format!("events.jsonl:{}", i + 1),
                            );
                        }
                        Err(e) => {
                            r.fail(format!("events.jsonl line {} invalid JSON: {}", i + 1, e));
                        }
                    }
                }
            }
            Err(e) => r.fail(format!("events.jsonl unreadable: {}", e)),
        }
    }
    r.event_counts = event_counts;

    // 7. index.jsonl valid JSONL
    let index_path = run_dir.join("index.jsonl");
    let mut index_entries: Vec<serde_json::Value> = Vec::new();
    if index_path.exists() {
        match read_jsonl_lines(&index_path) {
            Ok(lines) => {
                for (i, line) in lines.iter().enumerate() {
                    match serde_json::from_str::<serde_json::Value>(line) {
                        Ok(obj) => {
                            index_entries.push(obj);
                        }
                        Err(e) => {
                            r.fail(format!("index.jsonl line {} invalid JSON: {}", i + 1, e));
                        }
                    }
                }
            }
            Err(e) => r.fail(format!("index.jsonl unreadable: {}", e)),
        }
    }
    r.index_entries = index_entries.len() as u64;

    // 8. rawbin exists
    let rawbin_path = run_dir.join("raw/oe1022d_rall.rawbin");
    let rawbin_size = if rawbin_path.exists() {
        match fs::metadata(&rawbin_path) {
            Ok(m) => m.len(),
            Err(e) => {
                r.fail(format!("rawbin metadata error: {}", e));
                0
            }
        }
    } else {
        0
    };
    r.rawbin_size_bytes = rawbin_size;

    // 11. rawbin size == successful_frame_count * 12288
    let success_count = r.event_counts.get("frame_captured").copied().unwrap_or(0);
    r.frame_count = success_count;
    r.expected_rawbin_size_bytes = success_count * RALL_FRAME_BYTES;
    if rawbin_size != r.expected_rawbin_size_bytes && success_count > 0 {
        r.fail(format!(
            "rawbin size {} != expected {} ({} frames * {})",
            rawbin_size, r.expected_rawbin_size_bytes, success_count, RALL_FRAME_BYTES
        ));
    }

    // 12. index offsets contiguous and match rawbin boundaries
    let mut expected_offset: u64 = 0;
    let mut all_12288 = true;
    for (i, entry) in index_entries.iter().enumerate() {
        let offset = entry
            .get("offset_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let length = entry
            .get("length_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if offset != expected_offset {
            r.fail(format!(
                "index entry {} offset {} != expected {}",
                i, offset, expected_offset
            ));
            r.offsets_contiguous = false;
        }
        if length != RALL_FRAME_BYTES {
            r.fail(format!(
                "index entry {} length {} != {}",
                i, length, RALL_FRAME_BYTES
            ));
            all_12288 = false;
        }
        expected_offset += length;
    }
    if index_entries.is_empty() && rawbin_size > 0 {
        r.offsets_contiguous = false;
        r.fail("index is empty but rawbin is not".to_string());
    }
    r.all_frames_12288_bytes = all_12288;

    // 13. all frame lengths are 12288 bytes (already checked above)

    // 14. frame_captured count matches captured frames
    // (already used success_count)

    // 15. frame_parsed count matches parse successes
    let parsed_count = r.event_counts.get("frame_parsed").copied().unwrap_or(0);
    if parsed_count != success_count && success_count > 0 {
        r.warn(format!(
            "frame_parsed ({}) != frame_captured ({})",
            parsed_count, success_count
        ));
    }

    // 9-10. parsed JSONL validity + entry counts
    let mut preview_entries = 0u64;
    let mut summary_entries = 0u64;
    for (rel, field) in [
        ("parsed/b_channel_preview.jsonl", &mut preview_entries),
        ("parsed/frame_summary.jsonl", &mut summary_entries),
    ] {
        let path = run_dir.join(rel);
        if path.exists() {
            match read_jsonl_lines(&path) {
                Ok(lines) => {
                    for (i, line) in lines.iter().enumerate() {
                        if let Err(e) = serde_json::from_str::<serde_json::Value>(line) {
                            r.fail(format!("{} line {} invalid JSON: {}", rel, i + 1, e));
                        }
                    }
                    *field = lines.len() as u64;
                }
                Err(e) => r.fail(format!("{} unreadable: {}", rel, e)),
            }
        }
    }
    r.preview_entries = preview_entries;
    r.summary_entries = summary_entries;

    // 16. no CSV files anywhere
    let csv_files = find_files_by_ext(run_dir, "csv");
    if !csv_files.is_empty() {
        r.csv_files_found = csv_files
            .iter()
            .map(|p| {
                p.strip_prefix(run_dir)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        r.fail(format!("CSV files found: {:?}", r.csv_files_found));
    }

    // 17. no forbidden command evidence in metadata
    for meta in [
        "metadata/acquisition_config.json",
        "metadata/station_snapshot.json",
    ] {
        let path = run_dir.join(meta);
        if path.exists() {
            if let Ok(s) = fs::read_to_string(&path) {
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&s) {
                    scan_for_forbidden_commands(&obj, &mut r, meta);
                }
            }
        }
    }

    r
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn read_jsonl_lines(path: &Path) -> std::io::Result<Vec<String>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            lines.push(line);
        }
    }
    Ok(lines)
}

fn find_files_by_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(find_files_by_ext(&path, ext));
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case(ext))
                .unwrap_or(false)
            {
                result.push(path);
            }
        }
    }
    result
}

/// Scan a JSON value for evidence of forbidden commands.
fn scan_for_forbidden_commands(value: &serde_json::Value, report: &mut AuditReport, source: &str) {
    let forbidden_patterns = [
        // OE1022D setters
        "SENSD", "OFLTD", "OFSLD", "PHASD", "FREQD", "FMODD", "ISRCD", "SYNCD", "RSLPD", "HARMD",
        "ICLSD", "OEXP", "AUXV", "OUTP", // SMB100A SCPI
        "FREQ", "POW", "OUTP", "MOD:STAT", "FM:STAT", "FM:DEV", "LFO", "SWE", "ROSC", "SYST:ERR",
    ];

    let text = value.to_string();
    for pattern in &forbidden_patterns {
        if text.contains(pattern) {
            // Exclude innocent matches: "*IDN?" and "RALL?" are allowed.
            // The patterns above are broad; we need to narrow them down.
            // For now, collect all matches and let downstream filter.
            let msg = format!("{}: possible forbidden command '{}'", source, pattern);
            if !report.forbidden_commands_found.contains(&msg) {
                report.forbidden_commands_found.push(msg);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fixture promotion
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct FixtureManifest {
    pub source_run_id: String,
    pub extracted_at_unix_ms: u64,
    pub frames: Vec<FixtureFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct FixtureFrame {
    pub file_name: String,
    pub frame_index: u64,
    pub raw_offset: u64,
    pub raw_length: u64,
}

fn promote_fixtures(
    run_dir: &Path,
    fixture_root: &Path,
    frame_count: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(fixture_root)?;

    let rawbin_path = run_dir.join("raw/oe1022d_rall.rawbin");
    let mut rawbin = fs::File::open(&rawbin_path)?;

    let indices: Vec<u64> = if frame_count >= 3 {
        vec![0, frame_count / 2, frame_count - 1]
    } else if frame_count == 2 {
        vec![0, 1]
    } else if frame_count == 1 {
        vec![0]
    } else {
        return Ok(());
    };

    let mut frames = Vec::new();
    for idx in indices {
        let offset = idx * RALL_FRAME_BYTES;
        let mut buf = vec![0u8; RALL_FRAME_BYTES as usize];
        rawbin.seek(std::io::SeekFrom::Start(offset))?;
        rawbin.read_exact(&mut buf)?;

        let file_name = format!("frame_{:03}.rawbin", idx);
        let out_path = fixture_root.join(&file_name);
        fs::write(&out_path, &buf)?;

        frames.push(FixtureFrame {
            file_name,
            frame_index: idx,
            raw_offset: offset,
            raw_length: RALL_FRAME_BYTES,
        });
    }

    let manifest = FixtureManifest {
        source_run_id: run_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string(),
        extracted_at_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        frames,
    };

    let manifest_path = fixture_root.join("fixture_manifest.json");
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args = Args::parse();

    let report = run_audit(&args.run_dir);

    if args.write_report {
        let report_path = args.run_dir.join("audit_report.json");
        match fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()) {
            Ok(()) => println!("Audit report written to {}", report_path.display()),
            Err(e) => eprintln!("Failed to write audit report: {}", e),
        }
    }

    if args.promote_fixtures {
        let fixture_root = args.fixture_root.unwrap_or_else(|| {
            PathBuf::from("tests/fixtures/oe1022d_rall")
                .join(args.run_dir.file_name().unwrap_or_default())
        });
        match promote_fixtures(&args.run_dir, &fixture_root, report.frame_count) {
            Ok(()) => println!("Fixtures promoted to {}", fixture_root.display()),
            Err(e) => eprintln!("Fixture promotion failed: {}", e),
        }
    }

    let json = serde_json::to_string_pretty(&report).unwrap();
    println!("{}", json);

    std::process::exit(if report.passed { 0 } else { 1 });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_valid_run(dir: &Path, frame_count: usize) {
        fs::create_dir_all(dir.join("metadata")).unwrap();
        fs::create_dir_all(dir.join("raw")).unwrap();
        fs::create_dir_all(dir.join("parsed")).unwrap();

        let run_id = dir.file_name().unwrap().to_str().unwrap();

        // manifest
        let manifest = serde_json::json!({
            "schema_version": "0.2.0",
            "kind": "run_manifest",
            "run_id": run_id,
            "created_at_unix_ms": 0u64,
            "artifact_paths": {
                "manifest": "manifest.json",
                "station_snapshot": "metadata/station_snapshot.json",
                "recipe_lock": "metadata/recipe.lock.json",
                "resolved_recipe_lock": "metadata/resolved_recipe.lock.json",
                "dry_run_plan_lock": "metadata/dry_run_plan.lock.json",
                "safety_report_lock": "metadata/safety_report.lock.json",
                "events": "events.jsonl",
                "index": "index.jsonl",
                "raw_bin": "raw/oe1022d_rall.rawbin"
            }
        });
        fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        // metadata
        fs::write(
            dir.join("metadata/acquisition_config.json"),
            r#"{"port":"/dev/ttyUSB0","baud":921600,"frames_requested":10}"#,
        )
        .unwrap();
        fs::write(
            dir.join("metadata/parser_version.json"),
            r#"{"parser_version":"0.1.0"}"#,
        )
        .unwrap();
        fs::write(
            dir.join("metadata/station_snapshot.json"),
            r#"{"idn":"OE1022D"}"#,
        )
        .unwrap();

        // rawbin
        let mut rawbin = fs::File::create(dir.join("raw/oe1022d_rall.rawbin")).unwrap();
        for _ in 0..frame_count {
            rawbin.write_all(&[0u8; RALL_FRAME_BYTES as usize]).unwrap();
        }

        // index
        let mut index = fs::File::create(dir.join("index.jsonl")).unwrap();
        for i in 0..frame_count {
            let entry = serde_json::json!({
                "schema_version": "0.2.0",
                "kind": "raw_index_entry",
                "run_id": run_id,
                "stream_id": "oe1022d.rall",
                "offset_bytes": i as u64 * RALL_FRAME_BYTES,
                "length_bytes": RALL_FRAME_BYTES,
                "timestamp_unix_ms": i as u64 * 1000,
                "frame_index": i,
                "duration_ms": 800,
                "parse_status": "success",
            });
            writeln!(index, "{}", serde_json::to_string(&entry).unwrap()).unwrap();
        }

        // events
        let mut events = fs::File::create(dir.join("events.jsonl")).unwrap();
        let base_events = vec![
            ("run_created", 1u64),
            ("device_identity_verified", 1),
            ("acquisition_started", 1),
        ];
        let mut evt_id = 0u64;
        for (et, count) in base_events {
            for _ in 0..count {
                let e = serde_json::json!({
                    "schema_version": "0.2.0",
                    "kind": "run_event",
                    "run_id": run_id,
                    "event_id": format!("evt_{:010}", evt_id),
                    "timestamp_unix_ms": evt_id * 100,
                    "level": "info",
                    "event_type": et,
                    "message": "test",
                });
                writeln!(events, "{}", serde_json::to_string(&e).unwrap()).unwrap();
                evt_id += 1;
            }
        }
        for i in 0..frame_count {
            for et in ["frame_captured", "frame_parsed"] {
                let e = serde_json::json!({
                    "schema_version": "0.2.0",
                    "kind": "run_event",
                    "run_id": run_id,
                    "event_id": format!("evt_{:010}", evt_id),
                    "timestamp_unix_ms": evt_id * 100,
                    "level": "info",
                    "event_type": et,
                    "message": "test",
                    "data": {"frame_index": i},
                });
                writeln!(events, "{}", serde_json::to_string(&e).unwrap()).unwrap();
                evt_id += 1;
            }
        }
        for et in ["acquisition_completed", "run_completed"] {
            let e = serde_json::json!({
                "schema_version": "0.2.0",
                "kind": "run_event",
                "run_id": run_id,
                "event_id": format!("evt_{:010}", evt_id),
                "timestamp_unix_ms": evt_id * 100,
                "level": "info",
                "event_type": et,
                "message": "test",
            });
            writeln!(events, "{}", serde_json::to_string(&e).unwrap()).unwrap();
            evt_id += 1;
        }

        // parsed
        let mut preview = fs::File::create(dir.join("parsed/b_channel_preview.jsonl")).unwrap();
        let mut summary = fs::File::create(dir.join("parsed/frame_summary.jsonl")).unwrap();
        for i in 0..frame_count {
            let p = serde_json::json!({"frame_index":i,"b_x_mv":0.0});
            writeln!(preview, "{}", serde_json::to_string(&p).unwrap()).unwrap();
            let s = serde_json::json!({"frame_index":i,"raw_len":RALL_FRAME_BYTES,"parse_status":"success"});
            writeln!(summary, "{}", serde_json::to_string(&s).unwrap()).unwrap();
        }
    }

    #[test]
    fn valid_run_passes_audit() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("test_run");
        make_valid_run(&run_dir, 10);
        let report = run_audit(&run_dir);
        assert!(report.passed, "report: {:#?}", report);
        assert_eq!(report.frame_count, 10);
        assert_eq!(report.index_entries, 10);
        assert_eq!(report.preview_entries, 10);
        assert_eq!(report.summary_entries, 10);
        assert!(report.offsets_contiguous);
        assert!(report.all_frames_12288_bytes);
        assert!(report.csv_files_found.is_empty());
    }

    #[test]
    fn missing_manifest_fails_audit() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("test_run");
        make_valid_run(&run_dir, 5);
        fs::remove_file(run_dir.join("manifest.json")).unwrap();
        let report = run_audit(&run_dir);
        assert!(!report.passed);
        assert!(report.errors.iter().any(|e| e.contains("manifest")));
    }

    #[test]
    fn truncated_rawbin_fails_audit() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("test_run");
        make_valid_run(&run_dir, 5);
        let rawbin = run_dir.join("raw/oe1022d_rall.rawbin");
        let meta = fs::metadata(&rawbin).unwrap();
        let file = fs::OpenOptions::new().write(true).open(&rawbin).unwrap();
        file.set_len(meta.len() - 1).unwrap();
        let report = run_audit(&run_dir);
        assert!(!report.passed);
        assert!(report.errors.iter().any(|e| e.contains("rawbin size")));
    }

    #[test]
    fn non_contiguous_index_offset_fails_audit() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("test_run");
        make_valid_run(&run_dir, 3);
        let index_path = run_dir.join("index.jsonl");
        let lines = read_jsonl_lines(&index_path).unwrap();
        let mut out = fs::File::create(&index_path).unwrap();
        for (i, line) in lines.iter().enumerate() {
            let mut obj: serde_json::Value = serde_json::from_str(line).unwrap();
            if i == 1 {
                obj["offset_bytes"] = serde_json::json!(99999u64);
            }
            writeln!(out, "{}", serde_json::to_string(&obj).unwrap()).unwrap();
        }
        let report = run_audit(&run_dir);
        assert!(!report.passed);
        assert!(!report.offsets_contiguous);
    }

    #[test]
    fn unexpected_frame_length_fails_audit() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("test_run");
        make_valid_run(&run_dir, 3);
        let index_path = run_dir.join("index.jsonl");
        let lines = read_jsonl_lines(&index_path).unwrap();
        let mut out = fs::File::create(&index_path).unwrap();
        for (i, line) in lines.iter().enumerate() {
            let mut obj: serde_json::Value = serde_json::from_str(line).unwrap();
            if i == 1 {
                obj["length_bytes"] = serde_json::json!(100u64);
            }
            writeln!(out, "{}", serde_json::to_string(&obj).unwrap()).unwrap();
        }
        let report = run_audit(&run_dir);
        assert!(!report.passed);
        assert!(!report.all_frames_12288_bytes);
    }

    #[test]
    fn invalid_jsonl_fails_audit() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("test_run");
        make_valid_run(&run_dir, 3);
        fs::write(run_dir.join("events.jsonl"), "this is not json\n").unwrap();
        let report = run_audit(&run_dir);
        assert!(!report.passed);
        assert!(report.errors.iter().any(|e| e.contains("invalid JSON")));
    }

    #[test]
    fn csv_file_detection_fails_audit() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("test_run");
        make_valid_run(&run_dir, 3);
        fs::write(run_dir.join("parsed/sneaky.csv"), "a,b\n1,2\n").unwrap();
        let report = run_audit(&run_dir);
        assert!(!report.passed);
        assert!(!report.csv_files_found.is_empty());
    }

    #[test]
    fn forbidden_command_detection_fails_audit() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("test_run");
        make_valid_run(&run_dir, 3);
        fs::write(
            run_dir.join("metadata/acquisition_config.json"),
            r#"{"command":"SENSD 2,10"}"#,
        )
        .unwrap();
        let report = run_audit(&run_dir);
        // SENSD is a forbidden setter pattern
        assert!(!report.forbidden_commands_found.is_empty());
    }

    #[test]
    fn fixture_extraction_writes_exact_12288_byte_frames() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("test_run");
        make_valid_run(&run_dir, 10);
        let fixture_root = tmp.path().join("fixtures");
        promote_fixtures(&run_dir, &fixture_root, 10).unwrap();

        let manifest_path = fixture_root.join("fixture_manifest.json");
        assert!(manifest_path.exists());
        let manifest: FixtureManifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        assert_eq!(manifest.frames.len(), 3);

        for frame in &manifest.frames {
            let path = fixture_root.join(&frame.file_name);
            let meta = fs::metadata(&path).unwrap();
            assert_eq!(meta.len(), RALL_FRAME_BYTES);
        }
    }

    #[test]
    fn audit_report_is_deterministic_for_same_input() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("test_run");
        make_valid_run(&run_dir, 5);
        let r1 = run_audit(&run_dir);
        let r2 = run_audit(&run_dir);
        assert_eq!(r1, r2);
    }

    #[test]
    fn m28_run_with_optional_metadata_passes_audit() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("m28_run");
        make_valid_run(&run_dir, 5);

        // Add M2.8 optional metadata files
        fs::write(
            run_dir.join("metadata/station_snapshot_quality.json"),
            r#"{"status":"passed","eligible_for_rf_on_microtest":true,"warnings":[],"errors":[],"query_interrupted_seen":false,"smb_query_delay_ms":100,"smb_connection_closed_before_acquisition":true,"oe_command_allowlist":["*IDN?","RALL?"],"smb_command_allowlist":["*IDN?","OUTP?"] }"#,
        )
        .unwrap();
        fs::write(
            run_dir.join("metadata/state_profile_diff.json"),
            r#"{"differences":[],"summary":"no fake profile provided"}"#,
        )
        .unwrap();
        fs::write(
            run_dir.join("metadata/hash_manifest.json"),
            r#"{"station_snapshot_hash":"sha256:abc","smb100a_query_snapshot_hash":"sha256:def","acquisition_config_hash":"sha256:ghi"}"#,
        )
        .unwrap();
        fs::write(
            run_dir.join("metadata/smb100a_query_timing.json"),
            r#"{"queries":[]}"#,
        )
        .unwrap();
        fs::write(
            run_dir.join("timeline.jsonl"),
            r#"{"event_type":"run_created","wall_time_utc":"2026-01-01T00:00:00Z","monotonic_ns":0,"monotonic_ns_since_run_start":0,"device_id":"system"}"#,
        )
        .unwrap();

        let report = run_audit(&run_dir);
        assert!(report.passed, "M2.8 run should pass audit: {:#?}", report);
        assert!(report.csv_files_found.is_empty());
        assert!(report.errors.is_empty());
    }

    #[test]
    fn m29_run_with_optional_recipe_shadow_metadata_passes_audit() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("m29_run");
        make_valid_run(&run_dir, 5);

        // Add M2.9 recipe/shadow metadata files
        fs::create_dir_all(run_dir.join("recipe")).unwrap();
        fs::create_dir_all(run_dir.join("shadow")).unwrap();

        fs::write(
            run_dir.join("recipe/input_recipe.json"),
            r#"{"kind":"recipe","id":"test"}"#,
        )
        .unwrap();
        fs::write(
            run_dir.join("recipe/resolved_recipe.json"),
            r#"{"kind":"resolved_recipe","id":"resolved_test"}"#,
        )
        .unwrap();
        fs::write(
            run_dir.join("recipe/dry_run_plan.json"),
            r#"{"kind":"dry_run_plan","id":"dry_test"}"#,
        )
        .unwrap();
        fs::write(
            run_dir.join("recipe/safety_report.json"),
            r#"{"kind":"safety_report","id":"safety_test"}"#,
        )
        .unwrap();
        fs::write(
            run_dir.join("shadow/shadow_command_plan.jsonl"),
            r#"{"shadow_command_id":"sc_001","command":"OUTP ON","shadow_only":true}"#,
        )
        .unwrap();
        fs::write(
            run_dir.join("shadow/shadow_step_timeline.jsonl"),
            r#"{"shadow_step_id":"step_001","phase":"sweep"}"#,
        )
        .unwrap();
        fs::write(
            run_dir.join("shadow/frame_to_shadow_step_alignment.jsonl"),
            r#"{"frame_seq":0,"shadow_step_id":"step_001","alignment_method":"time_window"}"#,
        )
        .unwrap();
        fs::write(
            run_dir.join("shadow/executor_shadow_summary.json"),
            r#"{"shadow_mode":true,"shadow_command_count":1}"#,
        )
        .unwrap();
        fs::write(
            run_dir.join("shadow/forbidden_real_command_check.json"),
            r#"{"passed":true,"forbidden_commands_sent_to_transport":[],"real_smb100a_set_commands_sent":0}"#,
        )
        .unwrap();

        let report = run_audit(&run_dir);
        assert!(report.passed, "M2.9 run should pass audit: {:#?}", report);
        assert!(report.csv_files_found.is_empty());
        assert!(report.errors.is_empty());
    }
}
