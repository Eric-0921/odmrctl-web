//! Pure SCPI command builders for the R&S SMB100A.
//!
//! All functions return `String` — no I/O, no side effects.
//!
//! Source of truth:
//! - `examples/smb100a_fig1_main_settings_commands.json`
//! - `examples/smb100a_fig3_lf_generator_output_settings_commands.json`
//! - `examples/smb100a_fig4_modulation_menu_settings_commands.json`
//! - `examples/smb100a_fig5_frequency_modulation_settings_commands.json`
//! - `examples/smb100a_fig6_frequency_modulation_on_settings_commands.json`
//! - `docs/equipment_manual/smb100a/06l_source_subsystem.md` (SCPI reference)

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// `*IDN?` — query instrument identification.
pub fn idn_query() -> &'static str {
    "*IDN?"
}

// ---------------------------------------------------------------------------
// RF Frequency
// ---------------------------------------------------------------------------

/// `FREQ <val>Hz` — set RF frequency in Hz.
///
/// Source: smb100a_fig1_main_settings_commands.json step 4
pub fn set_frequency_hz(hz: f64) -> String {
    format!("FREQ {hz}")
}

/// `FREQ?` — query RF frequency.
pub fn query_frequency() -> &'static str {
    "FREQ?"
}

/// `FREQ:MODE CW` — set fixed-frequency CW mode.
///
/// Source: smb100a_fig1_main_settings_commands.json step 3
pub fn set_freq_mode_cw() -> &'static str {
    "FREQ:MODE CW"
}

// ---------------------------------------------------------------------------
// RF Power / Level
// ---------------------------------------------------------------------------

/// `POW <val>dBm` — set RF output power in dBm.
///
/// Source: smb100a_fig1_main_settings_commands.json step 5
pub fn set_power_dbm(db: f64) -> String {
    format!("POW {db}dBm")
}

/// `POW?` — query RF output power.
pub fn query_power() -> &'static str {
    "POW?"
}

/// `POW:ALC AUTO` — set ALC to auto mode.
///
/// Source: smb100a_fig1_main_settings_commands.json step 6
pub fn set_alc_auto() -> &'static str {
    "POW:ALC AUTO"
}

/// `POW:ALC?` — query ALC state.
pub fn query_alc() -> &'static str {
    "POW:ALC?"
}

// ---------------------------------------------------------------------------
// RF Output
// ---------------------------------------------------------------------------

/// `OUTP OFF` / `OUTP ON` — set RF output state.
///
/// Source: smb100a_fig1_main_settings_commands.json step 1
pub fn set_output(state: bool) -> &'static str {
    if state {
        "OUTP ON"
    } else {
        "OUTP OFF"
    }
}

/// `OUTP?` — query RF output state.
pub fn query_output() -> &'static str {
    "OUTP?"
}

// ---------------------------------------------------------------------------
// Modulation Global
// ---------------------------------------------------------------------------

/// `MOD:STAT OFF` / `MOD:STAT ON` — set modulation global state.
///
/// Source: smb100a_fig1_main_settings_commands.json step 2
pub fn set_modulation_global(state: bool) -> &'static str {
    if state {
        "MOD:STAT ON"
    } else {
        "MOD:STAT OFF"
    }
}

/// `MOD:STAT?` — query modulation global state.
pub fn query_modulation_global() -> &'static str {
    "MOD:STAT?"
}

// ---------------------------------------------------------------------------
// LF Output
// ---------------------------------------------------------------------------

/// `LFO OFF` / `LFO ON` — set LF output state.
///
/// Source: smb100a_fig3_lf_generator_output_settings_commands.json
pub fn set_lf_output(state: bool) -> &'static str {
    if state {
        "LFO ON"
    } else {
        "LFO OFF"
    }
}

/// `LFO?` — query LF output state.
pub fn query_lf_output() -> &'static str {
    "LFO?"
}

/// `LFO:FREQ <val>Hz` — set LF generator frequency.
///
/// Source: smb100a_fig3_lf_generator_output_settings_commands.json
pub fn set_lf_frequency_hz(hz: f64) -> String {
    format!("LFO:FREQ {hz}Hz")
}

/// `LFO:FREQ?` — query LF generator frequency.
pub fn query_lf_frequency() -> &'static str {
    "LFO:FREQ?"
}

/// `LFO:VOLT <val>V` — set LF output voltage (peak) in volts.
///
/// Source: smb100a_fig3_lf_generator_output_settings_commands.json
pub fn set_lf_voltage_v(v: f64) -> String {
    format!("LFO:VOLT {v}V")
}

/// `LFO:VOLT?` — query LF output voltage.
pub fn query_lf_voltage() -> &'static str {
    "LFO:VOLT?"
}

/// `LFO:SHAP SQUARE|SINE|TRIangle|RAMP` — set LF waveform shape.
///
/// Source: smb100a_fig3_lf_generator_output_settings_commands.json
pub fn set_lf_shape(shape: &str) -> String {
    format!("LFO:SHAP {shape}")
}

/// `LFO:SHAP?` — query LF waveform shape.
pub fn query_lf_shape() -> &'static str {
    "LFO:SHAP?"
}

// ---------------------------------------------------------------------------
// FM (Frequency Modulation)
// ---------------------------------------------------------------------------

/// `FM:STAT OFF` / `FM:STAT ON` — set FM state.
///
/// Source: smb100a_fig5_frequency_modulation_settings_commands.json
/// Source: smb100a_fig6_frequency_modulation_on_settings_commands.json
pub fn set_fm_state(state: bool) -> &'static str {
    if state {
        "FM:STAT ON"
    } else {
        "FM:STAT OFF"
    }
}

/// `FM:STAT?` — query FM state.
pub fn query_fm_state() -> &'static str {
    "FM:STAT?"
}

/// `FM:SOUR INTernal|EXTernal` — set FM source.
///
/// Source: smb100a_fig5_frequency_modulation_settings_commands.json
pub fn set_fm_source(source: &str) -> String {
    format!("FM:SOUR {source}")
}

/// `FM:SOUR?` — query FM source.
pub fn query_fm_source() -> &'static str {
    "FM:SOUR?"
}

/// `FM:DEV <val>Hz` — set FM deviation in Hz.
///
/// Source: smb100a_fig5_frequency_modulation_settings_commands.json
pub fn set_fm_deviation_hz(hz: f64) -> String {
    format!("FM:DEV {hz}Hz")
}

/// `FM:DEV?` — query FM deviation.
pub fn query_fm_deviation() -> &'static str {
    "FM:DEV?"
}

// ---------------------------------------------------------------------------
// Frequency sweep range (FREQ subsystem)
// ---------------------------------------------------------------------------
// Source: docs/equipment_manual/smb100a/06l_source_subsystem.md
// Start/stop belong to SOURce:FREQuency, not SOURce:SWEep.

/// `FREQ:STAR <val>Hz` — set sweep start frequency.
pub fn set_freq_start_hz(hz: f64) -> String {
    format!("FREQ:STAR {hz}Hz")
}

/// `FREQ:STAR?` — query sweep start frequency.
pub fn query_freq_start() -> &'static str {
    "FREQ:STAR?"
}

/// `FREQ:STOP <val>Hz` — set sweep stop frequency.
pub fn set_freq_stop_hz(hz: f64) -> String {
    format!("FREQ:STOP {hz}Hz")
}

/// `FREQ:STOP?` — query sweep stop frequency.
pub fn query_freq_stop() -> &'static str {
    "FREQ:STOP?"
}

// ---------------------------------------------------------------------------
// Sweep parameters (SWEep subsystem)
// ---------------------------------------------------------------------------

/// `SWE:FREQ:STEP <val>Hz` — set sweep step size.
pub fn set_sweep_step_hz(hz: f64) -> String {
    format!("SWE:FREQ:STEP {hz}Hz")
}

/// `SWE:FREQ:STEP?` — query sweep step size.
pub fn query_sweep_step() -> &'static str {
    "SWE:FREQ:STEP?"
}

/// `SWE:FREQ:DWEL <val>ms` — set sweep dwell time.
pub fn set_sweep_dwell_ms(ms: u64) -> String {
    format!("SWE:FREQ:DWEL {ms}ms")
}

/// `SWE:FREQ:DWEL?` — query sweep dwell time.
pub fn query_sweep_dwell() -> &'static str {
    "SWE:FREQ:DWEL?"
}

/// `SWE:SPAC LIN|LOG` — set sweep spacing mode.
pub fn set_sweep_spacing(spacing: &str) -> String {
    format!("SWE:SPAC {spacing}")
}

/// `SWE:SPAC?` — query sweep spacing mode.
pub fn query_sweep_spacing() -> &'static str {
    "SWE:SPAC?"
}

/// `SWE:MODE AUTO|MAN|STEP` — set sweep mode.
pub fn set_sweep_mode(mode: &str) -> String {
    format!("SWE:MODE {mode}")
}

/// `SWE:MODE?` — query sweep mode.
pub fn query_sweep_mode() -> &'static str {
    "SWE:MODE?"
}
