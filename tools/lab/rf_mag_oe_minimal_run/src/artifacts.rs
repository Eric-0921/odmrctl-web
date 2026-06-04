//! Artifact output: JSON and JSONL file writers.

use crate::types::*;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn write_all_artifacts(
    out_dir: &Path,
    manifest: &CombinedRunManifest,
    report: &CombinedRunReport,
    events: &[CombinedRunEvent],
    smb_audit: &[CommandAuditEntry],
    maynuo_audit: &[CommandAuditEntry],
    oe_audit: &[CommandAuditEntry],
    smb_snapshot: &SmbSnapshot,
    oe_snapshot: &OeSnapshot,
    mag_snapshot: &MagSnapshot,
) -> Result<Vec<String>, String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("create out dir: {e}"))?;

    let mut files = Vec::new();

    write_json(out_dir, "manifest.json", manifest)?;
    files.push("manifest.json".into());

    write_json(out_dir, "combined_run_report.json", report)?;
    files.push("combined_run_report.json".into());

    write_jsonl(out_dir, "combined_events.jsonl", events)?;
    files.push("combined_events.jsonl".into());

    write_jsonl(out_dir, "smb_command_audit.jsonl", smb_audit)?;
    files.push("smb_command_audit.jsonl".into());

    write_jsonl(out_dir, "maynuo_command_audit.jsonl", maynuo_audit)?;
    files.push("maynuo_command_audit.jsonl".into());

    write_jsonl(out_dir, "oe_command_audit.jsonl", oe_audit)?;
    files.push("oe_command_audit.jsonl".into());

    write_json(out_dir, "rf_snapshot.json", smb_snapshot)?;
    files.push("rf_snapshot.json".into());

    write_json(out_dir, "oe_snapshot.json", oe_snapshot)?;
    files.push("oe_snapshot.json".into());

    write_json(out_dir, "magnetic_snapshot.json", mag_snapshot)?;
    files.push("magnetic_snapshot.json".into());

    Ok(files)
}

pub fn write_raw_bin(out_dir: &Path, data: &[u8]) -> Result<(), String> {
    let path = out_dir.join("raw.bin");
    fs::write(&path, data).map_err(|e| format!("write raw.bin: {e}"))
}

pub fn write_jsonl_lines(out_dir: &Path, filename: &str, lines: &[String]) -> Result<(), String> {
    let path = out_dir.join(filename);
    let file = fs::File::create(&path).map_err(|e| format!("create {}: {e}", path.display()))?;
    let mut writer = std::io::BufWriter::new(file);
    for line in lines {
        writeln!(writer, "{line}").map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    writer.flush().map_err(|e| format!("flush {}: {e}", path.display()))?;
    Ok(())
}

fn write_json<T: Serialize>(out_dir: &Path, filename: &str, value: &T) -> Result<(), String> {
    let path = out_dir.join(filename);
    let json =
        serde_json::to_string_pretty(value).map_err(|e| format!("serialize {filename}: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write {}: {e}", path.display()))
}

fn write_jsonl<T: Serialize>(
    out_dir: &Path,
    filename: &str,
    entries: &[T],
) -> Result<(), String> {
    let path = out_dir.join(filename);
    let file = fs::File::create(&path).map_err(|e| format!("create {}: {e}", path.display()))?;
    let mut writer = std::io::BufWriter::new(file);
    for entry in entries {
        let line = serde_json::to_string(entry).map_err(|e| format!("serialize jsonl: {e}"))?;
        writeln!(writer, "{line}").map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    writer
        .flush()
        .map_err(|e| format!("flush {}: {e}", path.display()))?;
    Ok(())
}
