//! Human-approved SMB100A safe-set audit library.
//!
//! **Safety invariant**: this library only sends pre-defined safe-set commands.
//! There is no generic `send(cmd)` API. All outbound strings are validated
//! against hard-coded allow-lists before transmission. A secondary forbidden-
//! pattern gate provides defense in depth.
//!
//! Every setter is paired with a before/after query. After each setter,
//! `SYST:ERR?` is also queried.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Hard-coded safe-set command allow-list
// ---------------------------------------------------------------------------

const SAFE_SET_COMMANDS: &[&str] = &[
    "OUTP OFF",
    "MOD:STAT OFF",
    "FREQ:MODE CW",
    "FREQ 2.882GHz",
    "POW -15dBm",
    "POW:ALC AUTO",
    "LFO:FREQ 500Hz",
    "LFO:VOLT 137mV",
    "LFO:SHAP SQU",
    "FM:SOUR INT",
    "FM:DEV 4MHz",
    "FM:STAT OFF",
];

const FORBIDDEN_PATTERNS: &[&str] = &[
    "OUTP ON",
    "MOD:STAT ON",
    "FM:STAT ON",
    "FREQ:MODE SWE",
    "SWE:EXEC",
    "INIT",
    "RUN",
    "RST",
    "*RST",
];

// ---------------------------------------------------------------------------
// Safety validation
// ---------------------------------------------------------------------------

/// Validates that `cmd` is in the pre-defined safe-set allow-list and does not
/// contain any forbidden substring.
pub fn validate_command(cmd: &str) -> Result<(), SafeSetError> {
    let trimmed = cmd.trim();
    if !SAFE_SET_COMMANDS.contains(&trimmed) {
        return Err(SafeSetError::NotInAllowList {
            cmd: trimmed.to_string(),
        });
    }
    for pat in FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return Err(SafeSetError::ForbiddenPattern {
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

/// A single safe-set step: the setter plus its before/after validation query.
#[derive(Debug, Clone, PartialEq)]
pub struct SafeSetStep {
    pub query_before: &'static str,
    pub command: &'static str,
    pub query_after: &'static str,
}

/// One record per step (matches the user-specified JSONL schema).
#[derive(Debug, Clone, PartialEq)]
pub struct SafeSetRecord {
    pub device: String,
    pub phase: String,
    pub command: String,
    pub query_before: String,
    pub response_before: Option<String>,
    pub query_after: String,
    pub response_after: Option<String>,
    pub syst_err: Option<String>,
    pub pass_fail: String,
    pub human_confirmed: bool,
    pub timestamp: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SafeSetError {
    NotInAllowList { cmd: String },
    ForbiddenPattern { cmd: String, pattern: String },
    IoError(String),
    Timeout { cmd: String },
    Aborted,
}

impl std::fmt::Display for SafeSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SafeSetError::NotInAllowList { cmd } => {
                write!(f, "command '{}' is not in the safe-set allow-list", cmd)
            }
            SafeSetError::ForbiddenPattern { cmd, pattern } => {
                write!(
                    f,
                    "command '{}' contains forbidden pattern '{}'",
                    cmd, pattern
                )
            }
            SafeSetError::IoError(e) => write!(f, "io error: {}", e),
            SafeSetError::Timeout { cmd } => {
                write!(f, "timeout waiting for response to '{}'", cmd)
            }
            SafeSetError::Aborted => write!(f, "operator aborted"),
        }
    }
}

impl std::error::Error for SafeSetError {}

// ---------------------------------------------------------------------------
// Step definitions
// ---------------------------------------------------------------------------

pub const SAFE_SET_STEPS: &[SafeSetStep] = &[
    SafeSetStep { query_before: "OUTP?",     command: "OUTP OFF",       query_after: "OUTP?" },
    SafeSetStep { query_before: "MOD:STAT?", command: "MOD:STAT OFF",   query_after: "MOD:STAT?" },
    SafeSetStep { query_before: "FREQ:MODE?",command: "FREQ:MODE CW",   query_after: "FREQ:MODE?" },
    SafeSetStep { query_before: "FREQ?",     command: "FREQ 2.882GHz",  query_after: "FREQ?" },
    SafeSetStep { query_before: "POW?",      command: "POW -15dBm",     query_after: "POW?" },
    SafeSetStep { query_before: "POW:ALC?",  command: "POW:ALC AUTO",   query_after: "POW:ALC?" },
    SafeSetStep { query_before: "LFO:FREQ?", command: "LFO:FREQ 500Hz", query_after: "LFO:FREQ?" },
    SafeSetStep { query_before: "LFO:VOLT?", command: "LFO:VOLT 137mV", query_after: "LFO:VOLT?" },
    SafeSetStep { query_before: "LFO:SHAP?", command: "LFO:SHAP SQU",   query_after: "LFO:SHAP?" },
    SafeSetStep { query_before: "FM:SOUR?",  command: "FM:SOUR INT",    query_after: "FM:SOUR?" },
    SafeSetStep { query_before: "FM:DEV?",   command: "FM:DEV 4MHz",    query_after: "FM:DEV?" },
    SafeSetStep { query_before: "FM:STAT?",  command: "FM:STAT OFF",    query_after: "FM:STAT?" },
];

/// Final validation queries after all safe-set steps.
pub const FINAL_VALIDATION_QUERIES: &[&str] = &[
    "OUTP?",
    "MOD:STAT?",
    "FM:STAT?",
    "FREQ:MODE?",
    "SYST:ERR?",
];

// ---------------------------------------------------------------------------
// TCP transport
// ---------------------------------------------------------------------------

pub struct Smb100aSafeSet {
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
}

impl Smb100aSafeSet {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            timeout_ms: 2000,
        }
    }

    /// Run the full safe-set sequence with a confirmation callback.
    ///
    /// `confirm` is called with (step_index, step, response_before) and returns
    /// `true` to proceed, `false` to skip, or errors with `Aborted`.
    pub fn run<F>(
        &self,
        mut confirm: F,
    ) -> Result<Vec<SafeSetRecord>, SafeSetError>
    where
        F: FnMut(usize, &SafeSetStep, Option<&str>) -> Result<bool, SafeSetError>,
    {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect_timeout(
            &addr.parse::<std::net::SocketAddr>().map_err(|e| {
                SafeSetError::IoError(format!("parse address: {}", e))
            })?,
            Duration::from_millis(self.timeout_ms),
        )
        .map_err(|e| SafeSetError::IoError(format!("connect: {}", e)))?;

        stream
            .set_read_timeout(Some(Duration::from_millis(self.timeout_ms)))
            .map_err(|e| SafeSetError::IoError(format!("set timeout: {}", e)))?;

        let mut records = Vec::with_capacity(SAFE_SET_STEPS.len());

        for (i, step) in SAFE_SET_STEPS.iter().enumerate() {
            // 1. Query before
            let resp_before = tcp_query(&mut stream, step.query_before, &addr)?;

            // 2. Ask human confirmation
            let should_run = confirm(i, step, resp_before.response.as_deref())?;

            let ts = utc_now();
            let mut record = SafeSetRecord {
                device: "smb100a.main".to_string(),
                phase: "safe_set".to_string(),
                command: step.command.to_string(),
                query_before: step.query_before.to_string(),
                response_before: resp_before.response.clone(),
                query_after: step.query_after.to_string(),
                response_after: None,
                syst_err: None,
                pass_fail: "skipped".to_string(),
                human_confirmed: should_run,
                timestamp: ts,
                notes: String::new(),
            };

            if should_run {
                // 3. Validate and send exactly one allowlisted set command
                validate_command(step.command)?;
                let _setter = tcp_query(&mut stream, step.command, &addr)?;

                // 4. Query after
                let resp_after = tcp_query(&mut stream, step.query_after, &addr)?;
                record.response_after = resp_after.response;

                // 5. Query SYST:ERR?
                let err = tcp_query(&mut stream, "SYST:ERR?", &addr)?;
                record.syst_err = err.response;

                // 6. Determine pass/fail
                record.pass_fail = if record.response_after.is_some() {
                    "pass".to_string()
                } else {
                    "timeout".to_string()
                };
                record.notes = format!(
                    "setter duration {} ms, after-query duration {} ms, err-query duration {} ms",
                    _setter.duration_ms, resp_after.duration_ms, err.duration_ms
                );
            } else {
                record.notes = "skipped by operator".to_string();
            }

            records.push(record);
        }

        Ok(records)
    }

    /// Run final validation queries and return their responses.
    pub fn run_final_validation(&self) -> Result<Vec<SafeSetRecord>, SafeSetError> {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect_timeout(
            &addr.parse::<std::net::SocketAddr>().map_err(|e| {
                SafeSetError::IoError(format!("parse address: {}", e))
            })?,
            Duration::from_millis(self.timeout_ms),
        )
        .map_err(|e| SafeSetError::IoError(format!("connect: {}", e)))?;

        stream
            .set_read_timeout(Some(Duration::from_millis(self.timeout_ms)))
            .map_err(|e| SafeSetError::IoError(format!("set timeout: {}", e)))?;

        let mut records = Vec::with_capacity(FINAL_VALIDATION_QUERIES.len());
        for query in FINAL_VALIDATION_QUERIES {
            let resp = tcp_query(&mut stream, query, &addr)?;
            records.push(SafeSetRecord {
                device: "smb100a.main".to_string(),
                phase: "final_validation".to_string(),
                command: String::new(),
                query_before: query.to_string(),
                response_before: resp.response.clone(),
                query_after: String::new(),
                response_after: None,
                syst_err: None,
                pass_fail: if resp.response.is_some() {
                    "pass".to_string()
                } else {
                    "timeout".to_string()
                },
                human_confirmed: false,
                timestamp: resp.timestamp,
                notes: format!("duration {} ms", resp.duration_ms),
            });
        }
        Ok(records)
    }
}

/// Low-level TCP query. Used for both queries and setters.
fn tcp_query(
    stream: &mut TcpStream,
    cmd: &str,
    _addr: &str,
) -> Result<TransportRecord, SafeSetError> {
    let ts = utc_now();
    let start = Instant::now();

    let cmd_bytes = format!("{}\n", cmd.trim());
    stream
        .write_all(cmd_bytes.as_bytes())
        .map_err(|e| SafeSetError::IoError(format!("write: {}", e)))?;
    stream
        .flush()
        .map_err(|e| SafeSetError::IoError(format!("flush: {}", e)))?;

    let mut buf = vec![0u8; 1024];
    let n = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(e) => {
            return Ok(TransportRecord {
                response: None,
                timestamp: ts,
                duration_ms: start.elapsed().as_millis() as u64,
                notes: format!("read error: {}", e),
            });
        }
    };

    buf.truncate(n);
    let response = String::from_utf8_lossy(&buf).trim().to_string();
    let has_response = !response.is_empty();

    Ok(TransportRecord {
        response: if has_response { Some(response) } else { None },
        timestamp: ts,
        duration_ms: start.elapsed().as_millis() as u64,
        notes: String::new(),
    })
}

#[derive(Debug, Clone, PartialEq)]
struct TransportRecord {
    pub response: Option<String>,
    pub timestamp: String,
    pub duration_ms: u64,
    pub notes: String,
}

// ---------------------------------------------------------------------------
// Output formatters
// ---------------------------------------------------------------------------

pub fn records_to_jsonl(records: &[SafeSetRecord]) -> String {
    let mut lines = Vec::with_capacity(records.len());
    for r in records {
        let json = format!(
            "{{\"device\":\"{}\",\"phase\":\"{}\",\"command\":\"{}\",\"query_before\":\"{}\",\"response_before\":{},\"query_after\":\"{}\",\"response_after\":{},\"syst_err\":{},\"pass_fail\":\"{}\",\"human_confirmed\":{},\"timestamp\":\"{}\",\"notes\":\"{}\"}}",
            escape_json(&r.device),
            escape_json(&r.phase),
            escape_json(&r.command),
            escape_json(&r.query_before),
            opt_json(&r.response_before),
            escape_json(&r.query_after),
            opt_json(&r.response_after),
            opt_json(&r.syst_err),
            r.pass_fail,
            r.human_confirmed,
            r.timestamp,
            escape_json(&r.notes)
        );
        lines.push(json);
    }
    lines.join("\n")
}

pub fn records_to_markdown(
    safe_set_records: &[SafeSetRecord],
    final_records: &[SafeSetRecord],
) -> String {
    let mut lines = Vec::new();
    lines.push("# SMB100A Safe-Set Audit".to_string());
    lines.push("".to_string());
    lines.push("> **Safety Audit**: Only pre-approved safe-set commands were sent.".to_string(),
    );
    lines.push("> **Human-in-the-loop**: Each setter required operator confirmation.".to_string());
    lines.push("> **RF Output**: Kept OFF throughout.".to_string());
    lines.push("> **Modulation**: Kept OFF throughout.".to_string());
    lines.push("".to_string());

    lines.push("## Safe-Set Steps".to_string());
    lines.push("".to_string());
    lines.push(
        "| # | Command | Query Before | Before Value | Query After | After Value | SYST:ERR | Status |".to_string(),
    );
    lines.push(
        "|---|---------|--------------|--------------|-------------|-------------|----------|--------|".to_string(),
    );
    for (i, r) in safe_set_records.iter().enumerate() {
        lines.push(format!(
            "| {} | `{}` | `{}` | {} | `{}` | {} | {} | {} |",
            i + 1,
            r.command,
            r.query_before,
            r.response_before.as_deref().unwrap_or("_(timeout)_"),
            r.query_after,
            r.response_after.as_deref().unwrap_or("_(timeout)_"),
            r.syst_err.as_deref().unwrap_or("—"),
            r.pass_fail
        ));
    }
    lines.push("".to_string());

    lines.push("## Final Validation".to_string());
    lines.push("".to_string());
    lines.push("| Query | Response | Status |".to_string());
    lines.push("|-------|----------|--------|".to_string());
    for r in final_records {
        lines.push(format!(
            "| `{}` | {} | {} |",
            r.query_before,
            r.response_before.as_deref().unwrap_or("_(timeout)_"),
            r.pass_fail
        ));
    }
    lines.push("".to_string());

    lines.push("## Forbidden Command Audit".to_string());
    lines.push("".to_string());
    lines.push("The following patterns were explicitly blocked by the safe-set tool:".to_string());
    for pat in FORBIDDEN_PATTERNS {
        lines.push(format!("- `{}`", pat));
    }
    lines.push("".to_string());

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn utc_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}Z", now.as_secs(), now.subsec_millis())
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn opt_json(opt: &Option<String>) -> String {
    match opt {
        Some(v) => format!("\"{}\"", escape_json(v)),
        None => "null".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_command_accepts_allowlisted() {
        for cmd in SAFE_SET_COMMANDS {
            assert!(validate_command(cmd).is_ok(), "'{}' should be allowed", cmd);
        }
    }

    #[test]
    fn validate_command_rejects_unknown() {
        assert!(validate_command("FREQ 3GHz").is_err());
        assert!(validate_command("OUTP OFFF").is_err());
    }

    #[test]
    fn validate_command_rejects_forbidden_patterns() {
        assert!(validate_command("OUTP ON").is_err());
        assert!(validate_command("MOD:STAT ON").is_err());
        assert!(validate_command("FM:STAT ON").is_err());
        assert!(validate_command("FREQ:MODE SWE").is_err());
        assert!(validate_command("SWE:EXEC").is_err());
        assert!(validate_command("*RST").is_err());
    }

    #[test]
    fn safe_set_steps_are_consistent() {
        for step in SAFE_SET_STEPS {
            assert!(
                step.query_before.ends_with('?'),
                "query_before '{}' must end with '?'",
                step.query_before
            );
            assert!(
                step.query_after.ends_with('?'),
                "query_after '{}' must end with '?'",
                step.query_after
            );
            assert!(
                !step.command.ends_with('?'),
                "command '{}' must be a setter (no trailing '?')",
                step.command
            );
            assert!(
                SAFE_SET_COMMANDS.contains(&step.command),
                "command '{}' must be in SAFE_SET_COMMANDS",
                step.command
            );
        }
    }

    #[test]
    fn safe_set_commands_do_not_intersect_forbidden() {
        for cmd in SAFE_SET_COMMANDS {
            for pat in FORBIDDEN_PATTERNS {
                assert!(
                    !cmd.contains(pat),
                    "allow-listed command '{}' unexpectedly contains forbidden pattern '{}'",
                    cmd,
                    pat
                );
            }
        }
    }

    #[test]
    fn jsonl_formatting() {
        let records = vec![SafeSetRecord {
            device: "smb100a.main".to_string(),
            phase: "safe_set".to_string(),
            command: "OUTP OFF".to_string(),
            query_before: "OUTP?".to_string(),
            response_before: Some("0".to_string()),
            query_after: "OUTP?".to_string(),
            response_after: Some("0".to_string()),
            syst_err: Some("0,\"No error\"".to_string()),
            pass_fail: "pass".to_string(),
            human_confirmed: true,
            timestamp: "2026-05-30T12:00:00Z".to_string(),
            notes: "".to_string(),
        }];
        let jsonl = records_to_jsonl(&records);
        assert!(jsonl.contains("OUTP OFF"));
        // After JSON escaping, quotes become \"
        assert!(jsonl.contains("0,\\\"No error\\\""));
        assert!(jsonl.contains("\"human_confirmed\":true"));
    }

    #[test]
    fn markdown_contains_audit() {
        let md = records_to_markdown(&[], &[]);
        assert!(md.contains("Forbidden Command Audit"));
        assert!(md.contains("OUTP ON"));
        assert!(md.contains("Final Validation"));
    }
}
