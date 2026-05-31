//! Safety constants, allow-lists, and command validation for the M3.1 FM/MOD micro-test tool.

use crate::types::CommandAuditEntry;

pub const SMB_QUERY_ALLOWLIST: &[&str] = &[
    "*IDN?",
    "OUTP?",
    "MOD:STAT?",
    "FREQ?",
    "POW?",
    "POW:ALC?",
    "FM:STAT?",
    "FM:SOUR?",
    "FM:DEV?",
    "LFO?",
    "LFO:FREQ?",
    "LFO:VOLT?",
    "LFO:SHAP?",
    "SYST:ERR?",
];

/// Set commands allowed in M3.1 FM/MOD micro-test mode (after preflight passes).
pub const SMB_MICROTEST_SET_ALLOWLIST: &[&str] = &[
    "FREQ ",
    "POW ",
    "POW:ALC ",
    "FM:SOUR ",
    "FM:DEV ",
    "FM:STAT ",
    "MOD:STAT ",
    "OUTP ON",
    "OUTP OFF",
    "LFO:FREQ ",
    "LFO:SHAP ",
    "LFO:VOLT ",
];

/// Commands forbidden in M3.1.
pub const SMB_FORBIDDEN_PATTERNS: &[&str] = &[
    "LFO ",
    "LFO ON",
    "LFO OFF",
    "FREQ:STAR ",
    "FREQ:STOP ",
    "FREQ:MODE ",
    "SWE:MODE ",
    "SWE:SPAC ",
    "SWE:FREQ:STEP ",
    "SWE:FREQ:DWEL ",
    "AM:STAT ",
    "PM:STAT ",
    "PULM:STAT ",
    "*RST",
    "RST ",
    "INIT",
];

/// Valid LF generator shape values per SMB100A manual §6.13.6.
pub const LF_SHAPE_ALLOWLIST: &[&str] = &[
    "SIN", "SQU", "TRI", "SAW", "ISAW", "SINE", "SQUARE", "TRIANGLE", "SAWTOOTH",
    "ISAWTOOTH",
];

pub fn validate_smb_query_only(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if trimmed.contains(';') {
        return Err(format!(
            "SMB query '{}' contains semicolon (SCPI command chaining rejected)",
            trimmed
        ));
    }
    if !trimmed.ends_with('?') {
        return Err(format!(
            "SMB command '{}' is not a query (does not end in '?')",
            trimmed
        ));
    }
    for pat in SMB_FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return Err(format!(
                "SMB query '{}' contains forbidden pattern '{}'",
                trimmed, pat
            ));
        }
    }
    if !SMB_QUERY_ALLOWLIST.contains(&trimmed) {
        return Err(format!(
            "SMB query '{}' is not in the query allow-list",
            trimmed
        ));
    }
    Ok(())
}

pub fn validate_microtest_set_command(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    // Reject SCPI command chaining first (defense-in-depth)
    if trimmed.contains(';') {
        return Err(format!(
            "SMB set command '{}' contains semicolon (SCPI command chaining rejected)",
            trimmed
        ));
    }
    // Reject forbidden patterns first
    for pat in SMB_FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return Err(format!(
                "SMB set command '{}' contains forbidden pattern '{}'",
                trimmed, pat
            ));
        }
    }
    // Allow only the explicit microtest set commands
    for allowed in SMB_MICROTEST_SET_ALLOWLIST {
        if trimmed.starts_with(allowed) {
            return Ok(());
        }
    }
    Err(format!(
        "SMB set command '{}' is not in the M3.1 micro-test allow-list",
        trimmed
    ))
}

pub fn classify_command_for_audit(cmd: &str) -> &'static str {
    let trimmed = cmd.trim();
    if trimmed.ends_with('?') {
        "query"
    } else {
        "set"
    }
}

pub fn is_safety_relevant(cmd: &str) -> bool {
    matches!(
        cmd.trim(),
        "OUTP?"
            | "MOD:STAT?"
            | "SYST:ERR?"
            | "OUTP ON"
            | "OUTP OFF"
            | "MOD:STAT ON"
            | "MOD:STAT OFF"
            | "FM:STAT ON"
            | "FM:STAT OFF"
    )
}

pub fn validate_lf_shape(shape: &str) -> Result<(), String> {
    let trimmed = shape.trim();
    if trimmed.contains(';') {
        return Err(format!(
            "LF shape '{}' contains semicolon (SCPI command chaining rejected)",
            trimmed
        ));
    }
    if !LF_SHAPE_ALLOWLIST.contains(&trimmed) {
        return Err(format!(
            "LF shape '{}' is not a valid SMB100A LF shape. Allowed: {:?}",
            trimmed, LF_SHAPE_ALLOWLIST
        ));
    }
    Ok(())
}

pub fn count_forbidden_category(audit: &[CommandAuditEntry], pattern: &str) -> usize {
    audit
        .iter()
        .filter(|a| a.sent_to_transport && a.command.contains(pattern))
        .count()
}
