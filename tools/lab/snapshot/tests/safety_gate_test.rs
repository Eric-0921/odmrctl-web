//! Safety gate tests for the snapshot tool.
//!
//! These tests prove that dangerous commands are rejected even if an attacker
//! (or buggy caller) tries to pass them into the snapshot API.

use lab_snapshot::validate_query;

const SMB100A_QUERIES: &[&str] = &[
    "*IDN?",
    "SYST:ERR?",
    "OUTP?",
    "MOD:STAT?",
    "FREQ:MODE?",
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
    "SWE:MODE?",
    "SWE:SPAC?",
    "SWE:FREQ:STEP?",
    "SWE:FREQ:DWEL?",
    "FREQ:STAR?",
    "FREQ:STOP?",
];

#[test]
fn reject_outp_on() {
    assert!(validate_query("OUTP ON", SMB100A_QUERIES).is_err());
}

#[test]
fn reject_mod_stat_on() {
    assert!(validate_query("MOD:STAT ON", SMB100A_QUERIES).is_err());
}

#[test]
fn reject_fm_stat_on() {
    assert!(validate_query("FM:STAT ON", SMB100A_QUERIES).is_err());
}

#[test]
fn reject_freq_mode_swe() {
    assert!(validate_query("FREQ:MODE SWE", SMB100A_QUERIES).is_err());
}

#[test]
fn reject_swe_exec() {
    assert!(validate_query("SWE:EXEC", SMB100A_QUERIES).is_err());
}

#[test]
fn reject_rst() {
    assert!(validate_query("*RST", SMB100A_QUERIES).is_err());
    assert!(validate_query("RST", SMB100A_QUERIES).is_err());
}

#[test]
fn reject_init_and_run() {
    assert!(validate_query("INIT", SMB100A_QUERIES).is_err());
    assert!(validate_query("RUN", SMB100A_QUERIES).is_err());
}

#[test]
fn reject_ssetd_and_rsetd() {
    assert!(validate_query("SSETD", SMB100A_QUERIES).is_err());
    assert!(validate_query("RSETD", SMB100A_QUERIES).is_err());
}

#[test]
fn all_smb100a_queries_end_with_question_mark() {
    for cmd in SMB100A_QUERIES {
        assert!(
            cmd.ends_with('?'),
            "SMB100A command '{}' does not end with '?'",
            cmd
        );
    }
}
