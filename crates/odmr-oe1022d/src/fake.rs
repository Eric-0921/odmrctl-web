//! Fake OE1022D — deterministic test double with no hardware access.

use odmr_device::{Device, DeviceError, DeviceResponse, DeviceStatus, FakeDevice};
use odmr_types::{DeviceId, DeviceKind};

/// State of a single OE1022D channel (Ch-A or Ch-B).
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelState {
    pub reference_source: u8, // 0=Ext, 1=Int, 2=IntSweep
    pub ref_slope: u8,        // 0=Sine, 1=TTL Rise, 2=TTL Fall
    pub reference_frequency_hz: f64,
    pub phase_deg: f64,
    pub input_source: u8,        // 0=A, 1=A-B, 2=I(1M), 3=I(100M)
    pub input_grounding: u8,     // 0=Float, 1=Ground
    pub input_coupling: u8,      // 0=AC, 1=DC
    pub line_notch_filter: u8,   // 0=Off, 1=50Hz, 2=100Hz, 3=Both
    pub sensitivity_index: u8,   // 0..26 → 1nV..1V
    pub dynamic_reserve: u8,     // 0=High, 1=Normal, 2=LowNoise
    pub time_constant_index: u8, // 0..19 → 10us..30ks
    pub filter_slope: u8,        // 0=6, 1=12, 2=18, 3=24 dB/oct
    pub harmonic: u8,            // 1..99
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            reference_source: 0, // External
            ref_slope: 1,        // TTL Rising
            reference_frequency_hz: 500.0,
            phase_deg: 0.0,
            input_source: 0,         // A (single-ended voltage)
            input_grounding: 1,      // Ground
            input_coupling: 0,       // AC
            line_notch_filter: 0,    // Off
            sensitivity_index: 7,    // ~10 uV
            dynamic_reserve: 1,      // Normal
            time_constant_index: 10, // ~100 ms
            filter_slope: 1,         // 12 dB/oct
            harmonic: 1,
        }
    }
}

/// Fake OE1022D instrument for mock-first testing.
pub struct FakeOe1022d {
    id: DeviceId,
    ch_a: ChannelState,
    ch_b: ChannelState,
    idn: String,
}

impl FakeOe1022d {
    pub fn new(id: DeviceId) -> Self {
        Self {
            id,
            ch_a: ChannelState::default(),
            ch_b: ChannelState::default(),
            idn: "SSI,OE1022D,12345678,V1.5".to_string(),
        }
    }

    pub fn channel(&self, i: u8) -> Option<&ChannelState> {
        match i {
            1 => Some(&self.ch_a),
            2 => Some(&self.ch_b),
            _ => None,
        }
    }

    pub fn channel_mut(&mut self, i: u8) -> Option<&mut ChannelState> {
        match i {
            1 => Some(&mut self.ch_a),
            2 => Some(&mut self.ch_b),
            _ => None,
        }
    }

    fn parse_two_u8(cmd: &str) -> Result<(u8, u8), DeviceError> {
        let mut parts = cmd.split(',');
        let a = parts
            .next()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .ok_or_else(|| DeviceError::InvalidParameter {
                cmd: cmd.to_string(),
                reason: "expected two comma-separated integers".to_string(),
            })?;
        let b = parts
            .next()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .ok_or_else(|| DeviceError::InvalidParameter {
                cmd: cmd.to_string(),
                reason: "missing second argument".to_string(),
            })?;
        Ok((a, b))
    }

    fn parse_channel_u8_f64(cmd: &str) -> Result<(u8, f64), DeviceError> {
        let mut parts = cmd.split(',');
        let a = parts
            .next()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .ok_or_else(|| DeviceError::InvalidParameter {
                cmd: cmd.to_string(),
                reason: "expected channel,fvalue".to_string(),
            })?;
        let b = parts
            .next()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .ok_or_else(|| DeviceError::InvalidParameter {
                cmd: cmd.to_string(),
                reason: "missing float argument".to_string(),
            })?;
        Ok((a, b))
    }

    fn handle_set(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        let head = cmd
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        let rest = cmd[head.len()..].trim_start();

        macro_rules! with_ch {
            ($setter:ident, $field:ident) => {{
                let (ch, val) = Self::parse_two_u8(rest)?;
                let state = self
                    .channel_mut(ch)
                    .ok_or_else(|| DeviceError::InvalidParameter {
                        cmd: cmd.to_string(),
                        reason: format!("invalid channel {ch}"),
                    })?;
                state.$field = val;
                Ok(DeviceResponse::Ack)
            }};
        }

        macro_rules! with_ch_f64 {
            ($setter:ident, $field:ident) => {{
                let (ch, val) = Self::parse_channel_u8_f64(rest)?;
                let state = self
                    .channel_mut(ch)
                    .ok_or_else(|| DeviceError::InvalidParameter {
                        cmd: cmd.to_string(),
                        reason: format!("invalid channel {ch}"),
                    })?;
                state.$field = val;
                Ok(DeviceResponse::Ack)
            }};
        }

        match head.as_str() {
            "FMODD" => with_ch!(set_reference_source, reference_source),
            "RSLPD" => with_ch!(set_ref_slope, ref_slope),
            "FREQD" => with_ch_f64!(set_ref_frequency, reference_frequency_hz),
            "PHASD" => with_ch_f64!(set_phase, phase_deg),
            "ISRCD" => with_ch!(set_input_source, input_source),
            "IGNDD" => with_ch!(set_input_grounding, input_grounding),
            "ICPLD" => with_ch!(set_input_coupling, input_coupling),
            "ILNFD" => with_ch!(set_line_notch_filter, line_notch_filter),
            "SENSD" => with_ch!(set_sensitivity, sensitivity_index),
            "RMODD" => with_ch!(set_dynamic_reserve, dynamic_reserve),
            "OFLTD" => with_ch!(set_time_constant, time_constant_index),
            "OFSLD" => with_ch!(set_filter_slope, filter_slope),
            "HARMD" => with_ch!(set_harmonic, harmonic),
            _ => Err(DeviceError::UnknownCommand(cmd.to_string())),
        }
    }

    fn handle_query(&self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        // OE1022D queries have the form "CMD? i" — question mark is part of the
        // command head, not at the end.  We split on whitespace and strip '?'
        // from the first token.
        let head_token = cmd.split_whitespace().next().unwrap_or("");
        let head = head_token.trim_end_matches('?').to_ascii_uppercase();
        let rest = cmd[head_token.len()..].trim_start();

        // *IDN is a special global query that does not take a channel argument
        if head == "*IDN" {
            return Ok(DeviceResponse::Value(self.idn.clone()));
        }

        let ch: u8 = rest.parse().map_err(|_| DeviceError::InvalidParameter {
            cmd: cmd.to_string(),
            reason: "expected channel number".to_string(),
        })?;
        let state = self
            .channel(ch)
            .ok_or_else(|| DeviceError::InvalidParameter {
                cmd: cmd.to_string(),
                reason: format!("invalid channel {ch}"),
            })?;

        let val = match head.as_str() {
            "FMODD" => state.reference_source.to_string(),
            "RSLPD" => state.ref_slope.to_string(),
            "FREQD" => state.reference_frequency_hz.to_string(),
            "PHASD" => state.phase_deg.to_string(),
            "ISRCD" => state.input_source.to_string(),
            "IGNDD" => state.input_grounding.to_string(),
            "ICPLD" => state.input_coupling.to_string(),
            "ILNFD" => state.line_notch_filter.to_string(),
            "SENSD" => state.sensitivity_index.to_string(),
            "RMODD" => state.dynamic_reserve.to_string(),
            "OFLTD" => state.time_constant_index.to_string(),
            "OFSLD" => state.filter_slope.to_string(),
            "HARMD" => state.harmonic.to_string(),
            _ => return Err(DeviceError::UnknownCommand(cmd.to_string())),
        };
        Ok(DeviceResponse::Value(val))
    }
}

impl Device for FakeOe1022d {
    fn id(&self) -> &DeviceId {
        &self.id
    }

    fn kind(&self) -> DeviceKind {
        DeviceKind::Oe1022d
    }

    fn status(&self) -> DeviceStatus {
        DeviceStatus {
            connected: true,
            error_queue_len: 0,
            busy: false,
        }
    }
}

impl FakeDevice for FakeOe1022d {
    fn send_command(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        // OE1022D queries have the form "CMD? i" (question mark before arguments)
        if cmd.contains('?') {
            return Err(DeviceError::QueryOnSetter(cmd.to_string()));
        }
        self.handle_set(cmd)
    }

    fn query(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        if !cmd.contains('?') {
            return Err(DeviceError::SetOnQuerier(cmd.to_string()));
        }
        self.handle_query(cmd)
    }

    fn idn(&self) -> &str {
        &self.idn
    }
}
