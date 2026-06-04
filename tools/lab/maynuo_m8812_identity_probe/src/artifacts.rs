//! Artifact file generation for the Maynuo M8812 identity probe tool.

use crate::types::{
    AxisMapping, IdentitySnapshot, ProbeEvent, ProbeManifest, ProbeReport,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Writes all artifact files to the output directory.
pub fn write_artifacts(
    out_dir: &Path,
    manifest: &ProbeManifest,
    snapshot: &IdentitySnapshot,
    mapping: &AxisMapping,
    report: &ProbeReport,
    events: &[ProbeEvent],
) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("create out dir: {e}"))?;

    write_json(out_dir, "manifest.json", manifest)?;
    write_json(out_dir, "maynuo_identity_snapshot.json", snapshot)?;
    write_json(out_dir, "maynuo_axis_mapping.json", mapping)?;
    write_json(out_dir, "maynuo_probe_report.json", report)?;
    write_jsonl(out_dir, "maynuo_identity_events.jsonl", events)?;

    Ok(())
}

fn write_json<T: serde::Serialize>(out_dir: &Path, filename: &str, value: &T) -> Result<(), String> {
    let path = out_dir.join(filename);
    let json = serde_json::to_string_pretty(value).map_err(|e| format!("serialize {filename}: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write {filename}: {e}"))?;
    Ok(())
}

fn write_jsonl(out_dir: &Path, filename: &str, events: &[ProbeEvent]) -> Result<(), String> {
    let path = out_dir.join(filename);
    let mut f = fs::File::create(&path).map_err(|e| format!("create {filename}: {e}"))?;
    for event in events {
        let line = serde_json::to_string(event).map_err(|e| format!("serialize event: {e}"))?;
        writeln!(f, "{line}").map_err(|e| format!("write {filename}: {e}"))?;
    }
    Ok(())
}

/// Compute SHA-256 hex digest of a file's content.
#[allow(dead_code)]
pub fn sha256_hex(path: &Path) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}
