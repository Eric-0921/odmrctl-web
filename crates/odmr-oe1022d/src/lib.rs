//! odmr-oe1022d — Command catalog and fake device for the SSI OE1022D DSP Lock-In Amplifier.

pub mod commands;
pub mod fake;
pub mod parser;

pub use fake::{ChannelState, FakeOe1022d};
pub use parser::*;

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
    fn golden_ch_b_reference_source_command() {
        // Source: oe1022d_labview_reference_signal_commands.json
        // Ch-B uses channel argument i = 2 (per manual section 5.1)
        assert_eq!(set_reference_source(2, 0), "FMODD 2,0");
    }

    #[test]
    fn golden_ch_b_ref_slope_ttl_rising() {
        // Source: oe1022d_labview_reference_signal_commands.json
        // V1.5: 0 = TTL Rising Edge
        assert_eq!(set_ref_slope(2, 0), "RSLPD 2,0");
    }

    #[test]
    fn golden_ch_b_ref_slope_sine_zero_crossing() {
        // Source: oe1022d_labview_reference_signal_commands.json
        // V1.5: 1 = Sine Zero Crossing
        assert_eq!(set_ref_slope(2, 1), "RSLPD 2,1");
    }

    #[test]
    fn golden_ch_b_ref_frequency_command() {
        assert_eq!(set_ref_frequency_hz(2, 500.0), "FREQD 2,500");
    }

    #[test]
    fn golden_ch_b_phase_command() {
        assert_eq!(set_phase_deg(2, 0.0), "PHASD 2,0");
    }

    #[test]
    fn golden_ch_b_input_source_command() {
        // Source: oe1022d_labview_input_filter_commands.json
        assert_eq!(set_input_source(2, 0), "ISRCD 2,0");
    }

    #[test]
    fn golden_ch_b_sensitivity_command() {
        // Source: oe1022d_labview_input_filter_commands.json
        assert_eq!(set_sensitivity(2, 7), "SENSD 2,7");
    }

    #[test]
    fn golden_ch_b_time_constant_command() {
        assert_eq!(set_time_constant(2, 10), "OFLTD 2,10");
    }

    #[test]
    fn golden_ch_b_filter_slope_command() {
        assert_eq!(set_filter_slope(2, 1), "OFSLD 2,1");
    }

    #[test]
    fn golden_ch_b_harmonic_command() {
        assert_eq!(set_harmonic(2, 1), "HARMD 2,1");
    }

    #[test]
    fn golden_read_all_placeholder() {
        assert_eq!(read_all(), "RALL?");
    }

    #[test]
    fn golden_sync_filter_command() {
        assert_eq!(set_sync_filter(2, 1), "SYNCD 2,1");
    }

    #[test]
    fn golden_query_sync_filter() {
        assert_eq!(query_sync_filter(1), "SYNCD? 1");
    }

    #[test]
    fn golden_query_input_overload() {
        assert_eq!(query_input_overload(2), "INOVD? 2");
    }

    #[test]
    fn golden_query_gain_overload() {
        assert_eq!(query_gain_overload(2), "GNOVD? 2");
    }

    #[test]
    fn golden_query_pll_locked() {
        assert_eq!(query_pll_locked(2), "*PLLD? 2");
    }

    #[test]
    fn golden_reset_command() {
        assert_eq!(reset(), "*RSTD");
    }

    #[test]
    fn golden_query_standard_idn() {
        assert_eq!(query_standard_idn(), "*IDN?");
    }

    #[test]
    fn golden_query_oe1022d_idn() {
        assert_eq!(query_oe1022d_idn(), "*IDND?");
    }

    #[test]
    fn golden_snapshot_command() {
        assert_eq!(query_snapshot(1, &[1, 2, 3]), "SNAPD? 1,1,2,3");
    }

    #[test]
    fn golden_output_command() {
        assert_eq!(query_output(2, 1), "OUTPD? 2,1");
    }

    // --- Sine Output golden tests ------------------------------------------

    #[test]
    fn golden_ch_b_sine_out_mode_fixed() {
        assert_eq!(set_sine_out_mode(2, 0), "SWVTD 2,0");
    }

    #[test]
    fn golden_ch_b_sine_out_voltage() {
        assert_eq!(set_sine_out_voltage(2, 1.0), "SLVLD 2,1");
    }

    #[test]
    fn golden_ch_b_sine_out_start_voltage() {
        assert_eq!(set_sine_out_start_voltage(2, 0.1), "SVLLD 2,0.1");
    }

    #[test]
    fn golden_ch_b_sine_out_stop_voltage() {
        assert_eq!(set_sine_out_stop_voltage(2, 1.0), "SVULD 2,1");
    }

    #[test]
    fn golden_ch_b_sine_out_linear_step() {
        assert_eq!(set_sine_out_linear_step(2, 0.01), "SVSLD 2,0.01");
    }

    #[test]
    fn golden_ch_b_sine_out_log_step() {
        assert_eq!(set_sine_out_log_step(2, 1.0), "SVSGD 2,1");
    }

    #[test]
    fn golden_ch_b_sine_out_step_time() {
        assert_eq!(set_sine_out_step_time(2, 100), "SVTMD 2,100");
    }

    #[test]
    fn golden_ch_b_sine_out_run_mode_single() {
        assert_eq!(set_sine_out_run_mode(2, 1), "SVRMD 2,1");
    }

    #[test]
    fn golden_ch_b_sine_out_dc_voltage() {
        assert_eq!(set_sine_out_dc_voltage(2, 0.0), "SVDCD 2,0");
    }

    // -----------------------------------------------------------------------
    // Fake-device tests
    // -----------------------------------------------------------------------

    #[test]
    fn fake_default_state_is_safe() {
        let dev = FakeOe1022d::new(DeviceId::new("oe1022d_01"));
        let ch_b = dev.channel(2).unwrap();
        assert_eq!(ch_b.reference_source, 0); // External
        assert_eq!(ch_b.ref_slope, 0); // TTL Rising Edge (V1.5)
        assert_eq!(ch_b.input_coupling, 0); // AC
        assert_eq!(ch_b.dynamic_reserve, 1); // Normal
        assert_eq!(ch_b.sine_out_mode, 0); // Fixed
        assert_eq!(ch_b.sine_out_voltage, 0.001);
        assert_eq!(ch_b.sine_out_dc_voltage, 0.0);
    }

    #[test]
    fn fake_command_updates_state() {
        let mut dev = FakeOe1022d::new(DeviceId::new("oe1022d_01"));

        dev.send_command("PHASD 2,45.0").unwrap();
        assert_eq!(dev.channel(2).unwrap().phase_deg, 45.0);

        dev.send_command("SENSD 2,10").unwrap();
        assert_eq!(dev.channel(2).unwrap().sensitivity_index, 10);
    }

    #[test]
    fn fake_query_returns_updated_state() {
        let mut dev = FakeOe1022d::new(DeviceId::new("oe1022d_01"));
        dev.send_command("PHASD 2,45.0").unwrap();

        let resp = dev.query("PHASD? 2").unwrap();
        assert_eq!(resp.to_string(), "45");
    }

    #[test]
    fn fake_unknown_command_fails() {
        let mut dev = FakeOe1022d::new(DeviceId::new("oe1022d_01"));
        let err = dev.send_command("XYZ 2,1").unwrap_err();
        assert!(err.to_string().contains("unknown command"));
    }

    #[test]
    fn fake_idn_matches() {
        let mut dev = FakeOe1022d::new(DeviceId::new("oe1022d_01"));
        let resp = dev.query("*IDN?").unwrap();
        assert!(resp.to_string().contains("OE1022D"));
    }

    #[test]
    fn fake_ch_a_and_ch_b_are_independent() {
        let mut dev = FakeOe1022d::new(DeviceId::new("oe1022d_01"));

        dev.send_command("PHASD 1,10.0").unwrap();
        dev.send_command("PHASD 2,20.0").unwrap();

        assert_eq!(dev.channel(1).unwrap().phase_deg, 10.0);
        assert_eq!(dev.channel(2).unwrap().phase_deg, 20.0);
    }

    #[test]
    fn fake_rslpd_query_returns_correct_value() {
        let mut dev = FakeOe1022d::new(DeviceId::new("oe1022d_01"));

        // Default is TTL Rising Edge (0) per V1.5
        let resp = dev.query("RSLPD? 2").unwrap();
        assert_eq!(resp.to_string(), "0");

        // Change to Sine Zero Crossing (1)
        dev.send_command("RSLPD 2,1").unwrap();
        let resp = dev.query("RSLPD? 2").unwrap();
        assert_eq!(resp.to_string(), "1");
    }

    #[test]
    fn fake_sine_out_command_updates_state() {
        let mut dev = FakeOe1022d::new(DeviceId::new("oe1022d_01"));

        dev.send_command("SWVTD 2,1").unwrap();
        assert_eq!(dev.channel(2).unwrap().sine_out_mode, 1);

        dev.send_command("SLVLD 2,2.5").unwrap();
        assert_eq!(dev.channel(2).unwrap().sine_out_voltage, 2.5);

        dev.send_command("SVDCD 2,3.0").unwrap();
        assert_eq!(dev.channel(2).unwrap().sine_out_dc_voltage, 3.0);
    }

    #[test]
    fn fake_sine_out_query_returns_updated_state() {
        let mut dev = FakeOe1022d::new(DeviceId::new("oe1022d_01"));

        dev.send_command("SWVTD 2,2").unwrap();
        let resp = dev.query("SWVTD? 2").unwrap();
        assert_eq!(resp.to_string(), "2");

        dev.send_command("SLVLD 2,1.5").unwrap();
        let resp = dev.query("SLVLD? 2").unwrap();
        assert_eq!(resp.to_string(), "1.5");
    }

    #[test]
    fn golden_ch_b_sweep_type_linear() {
        assert_eq!(set_sweep_type(2, 0), "SWTPD 2,0");
    }

    #[test]
    fn golden_ch_b_sweep_start() {
        assert_eq!(set_sweep_start_hz(2, 1000.0), "SLLMD 2,1000");
    }

    #[test]
    fn golden_ch_b_sweep_stop() {
        assert_eq!(set_sweep_stop_hz(2, 50000.0), "SULMD 2,50000");
    }

    #[test]
    fn golden_ch_b_sweep_linear_step() {
        assert_eq!(set_sweep_linear_step_hz(2, 100.0), "SSLLD 2,100");
    }

    #[test]
    fn golden_ch_b_sweep_log_step() {
        assert_eq!(set_sweep_log_step_pct(2, 10.0), "SSLGD 2,10");
    }

    #[test]
    fn golden_ch_b_sweep_step_time() {
        assert_eq!(set_sweep_step_time_ms(2, 200), "STLMD 2,200");
    }

    #[test]
    fn golden_ch_b_sweep_run_mode_loop() {
        assert_eq!(set_sweep_run_mode(2, 2), "SWRMD 2,2");
    }

    #[test]
    fn golden_ch_b_auto_phase() {
        assert_eq!(auto_phase(2), "APHSD 2");
    }

    #[test]
    fn golden_ch_b_auto_sensitivity() {
        assert_eq!(auto_sensitivity(2), "AGAND 2");
    }

    #[test]
    fn golden_ch_b_auto_reserve() {
        assert_eq!(auto_reserve(2), "ARSVD 2");
    }

    #[test]
    fn golden_ch_b_auto_scale() {
        assert_eq!(auto_scale(2), "ASCLD 2");
    }

    #[test]
    fn golden_ch1_channel_output_source() {
        assert_eq!(set_channel_output_source(1, 17), "FPOPD 1,17");
    }

    #[test]
    fn golden_ch1_channel_output_speed() {
        assert_eq!(set_channel_output_speed(1, 1), "SPEDD 1,1");
    }

    #[test]
    fn golden_ch1_channel_auxout() {
        assert_eq!(set_channel_auxout_voltage(1, 2.5), "CAUXD 1,2.5");
    }

    #[test]
    fn golden_equation_config() {
        assert_eq!(set_equation_config(2, 1, 0, 1, 2), "EQCDD 2,1,0,1,2");
    }

    #[test]
    fn golden_equation_constant() {
        assert_eq!(set_equation_constant(2, 1, 2.5), "EQCSD 2,1,2.5");
    }

    #[test]
    fn golden_save_settings() {
        assert_eq!(save_settings(1), "SSETD 1");
    }

    #[test]
    fn golden_recall_settings() {
        assert_eq!(recall_settings(5), "RSETD 5");
    }

    #[test]
    fn fake_channel_output_command_updates_state() {
        let mut dev = FakeOe1022d::new(DeviceId::new("oe1022d_01"));

        dev.send_command("FPOPD 1,17").unwrap();
        dev.send_command("SPEDD 2,1").unwrap();
        dev.send_command("CAUXD 1,2.5").unwrap();

        let resp = dev.query("FPOPD? 1").unwrap();
        assert_eq!(resp.to_string(), "17");

        let resp = dev.query("SPEDD? 2").unwrap();
        assert_eq!(resp.to_string(), "1");

        let resp = dev.query("CAUXD? 1").unwrap();
        assert_eq!(resp.to_string(), "2.5");
    }

    #[test]
    fn fake_sweep_command_updates_state() {
        let mut dev = FakeOe1022d::new(DeviceId::new("oe1022d_01"));

        dev.send_command("SWTPD 2,1").unwrap();
        dev.send_command("SLLMD 2,1000").unwrap();
        dev.send_command("SWRMD 2,2").unwrap();

        assert_eq!(dev.channel(2).unwrap().sweep_type, 1);
        assert_eq!(dev.channel(2).unwrap().sweep_start_hz, 1000.0);
        assert_eq!(dev.channel(2).unwrap().sweep_run_mode, 2);
    }

    #[test]
    fn fake_equation_command_updates_state() {
        let mut dev = FakeOe1022d::new(DeviceId::new("oe1022d_01"));

        dev.send_command("EQCDD 2,1,0,1,2").unwrap();
        dev.send_command("EQCSD 2,1,5.0").unwrap();

        assert_eq!(dev.channel(2).unwrap().equation_config[0], [0, 1, 2]);
        assert_eq!(dev.channel(2).unwrap().equation_c1, 5.0);
    }
}
