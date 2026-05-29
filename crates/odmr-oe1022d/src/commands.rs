//! Pure ASCII command builders for the SSI OE1022D DSP Lock-In Amplifier.
//!
//! All functions return `String` — no I/O, no side effects.
//!
//! Source of truth:
//! - `examples/oe1022d_labview_reference_signal_commands.json`
//! - `examples/oe1022d_labview_input_filter_commands.json`
//! - `examples/oe1022d_labview_channel_output_sine_output_commands.json`
//! - `examples/oe1022d_labview_formula_system_commands.json`
//! - `docs/equipment_manual/oe1022d/05_remote_programming.md`

// ---------------------------------------------------------------------------
// Channel argument convention
// ---------------------------------------------------------------------------
// i = 1 → Ch-A,  i = 2 → Ch-B (per manual section 5.1)

// ---------------------------------------------------------------------------
// Reference & Phase
// ---------------------------------------------------------------------------

/// `FMODD i,j` — set reference source for channel i.
///
/// j values (per manual):
/// 0 = External, 1 = Internal, 2 = Internal Sweep
///
/// Source: oe1022d_labview_reference_signal_commands.json
pub fn set_reference_source(channel: u8, source: u8) -> String {
    format!("FMODD {channel},{source}")
}

/// `FMODD? i` — query reference source.
pub fn query_reference_source(channel: u8) -> String {
    format!("FMODD? {channel}")
}

/// `RSLPD i,j` — set external reference trigger / slope for channel i.
///
/// j values (V1.5): 0 = TTL Rising Edge, 1 = Sine Zero Crossing
///
/// Source: oe1022d_labview_reference_signal_commands.json
pub fn set_ref_slope(channel: u8, slope: u8) -> String {
    format!("RSLPD {channel},{slope}")
}

/// `RSLPD? i` — query external reference trigger.
pub fn query_ref_slope(channel: u8) -> String {
    format!("RSLPD? {channel}")
}

/// `FREQD i,f` — set internal reference frequency (Hz) for channel i.
///
/// Source: oe1022d_labview_reference_signal_commands.json
pub fn set_ref_frequency_hz(channel: u8, hz: f64) -> String {
    format!("FREQD {channel},{hz}")
}

/// `FREQD? i` — query internal reference frequency.
pub fn query_ref_frequency(channel: u8) -> String {
    format!("FREQD? {channel}")
}

/// `PHASD i,p` — set phase offset (degrees) for channel i.
///
/// Source: oe1022d_labview_reference_signal_commands.json
pub fn set_phase_deg(channel: u8, deg: f64) -> String {
    format!("PHASD {channel},{deg}")
}

/// `PHASD? i` — query phase offset.
pub fn query_phase(channel: u8) -> String {
    format!("PHASD? {channel}")
}

// ---------------------------------------------------------------------------
// Input & Filter
// ---------------------------------------------------------------------------

/// `ISRCD i,j` — set input source for channel i.
///
/// j values: 0 = A (single-ended voltage), 1 = A-B (differential voltage),
///           2 = I (1MΩ), 3 = I (100MΩ)
///
/// Source: oe1022d_labview_input_filter_commands.json
pub fn set_input_source(channel: u8, source: u8) -> String {
    format!("ISRCD {channel},{source}")
}

/// `ISRCD? i` — query input source.
pub fn query_input_source(channel: u8) -> String {
    format!("ISRCD? {channel}")
}

/// `IGNDD i,j` — set input shield grounding for channel i.
///
/// j values: 0 = Float, 1 = Ground
///
/// Source: oe1022d_labview_input_filter_commands.json
pub fn set_input_grounding(channel: u8, grounding: u8) -> String {
    format!("IGNDD {channel},{grounding}")
}

/// `IGNDD? i` — query input shield grounding.
pub fn query_input_grounding(channel: u8) -> String {
    format!("IGNDD? {channel}")
}

/// `ICPLD i,j` — set input coupling for channel i.
///
/// j values: 0 = AC, 1 = DC
///
/// Source: oe1022d_labview_input_filter_commands.json
pub fn set_input_coupling(channel: u8, coupling: u8) -> String {
    format!("ICPLD {channel},{coupling}")
}

/// `ICPLD? i` — query input coupling.
pub fn query_input_coupling(channel: u8) -> String {
    format!("ICPLD? {channel}")
}

/// `ILNFD i,j` — set line notch filter for channel i.
///
/// j values: 0 = Off, 1 = 50Hz, 2 = 100Hz, 3 = Both
///
/// Source: oe1022d_labview_input_filter_commands.json
pub fn set_line_notch_filter(channel: u8, filter: u8) -> String {
    format!("ILNFD {channel},{filter}")
}

/// `ILNFD? i` — query line notch filter.
pub fn query_line_notch_filter(channel: u8) -> String {
    format!("ILNFD? {channel}")
}

// ---------------------------------------------------------------------------
// Sensitivity & Time Constant
// ---------------------------------------------------------------------------

/// `SENSD i,j` — set sensitivity (full-scale range) for channel i.
///
/// j is an index mapping to 1nV, 2nV, 5nV, 10nV, ... 1V (1-2-5 sequence).
/// See manual section 5.2.4 for exact table.
///
/// Source: oe1022d_labview_input_filter_commands.json
pub fn set_sensitivity(channel: u8, index: u8) -> String {
    format!("SENSD {channel},{index}")
}

/// `SENSD? i` — query sensitivity index.
pub fn query_sensitivity(channel: u8) -> String {
    format!("SENSD? {channel}")
}

/// `RMODD i,j` — set dynamic reserve for channel i.
///
/// j values: 0 = High Reserve, 1 = Normal, 2 = Low Noise
///
/// Source: oe1022d_labview_input_filter_commands.json
pub fn set_dynamic_reserve(channel: u8, mode: u8) -> String {
    format!("RMODD {channel},{mode}")
}

/// `RMODD? i` — query dynamic reserve.
pub fn query_dynamic_reserve(channel: u8) -> String {
    format!("RMODD? {channel}")
}

/// `OFLTD i,j` — set time constant for channel i.
///
/// j is an index mapping to 10us, 30us, 100us, ... 30ks (1-3-10 sequence).
/// See manual section 5.2.4 for exact table.
///
/// Source: oe1022d_labview_input_filter_commands.json
pub fn set_time_constant(channel: u8, index: u8) -> String {
    format!("OFLTD {channel},{index}")
}

/// `OFLTD? i` — query time constant index.
pub fn query_time_constant(channel: u8) -> String {
    format!("OFLTD? {channel}")
}

/// `OFSLD i,j` — set filter slope (dB/oct) for channel i.
///
/// j values: 0 = 6 dB/oct, 1 = 12 dB/oct, 2 = 18 dB/oct, 3 = 24 dB/oct
///
/// Source: oe1022d_labview_input_filter_commands.json
pub fn set_filter_slope(channel: u8, slope: u8) -> String {
    format!("OFSLD {channel},{slope}")
}

/// `OFSLD? i` — query filter slope.
pub fn query_filter_slope(channel: u8) -> String {
    format!("OFSLD? {channel}")
}

// ---------------------------------------------------------------------------
// Harmonic
// ---------------------------------------------------------------------------

/// `HARMD i,j` — set harmonic detection order for channel i.
///
/// j = 1..99 (harmonic number)
///
/// Source: oe1022d_labview_input_filter_commands.json
pub fn set_harmonic(channel: u8, harmonic: u8) -> String {
    format!("HARMD {channel},{harmonic}")
}

/// `HARMD? i` — query harmonic.
pub fn query_harmonic(channel: u8) -> String {
    format!("HARMD? {channel}")
}

// ---------------------------------------------------------------------------
// Data Query
// ---------------------------------------------------------------------------

/// `RALL?` — read all display values.
///
/// Response format depends on active channels and display settings.
/// This is a placeholder for the acquisition core to implement.
///
/// Source: OE1022D manual section 5.2.9
pub fn read_all() -> &'static str {
    "RALL?"
}
