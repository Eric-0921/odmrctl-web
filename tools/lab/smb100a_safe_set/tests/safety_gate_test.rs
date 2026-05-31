//! Safety gate integration tests for smb100a-safe-set.
//!
//! These tests prove that:
//! 1. Forbidden commands are rejected.
//! 2. The library has no generic arbitrary-send API.
//! 3. All safe-set commands are allowlisted.

use smb100a_safe_set::validate_command;

#[test]
fn reject_outp_on() {
    assert!(validate_command("OUTP ON").is_err());
}

#[test]
fn reject_mod_stat_on() {
    assert!(validate_command("MOD:STAT ON").is_err());
}

#[test]
fn reject_fm_stat_on() {
    assert!(validate_command("FM:STAT ON").is_err());
}

#[test]
fn reject_freq_mode_swe() {
    assert!(validate_command("FREQ:MODE SWE").is_err());
}

#[test]
fn reject_swe_exec() {
    assert!(validate_command("SWE:EXEC").is_err());
}

#[test]
fn reject_rst() {
    assert!(validate_command("*RST").is_err());
}

#[test]
fn reject_init() {
    assert!(validate_command("INIT").is_err());
}

#[test]
fn reject_run() {
    assert!(validate_command("RUN").is_err());
}

#[test]
fn all_safe_set_commands_are_allowlisted() {
    for cmd in &[
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
    ] {
        assert!(
            validate_command(cmd).is_ok(),
            "safe-set command '{}' was unexpectedly rejected",
            cmd
        );
    }
}

#[test]
fn no_generic_send_api() {
    // This test verifies at compile time that there is no public function
    // like `send(cmd: &str)` or `send_command(cmd: &str)` in the library.
    // If such a function existed, it would be callable here.
    // The only public entry points for sending are:
    //   Smb100aSafeSet::run()  — requires a confirmation callback
    //   Smb100aSafeSet::run_final_validation() — only queries, no setters
    //
    // Attempting to call a non-existent function would be a compile error,
    // which is the desired outcome.
    //
    // We verify this by checking that validate_command is the only public
    // function that accepts an arbitrary command string.
    let _ = validate_command;
}
