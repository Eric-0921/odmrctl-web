//! SHA-256 hashing and JSONL writing utilities.

use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[allow(dead_code)]
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    Ok(sha256_bytes(&data))
}

#[allow(dead_code)]
pub fn sha256_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    let json = serde_json::to_string(value).map_err(|e| format!("json: {}", e))?;
    Ok(sha256_bytes(json.as_bytes()))
}

pub fn write_jsonl<T: serde::Serialize>(path: &Path, rows: &[T]) -> Result<(), String> {
    let mut buf = Vec::new();
    for row in rows {
        let line = serde_json::to_string(row).map_err(|e| format!("json: {}", e))?;
        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
    }
    fs::write(path, &buf).map_err(|e| format!("write {}: {}", path.display(), e))?;
    Ok(())
}

#[allow(dead_code)]
pub fn append_jsonl<T: serde::Serialize>(path: &Path, row: &T) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open {}: {}", path.display(), e))?;
    let line = serde_json::to_string(row).map_err(|e| format!("json: {}", e))?;
    writeln!(file, "{}", line).map_err(|e| format!("write {}: {}", path.display(), e))?;
    Ok(())
}
