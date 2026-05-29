//! Fake SMB100A — deterministic test double with no hardware access.

use odmr_device::{Device, DeviceError, DeviceResponse, DeviceStatus, FakeDevice};
use odmr_types::{DeviceId, DeviceKind};

/// Internal state of a fake SMB100A.
#[derive(Debug, Clone, PartialEq)]
pub struct Smb100aState {
    pub rf_frequency_hz: f64,
    pub rf_power_dbm: f64,
    pub rf_output_enabled: bool,
    pub modulation_global_enabled: bool,
    pub fm_enabled: bool,
    pub fm_source: String,
    pub fm_deviation_hz: f64,
    pub lf_output_enabled: bool,
    pub lf_frequency_hz: f64,
    pub lf_voltage_v: f64,
    pub lf_shape: String,
    pub freq_start_hz: f64,
    pub freq_stop_hz: f64,
    pub sweep_step_hz: f64,
    pub sweep_dwell_ms: u64,
    pub sweep_spacing: String,
    pub sweep_mode: String,
}

impl Default for Smb100aState {
    fn default() -> Self {
        Self {
            rf_frequency_hz: 1e9,
            rf_power_dbm: -30.0,
            rf_output_enabled: false,
            modulation_global_enabled: false,
            fm_enabled: false,
            fm_source: "INT".to_string(),
            fm_deviation_hz: 0.0,
            lf_output_enabled: false,
            lf_frequency_hz: 1000.0,
            lf_voltage_v: 0.0,
            lf_shape: "SINE".to_string(),
            freq_start_hz: 1e9,
            freq_stop_hz: 2e9,
            sweep_step_hz: 1e6,
            sweep_dwell_ms: 10,
            sweep_spacing: "LIN".to_string(),
            sweep_mode: "AUTO".to_string(),
        }
    }
}

/// Fake SMB100A instrument for mock-first testing.
pub struct FakeSmb100a {
    id: DeviceId,
    state: Smb100aState,
    idn: String,
}

impl FakeSmb100a {
    pub fn new(id: DeviceId) -> Self {
        Self {
            id,
            state: Smb100aState::default(),
            idn: "Rohde&Schwarz,SMB100A,12345678,5.00.116.00".to_string(),
        }
    }

    pub fn state(&self) -> &Smb100aState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut Smb100aState {
        &mut self.state
    }

    fn handle_set(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Err(DeviceError::UnknownCommand("(empty)".to_string()));
        }

        let head = parts[0].to_ascii_uppercase();
        let tail = parts.get(1).copied();

        match head.as_str() {
            "OUTP" => {
                let st = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing ON/OFF".to_string(),
                })?;
                self.state.rf_output_enabled = parse_on_off(st)?;
                Ok(DeviceResponse::Ack)
            }
            "MOD:STAT" => {
                let st = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing ON/OFF".to_string(),
                })?;
                self.state.modulation_global_enabled = parse_on_off(st)?;
                Ok(DeviceResponse::Ack)
            }
            "FREQ:MODE" => {
                let mode = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing mode".to_string(),
                })?;
                if !mode.eq_ignore_ascii_case("CW") {
                    return Err(DeviceError::InvalidParameter {
                        cmd: cmd.to_string(),
                        reason: format!("unsupported mode {mode}"),
                    });
                }
                Ok(DeviceResponse::Ack)
            }
            "FREQ" => {
                let val = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing frequency value".to_string(),
                })?;
                self.state.rf_frequency_hz = parse_freq(val)?;
                Ok(DeviceResponse::Ack)
            }
            "POW" => {
                let val = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing power value".to_string(),
                })?;
                self.state.rf_power_dbm = parse_power(val)?;
                Ok(DeviceResponse::Ack)
            }
            "POW:ALC" => {
                let val = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing ALC mode".to_string(),
                })?;
                if !val.eq_ignore_ascii_case("AUTO") && !val.eq_ignore_ascii_case("OFF") {
                    return Err(DeviceError::InvalidParameter {
                        cmd: cmd.to_string(),
                        reason: format!("unsupported ALC mode {val}"),
                    });
                }
                Ok(DeviceResponse::Ack)
            }
            "LFO" => {
                let st = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing ON/OFF".to_string(),
                })?;
                self.state.lf_output_enabled = parse_on_off(st)?;
                Ok(DeviceResponse::Ack)
            }
            "LFO:FREQ" => {
                let val = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing frequency value".to_string(),
                })?;
                self.state.lf_frequency_hz = parse_freq(val)?;
                Ok(DeviceResponse::Ack)
            }
            "LFO:VOLT" => {
                let val = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing voltage value".to_string(),
                })?;
                self.state.lf_voltage_v = parse_volt(val)?;
                Ok(DeviceResponse::Ack)
            }
            "LFO:SHAP" => {
                let shape = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing shape".to_string(),
                })?;
                self.state.lf_shape = shape.to_ascii_uppercase();
                Ok(DeviceResponse::Ack)
            }
            "FM:STAT" => {
                let st = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing ON/OFF".to_string(),
                })?;
                self.state.fm_enabled = parse_on_off(st)?;
                Ok(DeviceResponse::Ack)
            }
            "FM:SOUR" => {
                let src = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing source".to_string(),
                })?;
                self.state.fm_source = src.to_ascii_uppercase();
                Ok(DeviceResponse::Ack)
            }
            "FM:DEV" => {
                let val = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing deviation value".to_string(),
                })?;
                self.state.fm_deviation_hz = parse_freq(val)?;
                Ok(DeviceResponse::Ack)
            }
            "FREQ:STAR" => {
                let val = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing start value".to_string(),
                })?;
                self.state.freq_start_hz = parse_freq(val)?;
                Ok(DeviceResponse::Ack)
            }
            "FREQ:STOP" => {
                let val = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing stop value".to_string(),
                })?;
                self.state.freq_stop_hz = parse_freq(val)?;
                Ok(DeviceResponse::Ack)
            }
            "SWE:FREQ:STEP" => {
                let val = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing step value".to_string(),
                })?;
                self.state.sweep_step_hz = parse_freq(val)?;
                Ok(DeviceResponse::Ack)
            }
            "SWE:FREQ:DWEL" => {
                let val = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing dwell value".to_string(),
                })?;
                self.state.sweep_dwell_ms = parse_time_ms(val)?;
                Ok(DeviceResponse::Ack)
            }
            "SWE:SPAC" => {
                let val = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing spacing".to_string(),
                })?;
                self.state.sweep_spacing = val.to_ascii_uppercase();
                Ok(DeviceResponse::Ack)
            }
            "SWE:MODE" => {
                let val = tail.ok_or_else(|| DeviceError::InvalidParameter {
                    cmd: cmd.to_string(),
                    reason: "missing mode".to_string(),
                })?;
                self.state.sweep_mode = val.to_ascii_uppercase();
                Ok(DeviceResponse::Ack)
            }
            _ => Err(DeviceError::UnknownCommand(cmd.to_string())),
        }
    }

    fn handle_query(&self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        let clean = cmd.trim_end_matches('?');
        let resp = match clean {
            "*IDN" => self.idn.clone(),
            "FREQ" => format!("{:.0}", self.state.rf_frequency_hz),
            "OUTP" => on_off_str(self.state.rf_output_enabled).to_string(),
            "MOD:STAT" => on_off_str(self.state.modulation_global_enabled).to_string(),
            "POW" => format!("{:.2}", self.state.rf_power_dbm),
            "POW:ALC" => "AUTO".to_string(),
            "LFO" => on_off_str(self.state.lf_output_enabled).to_string(),
            "LFO:FREQ" => format!("{:.2}", self.state.lf_frequency_hz),
            "LFO:VOLT" => format!("{:.3}", self.state.lf_voltage_v),
            "LFO:SHAP" => self.state.lf_shape.clone(),
            "FM:STAT" => on_off_str(self.state.fm_enabled).to_string(),
            "FM:SOUR" => self.state.fm_source.clone(),
            "FM:DEV" => format!("{:.0}", self.state.fm_deviation_hz),
            "FREQ:STAR" => format!("{:.0}", self.state.freq_start_hz),
            "FREQ:STOP" => format!("{:.0}", self.state.freq_stop_hz),
            "SWE:FREQ:STEP" => format!("{:.0}", self.state.sweep_step_hz),
            "SWE:FREQ:DWEL" => format!("{}", self.state.sweep_dwell_ms),
            "SWE:SPAC" => self.state.sweep_spacing.clone(),
            "SWE:MODE" => self.state.sweep_mode.clone(),
            _ => {
                return Err(DeviceError::UnknownCommand(cmd.to_string()));
            }
        };
        Ok(DeviceResponse::Value(resp))
    }
}

impl Device for FakeSmb100a {
    fn id(&self) -> &DeviceId {
        &self.id
    }

    fn kind(&self) -> DeviceKind {
        DeviceKind::Smb100a
    }

    fn status(&self) -> DeviceStatus {
        DeviceStatus {
            connected: true,
            error_queue_len: 0,
            busy: false,
        }
    }
}

impl FakeDevice for FakeSmb100a {
    fn send_command(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        if cmd.ends_with('?') {
            return Err(DeviceError::QueryOnSetter(cmd.to_string()));
        }
        self.handle_set(cmd)
    }

    fn query(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        if !cmd.ends_with('?') {
            return Err(DeviceError::SetOnQuerier(cmd.to_string()));
        }
        self.handle_query(cmd)
    }

    fn idn(&self) -> &str {
        &self.idn
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_on_off(s: &str) -> Result<bool, DeviceError> {
    match s.to_ascii_uppercase().as_str() {
        "ON" | "1" => Ok(true),
        "OFF" | "0" => Ok(false),
        _ => Err(DeviceError::InvalidParameter {
            cmd: s.to_string(),
            reason: "expected ON or OFF".to_string(),
        }),
    }
}

fn on_off_str(v: bool) -> &'static str {
    if v {
        "ON"
    } else {
        "OFF"
    }
}

fn parse_freq(s: &str) -> Result<f64, DeviceError> {
    let s = s.to_ascii_uppercase();
    if s.ends_with("GHZ") {
        s[..s.len() - 3].parse::<f64>().map(|v| v * 1e9)
    } else if s.ends_with("MHZ") {
        s[..s.len() - 3].parse::<f64>().map(|v| v * 1e6)
    } else if s.ends_with("KHZ") {
        s[..s.len() - 3].parse::<f64>().map(|v| v * 1e3)
    } else if s.ends_with("HZ") {
        s[..s.len() - 2].parse::<f64>()
    } else {
        s.parse::<f64>()
    }
    .map_err(|_| DeviceError::InvalidParameter {
        cmd: s,
        reason: "invalid frequency format".to_string(),
    })
}

fn parse_power(s: &str) -> Result<f64, DeviceError> {
    let s = s.to_ascii_uppercase();
    if s.ends_with("DBM") {
        s[..s.len() - 3].parse::<f64>()
    } else {
        s.parse::<f64>()
    }
    .map_err(|_| DeviceError::InvalidParameter {
        cmd: s,
        reason: "invalid power format".to_string(),
    })
}

fn parse_volt(s: &str) -> Result<f64, DeviceError> {
    let s = s.to_ascii_uppercase();
    if s.ends_with("MV") {
        s[..s.len() - 2].parse::<f64>().map(|v| v * 1e-3)
    } else if s.ends_with("V") {
        s[..s.len() - 1].parse::<f64>()
    } else {
        s.parse::<f64>()
    }
    .map_err(|_| DeviceError::InvalidParameter {
        cmd: s,
        reason: "invalid voltage format".to_string(),
    })
}

fn parse_time_ms(s: &str) -> Result<u64, DeviceError> {
    let s = s.to_ascii_uppercase();
    if s.ends_with("MS") {
        s[..s.len() - 2].parse::<u64>()
    } else if s.ends_with("S") {
        s[..s.len() - 1].parse::<u64>().map(|v| v * 1000)
    } else {
        s.parse::<u64>()
    }
    .map_err(|_| DeviceError::InvalidParameter {
        cmd: s,
        reason: "invalid time format".to_string(),
    })
}
