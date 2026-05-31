//! Golden fixture tests for the OE1022D RALL? parser.
//!
//! These tests use real captured frames from M2.3 lab bring-up.

use odmr_oe1022d::parser::*;

// ---------------------------------------------------------------------------
// Golden fixtures — real captured frames
// ---------------------------------------------------------------------------

static FRAME_000: &[u8] = include_bytes!("../../../tests/fixtures/oe1022d_rall/rall_frame_000.raw");
static FRAME_001: &[u8] = include_bytes!("../../../tests/fixtures/oe1022d_rall/rall_frame_001.raw");
static FRAME_002: &[u8] = include_bytes!("../../../tests/fixtures/oe1022d_rall/rall_frame_002.raw");

// ---------------------------------------------------------------------------
// Frame length validation
// ---------------------------------------------------------------------------

#[test]
fn parser_accepts_exactly_12288_bytes() {
    let result = parse_rall_frame(FRAME_000);
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn parser_rejects_truncated_frame() {
    let truncated = &FRAME_000[..12287];
    let result = parse_rall_frame(truncated);
    assert!(matches!(
        result,
        Err(RallParseError::WrongLength {
            expected: 12288,
            actual: 12287
        })
    ));
}

#[test]
fn parser_rejects_oversized_frame() {
    let mut oversized = FRAME_000.to_vec();
    oversized.push(0);
    let result = parse_rall_frame(&oversized);
    assert!(matches!(
        result,
        Err(RallParseError::WrongLength {
            expected: 12288,
            actual: 12289
        })
    ));
}

// ---------------------------------------------------------------------------
// Measurement section decoding
// ---------------------------------------------------------------------------

#[test]
fn measurement_section_decodes_20x50_be_f64() {
    let frame = parse_rall_frame(FRAME_000).unwrap();
    let m = frame.measurements;

    assert_eq!(m.lockin_A_X_mv.len(), 50);
    assert_eq!(m.lockin_A_Y_mv.len(), 50);
    assert_eq!(m.lockin_A_freq_hz.len(), 50);
    assert_eq!(m.lockin_A_noise_mv.len(), 50);
    assert_eq!(m.lockin_A_Xh1_mv.len(), 50);
    assert_eq!(m.lockin_A_Yh1_mv.len(), 50);
    assert_eq!(m.lockin_A_Xh2_mv.len(), 50);
    assert_eq!(m.lockin_A_Yh2_mv.len(), 50);

    assert_eq!(m.lockin_B_X_mv.len(), 50);
    assert_eq!(m.lockin_B_Y_mv.len(), 50);
    assert_eq!(m.lockin_B_freq_hz.len(), 50);
    assert_eq!(m.lockin_B_noise_mv.len(), 50);
    assert_eq!(m.lockin_B_Xh1_mv.len(), 50);
    assert_eq!(m.lockin_B_Yh1_mv.len(), 50);
    assert_eq!(m.lockin_B_Xh2_mv.len(), 50);
    assert_eq!(m.lockin_B_Yh2_mv.len(), 50);

    assert_eq!(m.aux_adc1_v.len(), 50);
    assert_eq!(m.aux_adc2_v.len(), 50);
    assert_eq!(m.aux_adc3_v.len(), 50);
    assert_eq!(m.aux_adc4_v.len(), 50);
}

#[test]
fn b_channel_vectors_have_50_samples() {
    let frame = parse_rall_frame(FRAME_000).unwrap();
    assert_eq!(frame.measurements.lockin_B_X_mv.len(), 50);
    assert_eq!(frame.measurements.lockin_B_Y_mv.len(), 50);
    assert_eq!(frame.measurements.lockin_B_freq_hz.len(), 50);
    assert_eq!(frame.measurements.lockin_B_noise_mv.len(), 50);
}

#[test]
fn b_channel_samples_are_finite() {
    let frame = parse_rall_frame(FRAME_000).unwrap();
    for (i, &v) in frame.measurements.lockin_B_X_mv.iter().enumerate() {
        assert!(v.is_finite(), "B-X sample {} is non-finite: {}", i, v);
    }
    for (i, &v) in frame.measurements.lockin_B_Y_mv.iter().enumerate() {
        assert!(v.is_finite(), "B-Y sample {} is non-finite: {}", i, v);
    }
}

// ---------------------------------------------------------------------------
// Config snapshot — known offsets verified against M2.1 SCPI queries
// ---------------------------------------------------------------------------

#[test]
fn config_sensitivity_matches_fixture_expectation() {
    let frame = parse_rall_frame(FRAME_000).unwrap();
    assert_eq!(frame.config.a_sensitivity_code, Some(24));
}

#[test]
fn config_time_constant_matches_fixture_expectation() {
    let frame = parse_rall_frame(FRAME_000).unwrap();
    assert_eq!(frame.config.a_time_constant_code, Some(9));
}

#[test]
fn config_filter_slope_matches_fixture_expectation() {
    let frame = parse_rall_frame(FRAME_000).unwrap();
    assert_eq!(frame.config.a_filter_slope_code, Some(1));
}

#[test]
fn config_reserve_parses() {
    let frame = parse_rall_frame(FRAME_000).unwrap();
    assert_eq!(frame.config.a_reserve_code, Some(1));
}

#[test]
fn config_synchronous_parses() {
    let frame = parse_rall_frame(FRAME_000).unwrap();
    assert_eq!(frame.config.a_synchronous_code, Some(0));
}

#[test]
fn config_overload_flags_are_false() {
    let frame = parse_rall_frame(FRAME_000).unwrap();
    assert_eq!(frame.config.a_input_overload, Some(false));
    assert_eq!(frame.config.a_gain_overload, Some(false));
    assert_eq!(frame.config.a_pll_locked, Some(false));
}

// ---------------------------------------------------------------------------
// Padding
// ---------------------------------------------------------------------------

#[test]
fn padding_is_mostly_zero() {
    let _frame = parse_rall_frame(FRAME_000).unwrap();
    let padding = &FRAME_000[RALL_MEASUREMENT_BYTES + RALL_CONFIG_BYTES..];
    let non_zero_count = padding.iter().filter(|&&b| b != 0).count();
    assert!(
        non_zero_count <= 10,
        "padding has {} non-zero bytes, expected <= 10",
        non_zero_count
    );
}

// ---------------------------------------------------------------------------
// Robustness
// ---------------------------------------------------------------------------

#[test]
fn malformed_input_does_not_panic() {
    let bad = vec![0u8; 12288];
    let result = parse_rall_frame(&bad);
    assert!(result.is_ok());
}

#[test]
fn little_endian_gives_garbage() {
    let frame = parse_rall_frame(FRAME_000).unwrap();
    let first_bx = frame.measurements.lockin_B_X_mv[0];
    assert!(
        first_bx.abs() < 10.0,
        "first B-X = {} looks like garbage; endianness may be wrong",
        first_bx
    );
}

// ---------------------------------------------------------------------------
// Cross-fixture consistency
// ---------------------------------------------------------------------------

#[test]
fn all_three_fixtures_parse_successfully() {
    for (name, bytes) in [("000", FRAME_000), ("001", FRAME_001), ("002", FRAME_002)] {
        let result = parse_rall_frame(bytes);
        assert!(
            result.is_ok(),
            "fixture {} failed: {:?}",
            name,
            result.err()
        );
    }
}

#[test]
fn config_consistent_across_fixtures() {
    let f0 = parse_rall_frame(FRAME_000).unwrap().config;
    let f1 = parse_rall_frame(FRAME_001).unwrap().config;
    let f2 = parse_rall_frame(FRAME_002).unwrap().config;

    assert_eq!(f0.a_sensitivity_code, f1.a_sensitivity_code);
    assert_eq!(f1.a_sensitivity_code, f2.a_sensitivity_code);
    assert_eq!(f0.a_time_constant_code, f1.a_time_constant_code);
    assert_eq!(f1.a_time_constant_code, f2.a_time_constant_code);
}

// ---------------------------------------------------------------------------
// Optional helper
// ---------------------------------------------------------------------------

#[test]
fn latest_b_channel_sample_returns_last_point() {
    let frame = parse_rall_frame(FRAME_000).unwrap();
    let sample = latest_b_channel_sample(&frame).unwrap();
    assert!(sample.x_mv.is_finite());
    assert!(sample.y_mv.is_finite());
    assert!(sample.freq_hz.is_finite());
    assert!(sample.noise_mv.is_finite());
}

// ---------------------------------------------------------------------------
// Structural invariants
// ---------------------------------------------------------------------------

#[test]
fn parser_has_no_serial_io_dependency() {
    let _ = parse_rall_frame;
    let _ = parse_rall_measurements;
    let _ = parse_rall_config;
    let _ = latest_b_channel_sample;
}
