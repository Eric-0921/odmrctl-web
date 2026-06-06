//! Pure ASCII command builders for the SSI OE1022D DSP Lock-In Amplifier.
//!
//! All functions return `String` — no I/O, no side effects.
//!
//! Source of truth:
//! - `examples/oe1022d_labview_reference_signal_commands.json`
//! - `examples/oe1022d_labview_input_filter_commands.json`
//! - `examples/oe1022d_labview_channel_output_sine_output_commands.json`
//! - `examples/oe1022d_labview_formula_system_commands.json`
//! - `docs/equipment_manual/oe1022d/05_oe1022d_remote_programming_commands_55_74.md`
//! - `docs/equipment_manual/oe1022d/05_oe1022d_rall_global_data_config_reading.md`

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

/// `ILIND i,j` — set line notch filter for channel i.
///
/// j values: 0 = Off, 1 = 50Hz, 2 = 100Hz, 3 = Both
///
/// Source: `docs/equipment_manual/oe1022d/05_oe1022d_remote_programming_commands_55_74.md`
pub fn set_line_notch_filter(channel: u8, filter: u8) -> String {
    format!("ILIND {channel},{filter}")
}

/// `ILIND? i` — query line notch filter.
pub fn query_line_notch_filter(channel: u8) -> String {
    format!("ILIND? {channel}")
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
/// j values: 0 = Low Noise, 1 = Normal, 2 = High Reserve
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
// Synchronous Filter
// ---------------------------------------------------------------------------

/// `SYNCD i,j` — set synchronous filter on/off for channel i.
///
/// j values: 0 = Off, 1 = On
///
/// Source: `docs/equipment_manual/oe1022d/05_oe1022d_remote_programming_commands_55_74.md`
pub fn set_sync_filter(channel: u8, on: u8) -> String {
    format!("SYNCD {channel},{on}")
}

/// `SYNCD? i` — query synchronous filter state.
pub fn query_sync_filter(channel: u8) -> String {
    format!("SYNCD? {channel}")
}

// ---------------------------------------------------------------------------
// Status Query — overload / PLL (single-point alternatives to RALL?)
// ---------------------------------------------------------------------------

/// `INOVD? i` — query input overload status for channel i.
///
/// Returns "0" (no overload) or "1" (overload).
///
/// Source: `docs/equipment_manual/oe1022d/05_oe1022d_remote_programming_commands_55_74.md`
pub fn query_input_overload(channel: u8) -> String {
    format!("INOVD? {channel}")
}

/// `GNOVD? i` — query gain overload status for channel i.
///
/// Returns "0" (no overload) or "1" (overload).
pub fn query_gain_overload(channel: u8) -> String {
    format!("GNOVD? {channel}")
}

/// `*PLLD? i` — query PLL lock status for channel i.
///
/// Returns "0" (unlocked) or "1" (locked).
pub fn query_pll_locked(channel: u8) -> String {
    format!("*PLLD? {channel}")
}

// ---------------------------------------------------------------------------
// Data Query
// ---------------------------------------------------------------------------

/// `RALL?` — read all display values and configuration.
///
/// Returns a fixed 12288-byte binary frame (20 params × 50 samples).
///
/// Source: `docs/equipment_manual/oe1022d/05_oe1022d_remote_programming_commands_55_74.md`
pub fn read_all() -> &'static str {
    "RALL?"
}

/// `SNAPD? i,j,k{,l,m,n}` — read multiple parameters at a single time point.
///
/// i = channel (1=A, 2=B)
/// j,k,l,m,n = parameter indices (see manual §5.2.9)
pub fn query_snapshot(channel: u8, params: &[u8]) -> String {
    let param_str = params
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("SNAPD? {channel},{param_str}")
}

/// `OUTPD? i,j` — read a single output parameter for channel i.
///
/// j = parameter index (see manual §5.2.9)
pub fn query_output(channel: u8, param: u8) -> String {
    format!("OUTPD? {channel},{param}")
}

// ---------------------------------------------------------------------------
// Data Buffer Sampling (Manual §5.2.8 / §5.2.9)
// ---------------------------------------------------------------------------

/// `SRATD i,x` — set sampling step time for channel i.
///
/// x is expressed in seconds in the manual-derived API layer here; callers are
/// responsible for staying within the device's supported 1 ms to 100 s range.
pub fn set_sample_step_time_s(channel: u8, seconds: f64) -> String {
    format!("SRATD {channel},{seconds}")
}

/// `SRATD? i` — query sampling step time for channel i.
pub fn query_sample_step_time(channel: u8) -> String {
    format!("SRATD? {channel}")
}

/// `SLEND i,j` — set sampling length for channel i.
///
/// j must not exceed 16384.
pub fn set_sample_length(channel: u8, length: u32) -> String {
    format!("SLEND {channel},{length}")
}

/// `SLEND? i` — query sampling length for channel i.
pub fn query_sample_length(channel: u8) -> String {
    format!("SLEND? {channel}")
}

/// `SSLED i,j,k` — bind buffer j on channel i to parameter k.
///
/// j = 1..4 (Buffer1..Buffer4)
/// k = 0..21 (R/X/Y/theta/.../Freq per manual)
pub fn set_sample_buffer_selector(channel: u8, buffer: u8, parameter: u8) -> String {
    format!("SSLED {channel},{buffer},{parameter}")
}

/// `SSLED? i,j` — query buffer j selector on channel i.
pub fn query_sample_buffer_selector(channel: u8, buffer: u8) -> String {
    format!("SSLED? {channel},{buffer}")
}

/// `STRGD i,j` — set sampling trigger mode for channel i.
///
/// j values: 0 = internal, 1 = external
pub fn set_sample_trigger_mode(channel: u8, mode: u8) -> String {
    format!("STRGD {channel},{mode}")
}

/// `STRGD? i` — query sampling trigger mode for channel i.
pub fn query_sample_trigger_mode(channel: u8) -> String {
    format!("STRGD? {channel}")
}

/// `SPRMD i,j` — set sampling run mode for channel i.
///
/// j values: 0 = single, 1 = loop
pub fn set_sample_run_mode(channel: u8, mode: u8) -> String {
    format!("SPRMD {channel},{mode}")
}

/// `SPRMD? i` — query sampling run mode for channel i.
pub fn query_sample_run_mode(channel: u8) -> String {
    format!("SPRMD? {channel}")
}

/// `STRDD i` — start or continue sampling on channel i.
pub fn start_sampling(channel: u8) -> String {
    format!("STRDD {channel}")
}

/// `PAUSD i` — pause sampling on channel i.
pub fn pause_sampling(channel: u8) -> String {
    format!("PAUSD {channel}")
}

/// `RESTD i` — reset all buffers on channel i.
pub fn reset_data_buffers(channel: u8) -> String {
    format!("RESTD {channel}")
}

/// `SPTSD? i` — query the number of stored points on channel i.
pub fn query_stored_point_count(channel: u8) -> String {
    format!("SPTSD ? {channel}")
}

/// `TRCAD? i,j,k,l` — read l points from buffer j on channel i, starting at k.
pub fn query_trace_data(channel: u8, buffer: u8, start: u32, length: u32) -> String {
    format!("TRCAD ? {channel},{buffer},{start},{length}")
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

/// `*RSTD` — reset the OE1022D to default state.
///
/// **Safety**: clears all data buffers. Use with caution.
pub fn reset() -> &'static str {
    "*RSTD"
}

/// `*IDN?` — IEEE 488.2 standard identification query.
///
/// Typical response: `"SSI,LLA-OE1022D, SNXXXXXX, VerXXX"`.
/// This is the primary path; confirmed working on real hardware (M2.1).
pub fn query_standard_idn() -> &'static str {
    "*IDN?"
}

/// `*IDND?` — OE1022D-proprietary identification query (per manual).
///
/// Falls back to this if `*IDN?` does not return a valid response.
pub fn query_oe1022d_idn() -> &'static str {
    "*IDND?"
}

// ---------------------------------------------------------------------------
// Sine Output
// ---------------------------------------------------------------------------

/// `SWVTD i,j` — set sine output mode for channel i.
///
/// j values: 0 = Fixed, 1 = Linear sweep, 2 = Log sweep, 3 = DC
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_sine_out_mode(channel: u8, mode: u8) -> String {
    format!("SWVTD {channel},{mode}")
}

/// `SWVTD? i` — query sine output mode.
pub fn query_sine_out_mode(channel: u8) -> String {
    format!("SWVTD? {channel}")
}

/// `SLVLD i,x` — set sine output level (Vrms) for channel i.
///
/// Range: 0.001 ≤ x ≤ 5.000
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_sine_out_voltage(channel: u8, vrms: f64) -> String {
    format!("SLVLD {channel},{vrms}")
}

/// `SLVLD? i` — query sine output level.
pub fn query_sine_out_voltage(channel: u8) -> String {
    format!("SLVLD? {channel}")
}

/// `SVLLD i,x` — set sine output sweep start voltage (Vrms) for channel i.
///
/// Range: 0.001 ≤ x ≤ 5.000
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_sine_out_start_voltage(channel: u8, vrms: f64) -> String {
    format!("SVLLD {channel},{vrms}")
}

/// `SVLLD? i` — query sine output sweep start voltage.
pub fn query_sine_out_start_voltage(channel: u8) -> String {
    format!("SVLLD? {channel}")
}

/// `SVULD i,x` — set sine output sweep stop voltage (Vrms) for channel i.
///
/// Range: 0.001 ≤ x ≤ 5.000
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_sine_out_stop_voltage(channel: u8, vrms: f64) -> String {
    format!("SVULD {channel},{vrms}")
}

/// `SVULD? i` — query sine output sweep stop voltage.
pub fn query_sine_out_stop_voltage(channel: u8) -> String {
    format!("SVULD? {channel}")
}

/// `SVSLD i,x` — set sine output linear sweep step (Vrms) for channel i.
///
/// Range: 0.001 ≤ x ≤ 5.000
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_sine_out_linear_step(channel: u8, vrms: f64) -> String {
    format!("SVSLD {channel},{vrms}")
}

/// `SVSLD? i` — query sine output linear sweep step.
pub fn query_sine_out_linear_step(channel: u8) -> String {
    format!("SVSLD? {channel}")
}

/// `SVSGD i,x` — set sine output log sweep step (%) for channel i.
///
/// Range: 0 ≤ x ≤ 100
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_sine_out_log_step(channel: u8, percent: f64) -> String {
    format!("SVSGD {channel},{percent}")
}

/// `SVSGD? i` — query sine output log sweep step.
pub fn query_sine_out_log_step(channel: u8) -> String {
    format!("SVSGD? {channel}")
}

/// `SVTMD i,x` — set sine output sweep step time (ms) for channel i.
///
/// Range: 1 ≤ x ≤ 100000
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_sine_out_step_time(channel: u8, ms: u32) -> String {
    format!("SVTMD {channel},{ms}")
}

/// `SVTMD? i` — query sine output sweep step time.
pub fn query_sine_out_step_time(channel: u8) -> String {
    format!("SVTMD? {channel}")
}

/// `SVRMD i,j` — set sine output run mode for channel i.
///
/// j values: 0 = Stop, 1 = Single, 2 = Loop
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_sine_out_run_mode(channel: u8, mode: u8) -> String {
    format!("SVRMD {channel},{mode}")
}

/// `SVRMD? i` — query sine output run mode.
pub fn query_sine_out_run_mode(channel: u8) -> String {
    format!("SVRMD? {channel}")
}

/// `SVDCD i,x` — set sine output DC voltage (Vdc) for channel i.
///
/// Range: -10.000 ≤ x ≤ 10.000
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_sine_out_dc_voltage(channel: u8, vdc: f64) -> String {
    format!("SVDCD {channel},{vdc}")
}

/// `SVDCD? i` — query sine output DC voltage.
pub fn query_sine_out_dc_voltage(channel: u8) -> String {
    format!("SVDCD? {channel}")
}

// ---------------------------------------------------------------------------
// Channel Output
// ---------------------------------------------------------------------------

/// `FPOPD j,k` — set channel output source for rear-panel channel j.
///
/// j = 1 (CH1) or 2 (CH2)
/// k = 0..34 (see manual §5.2.6 for full table)
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_channel_output_source(channel_j: u8, source_k: u8) -> String {
    format!("FPOPD {channel_j},{source_k}")
}

/// `FPOPD? j` — query channel output source.
pub fn query_channel_output_source(channel_j: u8) -> String {
    format!("FPOPD? {channel_j}")
}

/// `OEXPD j,k{,x,l}` — set offset/expand for channel output j, parameter k.
///
/// j = 1 (CH1) or 2 (CH2)
/// k = parameter type (0..19, see manual §5.2.6)
/// x = offset percentage (-100..100)
/// l = expand factor (1..256)
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_channel_offset_expand(
    channel_j: u8,
    param_k: u8,
    offset_pct: f64,
    expand: u16,
) -> String {
    format!("OEXPD {channel_j},{param_k},{offset_pct},{expand}")
}

/// `OEXPD? j,k` — query offset/expand for channel output j, parameter k.
pub fn query_channel_offset_expand(channel_j: u8, param_k: u8) -> String {
    format!("OEXPD? {channel_j},{param_k}")
}

/// `SPEDD j,k` — set channel output speed for rear-panel channel j.
///
/// j = 1 (CH1) or 2 (CH2)
/// k = 0 (Slow, 10 Hz) or 1 (Fast, 312.5 kHz)
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_channel_output_speed(channel_j: u8, speed: u8) -> String {
    format!("SPEDD {channel_j},{speed}")
}

/// `SPEDD? j` — query channel output speed.
pub fn query_channel_output_speed(channel_j: u8) -> String {
    format!("SPEDD? {channel_j}")
}

/// `CAUXD j,x` — set AUXOUT DC voltage for rear-panel channel j.
///
/// j = 1 (CH1) or 2 (CH2)
/// x = voltage (-10.000..10.000)
///
/// Source: oe1022d_labview_channel_output_sine_output_commands.json
pub fn set_channel_auxout_voltage(channel_j: u8, vdc: f64) -> String {
    format!("CAUXD {channel_j},{vdc}")
}

/// `CAUXD? j` — query AUXOUT DC voltage.
pub fn query_channel_auxout_voltage(channel_j: u8) -> String {
    format!("CAUXD? {channel_j}")
}

// ---------------------------------------------------------------------------
// Reference Sweep (Internal Sweep Reference)
// ---------------------------------------------------------------------------

/// `SWTPD i,j` — set internal sweep type for channel i.
///
/// j values: 0 = Linear, 1 = Log
///
/// Source: oe1022d_labview_reference_signal_commands.json
pub fn set_sweep_type(channel: u8, sweep_type: u8) -> String {
    format!("SWTPD {channel},{sweep_type}")
}

/// `SWTPD? i` — query internal sweep type.
pub fn query_sweep_type(channel: u8) -> String {
    format!("SWTPD? {channel}")
}

/// `SLLMD i,f` — set internal sweep start frequency (Hz) for channel i.
///
/// Range: 0–102 kHz, resolution 1 mHz
///
/// Source: oe1022d_labview_reference_signal_commands.json
pub fn set_sweep_start_hz(channel: u8, hz: f64) -> String {
    format!("SLLMD {channel},{hz}")
}

/// `SLLMD? i` — query internal sweep start frequency.
pub fn query_sweep_start_hz(channel: u8) -> String {
    format!("SLLMD? {channel}")
}

/// `SULMD i,f` — set internal sweep stop frequency (Hz) for channel i.
///
/// Range: 0–102 kHz, resolution 1 mHz
///
/// Source: oe1022d_labview_reference_signal_commands.json
pub fn set_sweep_stop_hz(channel: u8, hz: f64) -> String {
    format!("SULMD {channel},{hz}")
}

/// `SULMD? i` — query internal sweep stop frequency.
pub fn query_sweep_stop_hz(channel: u8) -> String {
    format!("SULMD? {channel}")
}

/// `SSLLD i,f` — set internal sweep linear step (Hz) for channel i.
///
/// Range: 0–102 kHz, resolution 1 mHz
///
/// Source: oe1022d_labview_reference_signal_commands.json
pub fn set_sweep_linear_step_hz(channel: u8, hz: f64) -> String {
    format!("SSLLD {channel},{hz}")
}

/// `SSLLD? i` — query internal sweep linear step.
pub fn query_sweep_linear_step_hz(channel: u8) -> String {
    format!("SSLLD? {channel}")
}

/// `SSLGD i,x` — set internal sweep log step (%) for channel i.
///
/// Range: 0–100, resolution 0.001
///
/// Source: oe1022d_labview_reference_signal_commands.json
pub fn set_sweep_log_step_pct(channel: u8, pct: f64) -> String {
    format!("SSLGD {channel},{pct}")
}

/// `SSLGD? i` — query internal sweep log step.
pub fn query_sweep_log_step_pct(channel: u8) -> String {
    format!("SSLGD? {channel}")
}

/// `STLMD i,j` — set internal sweep step time (ms) for channel i.
///
/// Range: 1–100000 ms
///
/// Source: oe1022d_labview_reference_signal_commands.json
pub fn set_sweep_step_time_ms(channel: u8, ms: u32) -> String {
    format!("STLMD {channel},{ms}")
}

/// `STLMD? i` — query internal sweep step time.
pub fn query_sweep_step_time_ms(channel: u8) -> String {
    format!("STLMD? {channel}")
}

/// `SWRMD i,j` — set internal sweep run mode for channel i.
///
/// j values: 0 = Stop, 1 = Single, 2 = Loop
///
/// Source: oe1022d_labview_reference_signal_commands.json
pub fn set_sweep_run_mode(channel: u8, mode: u8) -> String {
    format!("SWRMD {channel},{mode}")
}

/// `SWRMD? i` — query internal sweep run mode.
pub fn query_sweep_run_mode(channel: u8) -> String {
    format!("SWRMD? {channel}")
}

// ---------------------------------------------------------------------------
// Auto Settings
// ---------------------------------------------------------------------------

/// `AGAND i` — auto sensitivity / auto gain for channel i.
///
/// Source: oe1022d_labview_input_filter_commands.json
pub fn auto_sensitivity(channel: u8) -> String {
    format!("AGAND {channel}")
}

/// `ARSVD i` — auto reserve for channel i.
///
/// Source: oe1022d_labview_input_filter_commands.json
pub fn auto_reserve(channel: u8) -> String {
    format!("ARSVD {channel}")
}

/// `APHSD i` — auto phase for channel i.
///
/// Source: oe1022d_labview_reference_signal_commands.json
pub fn auto_phase(channel: u8) -> String {
    format!("APHSD {channel}")
}

/// `ASCLD i` — auto scale for channel i.
///
/// Source: oe1022d_labview_input_filter_commands.json
pub fn auto_scale(channel: u8) -> String {
    format!("ASCLD {channel}")
}

// ---------------------------------------------------------------------------
// Equation System
// ---------------------------------------------------------------------------

/// `EQCDD i,j,k,l,m` — configure equation j for channel i.
///
/// j = 1..4 (E1–E4)
/// k, l, m = A, B, C coefficient selectors (0..19)
///
/// Source: oe1022d_labview_formula_system_commands.json
pub fn set_equation_config(channel: u8, equation: u8, a: u8, b: u8, c: u8) -> String {
    format!("EQCDD {channel},{equation},{a},{b},{c}")
}

/// `EQCDD? i,j` — query equation config for channel i, equation j.
pub fn query_equation_config(channel: u8, equation: u8) -> String {
    format!("EQCDD? {channel},{equation}")
}

/// `EQCSD i,j,x` — set equation constant Cj for channel i.
///
/// j = 1 (C1) or 2 (C2)
/// x = constant value (-10.000..10.000)
///
/// Source: oe1022d_labview_formula_system_commands.json
pub fn set_equation_constant(channel: u8, idx: u8, val: f64) -> String {
    format!("EQCSD {channel},{idx},{val}")
}

/// `EQCSD? i,j` — query equation constant.
pub fn query_equation_constant(channel: u8, idx: u8) -> String {
    format!("EQCSD? {channel},{idx}")
}

// ---------------------------------------------------------------------------
// Save / Recall Settings
// ---------------------------------------------------------------------------

/// `SSETD i` — save current settings to buffer i (1–4).
///
/// Source: oe1022d_labview_formula_system_commands.json
pub fn save_settings(buffer: u8) -> String {
    format!("SSETD {buffer}")
}

/// `RSETD i` — recall settings from buffer i (1–5).
///
/// i = 5 is factory default.
///
/// Source: oe1022d_labview_formula_system_commands.json
pub fn recall_settings(buffer: u8) -> String {
    format!("RSETD {buffer}")
}
