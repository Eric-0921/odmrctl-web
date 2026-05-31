//! OE1022D RALL? binary frame parser.
//!
//! Parses the 12288-byte binary frame returned by `RALL?` into structured
//! measurements and configuration snapshot.
//!
//! **Frame layout** (confirmed on real hardware, M2.3):
//!
//! ```text
//! 0      .. 7999   measurement section   (20 params × 50 samples × 8 bytes)
//! 8000   .. 9215   configuration section (1216 bytes)
//! 9216   .. 12287  padding               (3072 bytes, expected all zero)
//! ```
//!
//! Measurement encoding: big-endian IEEE 754 f64.
//!
//! Configuration offsets documented in:
//! `docs/equipment_manual/oe1022d/05_oe1022d_rall_global_data_config_reading.md`

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const RALL_FRAME_BYTES: usize = 12288;
pub const RALL_MEASUREMENT_BYTES: usize = 8000;
pub const RALL_CONFIG_BYTES: usize = 1216;
pub const RALL_PADDING_BYTES: usize = 3072;

pub const RALL_PARAM_COUNT: usize = 20;
pub const RALL_SAMPLE_COUNT: usize = 50;

const PARAM_BLOCK_BYTES: usize = RALL_SAMPLE_COUNT * 8; // 400

// ---------------------------------------------------------------------------
// Parameter order (matching oe1022d_acquisition_guide.md §2.2)
// ---------------------------------------------------------------------------

const PARAM_NAMES: &[&str] = &[
    "A-X", "A-Y", "A-Freq", "A-Noise", "A-Xh1", "A-Yh1", "A-Xh2", "A-Yh2", "B-X", "B-Y", "B-Freq",
    "B-Noise", "B-Xh1", "B-Yh1", "B-Xh2", "B-Yh2", "AUXADC1", "AUXADC2", "AUXADC3", "AUXADC4",
];

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum RallParseError {
    WrongLength {
        expected: usize,
        actual: usize,
    },
    MeasurementLengthMismatch {
        expected: usize,
        actual: usize,
    },
    NonFiniteValue {
        parameter: String,
        sample_index: usize,
    },
    ConfigOutOfRange {
        field: String,
        value: u8,
    },
}

impl std::fmt::Display for RallParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RallParseError::WrongLength { expected, actual } => {
                write!(
                    f,
                    "wrong frame length: expected {}, got {}",
                    expected, actual
                )
            }
            RallParseError::MeasurementLengthMismatch { expected, actual } => {
                write!(
                    f,
                    "measurement section length mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            RallParseError::NonFiniteValue {
                parameter,
                sample_index,
            } => {
                write!(
                    f,
                    "non-finite value in parameter '{}' at sample index {}",
                    parameter, sample_index
                )
            }
            RallParseError::ConfigOutOfRange { field, value } => {
                write!(
                    f,
                    "config field '{}' has out-of-range value {}",
                    field, value
                )
            }
        }
    }
}

impl std::error::Error for RallParseError {}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A fully parsed RALL? frame.
#[derive(Debug, Clone, PartialEq)]
pub struct RallFrame {
    pub measurements: RallMeasurements,
    pub config: RallConfigSnapshot,
    pub padding_all_zero: bool,
}

/// 20 parameters × 50 samples from the measurement section.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq)]
pub struct RallMeasurements {
    pub lockin_A_X_mv: Vec<f64>,
    pub lockin_A_Y_mv: Vec<f64>,
    pub lockin_A_freq_hz: Vec<f64>,
    pub lockin_A_noise_mv: Vec<f64>,
    pub lockin_A_Xh1_mv: Vec<f64>,
    pub lockin_A_Yh1_mv: Vec<f64>,
    pub lockin_A_Xh2_mv: Vec<f64>,
    pub lockin_A_Yh2_mv: Vec<f64>,

    pub lockin_B_X_mv: Vec<f64>,
    pub lockin_B_Y_mv: Vec<f64>,
    pub lockin_B_freq_hz: Vec<f64>,
    pub lockin_B_noise_mv: Vec<f64>,
    pub lockin_B_Xh1_mv: Vec<f64>,
    pub lockin_B_Yh1_mv: Vec<f64>,
    pub lockin_B_Xh2_mv: Vec<f64>,
    pub lockin_B_Yh2_mv: Vec<f64>,

    pub aux_adc1_v: Vec<f64>,
    pub aux_adc2_v: Vec<f64>,
    pub aux_adc3_v: Vec<f64>,
    pub aux_adc4_v: Vec<f64>,
}

/// Configuration snapshot parsed from offset 8000..9215.
///
/// Fields are `Option<T>` because:
/// - some offsets may be reserved/unknown (gaps in the documented table)
/// - some fields may not be initialised by firmware in the current mode
/// - the parser must never panic on unexpected frame content
///
/// Offsets sourced from `05_oe1022d_rall_global_data_config_reading.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct RallConfigSnapshot {
    // A-channel Ref Phase (8200..8237)
    pub a_ref_phase_deg: Option<f32>,
    pub a_ref_source_code: Option<u8>,
    pub a_ref_current_freq_hz: Option<f64>,
    pub a_ref_internal_freq_hz: Option<f64>,
    pub a_ref_slope_code: Option<u8>,
    pub a_harmonic_1: Option<i64>,
    pub a_harmonic_2: Option<i64>,

    // A-channel Ref Sweep (8246..8283)
    pub a_sweep_type_code: Option<u8>,
    pub a_sweep_start_freq_hz: Option<f64>,
    pub a_sweep_stop_freq_hz: Option<f64>,
    pub a_sweep_step_freq_hz: Option<f64>,
    pub a_sweep_step_percent: Option<f32>,
    pub a_sweep_time_ms: Option<i64>,
    pub a_sweep_run: Option<u8>,

    // A-channel Sineout (8292..8325)
    pub a_sineout_voltage_v: Option<f32>,
    pub a_sineout_sweep_mode: Option<u8>,
    pub a_sineout_sweep_start_v: Option<f32>,
    pub a_sineout_sweep_stop_v: Option<f32>,
    pub a_sineout_sweep_step_v: Option<f32>,
    pub a_sineout_sweep_step_percent: Option<f32>,
    pub a_sineout_sweep_time_ms: Option<i64>,
    pub a_sineout_sweep_run: Option<u8>,
    pub a_sineout_dc_voltage_v: Option<f32>,

    // A-channel Equation (8330..8357)
    pub a_equation_c1: Option<f64>,
    pub a_equation_c2: Option<f64>,
    pub a_equation_1a_source: Option<u8>,
    pub a_equation_2a_source: Option<u8>,
    pub a_equation_3a_source: Option<u8>,
    pub a_equation_4a_source: Option<u8>,
    pub a_equation_1b_source: Option<u8>,
    pub a_equation_2b_source: Option<u8>,
    pub a_equation_3b_source: Option<u8>,
    pub a_equation_4b_source: Option<u8>,
    pub a_equation_1c_source: Option<u8>,
    pub a_equation_2c_source: Option<u8>,
    pub a_equation_3c_source: Option<u8>,
    pub a_equation_4c_source: Option<u8>,

    // A-channel Gain / TC / Input / Filter (8390..8406)
    pub a_sensitivity_code: Option<u8>,
    pub a_reserve_code: Option<u8>,
    pub a_source_code: Option<u8>,
    pub a_grounding_code: Option<u8>,
    pub a_coupling_code: Option<u8>,
    pub a_line_notch_code: Option<u8>,
    pub a_time_constant_code: Option<u8>,
    pub a_filter_slope_code: Option<u8>,
    pub a_synchronous_code: Option<u8>,

    // CHOUT (8415..8438)
    pub ch1_output_source: Option<u8>,
    pub ch2_output_source: Option<u8>,
    pub ch1_offset: Option<f32>,
    pub ch2_offset: Option<f32>,
    pub ch1_expand: Option<i16>,
    pub ch2_expand: Option<i16>,
    pub ch1_output_speed: Option<u8>,
    pub ch2_output_speed: Option<u8>,
    pub ch1_auxout_v: Option<f32>,
    pub ch2_auxout_v: Option<f32>,

    // A-channel Sample (8441..8470)
    pub a_sample_time_s: Option<f64>,
    pub a_sample_length: Option<i64>,
    pub a_sample_buffer1: Option<u8>,
    pub a_sample_buffer2: Option<u8>,
    pub a_sample_buffer3: Option<u8>,
    pub a_sample_buffer4: Option<u8>,
    pub a_sample_trigger_mode: Option<u8>,
    pub a_sample_mode_code: Option<u8>,
    pub a_sample_current_point: Option<i64>,

    // A-channel Status (8479..8481)
    pub a_input_overload: Option<bool>,
    pub a_gain_overload: Option<bool>,
    pub a_pll_locked: Option<bool>,

    // B-channel Ref Phase (8500..8537)
    pub b_ref_phase_deg: Option<f32>,
    pub b_ref_source_code: Option<u8>,
    pub b_ref_current_freq_hz: Option<f64>,
    pub b_ref_internal_freq_hz: Option<f64>,
    pub b_ref_slope_code: Option<u8>,
    pub b_harmonic_1: Option<i64>,
    pub b_harmonic_2: Option<i64>,

    // B-channel Ref Sweep (8546..8583)
    pub b_sweep_type_code: Option<u8>,
    pub b_sweep_start_freq_hz: Option<f64>,
    pub b_sweep_stop_freq_hz: Option<f64>,
    pub b_sweep_step_freq_hz: Option<f64>,
    pub b_sweep_step_percent: Option<f32>,
    pub b_sweep_time_ms: Option<i64>,
    pub b_sweep_run: Option<u8>,

    // B-channel Sineout (8592..8625)
    pub b_sineout_voltage_v: Option<f32>,
    pub b_sineout_sweep_mode: Option<u8>,
    pub b_sineout_sweep_start_v: Option<f32>,
    pub b_sineout_sweep_stop_v: Option<f32>,
    pub b_sineout_sweep_step_v: Option<f32>,
    pub b_sineout_sweep_step_percent: Option<f32>,
    pub b_sineout_sweep_time_ms: Option<i64>,
    pub b_sineout_sweep_run: Option<u8>,
    pub b_sineout_dc_voltage_v: Option<f32>,

    // B-channel Equation (8630..8657)
    pub b_equation_c1: Option<f64>,
    pub b_equation_c2: Option<f64>,
    pub b_equation_1a_source: Option<u8>,
    pub b_equation_2a_source: Option<u8>,
    pub b_equation_3a_source: Option<u8>,
    pub b_equation_4a_source: Option<u8>,
    pub b_equation_1b_source: Option<u8>,
    pub b_equation_2b_source: Option<u8>,
    pub b_equation_3b_source: Option<u8>,
    pub b_equation_4b_source: Option<u8>,
    pub b_equation_1c_source: Option<u8>,
    pub b_equation_2c_source: Option<u8>,
    pub b_equation_3c_source: Option<u8>,
    pub b_equation_4c_source: Option<u8>,

    // B-channel Gain / TC / Input / Filter (8690..8706)
    pub b_sensitivity_code: Option<u8>,
    pub b_reserve_code: Option<u8>,
    pub b_source_code: Option<u8>,
    pub b_grounding_code: Option<u8>,
    pub b_coupling_code: Option<u8>,
    pub b_line_notch_code: Option<u8>,
    pub b_time_constant_code: Option<u8>,
    pub b_filter_slope_code: Option<u8>,
    pub b_synchronous_code: Option<u8>,

    // B-channel Sample (8741..8770)
    pub b_sample_time_s: Option<f64>,
    pub b_sample_length: Option<i64>,
    pub b_sample_buffer1: Option<u8>,
    pub b_sample_buffer2: Option<u8>,
    pub b_sample_buffer3: Option<u8>,
    pub b_sample_buffer4: Option<u8>,
    pub b_sample_trigger_mode: Option<u8>,
    pub b_sample_mode_code: Option<u8>,
    pub b_sample_current_point: Option<i64>,

    // B-channel Status (8779..8781)
    pub b_input_overload: Option<bool>,
    pub b_gain_overload: Option<bool>,
    pub b_pll_locked: Option<bool>,

    // IDN serial number (9170..9209)
    pub idn_serial: Option<String>,
}

/// Latest sample from the B-channel (useful for ODMR single-point readout).
#[derive(Debug, Clone, PartialEq)]
pub struct BChannelSample {
    pub x_mv: f64,
    pub y_mv: f64,
    pub freq_hz: f64,
    pub noise_mv: f64,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a full 12288-byte RALL? frame.
pub fn parse_rall_frame(bytes: &[u8]) -> Result<RallFrame, RallParseError> {
    if bytes.len() != RALL_FRAME_BYTES {
        return Err(RallParseError::WrongLength {
            expected: RALL_FRAME_BYTES,
            actual: bytes.len(),
        });
    }

    let measurements = parse_rall_measurements(&bytes[0..RALL_MEASUREMENT_BYTES])?;
    let config = parse_rall_config(bytes)?;
    let padding_all_zero = bytes[RALL_MEASUREMENT_BYTES + RALL_CONFIG_BYTES..RALL_FRAME_BYTES]
        .iter()
        .all(|&b| b == 0);

    Ok(RallFrame {
        measurements,
        config,
        padding_all_zero,
    })
}

/// Parse the 8000-byte measurement section.
pub fn parse_rall_measurements(bytes: &[u8]) -> Result<RallMeasurements, RallParseError> {
    if bytes.len() != RALL_MEASUREMENT_BYTES {
        return Err(RallParseError::MeasurementLengthMismatch {
            expected: RALL_MEASUREMENT_BYTES,
            actual: bytes.len(),
        });
    }

    let mut param_vectors: Vec<Vec<f64>> = Vec::with_capacity(RALL_PARAM_COUNT);

    #[allow(clippy::needless_range_loop)]
    for param_idx in 0..RALL_PARAM_COUNT {
        let offset = param_idx * PARAM_BLOCK_BYTES;
        let block = &bytes[offset..offset + PARAM_BLOCK_BYTES];
        let mut samples = Vec::with_capacity(RALL_SAMPLE_COUNT);

        for sample_idx in 0..RALL_SAMPLE_COUNT {
            let sample_offset = sample_idx * 8;
            let chunk: [u8; 8] = block[sample_offset..sample_offset + 8].try_into().unwrap();
            let value = f64::from_bits(u64::from_be_bytes(chunk));
            if !value.is_finite() {
                return Err(RallParseError::NonFiniteValue {
                    parameter: PARAM_NAMES[param_idx].to_string(),
                    sample_index: sample_idx,
                });
            }
            samples.push(value);
        }
        param_vectors.push(samples);
    }

    Ok(RallMeasurements {
        lockin_A_X_mv: param_vectors[0].clone(),
        lockin_A_Y_mv: param_vectors[1].clone(),
        lockin_A_freq_hz: param_vectors[2].clone(),
        lockin_A_noise_mv: param_vectors[3].clone(),
        lockin_A_Xh1_mv: param_vectors[4].clone(),
        lockin_A_Yh1_mv: param_vectors[5].clone(),
        lockin_A_Xh2_mv: param_vectors[6].clone(),
        lockin_A_Yh2_mv: param_vectors[7].clone(),

        lockin_B_X_mv: param_vectors[8].clone(),
        lockin_B_Y_mv: param_vectors[9].clone(),
        lockin_B_freq_hz: param_vectors[10].clone(),
        lockin_B_noise_mv: param_vectors[11].clone(),
        lockin_B_Xh1_mv: param_vectors[12].clone(),
        lockin_B_Yh1_mv: param_vectors[13].clone(),
        lockin_B_Xh2_mv: param_vectors[14].clone(),
        lockin_B_Yh2_mv: param_vectors[15].clone(),

        aux_adc1_v: param_vectors[16].clone(),
        aux_adc2_v: param_vectors[17].clone(),
        aux_adc3_v: param_vectors[18].clone(),
        aux_adc4_v: param_vectors[19].clone(),
    })
}

/// Parse the configuration snapshot section.
///
/// Accepts either a full 12288-byte frame (uses global offsets) or a
/// 1216-byte config slice (uses relative offsets).  Prefer passing the full
/// frame to avoid offset confusion.
pub fn parse_rall_config(bytes: &[u8]) -> Result<RallConfigSnapshot, RallParseError> {
    let (_base, is_full_frame) = if bytes.len() == RALL_FRAME_BYTES {
        (RALL_MEASUREMENT_BYTES, true)
    } else if bytes.len() == RALL_CONFIG_BYTES {
        (0, false)
    } else {
        return Err(RallParseError::WrongLength {
            expected: RALL_FRAME_BYTES,
            actual: bytes.len(),
        });
    };

    let resolve_off = |global_off: usize| -> usize {
        if is_full_frame {
            global_off
        } else {
            global_off.saturating_sub(RALL_MEASUREMENT_BYTES)
        }
    };

    let read_u8 = |global_off: usize| -> Option<u8> {
        let off = resolve_off(global_off);
        if off < bytes.len() {
            Some(bytes[off])
        } else {
            None
        }
    };

    let read_f64 = |global_off: usize| -> Option<f64> {
        let off = resolve_off(global_off);
        if off + 8 <= bytes.len() {
            let chunk: [u8; 8] = bytes[off..off + 8].try_into().unwrap();
            Some(f64::from_bits(u64::from_be_bytes(chunk)))
        } else {
            None
        }
    };

    let read_f32 = |global_off: usize| -> Option<f32> {
        let off = resolve_off(global_off);
        if off + 4 <= bytes.len() {
            let chunk: [u8; 4] = bytes[off..off + 4].try_into().unwrap();
            Some(f32::from_bits(u32::from_be_bytes(chunk)))
        } else {
            None
        }
    };

    let read_i64 = |global_off: usize| -> Option<i64> {
        let off = resolve_off(global_off);
        if off + 8 <= bytes.len() {
            let chunk: [u8; 8] = bytes[off..off + 8].try_into().unwrap();
            Some(i64::from_be_bytes(chunk))
        } else {
            None
        }
    };

    let read_i16 = |global_off: usize| -> Option<i16> {
        let off = resolve_off(global_off);
        if off + 2 <= bytes.len() {
            let chunk: [u8; 2] = bytes[off..off + 2].try_into().unwrap();
            Some(i16::from_be_bytes(chunk))
        } else {
            None
        }
    };

    let read_idn_string = |global_off: usize| -> Option<String> {
        let off = resolve_off(global_off);
        const IDN_LEN: usize = 40;
        if off + IDN_LEN <= bytes.len() {
            let raw = &bytes[off..off + IDN_LEN];
            let end = raw.iter().position(|&b| b == 0).unwrap_or(IDN_LEN);
            Some(String::from_utf8_lossy(&raw[..end]).to_string())
        } else {
            None
        }
    };

    let bool_from_u8 = |v: Option<u8>| -> Option<bool> {
        v.map(|b| match b {
            0 => false,
            1 => true,
            _ => true, // treat non-zero as true (alarm active)
        })
    };

    Ok(RallConfigSnapshot {
        // A-channel Ref Phase (8200..8237)
        a_ref_phase_deg: read_f32(8200),
        a_ref_source_code: read_u8(8204),
        a_ref_current_freq_hz: read_f64(8205),
        a_ref_internal_freq_hz: read_f64(8213),
        a_ref_slope_code: read_u8(8221),
        a_harmonic_1: read_i64(8222),
        a_harmonic_2: read_i64(8230),

        // A-channel Ref Sweep (8246..8283)
        a_sweep_type_code: read_u8(8246),
        a_sweep_start_freq_hz: read_f64(8247),
        a_sweep_stop_freq_hz: read_f64(8255),
        a_sweep_step_freq_hz: read_f64(8263),
        a_sweep_step_percent: read_f32(8271),
        a_sweep_time_ms: read_i64(8275),
        a_sweep_run: read_u8(8283),

        // A-channel Sineout (8292..8325)
        a_sineout_voltage_v: read_f32(8292),
        a_sineout_sweep_mode: read_u8(8296),
        a_sineout_sweep_start_v: read_f32(8297),
        a_sineout_sweep_stop_v: read_f32(8301),
        a_sineout_sweep_step_v: read_f32(8305),
        a_sineout_sweep_step_percent: read_f32(8309),
        a_sineout_sweep_time_ms: read_i64(8313),
        a_sineout_sweep_run: read_u8(8321),
        a_sineout_dc_voltage_v: read_f32(8322),

        // A-channel Equation (8330..8357)
        a_equation_c1: read_f64(8330),
        a_equation_c2: read_f64(8338),
        a_equation_1a_source: read_u8(8346),
        a_equation_2a_source: read_u8(8347),
        a_equation_3a_source: read_u8(8348),
        a_equation_4a_source: read_u8(8349),
        a_equation_1b_source: read_u8(8350),
        a_equation_2b_source: read_u8(8351),
        a_equation_3b_source: read_u8(8352),
        a_equation_4b_source: read_u8(8353),
        a_equation_1c_source: read_u8(8354),
        a_equation_2c_source: read_u8(8355),
        a_equation_3c_source: read_u8(8356),
        a_equation_4c_source: read_u8(8357),

        // A-channel Gain / TC / Input / Filter (8390..8406)
        a_sensitivity_code: read_u8(8390),
        a_reserve_code: read_u8(8391),
        a_source_code: read_u8(8392),
        a_grounding_code: read_u8(8393),
        a_coupling_code: read_u8(8394),
        a_line_notch_code: read_u8(8395),
        a_time_constant_code: read_u8(8404),
        a_filter_slope_code: read_u8(8405),
        a_synchronous_code: read_u8(8406),

        // CHOUT (8415..8438)
        ch1_output_source: read_u8(8415),
        ch2_output_source: read_u8(8416),
        ch1_offset: read_f32(8417),
        ch2_offset: read_f32(8421),
        ch1_expand: read_i16(8425),
        ch2_expand: read_i16(8427),
        ch1_output_speed: read_u8(8429),
        ch2_output_speed: read_u8(8430),
        ch1_auxout_v: read_f32(8431),
        ch2_auxout_v: read_f32(8435),

        // A-channel Sample (8441..8470)
        a_sample_time_s: read_f64(8441),
        a_sample_length: read_i64(8449),
        a_sample_buffer1: read_u8(8457),
        a_sample_buffer2: read_u8(8458),
        a_sample_buffer3: read_u8(8459),
        a_sample_buffer4: read_u8(8460),
        a_sample_trigger_mode: read_u8(8461),
        a_sample_mode_code: read_u8(8462),
        a_sample_current_point: read_i64(8463),

        // A-channel Status (8479..8481)
        a_input_overload: bool_from_u8(read_u8(8479)),
        a_gain_overload: bool_from_u8(read_u8(8480)),
        a_pll_locked: bool_from_u8(read_u8(8481)),

        // B-channel Ref Phase (8500..8537)
        b_ref_phase_deg: read_f32(8500),
        b_ref_source_code: read_u8(8504),
        b_ref_current_freq_hz: read_f64(8505),
        b_ref_internal_freq_hz: read_f64(8513),
        b_ref_slope_code: read_u8(8521),
        b_harmonic_1: read_i64(8522),
        b_harmonic_2: read_i64(8530),

        // B-channel Ref Sweep (8546..8583)
        b_sweep_type_code: read_u8(8546),
        b_sweep_start_freq_hz: read_f64(8547),
        b_sweep_stop_freq_hz: read_f64(8555),
        b_sweep_step_freq_hz: read_f64(8563),
        b_sweep_step_percent: read_f32(8571),
        b_sweep_time_ms: read_i64(8575),
        b_sweep_run: read_u8(8583),

        // B-channel Sineout (8592..8625)
        b_sineout_voltage_v: read_f32(8592),
        b_sineout_sweep_mode: read_u8(8596),
        b_sineout_sweep_start_v: read_f32(8597),
        b_sineout_sweep_stop_v: read_f32(8601),
        b_sineout_sweep_step_v: read_f32(8605),
        b_sineout_sweep_step_percent: read_f32(8609),
        b_sineout_sweep_time_ms: read_i64(8613),
        b_sineout_sweep_run: read_u8(8621),
        b_sineout_dc_voltage_v: read_f32(8622),

        // B-channel Equation (8630..8657)
        b_equation_c1: read_f64(8630),
        b_equation_c2: read_f64(8638),
        b_equation_1a_source: read_u8(8646),
        b_equation_2a_source: read_u8(8647),
        b_equation_3a_source: read_u8(8648),
        b_equation_4a_source: read_u8(8649),
        b_equation_1b_source: read_u8(8650),
        b_equation_2b_source: read_u8(8651),
        b_equation_3b_source: read_u8(8652),
        b_equation_4b_source: read_u8(8653),
        b_equation_1c_source: read_u8(8654),
        b_equation_2c_source: read_u8(8655),
        b_equation_3c_source: read_u8(8656),
        b_equation_4c_source: read_u8(8657),

        // B-channel Gain / TC / Input / Filter (8690..8706)
        b_sensitivity_code: read_u8(8690),
        b_reserve_code: read_u8(8691),
        b_source_code: read_u8(8692),
        b_grounding_code: read_u8(8693),
        b_coupling_code: read_u8(8694),
        b_line_notch_code: read_u8(8695),
        b_time_constant_code: read_u8(8704),
        b_filter_slope_code: read_u8(8705),
        b_synchronous_code: read_u8(8706),

        // B-channel Sample (8741..8770)
        b_sample_time_s: read_f64(8741),
        b_sample_length: read_i64(8749),
        b_sample_buffer1: read_u8(8757),
        b_sample_buffer2: read_u8(8758),
        b_sample_buffer3: read_u8(8759),
        b_sample_buffer4: read_u8(8760),
        b_sample_trigger_mode: read_u8(8761),
        b_sample_mode_code: read_u8(8762),
        b_sample_current_point: read_i64(8763),

        // B-channel Status (8779..8781)
        b_input_overload: bool_from_u8(read_u8(8779)),
        b_gain_overload: bool_from_u8(read_u8(8780)),
        b_pll_locked: bool_from_u8(read_u8(8781)),

        // IDN serial number (9170..9209)
        idn_serial: read_idn_string(9170),
    })
}

/// Extract the latest (most recent) B-channel sample from a parsed frame.
pub fn latest_b_channel_sample(frame: &RallFrame) -> Option<BChannelSample> {
    let m = &frame.measurements;
    let last = RALL_SAMPLE_COUNT.checked_sub(1)?;
    Some(BChannelSample {
        x_mv: *m.lockin_B_X_mv.get(last)?,
        y_mv: *m.lockin_B_Y_mv.get(last)?,
        freq_hz: *m.lockin_B_freq_hz.get(last)?,
        noise_mv: *m.lockin_B_noise_mv.get(last)?,
    })
}
