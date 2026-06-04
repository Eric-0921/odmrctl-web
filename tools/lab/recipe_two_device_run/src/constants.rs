//! Shared constants: SMB forbidden command patterns.

/// SMB100A command substrings that must never appear in any command sent to the device.
/// Used by both the transport-layer validator (smb_bridge) and the post-hoc command audit
/// comparison (command_audit_compare). Matching is case-insensitive substring match.
pub const SMB_FORBIDDEN_PATTERNS: &[&str] = &[
    "FREQ:MODE ",
    "FREQ:STAR ",
    "FREQ:STOP ",
    "SWE",
    "SWE:STEP",
    "SWE:DWEL",
    "LIST",
    "TRIG",
    "INIT",
    "LFO ",
    "LFO ON",
    "LFO OFF",
    "AM:STAT ",
    "PM:STAT ",
    "PULM:STAT ",
    "*RST",
    "RST ",
    "*CLS",
];
