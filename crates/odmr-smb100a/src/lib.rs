//! odmr-smb100a — SCPI command catalog and fake device for the R&S SMB100A.

pub mod commands;
pub mod fake;

pub use fake::{FakeSmb100a, Smb100aState};

#[cfg(test)]
mod tests {
    use super::commands::*;
    use super::fake::*;
    use odmr_device::FakeDevice;
    use odmr_types::DeviceId;

    // -----------------------------------------------------------------------
    // Golden tests — command string exact match
    // -----------------------------------------------------------------------

    #[test]
    fn golden_idn_query() {
        assert_eq!(idn_query(), "*IDN?");
    }

    #[test]
    fn golden_set_frequency_2_882_ghz() {
        // Source: smb100a_fig1_main_settings_commands.json step 4
        assert_eq!(set_frequency_hz(2.882e9), "FREQ 2882000000");
    }

    #[test]
    fn golden_query_frequency() {
        assert_eq!(query_frequency(), "FREQ?");
    }

    #[test]
    fn golden_set_power_minus_15_dbm() {
        // Source: smb100a_fig1_main_settings_commands.json step 5
        assert_eq!(set_power_dbm(-15.0), "POW -15dBm");
    }

    #[test]
    fn golden_query_power() {
        assert_eq!(query_power(), "POW?");
    }

    #[test]
    fn golden_set_alc_auto() {
        // Source: smb100a_fig1_main_settings_commands.json step 6
        assert_eq!(set_alc_auto(), "POW:ALC AUTO");
    }

    #[test]
    fn golden_set_output_off() {
        // Source: smb100a_fig1_main_settings_commands.json step 1
        assert_eq!(set_output(false), "OUTP OFF");
    }

    #[test]
    fn golden_set_modulation_global_off() {
        // Source: smb100a_fig1_main_settings_commands.json step 2
        assert_eq!(set_modulation_global(false), "MOD:STAT OFF");
    }

    #[test]
    fn golden_set_freq_mode_cw() {
        // Source: smb100a_fig1_main_settings_commands.json step 3
        assert_eq!(set_freq_mode_cw(), "FREQ:MODE CW");
    }

    #[test]
    fn golden_set_lf_frequency_500_hz() {
        // Source: smb100a_fig3_lf_generator_output_settings_commands.json
        assert_eq!(set_lf_frequency_hz(500.0), "LFO:FREQ 500Hz");
    }

    #[test]
    fn golden_set_lf_voltage_137_mv() {
        // Source: smb100a_fig3_lf_generator_output_settings_commands.json
        assert_eq!(set_lf_voltage_v(0.137), "LFO:VOLT 0.137V");
    }

    #[test]
    fn golden_set_lf_shape_square() {
        assert_eq!(set_lf_shape("SQUARE"), "LFO:SHAP SQUARE");
    }

    #[test]
    fn golden_set_fm_deviation_4_mhz() {
        // Source: smb100a_fig5_frequency_modulation_settings_commands.json
        assert_eq!(set_fm_deviation_hz(4e6), "FM:DEV 4000000Hz");
    }

    #[test]
    fn golden_set_fm_state_on() {
        // Source: smb100a_fig6_frequency_modulation_on_settings_commands.json
        assert_eq!(set_fm_state(true), "FM:STAT ON");
    }

    #[test]
    fn golden_freq_start_stop_commands_match_manual() {
        // Source: docs/equipment_manual/smb100a/06l_source_subsystem.md
        // FREQ:STAR and FREQ:STOP belong to FREQUENCY subsystem, not SWEep.
        assert_eq!(set_freq_start_hz(1e9), "FREQ:STAR 1000000000Hz");
        assert_eq!(set_freq_stop_hz(5e9), "FREQ:STOP 5000000000Hz");
        assert_eq!(query_freq_start(), "FREQ:STAR?");
        assert_eq!(query_freq_stop(), "FREQ:STOP?");
    }

    #[test]
    fn golden_sweep_step_dwell_commands() {
        assert_eq!(set_sweep_step_hz(500_000.0), "SWE:FREQ:STEP 500000Hz");
        assert_eq!(set_sweep_dwell_ms(500), "SWE:FREQ:DWEL 500ms");
        assert_eq!(query_sweep_step(), "SWE:FREQ:STEP?");
        assert_eq!(query_sweep_dwell(), "SWE:FREQ:DWEL?");
    }

    #[test]
    fn golden_sweep_spacing_command_is_swe_spac() {
        assert_eq!(set_sweep_spacing("LIN"), "SWE:SPAC LIN");
        assert_eq!(query_sweep_spacing(), "SWE:SPAC?");
    }

    #[test]
    fn golden_sweep_mode_command_is_swe_mode() {
        assert_eq!(set_sweep_mode("AUTO"), "SWE:MODE AUTO");
        assert_eq!(query_sweep_mode(), "SWE:MODE?");
    }

    // -----------------------------------------------------------------------
    // Fake-device tests
    // -----------------------------------------------------------------------

    #[test]
    fn fake_default_state_is_safe() {
        let dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        let st = dev.state();
        assert!(!st.rf_output_enabled, "RF output must be OFF by default");
        assert!(!st.modulation_global_enabled, "MOD must be OFF by default");
        assert!(!st.fm_enabled, "FM must be OFF by default");
    }

    #[test]
    fn fake_command_updates_state() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));

        dev.send_command("FREQ 2.882GHz").unwrap();
        assert_eq!(dev.state().rf_frequency_hz, 2.882e9);

        dev.send_command("POW -15dBm").unwrap();
        assert_eq!(dev.state().rf_power_dbm, -15.0);

        dev.send_command("OUTP ON").unwrap();
        assert!(dev.state().rf_output_enabled);
    }

    #[test]
    fn fake_query_returns_updated_state() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        dev.send_command("FREQ 2.882GHz").unwrap();

        let resp = dev.query("FREQ?").unwrap();
        assert_eq!(resp.to_string(), "2882000000");
    }

    #[test]
    fn fake_unknown_command_fails() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        let err = dev.send_command("FOOBAR 123").unwrap_err();
        assert!(err.to_string().contains("unknown command"));
    }

    #[test]
    fn fake_idn_matches() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));
        let resp = dev.query("*IDN?").unwrap();
        assert!(resp.to_string().contains("SMB100A"));
    }

    #[test]
    fn fake_lf_and_fm_roundtrip() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));

        dev.send_command("LFO ON").unwrap();
        dev.send_command("LFO:FREQ 500Hz").unwrap();
        dev.send_command("LFO:VOLT 0.137V").unwrap();
        dev.send_command("LFO:SHAP SQUARE").unwrap();
        dev.send_command("FM:STAT ON").unwrap();
        dev.send_command("FM:SOUR INT").unwrap();
        dev.send_command("FM:DEV 4MHz").unwrap();

        assert!(dev.state().lf_output_enabled);
        assert_eq!(dev.state().lf_frequency_hz, 500.0);
        assert_eq!(dev.state().lf_voltage_v, 0.137);
        assert_eq!(dev.state().lf_shape, "SQUARE");
        assert!(dev.state().fm_enabled);
        assert_eq!(dev.state().fm_source, "INT");
        assert_eq!(dev.state().fm_deviation_hz, 4e6);
    }

    #[test]
    fn fake_device_parses_freq_star_stop() {
        let mut dev = FakeSmb100a::new(DeviceId::new("smb100a_01"));

        dev.send_command("FREQ:STAR 1GHz").unwrap();
        dev.send_command("FREQ:STOP 5GHz").unwrap();
        dev.send_command("SWE:SPAC LIN").unwrap();
        dev.send_command("SWE:MODE AUTO").unwrap();

        assert_eq!(dev.state().freq_start_hz, 1e9);
        assert_eq!(dev.state().freq_stop_hz, 5e9);
        assert_eq!(dev.state().sweep_spacing, "LIN");
        assert_eq!(dev.state().sweep_mode, "AUTO");

        let resp = dev.query("FREQ:STAR?").unwrap();
        assert_eq!(resp.to_string(), "1000000000");

        let resp = dev.query("FREQ:STOP?").unwrap();
        assert_eq!(resp.to_string(), "5000000000");
    }
}
