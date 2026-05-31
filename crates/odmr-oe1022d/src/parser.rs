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
/// Fields are `Option<T>` because some offsets may be reserved/unknown.
#[derive(Debug, Clone, PartialEq)]
pub struct RallConfigSnapshot {
    pub a_sensitivity_code: Option<u8>,
    pub a_reserve_code: Option<u8>,
    pub a_time_constant_code: Option<u8>,
    pub a_filter_slope_code: Option<u8>,
    pub a_synchronous_code: Option<u8>,
    pub a_sample_time_s: Option<f64>,
    pub a_sample_length: Option<i64>,
    pub a_sample_mode_code: Option<u8>,
    pub a_input_overload: Option<bool>,
    pub a_gain_overload: Option<bool>,
    pub a_pll_locked: Option<bool>,
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

    let read_u8 = |global_off: usize| -> Option<u8> {
        let off = if is_full_frame {
            global_off
        } else {
            global_off - RALL_MEASUREMENT_BYTES
        };
        if off < bytes.len() {
            Some(bytes[off])
        } else {
            None
        }
    };

    let read_f64 = |global_off: usize| -> Option<f64> {
        let off = if is_full_frame {
            global_off
        } else {
            global_off - RALL_MEASUREMENT_BYTES
        };
        if off + 8 <= bytes.len() {
            let chunk: [u8; 8] = bytes[off..off + 8].try_into().unwrap();
            Some(f64::from_bits(u64::from_be_bytes(chunk)))
        } else {
            None
        }
    };

    let read_i64 = |global_off: usize| -> Option<i64> {
        let off = if is_full_frame {
            global_off
        } else {
            global_off - RALL_MEASUREMENT_BYTES
        };
        if off + 8 <= bytes.len() {
            let chunk: [u8; 8] = bytes[off..off + 8].try_into().unwrap();
            Some(i64::from_be_bytes(chunk))
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
        a_sensitivity_code: read_u8(8390),
        a_reserve_code: read_u8(8391),
        a_time_constant_code: read_u8(8404),
        a_filter_slope_code: read_u8(8405),
        a_synchronous_code: read_u8(8406),
        a_sample_time_s: read_f64(8441),
        a_sample_length: read_i64(8449),
        a_sample_mode_code: read_u8(8462),
        a_input_overload: bool_from_u8(read_u8(8479)),
        a_gain_overload: bool_from_u8(read_u8(8480)),
        a_pll_locked: bool_from_u8(read_u8(8481)),
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
