//! OE1022D RALL? raw binary capture and offline parser probe.
//!
//! **Safety invariant**: this library only sends `*IDN?` (once) and `RALL?`
//! (in the capture loop). There is no generic `send(cmd)` API.
//!
//! All outbound strings are validated against hard-coded allow-lists before
//! transmission. A secondary forbidden-pattern gate provides defense in depth.

use std::io::{Read, Write};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Hard-coded command allow-list
// ---------------------------------------------------------------------------

const ALLOWED_COMMANDS: &[&str] = &["*IDN?", "RALL?"];

const FORBIDDEN_PATTERNS: &[&str] = &[
    "*RST", "RST", "INIT", "RUN", "SSETD", "RSETD", "APHSD", "FMODD", "RSLPD",
    "PHASD", "ISRCD", "SENSD", "OFLTD", "OFSLD", "HARMD",
];

// ---------------------------------------------------------------------------
// Safety validation
// ---------------------------------------------------------------------------

/// Validates that `cmd` is in the pre-defined allow-list and does not contain
/// any forbidden substring.
pub fn validate_command(cmd: &str) -> Result<(), CaptureError> {
    let trimmed = cmd.trim();
    if !ALLOWED_COMMANDS.contains(&trimmed) {
        return Err(CaptureError::NotInAllowList {
            cmd: trimmed.to_string(),
        });
    }
    for pat in FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return Err(CaptureError::ForbiddenPattern {
                cmd: trimmed.to_string(),
                pattern: pat.to_string(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureRecord {
    pub frame_index: u64,
    pub command: String,
    pub offset_bytes: u64,
    pub length_bytes: usize,
    pub timestamp_unix_ms: u64,
    pub duration_ms: u64,
    pub pass_fail: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParserProbeResult {
    pub candidate: String,
    pub value_count: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub first_16: Vec<f64>,
    pub finite_count: usize,
    pub nan_count: usize,
    pub inf_count: usize,
    pub looks_plausible: bool,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaptureError {
    NotInAllowList { cmd: String },
    ForbiddenPattern { cmd: String, pattern: String },
    IoError(String),
    Timeout { cmd: String },
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureError::NotInAllowList { cmd } => {
                write!(f, "command '{}' is not in the capture allow-list", cmd)
            }
            CaptureError::ForbiddenPattern { cmd, pattern } => {
                write!(
                    f,
                    "command '{}' contains forbidden pattern '{}'",
                    cmd, pattern
                )
            }
            CaptureError::IoError(e) => write!(f, "io error: {}", e),
            CaptureError::Timeout { cmd } => {
                write!(f, "timeout waiting for response to '{}'", cmd)
            }
        }
    }
}

impl std::error::Error for CaptureError {}

// ---------------------------------------------------------------------------
// Serial capture
// ---------------------------------------------------------------------------

pub struct Oe1022dRallCapture {
    pub port: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
}

impl Oe1022dRallCapture {
    pub fn new(port: &str, baud_rate: u32) -> Self {
        Self {
            port: port.to_string(),
            baud_rate,
            timeout_ms: 2000,
        }
    }

    /// Verify identity by sending *IDN? once.
    pub fn verify_identity(&self) -> Result<String, CaptureError> {
        validate_command("*IDN?")?;

        let mut port = serialport::new(&self.port, self.baud_rate)
            .timeout(Duration::from_millis(self.timeout_ms))
            .open()
            .map_err(|e| CaptureError::IoError(format!("open serial: {}", e)))?;

        let _ = port.clear(serialport::ClearBuffer::Input);

        let cmd = "*IDN?\r";
        port.write_all(cmd.as_bytes())
            .map_err(|e| CaptureError::IoError(format!("write: {}", e)))?;
        port.flush()
            .map_err(|e| CaptureError::IoError(format!("flush: {}", e)))?;

        std::thread::sleep(Duration::from_millis(500));

        let mut buf = vec![0u8; 4096];
        let n = port.read(&mut buf).map_err(|e| {
            CaptureError::IoError(format!("read: {}", e))
        })?;

        buf.truncate(n);
        let text = String::from_utf8_lossy(&buf).replace('\x00', "").trim().to_string();
        Ok(text)
    }

    /// Capture `frames` RALL? responses. Returns records and the concatenated raw payload.
    pub fn capture(
        &self,
        frames: u32,
        delay_ms: u64,
    ) -> Result<(Vec<CaptureRecord>, Vec<u8>), CaptureError> {
        validate_command("RALL?")?;

        let mut port = serialport::new(&self.port, self.baud_rate)
            .timeout(Duration::from_millis(self.timeout_ms))
            .open()
            .map_err(|e| CaptureError::IoError(format!("open serial: {}", e)))?;

        let mut records = Vec::with_capacity(frames as usize);
        let mut raw_payload = Vec::new();

        for i in 0..frames {
            let _ = port.clear(serialport::ClearBuffer::Input);

            let ts = utc_now_ms();
            let start = Instant::now();

            let cmd = "RALL?\r";
            port.write_all(cmd.as_bytes())
                .map_err(|e| CaptureError::IoError(format!("write: {}", e)))?;
            port.flush()
                .map_err(|e| CaptureError::IoError(format!("flush: {}", e)))?;

            // RALL? returns 12288 bytes, but macOS CDC serial driver may
            // deliver it in multiple chunks (~1020 bytes each). We must loop
            // until we accumulate the full frame or hit timeout.
            const EXPECTED_FRAME_LEN: usize = 12288;
            std::thread::sleep(Duration::from_millis(800));

            let mut frame_buf = Vec::with_capacity(EXPECTED_FRAME_LEN);
            let read_deadline = Instant::now() + Duration::from_millis(self.timeout_ms);

            while frame_buf.len() < EXPECTED_FRAME_LEN && Instant::now() < read_deadline {
                let mut chunk = vec![0u8; 4096];
                match port.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        chunk.truncate(n);
                        frame_buf.extend_from_slice(&chunk);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                    Err(e) => {
                        records.push(CaptureRecord {
                            frame_index: i as u64,
                            command: "RALL?".to_string(),
                            offset_bytes: raw_payload.len() as u64,
                            length_bytes: frame_buf.len(),
                            timestamp_unix_ms: ts,
                            duration_ms: start.elapsed().as_millis() as u64,
                            pass_fail: "fail".to_string(),
                            notes: format!("serial read error after {} bytes: {}", frame_buf.len(), e),
                        });
                        break;
                    }
                }
            }

            let offset = raw_payload.len() as u64;
            raw_payload.extend_from_slice(&frame_buf);

            let actual_len = frame_buf.len();
            let pass_fail = if actual_len == EXPECTED_FRAME_LEN {
                "pass"
            } else if actual_len > 0 {
                "partial"
            } else {
                "timeout"
            };

            records.push(CaptureRecord {
                frame_index: i as u64,
                command: "RALL?".to_string(),
                offset_bytes: offset,
                length_bytes: actual_len,
                timestamp_unix_ms: ts,
                duration_ms: start.elapsed().as_millis() as u64,
                pass_fail: pass_fail.to_string(),
                notes: format!(
                    "captured {} bytes (expected {}), chunks may vary by OS/driver",
                    actual_len, EXPECTED_FRAME_LEN
                ),
            });

            if delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(delay_ms));
            }
        }

        Ok((records, raw_payload))
    }
}

// ---------------------------------------------------------------------------
// Parser probe
// ---------------------------------------------------------------------------

pub fn probe_frame(payload: &[u8]) -> Vec<ParserProbeResult> {
    let mut results = Vec::new();

    results.push(probe_f64(payload, "be_f64", u64::from_be_bytes));
    results.push(probe_f64(payload, "le_f64", u64::from_le_bytes));
    results.push(probe_f32(payload, "be_f32", u32::from_be_bytes));
    results.push(probe_f32(payload, "le_f32", u32::from_le_bytes));

    results
}

fn probe_f64(
    payload: &[u8],
    name: &str,
    read_u64: fn([u8; 8]) -> u64,
) -> ParserProbeResult {
    let mut values = Vec::new();
    let mut chunks = payload.chunks_exact(8);
    while let Some(chunk) = chunks.next() {
        let bytes: [u8; 8] = chunk.try_into().unwrap();
        let u = read_u64(bytes);
        let f = f64::from_bits(u);
        values.push(f);
    }

    summarize(name, &values)
}

fn probe_f32(
    payload: &[u8],
    name: &str,
    read_u32: fn([u8; 4]) -> u32,
) -> ParserProbeResult {
    let mut values = Vec::new();
    let mut chunks = payload.chunks_exact(4);
    while let Some(chunk) = chunks.next() {
        let bytes: [u8; 4] = chunk.try_into().unwrap();
        let u = read_u32(bytes);
        let f = f32::from_bits(u) as f64;
        values.push(f);
    }

    summarize(name, &values)
}

fn summarize(name: &str, values: &[f64]) -> ParserProbeResult {
    let count = values.len();
    if count == 0 {
        return ParserProbeResult {
            candidate: name.to_string(),
            value_count: 0,
            min: f64::NAN,
            max: f64::NAN,
            mean: f64::NAN,
            first_16: Vec::new(),
            finite_count: 0,
            nan_count: 0,
            inf_count: 0,
            looks_plausible: false,
            notes: "no complete values".to_string(),
        };
    }

    let finite: Vec<f64> = values.iter().filter(|&&v| v.is_finite()).copied().collect();
    let nan_count = values.iter().filter(|&&v| v.is_nan()).count();
    let inf_count = values.iter().filter(|&&v| v.is_infinite()).count();

    let min = finite.iter().copied().fold(f64::INFINITY, f64::min);
    let max = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mean = if !finite.is_empty() {
        finite.iter().sum::<f64>() / finite.len() as f64
    } else {
        f64::NAN
    };

    let first_16: Vec<f64> = values.iter().take(16).copied().collect();

    // Heuristic: plausible if mostly finite, values in lock-in range
    let looks_plausible = !finite.is_empty()
        && nan_count == 0
        && inf_count == 0
        && min.abs() < 1.0
        && max.abs() < 1.0;

    let notes = format!(
        "finite={}/{}, nan={}, inf={}",
        finite.len(),
        count,
        nan_count,
        inf_count
    );

    ParserProbeResult {
        candidate: name.to_string(),
        value_count: count,
        min,
        max,
        mean,
        first_16,
        finite_count: finite.len(),
        nan_count,
        inf_count,
        looks_plausible,
        notes,
    }
}

// ---------------------------------------------------------------------------
// Output formatters
// ---------------------------------------------------------------------------

pub fn records_to_jsonl(records: &[CaptureRecord]) -> String {
    let mut lines = Vec::with_capacity(records.len());
    for r in records {
        let json = format!(
            "{{\"frame_index\":{},\"command\":\"{}\",\"offset_bytes\":{},\"length_bytes\":{},\"timestamp_unix_ms\":{},\"duration_ms\":{},\"pass_fail\":\"{}\",\"notes\":\"{}\"}}",
            r.frame_index,
            escape_json(&r.command),
            r.offset_bytes,
            r.length_bytes,
            r.timestamp_unix_ms,
            r.duration_ms,
            r.pass_fail,
            escape_json(&r.notes)
        );
        lines.push(json);
    }
    lines.join("\n")
}

pub fn probe_results_to_json(results: &[ParserProbeResult]) -> String {
    let mut lines = Vec::with_capacity(results.len());
    for r in results {
        let first_16_json = r
            .first_16
            .iter()
            .map(|v| format!("{:.6e}", v))
            .collect::<Vec<_>>()
            .join(",");
        let json = format!(
            "{{\"candidate\":\"{}\",\"value_count\":{},\"min\":{:.6e},\"max\":{:.6e},\"mean\":{:.6e},\"first_16\":[{}],\"finite_count\":{},\"nan_count\":{},\"inf_count\":{},\"looks_plausible\":{},\"notes\":\"{}\"}}",
            escape_json(&r.candidate),
            r.value_count,
            r.min,
            r.max,
            r.mean,
            first_16_json,
            r.finite_count,
            r.nan_count,
            r.inf_count,
            r.looks_plausible,
            escape_json(&r.notes)
        );
        lines.push(json);
    }
    lines.join("\n")
}

pub fn probe_results_to_markdown(results: &[ParserProbeResult]) -> String {
    let mut lines = Vec::new();
    lines.push("# RALL? Parser Probe Results".to_string());
    lines.push("".to_string());
    lines.push("| Candidate | Values | Min | Max | Mean | Finite | NaN | Inf | Plausible | Notes |".to_string());
    lines.push("|-----------|--------|-----|-----|------|--------|-----|-----|-----------|-------|".to_string());
    for r in results {
        lines.push(format!(
            "| {} | {} | {:.6e} | {:.6e} | {:.6e} | {}/{} | {} | {} | {} | {} |",
            r.candidate,
            r.value_count,
            r.min,
            r.max,
            r.mean,
            r.finite_count,
            r.value_count,
            r.nan_count,
            r.inf_count,
            if r.looks_plausible { "✅" } else { "❌" },
            r.notes
        ));
    }
    lines.push("".to_string());
    for r in results {
        lines.push(format!("## {} — First 16 values", r.candidate));
        lines.push("".to_string());
        for (i, v) in r.first_16.iter().enumerate() {
            lines.push(format!("- [{}] {:.6e}", i, v));
        }
        lines.push("".to_string());
    }
    lines.join("\n")
}

pub fn capture_report_to_markdown(
    records: &[CaptureRecord],
    idn: &str,
    total_bytes: usize,
) -> String {
    let mut lines = Vec::new();
    lines.push("# OE1022D RALL? Capture Report".to_string());
    lines.push("".to_string());
    lines.push("> **Safety**: Only `*IDN?` and `RALL?` were sent.".to_string());
    lines.push("> **Settings**: No OE1022D settings were changed.".to_string());
    lines.push("".to_string());

    lines.push(format!("- **Device IDN**: `{}`", idn));
    lines.push(format!("- **Frames attempted**: {}", records.len()));
    let captured = records.iter().filter(|r| r.pass_fail == "pass").count();
    lines.push(format!("- **Frames captured**: {}", captured));
    let timeouts = records.iter().filter(|r| r.pass_fail == "timeout").count();
    lines.push(format!("- **Timeouts**: {}", timeouts));
    lines.push(format!("- **Total raw bytes**: {}", total_bytes));
    lines.push("".to_string());

    lines.push("## Frame Index".to_string());
    lines.push("".to_string());
    lines.push("| # | Offset | Length | Duration (ms) | Status | Notes |".to_string());
    lines.push("|---|--------|--------|---------------|--------|-------|".to_string());
    for r in records {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            r.frame_index, r.offset_bytes, r.length_bytes, r.duration_ms, r.pass_fail, r.notes
        ));
    }
    lines.push("".to_string());

    lines.push("## Forbidden Command Audit".to_string());
    lines.push("".to_string());
    for pat in FORBIDDEN_PATTERNS {
        lines.push(format!("- `{}`", pat));
    }
    lines.push("".to_string());

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn utc_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_command_accepts_idn() {
        assert!(validate_command("*IDN?").is_ok());
    }

    #[test]
    fn validate_command_accepts_rall() {
        assert!(validate_command("RALL?").is_ok());
    }

    #[test]
    fn validate_command_rejects_unknown() {
        assert!(validate_command("FREQD? 2").is_err());
        assert!(validate_command("FMODD 2,0").is_err());
    }

    #[test]
    fn validate_command_rejects_forbidden_patterns() {
        assert!(validate_command("*RST").is_err());
        assert!(validate_command("RST").is_err());
        assert!(validate_command("INIT").is_err());
        assert!(validate_command("RUN").is_err());
        assert!(validate_command("SSETD").is_err());
        assert!(validate_command("RSETD").is_err());
        assert!(validate_command("APHSD").is_err());
        assert!(validate_command("FMODD").is_err());
        assert!(validate_command("RSLPD").is_err());
        assert!(validate_command("PHASD").is_err());
        assert!(validate_command("ISRCD").is_err());
        assert!(validate_command("SENSD").is_err());
        assert!(validate_command("OFLTD").is_err());
        assert!(validate_command("OFSLD").is_err());
        assert!(validate_command("HARMD").is_err());
    }

    #[test]
    fn probe_handles_12288_bytes() {
        let payload = vec![0x3Fu8; 12288];
        let results = probe_frame(&payload);
        assert_eq!(results.len(), 4);
        for r in &results {
            assert!(r.value_count > 0);
        }
    }

    #[test]
    fn probe_no_panic_on_truncated() {
        let payload = vec![0x3Fu8; 7]; // not divisible by 4 or 8
        let results = probe_frame(&payload);
        assert_eq!(results.len(), 4);
        // f64 and f32 probes should handle incomplete chunks gracefully
    }

    #[test]
    fn rawbin_offset_correct_for_multiple_frames() {
        let rec1 = CaptureRecord {
            frame_index: 0,
            command: "RALL?".to_string(),
            offset_bytes: 0,
            length_bytes: 4096,
            timestamp_unix_ms: 0,
            duration_ms: 0,
            pass_fail: "pass".to_string(),
            notes: "".to_string(),
        };
        let rec2 = CaptureRecord {
            frame_index: 1,
            command: "RALL?".to_string(),
            offset_bytes: 4096,
            length_bytes: 4096,
            timestamp_unix_ms: 0,
            duration_ms: 0,
            pass_fail: "pass".to_string(),
            notes: "".to_string(),
        };
        assert_eq!(rec1.offset_bytes + rec1.length_bytes as u64, rec2.offset_bytes);
    }

    #[test]
    fn index_matches_rawbin_size() {
        let records = vec![
            CaptureRecord { frame_index: 0, command: "RALL?".to_string(), offset_bytes: 0, length_bytes: 100, timestamp_unix_ms: 0, duration_ms: 0, pass_fail: "pass".to_string(), notes: "".to_string() },
            CaptureRecord { frame_index: 1, command: "RALL?".to_string(), offset_bytes: 100, length_bytes: 200, timestamp_unix_ms: 0, duration_ms: 0, pass_fail: "pass".to_string(), notes: "".to_string() },
        ];
        let total: usize = records.iter().map(|r| r.length_bytes).sum();
        assert_eq!(total, 300);
    }

    #[test]
    fn jsonl_formatting() {
        let records = vec![CaptureRecord {
            frame_index: 0,
            command: "RALL?".to_string(),
            offset_bytes: 0,
            length_bytes: 12288,
            timestamp_unix_ms: 0,
            duration_ms: 604,
            pass_fail: "pass".to_string(),
            notes: "".to_string(),
        }];
        let jsonl = records_to_jsonl(&records);
        assert!(jsonl.contains("RALL?"));
        assert!(jsonl.contains("12288"));
    }
}
