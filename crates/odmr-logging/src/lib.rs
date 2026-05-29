//! odmr-logging — Layer 3 run artifact and raw-first persistence layer.
//!
//! Creates run directories, writes metadata lock files, events/index JSONL,
//! and provides an append-only raw bin writer.
//!
//! No hardware access. No fake devices. No executor. No GUI. No CSV.

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during logging operations.
#[derive(Debug)]
pub enum LoggingError {
    Io(io::Error),
    Json(serde_json::Error),
    RunDirectoryExists(PathBuf),
}

impl std::fmt::Display for LoggingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoggingError::Io(e) => write!(f, "io error: {e}"),
            LoggingError::Json(e) => write!(f, "json error: {e}"),
            LoggingError::RunDirectoryExists(p) => {
                write!(f, "run directory already exists: {}", p.display())
            }
        }
    }
}

impl std::error::Error for LoggingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoggingError::Io(e) => Some(e),
            LoggingError::Json(e) => Some(e),
            LoggingError::RunDirectoryExists(_) => None,
        }
    }
}

impl From<io::Error> for LoggingError {
    fn from(e: io::Error) -> Self {
        LoggingError::Io(e)
    }
}

impl From<serde_json::Error> for LoggingError {
    fn from(e: serde_json::Error) -> Self {
        LoggingError::Json(e)
    }
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Severity level for a run event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Classification of a run event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunEventType {
    RunCreated,
    ArtifactWritten,
    SafetyChecked,
    RunStarted,
    StepStarted,
    StepCompleted,
    RunCompleted,
    RunFailed,
}

// ---------------------------------------------------------------------------
// RunManifest
// ---------------------------------------------------------------------------

/// Top-level manifest for a single experiment run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunManifest {
    pub schema_version: String,
    pub kind: String,
    pub run_id: String,
    pub created_at_unix_ms: u64,
    pub artifact_paths: RunArtifactPaths,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_recipe_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_report_id: Option<String>,
}

/// Relative paths to all artifacts within a run directory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunArtifactPaths {
    pub manifest: String,
    pub station_snapshot: String,
    pub recipe_lock: String,
    pub resolved_recipe_lock: String,
    pub dry_run_plan_lock: String,
    pub safety_report_lock: String,
    pub events: String,
    pub index: String,
    pub raw_bin: String,
}

impl Default for RunArtifactPaths {
    fn default() -> Self {
        Self {
            manifest: "manifest.json".into(),
            station_snapshot: "metadata/station_snapshot.json".into(),
            recipe_lock: "metadata/recipe.lock.json".into(),
            resolved_recipe_lock: "metadata/resolved_recipe.lock.json".into(),
            dry_run_plan_lock: "metadata/dry_run_plan.lock.json".into(),
            safety_report_lock: "metadata/safety_report.lock.json".into(),
            events: "events.jsonl".into(),
            index: "index.jsonl".into(),
            raw_bin: "raw/oe1022d.rawbin".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// RunEvent
// ---------------------------------------------------------------------------

/// A single event emitted during an experiment run.
///
/// Stored as one compact JSON object per line in `events.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunEvent {
    pub schema_version: String,
    pub kind: String,
    pub run_id: String,
    pub event_id: String,
    pub timestamp_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_monotonic_ns: Option<u64>,
    pub level: EventLevel,
    pub event_type: RunEventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// RawIndexEntry
// ---------------------------------------------------------------------------

/// Index record pointing to a frame inside the raw bin file.
///
/// Stored as one compact JSON object per line in `index.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawIndexEntry {
    pub schema_version: String,
    pub kind: String,
    pub run_id: String,
    pub stream_id: String,
    pub offset_bytes: u64,
    pub length_bytes: u64,
    pub timestamp_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_count: Option<u64>,
}

// ---------------------------------------------------------------------------
// RunDirectory
// ---------------------------------------------------------------------------

/// Represents a created run directory and provides methods to write artifacts.
pub struct RunDirectory {
    root: PathBuf,
    run_id: String,
}

/// Create a new run directory under `root` with the given `run_id`.
///
/// Returns `LoggingError::RunDirectoryExists` if the directory already exists.
pub fn create_run_directory(root: &Path, run_id: &str) -> Result<RunDirectory, LoggingError> {
    RunDirectory::create(root, run_id)
}

impl RunDirectory {
    /// Create a new run directory with the exact layout specified in M1.4.
    pub fn create(root: &Path, run_id: &str) -> Result<Self, LoggingError> {
        let run_path = root.join(run_id);
        if run_path.exists() {
            return Err(LoggingError::RunDirectoryExists(run_path));
        }

        fs::create_dir_all(&run_path)?;
        fs::create_dir_all(run_path.join("metadata"))?;
        fs::create_dir_all(run_path.join("raw"))?;

        Ok(Self {
            root: root.to_path_buf(),
            run_id: run_id.to_string(),
        })
    }

    /// Return the full path to the run directory.
    pub fn run_directory_path(&self) -> PathBuf {
        self.root.join(&self.run_id)
    }

    /// Write a generic JSON artifact to a relative path inside the run directory.
    ///
    /// The JSON is pretty-printed for human readability.
    pub fn write_json_artifact<T: Serialize>(
        &self,
        relative_path: &str,
        value: &T,
    ) -> Result<(), LoggingError> {
        let path = self.run_directory_path().join(relative_path);
        let json = serde_json::to_string_pretty(value)?;
        fs::write(&path, json)?;
        Ok(())
    }

    pub fn write_station_snapshot_json<T: Serialize>(&self, value: &T) -> Result<(), LoggingError> {
        self.write_json_artifact("metadata/station_snapshot.json", value)
    }

    pub fn write_recipe_lock_json<T: Serialize>(&self, value: &T) -> Result<(), LoggingError> {
        self.write_json_artifact("metadata/recipe.lock.json", value)
    }

    pub fn write_resolved_recipe_lock_json<T: Serialize>(
        &self,
        value: &T,
    ) -> Result<(), LoggingError> {
        self.write_json_artifact("metadata/resolved_recipe.lock.json", value)
    }

    pub fn write_dry_run_plan_lock_json<T: Serialize>(
        &self,
        value: &T,
    ) -> Result<(), LoggingError> {
        self.write_json_artifact("metadata/dry_run_plan.lock.json", value)
    }

    pub fn write_safety_report_lock_json<T: Serialize>(
        &self,
        value: &T,
    ) -> Result<(), LoggingError> {
        self.write_json_artifact("metadata/safety_report.lock.json", value)
    }

    pub fn write_manifest(&self, manifest: &RunManifest) -> Result<(), LoggingError> {
        self.write_json_artifact("manifest.json", manifest)
    }

    pub fn open_event_writer(&self) -> Result<EventWriter, LoggingError> {
        let path = self.run_directory_path().join("events.jsonl");
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(EventWriter {
            writer: BufWriter::new(file),
        })
    }

    pub fn open_index_writer(&self) -> Result<IndexWriter, LoggingError> {
        let path = self.run_directory_path().join("index.jsonl");
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(IndexWriter {
            writer: BufWriter::new(file),
        })
    }

    pub fn open_raw_bin_writer(&self) -> Result<RawBinWriter, LoggingError> {
        let path = self.run_directory_path().join("raw").join("oe1022d.rawbin");
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(RawBinWriter { file })
    }
}

// ---------------------------------------------------------------------------
// Writers
// ---------------------------------------------------------------------------

/// Buffered writer for `events.jsonl`.
pub struct EventWriter {
    writer: BufWriter<File>,
}

impl EventWriter {
    /// Serialize a single event to one JSONL line and flush the buffer.
    pub fn write_event(&mut self, event: &RunEvent) -> Result<(), LoggingError> {
        let line = serde_json::to_string(event)?;
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }
}

/// Buffered writer for `index.jsonl`.
pub struct IndexWriter {
    writer: BufWriter<File>,
}

impl IndexWriter {
    /// Serialize a single index entry to one JSONL line and flush the buffer.
    pub fn write_entry(&mut self, entry: &RawIndexEntry) -> Result<(), LoggingError> {
        let line = serde_json::to_string(entry)?;
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }
}

/// Append-only binary writer for the raw bin file.
pub struct RawBinWriter {
    file: File,
}

impl RawBinWriter {
    /// Append a raw frame to the bin file and return its index entry.
    ///
    /// The returned `RawIndexEntry` has `offset_bytes` and `length_bytes` set
    /// correctly. The caller should fill in `run_id`, `timestamp_unix_ms`,
    /// `step_id`, and `sample_count` as appropriate.
    pub fn append_frame(&mut self, bytes: &[u8]) -> Result<RawIndexEntry, LoggingError> {
        let offset = self.file.seek(SeekFrom::End(0))?;
        self.file.write_all(bytes)?;
        self.file.flush()?;
        Ok(RawIndexEntry {
            schema_version: "0.2.0".into(),
            kind: "raw_index_entry".into(),
            run_id: String::new(),
            stream_id: "oe1022d".into(),
            offset_bytes: offset,
            length_bytes: bytes.len() as u64,
            timestamp_unix_ms: 0,
            step_id: None,
            sample_count: None,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_run_dir() -> (tempfile::TempDir, RunDirectory) {
        let tmp = tempfile::tempdir().unwrap();
        let run = RunDirectory::create(tmp.path(), "test_run_001").unwrap();
        (tmp, run)
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    #[test]
    fn create_run_directory_creates_exact_layout() {
        let (_tmp, run) = temp_run_dir();
        let root = run.run_directory_path();

        assert!(root.is_dir());
        assert!(root.join("metadata").is_dir());
        assert!(root.join("raw").is_dir());
    }

    #[test]
    fn metadata_json_artifacts_are_written() {
        let (_tmp, run) = temp_run_dir();
        let value = serde_json::json!({ "key": "value", "number": 42 });

        run.write_station_snapshot_json(&value).unwrap();
        run.write_recipe_lock_json(&value).unwrap();
        run.write_resolved_recipe_lock_json(&value).unwrap();
        run.write_dry_run_plan_lock_json(&value).unwrap();
        run.write_safety_report_lock_json(&value).unwrap();

        let root = run.run_directory_path();
        assert!(root.join("metadata/station_snapshot.json").exists());
        assert!(root.join("metadata/recipe.lock.json").exists());
        assert!(root.join("metadata/resolved_recipe.lock.json").exists());
        assert!(root.join("metadata/dry_run_plan.lock.json").exists());
        assert!(root.join("metadata/safety_report.lock.json").exists());

        // Verify readable
        let content = fs::read_to_string(root.join("metadata/station_snapshot.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["key"], "value");
    }

    #[test]
    fn manifest_is_written_and_readable() {
        let (_tmp, run) = temp_run_dir();
        let manifest = RunManifest {
            schema_version: "0.2.0".into(),
            kind: "run_manifest".into(),
            run_id: "test_run_001".into(),
            created_at_unix_ms: now_ms(),
            artifact_paths: RunArtifactPaths::default(),
            recipe_hash: Some("abcd".into()),
            resolved_recipe_id: Some("resolved_001".into()),
            safety_report_id: Some("safety_001".into()),
        };

        run.write_manifest(&manifest).unwrap();

        let path = run.run_directory_path().join("manifest.json");
        let content = fs::read_to_string(&path).unwrap();
        let parsed: RunManifest = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.run_id, "test_run_001");
        assert_eq!(parsed.recipe_hash, Some("abcd".into()));
    }

    #[test]
    fn event_writer_writes_valid_jsonl() {
        let (_tmp, run) = temp_run_dir();
        let mut writer = run.open_event_writer().unwrap();

        let event = RunEvent {
            schema_version: "0.2.0".into(),
            kind: "run_event".into(),
            run_id: "test_run_001".into(),
            event_id: "evt_000001".into(),
            timestamp_unix_ms: now_ms(),
            timestamp_monotonic_ns: None,
            level: EventLevel::Info,
            event_type: RunEventType::RunCreated,
            step_id: None,
            device_id: None,
            message: "Run directory created".into(),
            data: None,
        };

        writer.write_event(&event).unwrap();

        let path = run.run_directory_path().join("events.jsonl");
        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1);

        let parsed: RunEvent = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed.event_id, "evt_000001");
    }

    #[test]
    fn index_writer_writes_valid_jsonl() {
        let (_tmp, run) = temp_run_dir();
        let mut writer = run.open_index_writer().unwrap();

        let entry = RawIndexEntry {
            schema_version: "0.2.0".into(),
            kind: "raw_index_entry".into(),
            run_id: "test_run_001".into(),
            stream_id: "oe1022d".into(),
            offset_bytes: 0,
            length_bytes: 64,
            timestamp_unix_ms: now_ms(),
            step_id: Some("step_000001".into()),
            sample_count: Some(16),
        };

        writer.write_entry(&entry).unwrap();

        let path = run.run_directory_path().join("index.jsonl");
        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1);

        let parsed: RawIndexEntry = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed.offset_bytes, 0);
        assert_eq!(parsed.length_bytes, 64);
    }

    #[test]
    fn raw_bin_writer_appends_bytes_and_reports_correct_offset_length() {
        let (_tmp, run) = temp_run_dir();
        let mut writer = run.open_raw_bin_writer().unwrap();

        let frame1 = b"hello world";
        let entry1 = writer.append_frame(frame1).unwrap();
        assert_eq!(entry1.offset_bytes, 0);
        assert_eq!(entry1.length_bytes, frame1.len() as u64);

        let frame2 = b"second frame";
        let entry2 = writer.append_frame(frame2).unwrap();
        assert_eq!(entry2.offset_bytes, frame1.len() as u64);
        assert_eq!(entry2.length_bytes, frame2.len() as u64);
    }

    #[test]
    fn raw_bin_file_size_equals_sum_of_frame_lengths() {
        let (_tmp, run) = temp_run_dir();
        let mut writer = run.open_raw_bin_writer().unwrap();

        let frames: Vec<&[u8]> = vec![b"abc", b"defghij", b"klmnop"];
        let mut total = 0usize;
        for frame in &frames {
            writer.append_frame(frame).unwrap();
            total += frame.len();
        }

        let path = run.run_directory_path().join("raw/oe1022d.rawbin");
        let metadata = fs::metadata(&path).unwrap();
        assert_eq!(metadata.len() as usize, total);
    }

    #[test]
    fn writing_two_events_produces_two_lines() {
        let (_tmp, run) = temp_run_dir();
        let mut writer = run.open_event_writer().unwrap();

        for i in 0..2 {
            let event = RunEvent {
                schema_version: "0.2.0".into(),
                kind: "run_event".into(),
                run_id: "test_run_001".into(),
                event_id: format!("evt_{:06}", i),
                timestamp_unix_ms: now_ms(),
                timestamp_monotonic_ns: None,
                level: EventLevel::Info,
                event_type: RunEventType::RunCreated,
                step_id: None,
                device_id: None,
                message: format!("event {}", i),
                data: None,
            };
            writer.write_event(&event).unwrap();
        }

        let path = run.run_directory_path().join("events.jsonl");
        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn duplicate_run_directory_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let _run1 = RunDirectory::create(tmp.path(), "duplicate_run").unwrap();
        let result = RunDirectory::create(tmp.path(), "duplicate_run");
        assert!(matches!(result, Err(LoggingError::RunDirectoryExists(_))));
    }

    #[test]
    fn no_csv_files_created_anywhere() {
        let (_tmp, run) = temp_run_dir();
        // Write some artifacts to exercise the directory
        let value = serde_json::json!({ "test": true });
        run.write_manifest(&RunManifest {
            schema_version: "0.2.0".into(),
            kind: "run_manifest".into(),
            run_id: "test".into(),
            created_at_unix_ms: now_ms(),
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
            "no CSV files should be created"
        );
    }

    #[test]
    fn no_hardware_or_fake_device_dependency() {
        let (_tmp, run) = temp_run_dir();
        let mut ew = run.open_event_writer().unwrap();
        let event = RunEvent {
            schema_version: "0.2.0".into(),
            kind: "run_event".into(),
            run_id: "test".into(),
            event_id: "evt_001".into(),
            timestamp_unix_ms: now_ms(),
            timestamp_monotonic_ns: None,
            level: EventLevel::Info,
            event_type: RunEventType::RunCreated,
            step_id: None,
            device_id: None,
            message: "pure filesystem test".into(),
            data: None,
        };
        ew.write_event(&event).unwrap();
        // If we reach here, no hardware or fake device was invoked.
    }

    #[test]
    fn run_directory_provides_correct_path() {
        let tmp = tempfile::tempdir().unwrap();
        let run = RunDirectory::create(tmp.path(), "my_run").unwrap();
        let expected = tmp.path().join("my_run");
        assert_eq!(run.run_directory_path(), expected);
    }
}
