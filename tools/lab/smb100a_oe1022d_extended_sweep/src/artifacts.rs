use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("sha256:{:x}", hasher.finalize())
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    Ok(sha256_bytes(&data))
}

pub fn write_jsonl<T: serde::Serialize>(
    path: &PathBuf,
    rows: &[T],
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut writer = std::io::BufWriter::new(file);
    for row in rows {
        let line = serde_json::to_string(row)?;
        writeln!(writer, "{}", line)?;
    }
    writer.flush()?;
    Ok(())
}
