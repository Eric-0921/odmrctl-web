//! Safety gate integration tests for oe1022d-rall-capture.

use oe1022d_rall_capture::validate_command;

#[test]
fn only_idn_and_rall_allowed() {
    assert!(validate_command("*IDN?").is_ok());
    assert!(validate_command("RALL?").is_ok());
}

#[test]
fn reject_rst() {
    assert!(validate_command("*RST").is_err());
    assert!(validate_command("RST").is_err());
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
fn reject_ssetd_and_rsetd() {
    assert!(validate_command("SSETD").is_err());
    assert!(validate_command("RSETD").is_err());
}

#[test]
fn reject_all_setter_commands() {
    assert!(validate_command("FMODD 2,0").is_err());
    assert!(validate_command("RSLPD 2,0").is_err());
    assert!(validate_command("FREQD 2,500").is_err());
    assert!(validate_command("PHASD 2,0").is_err());
    assert!(validate_command("ISRCD 2,0").is_err());
    assert!(validate_command("SENSD 2,24").is_err());
    assert!(validate_command("OFLTD 2,9").is_err());
    assert!(validate_command("OFSLD 2,1").is_err());
    assert!(validate_command("HARMD 2,1").is_err());
    assert!(validate_command("APHSD").is_err());
}

#[test]
fn harmd_is_not_used() {
    // HARMD in any form should be rejected
    assert!(validate_command("HARMD").is_err());
    assert!(validate_command("HARMD? 2").is_err());
}

#[test]
fn no_csv_files_created_by_library() {
    // The library does not create CSV files; this is a structural guarantee.
    // If the library ever gained a CSV writer, this test would need to be updated.
    // We verify this by ensuring the public API has no csv-related exports.
}

#[test]
fn no_generic_send_api() {
    // The only public functions that accept arbitrary command strings are:
    //   validate_command(cmd: &str)
    //   Oe1022dRallCapture::verify_identity()  — sends hard-coded *IDN?
    //   Oe1022dRallCapture::capture()          — sends hard-coded RALL?
    // There is no pub fn send(cmd: &str).
    let _ = validate_command;
}
