//! odmr-replay — replay canonical `.rall` ODMR run artifacts.
//!
//! Canonical input layout:
//! - `events.jsonl`
//! - `index.jsonl`
//! - `raw/<step>.rall` or `raw/*.rall`
//! - optional `manifest.json`, `metadata/*`, `summary/*`
//!
//! Legacy compatibility:
//! - `raw/*.rawbin` or `rawbin + manifest` directories are adapted into the
//!   same typed replay session without exposing raw parsing details to GUI code.

use odmr_oe1022d::{parse_rall_frame, RallFrame, RALL_FRAME_BYTES};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ReplayError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Json(e) => write!(f, "json error: {e}"),
            Self::Invalid(msg) => write!(f, "invalid replay input: {msg}"),
        }
    }
}

impl std::error::Error for ReplayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Invalid(_) => None,
        }
    }
}

impl From<std::io::Error> for ReplayError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ReplayError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplayMode {
    AsFastAsPossible,
    OriginalTimestampPaced,
    ParseOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReplaySource {
    CanonicalRunDirectory { root: PathBuf },
    LegacyRawBinDirectory { root: PathBuf },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplaySession {
    pub source: ReplaySource,
    pub run_root: PathBuf,
    pub metadata: ReplayRunMetadata,
    pub events: Vec<ReplayEvent>,
    pub traces: Vec<ReplayTraceFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReplayRunMetadata {
    #[serde(default)]
    pub manifest: Option<serde_json::Value>,
    #[serde(default)]
    pub summary: Option<serde_json::Value>,
    #[serde(default)]
    pub index_entries: Vec<ReplayIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayEvent {
    #[serde(default)]
    pub schema_version: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub timestamp_unix_ms: Option<u64>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayIndexEntry {
    #[serde(default)]
    pub schema_version: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub step_id: Option<String>,
    #[serde(default)]
    pub step_index: Option<u64>,
    #[serde(default)]
    pub raw_path: Option<String>,
    #[serde(default)]
    pub timestamp_unix_ms: Option<u64>,
    #[serde(default)]
    pub offset_bytes: Option<u64>,
    #[serde(default)]
    pub length_bytes: Option<u64>,
    #[serde(default)]
    pub frame_index: Option<u64>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayTraceFrame {
    pub step_id: Option<String>,
    pub step_index: Option<u64>,
    pub frame_index: u64,
    pub source_path: String,
    pub timestamp_unix_ms: Option<u64>,
    pub raw_bytes: Vec<u8>,
    #[serde(default)]
    pub parsed: Option<ParsedRallFrameSummary>,
    #[serde(default)]
    pub parse_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedRallFrameSummary {
    pub sample_count: usize,
    pub latest_b_freq_hz: Option<f64>,
    pub latest_b_x_mv: Option<f64>,
    pub latest_b_y_mv: Option<f64>,
    pub padding_all_zero: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MigrationReport {
    pub source_root: PathBuf,
    pub output_root: PathBuf,
    pub files_written: Vec<String>,
    pub steps_migrated: usize,
    pub frames_migrated: usize,
}

pub fn open_replay_session(source: ReplaySource) -> Result<ReplaySession, ReplayError> {
    match &source {
        ReplaySource::CanonicalRunDirectory { root } => {
            open_canonical_session(root, source.clone())
        }
        ReplaySource::LegacyRawBinDirectory { root } => open_legacy_session(root, source.clone()),
    }
}

pub fn replay_trace(
    session: &ReplaySession,
    step_id: Option<&str>,
    mode: ReplayMode,
) -> Vec<ReplayTraceFrame> {
    let mut frames: Vec<_> = session
        .traces
        .iter()
        .filter(|frame| {
            step_id
                .map(|want| frame.step_id.as_deref() == Some(want))
                .unwrap_or(true)
        })
        .cloned()
        .collect();
    if matches!(mode, ReplayMode::ParseOnly) {
        for frame in &mut frames {
            frame.timestamp_unix_ms = None;
        }
    }
    frames
}

pub fn migrate_legacy_run_to_canonical(
    source_root: impl AsRef<Path>,
    output_root: impl AsRef<Path>,
) -> Result<MigrationReport, ReplayError> {
    let source_root = source_root.as_ref();
    let output_root = output_root.as_ref();
    let session = open_legacy_session(
        source_root,
        ReplaySource::LegacyRawBinDirectory {
            root: source_root.to_path_buf(),
        },
    )?;

    fs::create_dir_all(output_root.join("raw"))?;
    fs::create_dir_all(output_root.join("metadata"))?;
    fs::create_dir_all(output_root.join("summary"))?;

    if let Some(manifest) = &session.metadata.manifest {
        fs::write(
            output_root.join("manifest.json"),
            serde_json::to_vec_pretty(manifest)?,
        )?;
    }
    if let Some(summary) = &session.metadata.summary {
        fs::write(
            output_root.join("summary").join("migration_summary.json"),
            serde_json::to_vec_pretty(summary)?,
        )?;
    }
    write_jsonl(output_root.join("events.jsonl"), &session.events)?;

    let mut grouped: BTreeMap<String, Vec<&ReplayTraceFrame>> = BTreeMap::new();
    for frame in &session.traces {
        let key = frame
            .step_id
            .clone()
            .unwrap_or_else(|| "legacy_step_0000".into());
        grouped.entry(key).or_default().push(frame);
    }

    let mut canonical_index = Vec::new();
    let mut files_written = vec!["events.jsonl".into()];
    let mut migrated_frames = 0usize;
    for (step_idx, (step_id, frames)) in grouped.into_iter().enumerate() {
        let raw_rel = format!("raw/{step_id}.rall");
        let raw_path = output_root.join(&raw_rel);
        let mut bytes = Vec::with_capacity(frames.len() * RALL_FRAME_BYTES);
        for frame in &frames {
            bytes.extend_from_slice(&frame.raw_bytes);
        }
        fs::write(&raw_path, bytes)?;
        files_written.push(raw_rel.clone());
        migrated_frames += frames.len();
        canonical_index.push(ReplayIndexEntry {
            schema_version: Some("0.1.0".into()),
            kind: Some("spectrum_raw_index".into()),
            run_id: session
                .events
                .first()
                .and_then(|event| event.run_id.clone()),
            step_id: Some(step_id),
            step_index: Some(step_idx as u64),
            raw_path: Some(raw_rel),
            timestamp_unix_ms: frames.first().and_then(|frame| frame.timestamp_unix_ms),
            offset_bytes: None,
            length_bytes: None,
            frame_index: None,
            extra: serde_json::Value::Null,
        });
    }
    write_jsonl(output_root.join("index.jsonl"), &canonical_index)?;
    files_written.push("index.jsonl".into());

    Ok(MigrationReport {
        source_root: source_root.to_path_buf(),
        output_root: output_root.to_path_buf(),
        files_written,
        steps_migrated: canonical_index.len(),
        frames_migrated: migrated_frames,
    })
}

fn open_canonical_session(root: &Path, source: ReplaySource) -> Result<ReplaySession, ReplayError> {
    let events = load_jsonl::<ReplayEvent>(&root.join("events.jsonl"))?;
    let index_entries = load_jsonl::<ReplayIndexEntry>(&root.join("index.jsonl"))?;
    let manifest = load_optional_json(&root.join("manifest.json"))?;
    let summary = load_optional_json(&find_summary_file(root)?)?;
    let traces = build_traces_from_canonical(root, &index_entries)?;

    Ok(ReplaySession {
        source,
        run_root: root.to_path_buf(),
        metadata: ReplayRunMetadata {
            manifest,
            summary,
            index_entries,
        },
        events,
        traces,
    })
}

fn open_legacy_session(root: &Path, source: ReplaySource) -> Result<ReplaySession, ReplayError> {
    let events = load_jsonl::<ReplayEvent>(&root.join("events.jsonl")).unwrap_or_default();
    let index_entries = load_jsonl::<ReplayIndexEntry>(&root.join("index.jsonl"))?;
    let manifest = load_optional_json(&root.join("manifest.json"))?;
    let summary = load_optional_json(&find_summary_file(root)?)?;
    let raw_path = find_legacy_rawbin(root)?;
    let traces = build_traces_from_legacy_rawbin(&raw_path, &index_entries)?;

    Ok(ReplaySession {
        source,
        run_root: root.to_path_buf(),
        metadata: ReplayRunMetadata {
            manifest,
            summary,
            index_entries,
        },
        events,
        traces,
    })
}

fn build_traces_from_canonical(
    root: &Path,
    entries: &[ReplayIndexEntry],
) -> Result<Vec<ReplayTraceFrame>, ReplayError> {
    let mut traces = Vec::new();
    for entry in entries {
        let Some(raw_path) = entry.raw_path.as_ref() else {
            continue;
        };
        let data = fs::read(root.join(raw_path))?;
        for (idx, chunk) in data.chunks(RALL_FRAME_BYTES).enumerate() {
            let (parsed, parse_error) = parse_chunk(chunk);
            traces.push(ReplayTraceFrame {
                step_id: entry.step_id.clone(),
                step_index: entry.step_index,
                frame_index: idx as u64,
                source_path: raw_path.clone(),
                timestamp_unix_ms: entry.timestamp_unix_ms,
                raw_bytes: chunk.to_vec(),
                parsed,
                parse_error,
            });
        }
    }
    Ok(traces)
}

fn build_traces_from_legacy_rawbin(
    raw_path: &Path,
    entries: &[ReplayIndexEntry],
) -> Result<Vec<ReplayTraceFrame>, ReplayError> {
    let raw = fs::read(raw_path)?;
    let mut traces = Vec::new();
    for (ordinal, entry) in entries.iter().enumerate() {
        let offset = entry
            .offset_bytes
            .unwrap_or((ordinal * RALL_FRAME_BYTES) as u64) as usize;
        let length = entry.length_bytes.unwrap_or(RALL_FRAME_BYTES as u64) as usize;
        if offset + length > raw.len() {
            return Err(ReplayError::Invalid(format!(
                "legacy frame slice out of bounds: offset={offset}, length={length}, total={}",
                raw.len()
            )));
        }
        let chunk = &raw[offset..offset + length];
        let (parsed, parse_error) = parse_chunk(chunk);
        traces.push(ReplayTraceFrame {
            step_id: entry.step_id.clone(),
            step_index: entry.step_index,
            frame_index: entry.frame_index.unwrap_or(ordinal as u64),
            source_path: raw_path.display().to_string(),
            timestamp_unix_ms: entry.timestamp_unix_ms,
            raw_bytes: chunk.to_vec(),
            parsed,
            parse_error,
        });
    }
    Ok(traces)
}

fn parse_chunk(chunk: &[u8]) -> (Option<ParsedRallFrameSummary>, Option<String>) {
    match parse_rall_frame(chunk) {
        Ok(frame) => (Some(summarize_rall_frame(frame)), None),
        Err(err) => (None, Some(err.to_string())),
    }
}

fn summarize_rall_frame(frame: RallFrame) -> ParsedRallFrameSummary {
    ParsedRallFrameSummary {
        sample_count: frame.measurements.lockin_B_X_mv.len(),
        latest_b_freq_hz: frame.measurements.lockin_B_freq_hz.last().copied(),
        latest_b_x_mv: frame.measurements.lockin_B_X_mv.last().copied(),
        latest_b_y_mv: frame.measurements.lockin_B_Y_mv.last().copied(),
        padding_all_zero: frame.padding_all_zero,
    }
}

fn load_jsonl<T>(path: &Path) -> Result<Vec<T>, ReplayError>
where
    T: for<'de> Deserialize<'de>,
{
    let text = fs::read_to_string(path)?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ReplayError::from)
}

fn load_optional_json(path: &Path) -> Result<Option<serde_json::Value>, ReplayError> {
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read(path)?;
    let value = serde_json::from_slice(&data)?;
    Ok(Some(value))
}

fn write_jsonl<T: Serialize>(path: PathBuf, items: &[T]) -> Result<(), ReplayError> {
    let mut bytes = Vec::new();
    for item in items {
        bytes.extend_from_slice(serde_json::to_string(item)?.as_bytes());
        bytes.push(b'\n');
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn find_summary_file(root: &Path) -> Result<PathBuf, ReplayError> {
    let candidates = [
        root.join("summary").join("run_summary.json"),
        root.join("summary").join("migration_summary.json"),
    ];
    Ok(candidates
        .into_iter()
        .find(|path| path.exists())
        .unwrap_or_else(|| root.join("__missing_summary__.json")))
}

fn find_legacy_rawbin(root: &Path) -> Result<PathBuf, ReplayError> {
    let candidates = [
        root.join("raw").join("oe1022d.rawbin"),
        root.join("raw").join("oe1022d_rall.rawbin"),
        root.join("rawbin").join("oe1022d.rawbin"),
    ];
    candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| ReplayError::Invalid("legacy rawbin file not found".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_rall_frame() -> Vec<u8> {
        include_bytes!("../../../tests/fixtures/oe1022d_rall/rall_frame_000.raw").to_vec()
    }

    #[test]
    fn canonical_session_reads_step_scoped_rall() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("raw")).unwrap();
        fs::create_dir_all(dir.path().join("summary")).unwrap();
        fs::write(dir.path().join("events.jsonl"), b"{\"run_id\":\"r1\"}\n").unwrap();
        fs::write(
            dir.path().join("index.jsonl"),
            b"{\"step_id\":\"spectrum_0000\",\"step_index\":0,\"raw_path\":\"raw/spectrum_0000.rall\"}\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("raw").join("spectrum_0000.rall"),
            sample_rall_frame(),
        )
        .unwrap();
        fs::write(
            dir.path().join("summary").join("run_summary.json"),
            b"{\"state\":\"completed\"}",
        )
        .unwrap();

        let session = open_replay_session(ReplaySource::CanonicalRunDirectory {
            root: dir.path().to_path_buf(),
        })
        .unwrap();
        assert_eq!(session.traces.len(), 1);
        assert!(session.traces[0].parsed.is_some());
    }

    #[test]
    fn legacy_migration_writes_canonical_step_file() {
        let source = tempdir().unwrap();
        let output = tempdir().unwrap();
        fs::create_dir_all(source.path().join("raw")).unwrap();
        fs::write(
            source.path().join("events.jsonl"),
            b"{\"run_id\":\"legacy_run\"}\n",
        )
        .unwrap();
        fs::write(
            source.path().join("index.jsonl"),
            b"{\"step_id\":\"spectrum_0000\",\"offset_bytes\":0,\"length_bytes\":12288}\n",
        )
        .unwrap();
        fs::write(
            source.path().join("raw").join("oe1022d.rawbin"),
            sample_rall_frame(),
        )
        .unwrap();

        let report = migrate_legacy_run_to_canonical(source.path(), output.path()).unwrap();
        assert_eq!(report.steps_migrated, 1);
        assert!(output
            .path()
            .join("raw")
            .join("spectrum_0000.rall")
            .exists());
        assert!(output.path().join("index.jsonl").exists());
    }
}
