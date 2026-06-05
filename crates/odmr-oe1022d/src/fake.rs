//! Fake OE1022D — deterministic test double with no hardware access.

use odmr_device::{Device, DeviceError, DeviceResponse, DeviceStatus, FakeDevice};
use odmr_types::{DeviceId, DeviceKind};

/// State of a single OE1022D channel (Ch-A or Ch-B).
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelState {
    pub reference_source: u8, // 0=Ext, 1=Int, 2=IntSweep
    pub ref_slope: u8,        // 0=TTL Rising Edge, 1=Sine Zero Crossing
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
    pub sync_filter: u8,         // 0=Off, 1=On
    // Sine Output
    pub sine_out_mode: u8,           // 0=Fixed, 1=Linear, 2=Log, 3=DC
    pub sine_out_voltage: f64,       // Vrms
    pub sine_out_start_voltage: f64, // Vrms
    pub sine_out_stop_voltage: f64,  // Vrms
    pub sine_out_linear_step: f64,   // Vrms
    pub sine_out_log_step: f64,      // %
    pub sine_out_step_time: u32,     // ms
    pub sine_out_run_mode: u8,       // 0=Stop, 1=Single, 2=Loop
    pub sine_out_dc_voltage: f64,    // Vdc
    // Reference Sweep
    pub sweep_type: u8, // 0=Linear, 1=Log
    pub sweep_start_hz: f64,
    pub sweep_stop_hz: f64,
    pub sweep_linear_step_hz: f64,
    pub sweep_log_step_pct: f64,
    pub sweep_step_time_ms: u32,
    pub sweep_run_mode: u8, // 0=Stop, 1=Single, 2=Loop
    // Equation
    pub equation_config: [[u8; 3]; 4], // [E1..E4][A,B,C]
    pub equation_c1: f64,
    pub equation_c2: f64,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            reference_source: 0, // External
            ref_slope: 0,        // TTL Rising Edge
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
            sync_filter: 0, // Off
            // Sine Output defaults
            sine_out_mode: 0, // Fixed
            sine_out_voltage: 0.001,
            sine_out_start_voltage: 0.001,
            sine_out_stop_voltage: 5.0,
            sine_out_linear_step: 0.001,
            sine_out_log_step: 0.0,
            sine_out_step_time: 100,
            sine_out_run_mode: 0, // Stop
            sine_out_dc_voltage: 0.0,
            // Reference Sweep defaults
            sweep_type: 0, // Linear
            sweep_start_hz: 0.0,
            sweep_stop_hz: 102000.0,
            sweep_linear_step_hz: 1.0,
            sweep_log_step_pct: 0.0,
            sweep_step_time_ms: 100,
            sweep_run_mode: 0, // Stop
            // Equation defaults
            equation_config: [[0; 3]; 4],
            equation_c1: 0.0,
            equation_c2: 0.0,
        }
    }
}

/// Fake OE1022D instrument for mock-first testing.
pub struct FakeOe1022d {
    id: DeviceId,
    ch_a: ChannelState,
    ch_b: ChannelState,
    idn: String,
    // Rear-panel channel output (CH1/CH2) — global, not per lock-in channel
    ch1_output_source: u8,   // 0..34 (FPOPD)
    ch2_output_source: u8,   // 0..34
    ch1_output_speed: u8,    // 0=Slow, 1=Fast (SPEDD)
    ch2_output_speed: u8,    // 0=Slow, 1=Fast
    ch1_auxout_voltage: f64, // Vdc (CAUXD)
    ch2_auxout_voltage: f64, // Vdc
}

impl FakeOe1022d {
    pub fn new(id: DeviceId) -> Self {
        Self {
            id,
            ch_a: ChannelState::default(),
            ch_b: ChannelState::default(),
            idn: "SSI,OE1022D,12345678,V1.5".to_string(),
            ch1_output_source: 0, // A-R
            ch2_output_source: 0, // A-R
            ch1_output_speed: 0,  // Slow
            ch2_output_speed: 0,  // Slow
            ch1_auxout_voltage: 0.0,
            ch2_auxout_voltage: 0.0,
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

    fn parse_three_u8_f64(cmd: &str) -> Result<(u8, u8, f64), DeviceError> {
        let mut parts = cmd.split(',');
        let a = parts
            .next()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .ok_or_else(|| DeviceError::InvalidParameter {
                cmd: cmd.to_string(),
                reason: "expected three args: u8,u8,f64".to_string(),
            })?;
        let b = parts
            .next()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .ok_or_else(|| DeviceError::InvalidParameter {
                cmd: cmd.to_string(),
                reason: "missing second u8".to_string(),
            })?;
        let c = parts
            .next()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .ok_or_else(|| DeviceError::InvalidParameter {
                cmd: cmd.to_string(),
                reason: "missing f64".to_string(),
            })?;
        Ok((a, b, c))
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
            "ILIND" => with_ch!(set_line_notch_filter, line_notch_filter),
            "SENSD" => with_ch!(set_sensitivity, sensitivity_index),
            "RMODD" => with_ch!(set_dynamic_reserve, dynamic_reserve),
            "OFLTD" => with_ch!(set_time_constant, time_constant_index),
            "OFSLD" => with_ch!(set_filter_slope, filter_slope),
            "HARMD" => with_ch!(set_harmonic, harmonic),
            "SYNCD" => with_ch!(set_sync_filter, sync_filter),
            // Sine Output
            "SWVTD" => with_ch!(set_sine_out_mode, sine_out_mode),
            "SLVLD" => with_ch_f64!(set_sine_out_voltage, sine_out_voltage),
            "SVLLD" => with_ch_f64!(set_sine_out_start_voltage, sine_out_start_voltage),
            "SVULD" => with_ch_f64!(set_sine_out_stop_voltage, sine_out_stop_voltage),
            "SVSLD" => with_ch_f64!(set_sine_out_linear_step, sine_out_linear_step),
            "SVSGD" => with_ch_f64!(set_sine_out_log_step, sine_out_log_step),
            "SVTMD" => {
                let (ch, val) = Self::parse_two_u8(rest)?;
                let state = self
                    .channel_mut(ch)
                    .ok_or_else(|| DeviceError::InvalidParameter {
                        cmd: cmd.to_string(),
                        reason: format!("invalid channel {ch}"),
                    })?;
                state.sine_out_step_time = val as u32;
                Ok(DeviceResponse::Ack)
            }
            "SVRMD" => with_ch!(set_sine_out_run_mode, sine_out_run_mode),
            "SVDCD" => with_ch_f64!(set_sine_out_dc_voltage, sine_out_dc_voltage),
            // Channel Output
            "FPOPD" => {
                let (ch, val) = Self::parse_two_u8(rest)?;
                match ch {
                    1 => self.ch1_output_source = val,
                    2 => self.ch2_output_source = val,
                    _ => {
                        return Err(DeviceError::InvalidParameter {
                            cmd: cmd.to_string(),
                            reason: format!("invalid rear-panel channel {ch}"),
                        })
                    }
                }
                Ok(DeviceResponse::Ack)
            }
            "SPEDD" => {
                let (ch, val) = Self::parse_two_u8(rest)?;
                match ch {
                    1 => self.ch1_output_speed = val,
                    2 => self.ch2_output_speed = val,
                    _ => {
                        return Err(DeviceError::InvalidParameter {
                            cmd: cmd.to_string(),
                            reason: format!("invalid rear-panel channel {ch}"),
                        })
                    }
                }
                Ok(DeviceResponse::Ack)
            }
            "CAUXD" => {
                let (ch, val) = Self::parse_channel_u8_f64(rest)?;
                match ch {
                    1 => self.ch1_auxout_voltage = val,
                    2 => self.ch2_auxout_voltage = val,
                    _ => {
                        return Err(DeviceError::InvalidParameter {
                            cmd: cmd.to_string(),
                            reason: format!("invalid rear-panel channel {ch}"),
                        })
                    }
                }
                Ok(DeviceResponse::Ack)
            }
            "OEXPD" => {
                // Offset/Expand has 4 parameters (j,k,x,l) — accept but don't store detail in fake
                Ok(DeviceResponse::Ack)
            }
            // Reference Sweep
            "SWTPD" => with_ch!(set_sweep_type, sweep_type),
            "SLLMD" => with_ch_f64!(set_sweep_start, sweep_start_hz),
            "SULMD" => with_ch_f64!(set_sweep_stop, sweep_stop_hz),
            "SSLLD" => with_ch_f64!(set_sweep_linear_step, sweep_linear_step_hz),
            "SSLGD" => with_ch_f64!(set_sweep_log_step, sweep_log_step_pct),
            "STLMD" => {
                let (ch, val) = Self::parse_two_u8(rest)?;
                let state = self
                    .channel_mut(ch)
                    .ok_or_else(|| DeviceError::InvalidParameter {
                        cmd: cmd.to_string(),
                        reason: format!("invalid channel {ch}"),
                    })?;
                state.sweep_step_time_ms = val as u32;
                Ok(DeviceResponse::Ack)
            }
            "SWRMD" => with_ch!(set_sweep_run_mode, sweep_run_mode),
            // Auto Settings (action commands — no state stored in fake)
            "AGAND" | "ARSVD" | "APHSD" | "ASCLD" => Ok(DeviceResponse::Ack),
            // Equation
            "EQCDD" => {
                // EQCDD i,j,k,l,m — parse 5 args
                let mut parts = rest.split(',');
                let ch = parts
                    .next()
                    .and_then(|s| s.trim().parse::<u8>().ok())
                    .ok_or_else(|| DeviceError::InvalidParameter {
                        cmd: cmd.to_string(),
                        reason: "expected channel".to_string(),
                    })?;
                let eq = parts
                    .next()
                    .and_then(|s| s.trim().parse::<u8>().ok())
                    .ok_or_else(|| DeviceError::InvalidParameter {
                        cmd: cmd.to_string(),
                        reason: "expected equation index".to_string(),
                    })?;
                let a = parts
                    .next()
                    .and_then(|s| s.trim().parse::<u8>().ok())
                    .unwrap_or(0);
                let b = parts
                    .next()
                    .and_then(|s| s.trim().parse::<u8>().ok())
                    .unwrap_or(0);
                let c = parts
                    .next()
                    .and_then(|s| s.trim().parse::<u8>().ok())
                    .unwrap_or(0);
                let state = self
                    .channel_mut(ch)
                    .ok_or_else(|| DeviceError::InvalidParameter {
                        cmd: cmd.to_string(),
                        reason: format!("invalid channel {ch}"),
                    })?;
                if (1..=4).contains(&eq) {
                    state.equation_config[(eq - 1) as usize] = [a, b, c];
                }
                Ok(DeviceResponse::Ack)
            }
            "EQCSD" => {
                let (ch, idx, val) = Self::parse_three_u8_f64(rest)?;
                let state = self
                    .channel_mut(ch)
                    .ok_or_else(|| DeviceError::InvalidParameter {
                        cmd: cmd.to_string(),
                        reason: format!("invalid channel {ch}"),
                    })?;
                match idx {
                    1 => state.equation_c1 = val,
                    2 => state.equation_c2 = val,
                    _ => {}
                }
                Ok(DeviceResponse::Ack)
            }
            // Save / Recall (action commands — no state stored in fake)
            "SSETD" | "RSETD" => Ok(DeviceResponse::Ack),
            "*RSTD" => {
                self.ch_a = ChannelState::default();
                self.ch_b = ChannelState::default();
                Ok(DeviceResponse::Ack)
            }
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
            "ILIND" => state.line_notch_filter.to_string(),
            "SENSD" => state.sensitivity_index.to_string(),
            "RMODD" => state.dynamic_reserve.to_string(),
            "OFLTD" => state.time_constant_index.to_string(),
            "OFSLD" => state.filter_slope.to_string(),
            "HARMD" => state.harmonic.to_string(),
            "SYNCD" => state.sync_filter.to_string(),
            // Sine Output queries
            "SWVTD" => state.sine_out_mode.to_string(),
            "SLVLD" => state.sine_out_voltage.to_string(),
            "SVLLD" => state.sine_out_start_voltage.to_string(),
            "SVULD" => state.sine_out_stop_voltage.to_string(),
            "SVSLD" => state.sine_out_linear_step.to_string(),
            "SVSGD" => state.sine_out_log_step.to_string(),
            "SVTMD" => state.sine_out_step_time.to_string(),
            "SVRMD" => state.sine_out_run_mode.to_string(),
            "SVDCD" => state.sine_out_dc_voltage.to_string(),
            // Channel Output queries
            "FPOPD" => {
                let ch: u8 = rest.parse().map_err(|_| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "expected channel number".to_string(),
                })?;
                match ch {
                    1 => self.ch1_output_source.to_string(),
                    2 => self.ch2_output_source.to_string(),
                    _ => {
                        return Err(DeviceError::InvalidParameter {
                            cmd: cmd.to_string(),
                            reason: format!("invalid rear-panel channel {ch}"),
                        })
                    }
                }
            }
            "SPEDD" => {
                let ch: u8 = rest.parse().map_err(|_| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "expected channel number".to_string(),
                })?;
                match ch {
                    1 => self.ch1_output_speed.to_string(),
                    2 => self.ch2_output_speed.to_string(),
                    _ => {
                        return Err(DeviceError::InvalidParameter {
                            cmd: cmd.to_string(),
                            reason: format!("invalid rear-panel channel {ch}"),
                        })
                    }
                }
            }
            "CAUXD" => {
                let ch: u8 = rest.parse().map_err(|_| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "expected channel number".to_string(),
                })?;
                match ch {
                    1 => self.ch1_auxout_voltage.to_string(),
                    2 => self.ch2_auxout_voltage.to_string(),
                    _ => {
                        return Err(DeviceError::InvalidParameter {
                            cmd: cmd.to_string(),
                            reason: format!("invalid rear-panel channel {ch}"),
                        })
                    }
                }
            }
            "OEXPD" => "0,1".to_string(), // simplified fake response
            // Reference Sweep queries
            "SWTPD" => state.sweep_type.to_string(),
            "SLLMD" => state.sweep_start_hz.to_string(),
            "SULMD" => state.sweep_stop_hz.to_string(),
            "SSLLD" => state.sweep_linear_step_hz.to_string(),
            "SSLGD" => state.sweep_log_step_pct.to_string(),
            "STLMD" => state.sweep_step_time_ms.to_string(),
            "SWRMD" => state.sweep_run_mode.to_string(),
            // Equation queries
            "EQCDD" => {
                let rest_trim = rest.trim();
                let mut parts = rest_trim.split(',');
                let ch: u8 = parts
                    .next()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(1);
                let eq: u8 = parts
                    .next()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(1);
                let state = self.channel(ch).unwrap_or(&self.ch_a);
                let cfg = if (1..=4).contains(&eq) {
                    state.equation_config[(eq - 1) as usize]
                } else {
                    [0, 0, 0]
                };
                format!("{},{},{}", cfg[0], cfg[1], cfg[2])
            }
            "EQCSD" => {
                let rest_trim = rest.trim();
                let mut parts = rest_trim.split(',');
                let ch: u8 = parts
                    .next()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(1);
                let idx: u8 = parts
                    .next()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(1);
                let state = self.channel(ch).unwrap_or(&self.ch_a);
                let val = if idx == 1 {
                    state.equation_c1
                } else {
                    state.equation_c2
                };
                val.to_string()
            }
            "INOVD" => "0".to_string(),
            "GNOVD" => "0".to_string(),
            "*PLLD" => "0".to_string(),
            "OUTPD" => "0.0".to_string(),
            "SNAPD" => "0.0,0.0,0.0,0.0".to_string(),
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
