//! Artifact output: JSON and JSONL file writers.

use crate::types::{
    CommandAuditEntry, ZeroBaselineEvent, ZeroBaselineManifest, ZeroBaselineReport,
    ZeroBaselineSnapshot,
};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Write all five artifact files to `out_dir`.
pub fn write_artifacts(
    out_dir: &Path,
    manifest: &ZeroBaselineManifest,
    snapshot: &ZeroBaselineSnapshot,
    report: &ZeroBaselineReport,
    events: &[ZeroBaselineEvent],
    audit: &[CommandAuditEntry],
) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("create out dir: {e}"))?;

    write_json(out_dir, "manifest.json", manifest)?;
    write_json(out_dir, "zero_baseline_snapshot.json", snapshot)?;
    write_json(out_dir, "zero_baseline_report.json", report)?;
    write_jsonl(out_dir, "zero_baseline_events.jsonl", events)?;
    write_jsonl(out_dir, "command_audit.jsonl", audit)?;

    Ok(())
}

fn write_json<T: Serialize>(out_dir: &Path, filename: &str, value: &T) -> Result<(), String> {
    let path = out_dir.join(filename);
    let json = serde_json::to_string_pretty(value).map_err(|e| format!("serialize {filename}: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}

fn write_jsonl<T: Serialize>(out_dir: &Path, filename: &str, entries: &[T]) -> Result<(), String> {
    let path = out_dir.join(filename);
    let file = fs::File::create(&path).map_err(|e| format!("create {}: {e}", path.display()))?;
    let mut writer = std::io::BufWriter::new(file);
    for entry in entries {
        let line = serde_json::to_string(entry).map_err(|e| format!("serialize jsonl: {e}"))?;
        writeln!(writer, "{line}").map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    writer.flush().map_err(|e| format!("flush {}: {e}", path.display()))?;
    Ok(())
}

#[allow(dead_code)]
pub fn sha256_hex(path: &Path) -> Result<String, String> {
    use sha2::Digest;
    let data = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let hash = sha2::Sha256::digest(&data);
    Ok(hex::encode(hash))
}
