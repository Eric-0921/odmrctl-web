//! Safety allow-lists and command validation for M3.2 step-sweep.

use crate::types::CommandAuditEntry;

pub const SMB_SWEEP_QUERY_ALLOWLIST: &[&str] = &[
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

/// Set commands allowed in M3.2 software-stepped sweep mode.
pub const SMB_SWEEP_SET_ALLOWLIST: &[&str] = &[
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

/// Commands forbidden in M3.2 (software-stepped, no internal sweep).
pub const SMB_FORBIDDEN_PATTERNS: &[&str] = &[
    "LFO ",
    "LFO ON",
    "LFO OFF",
    "FREQ:MODE ",
    "FREQ:STAR ",
    "FREQ:STOP ",
    "SWE:MODE ",
    "SWE:SPAC ",
    "SWE:FREQ:STEP ",
    "SWE:FREQ:DWEL ",
    "LIST",
    "TRIG",
    "INIT",
    "AM:STAT ",
    "PM:STAT ",
    "PULM:STAT ",
    "*RST",
    "RST ",
];

/// OE1022D commands allowed in M3.2 (passive acquisition only).
pub const OE_ALLOWLIST: &[&str] = &["*IDN?", "RALL?"];

/// OE1022D setting commands forbidden in M3.2.
pub const OE_FORBIDDEN_PATTERNS: &[&str] = &[
    "SSETD", "RSETD", "APHSD", "FMODD", "PHASD", "ISRCD", "SENSD", "OFLTD", "OFSLD", "HARMD",
    "SLVLD", "SWVTD",
];

/// Valid LF generator shape values per SMB100A manual §6.13.6.
pub const LF_SHAPE_ALLOWLIST: &[&str] = &[
    "SIN",
    "SQU",
    "TRI",
    "SAW",
    "ISAW",
    "SINE",
    "SQUARE",
    "TRIANGLE",
    "SAWTOOTH",
    "ISAWTOOTH",
];

pub fn validate_smb_sweep_query(cmd: &str) -> Result<(), String> {
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
    if !SMB_SWEEP_QUERY_ALLOWLIST.contains(&trimmed) {
        return Err(format!(
            "SMB query '{}' is not in the sweep query allow-list",
            trimmed
        ));
    }
    Ok(())
}

pub fn validate_smb_sweep_set(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if trimmed.contains(';') {
        return Err(format!(
            "SMB set command '{}' contains semicolon (SCPI command chaining rejected)",
            trimmed
        ));
    }
    for pat in SMB_FORBIDDEN_PATTERNS {
        if trimmed.contains(pat) {
            return Err(format!(
                "SMB set command '{}' contains forbidden pattern '{}'",
                trimmed, pat
            ));
        }
    }
    for allowed in SMB_SWEEP_SET_ALLOWLIST {
        if trimmed.starts_with(allowed) {
            return Ok(());
        }
    }
    Err(format!(
        "SMB set command '{}' is not in the M3.2 sweep allow-list",
        trimmed
    ))
}

pub fn validate_oe_command(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if trimmed.contains(';') {
        return Err(format!(
            "OE command '{}' contains semicolon (command chaining rejected)",
            trimmed
        ));
    }
    for pat in OE_FORBIDDEN_PATTERNS {
        if trimmed.to_ascii_uppercase().contains(pat) {
            return Err(format!(
                "OE command '{}' contains forbidden pattern '{}'",
                trimmed, pat
            ));
        }
    }
    if !OE_ALLOWLIST.contains(&trimmed) {
        return Err(format!(
            "OE command '{}' is not in the OE allow-list (only *IDN? and RALL? allowed)",
            trimmed
        ));
    }
    Ok(())
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

pub fn classify_command_for_audit(cmd: &str) -> &'static str {
    let trimmed = cmd.trim();
    if trimmed.ends_with('?') {
        "query"
    } else {
        "set"
    }
}

pub fn classify_oe_command_for_audit(cmd: &str) -> &'static str {
    let trimmed = cmd.trim();
    if trimmed == "*IDN?" {
        "oe_identity"
    } else if trimmed == "RALL?" {
        "oe_acquisition"
    } else {
        "oe_unknown"
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

#[allow(dead_code)]
pub fn count_forbidden_category(audit: &[CommandAuditEntry], pattern: &str) -> usize {
    audit
        .iter()
        .filter(|a| a.sent_to_transport && a.command.contains(pattern))
        .count()
}
