//! odmr-oe1022d — Command catalog and fake device for the SSI OE1022D DSP Lock-In Amplifier.

pub mod commands;
pub mod fake;

pub use fake::{ChannelState, FakeOe1022d};

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
}
