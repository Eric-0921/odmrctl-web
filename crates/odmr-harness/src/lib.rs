//! odmr-harness — Test fixtures, fake devices, and mock transports for the ODMR system.
//!
//! This crate provides deterministic fake implementations of hardware devices
//! so that discovery logic, drivers, and integration tests can run without
//! touching real instruments.

use odmr_device::{Device, DeviceError, DeviceResponse, DeviceStatus, FakeDevice};
use odmr_types::{DeviceId, DeviceKind};

// ---------------------------------------------------------------------------
// Fake SMB100A
// ---------------------------------------------------------------------------

/// A fake R&S SMB100A that responds to safe read-only queries.
///
/// # Safety
/// This fake rejects any command not in the safe-query allow-list,
/// mirroring the behavior of the real discovery tool.
pub struct FakeSmb100a {
    id: DeviceId,
    status: DeviceStatus,
    idn: String,
    output_state: bool,
    mod_state: bool,
}

impl FakeSmb100a {
    pub fn new(id: &str) -> Self {
        Self {
            id: DeviceId::new(id),
            status: DeviceStatus {
                connected: true,
                error_queue_len: 0,
                busy: false,
            },
            idn: "Rohde&Schwarz,SMB100A,1406.6000k02/101623,3.1.19.15-3.20.390.24".to_string(),
            output_state: false,
            mod_state: false,
        }
    }

    /// Create a fake that believes RF output is ON (for testing query paths).
    pub fn with_output_on(id: &str) -> Self {
        let mut s = Self::new(id);
        s.output_state = true;
        s
    }

    /// Return a map of all M2.1 read-only query responses for this fake.
    pub fn readonly_snapshot(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        map.insert("*IDN?".to_string(), self.idn.clone());
        map.insert("SYST:ERR?".to_string(), "0,\"No error\"".to_string());
        map.insert(
            "OUTP?".to_string(),
            if self.output_state {
                "1".to_string()
            } else {
                "0".to_string()
            },
        );
        map.insert(
            "MOD:STAT?".to_string(),
            if self.mod_state {
                "1".to_string()
            } else {
                "0".to_string()
            },
        );
        map.insert("FREQ:MODE?".to_string(), "CW".to_string());
        map.insert("FREQ?".to_string(), "2882000000".to_string());
        map.insert("POW?".to_string(), "-15".to_string());
        map.insert("POW:ALC?".to_string(), "AUTO".to_string());
        map.insert("FM:STAT?".to_string(), "0".to_string());
        map.insert("FM:SOUR?".to_string(), "INT".to_string());
        map.insert("FM:DEV?".to_string(), "4000000".to_string());
        map.insert("LFO?".to_string(), "1".to_string());
        map.insert("LFO:FREQ?".to_string(), "500".to_string());
        map.insert("LFO:VOLT?".to_string(), "0.137".to_string());
        map.insert("LFO:SHAP?".to_string(), "SQUARE".to_string());
        map.insert("SWE:MODE?".to_string(), "AUTO".to_string());
        map.insert("SWE:SPAC?".to_string(), "LIN".to_string());
        map.insert("SWE:FREQ:STEP?".to_string(), "100000".to_string());
        map.insert("SWE:FREQ:DWEL?".to_string(), "100".to_string());
        map.insert("FREQ:STAR?".to_string(), "2820000000".to_string());
        map.insert("FREQ:STOP?".to_string(), "2920000000".to_string());
        map
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
        self.status.clone()
    }
}

impl FakeDevice for FakeSmb100a {
    fn send_command(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        let trimmed = cmd.trim();
        match trimmed {
            "OUTP OFF" => {
                self.output_state = false;
                Ok(DeviceResponse::Ack)
            }
            "MOD:STAT OFF" => {
                self.mod_state = false;
                Ok(DeviceResponse::Ack)
            }
            _ => Err(DeviceError::UnknownCommand(trimmed.to_string())),
        }
    }

    fn query(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        let trimmed = cmd.trim();
        match trimmed {
            "*IDN?" => Ok(DeviceResponse::Value(self.idn.clone())),
            "SYST:ERR?" => Ok(DeviceResponse::Value("0, 'No error'".to_string())),
            "OUTP?" => Ok(DeviceResponse::Value(if self.output_state {
                "1".to_string()
            } else {
                "0".to_string()
            })),
            "MOD:STAT?" => Ok(DeviceResponse::Value(if self.mod_state {
                "1".to_string()
            } else {
                "0".to_string()
            })),
            _ => Err(DeviceError::UnknownCommand(trimmed.to_string())),
        }
    }

    fn idn(&self) -> &str {
        &self.idn
    }
}

// ---------------------------------------------------------------------------
// Fake OE1022D
// ---------------------------------------------------------------------------

/// A fake SSI OE1022D that responds to safe read-only queries.
pub struct FakeOe1022d {
    id: DeviceId,
    status: DeviceStatus,
    idn: String,
}

impl FakeOe1022d {
    pub fn new(id: &str) -> Self {
        Self {
            id: DeviceId::new(id),
            status: DeviceStatus {
                connected: true,
                error_queue_len: 0,
                busy: false,
            },
            idn: "SSI LIA-OE1022D,SN:D6522078,Version:Ver6.3200831".to_string(),
        }
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
        self.status.clone()
    }
}

impl FakeOe1022d {
    /// Return a map of all M2.1 read-only query responses for this fake.
    pub fn readonly_snapshot(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        map.insert("*IDN?".to_string(), self.idn.clone());
        map.insert("FMODD? 2".to_string(), "0".to_string());
        map.insert("RSLPD? 2".to_string(), "0".to_string());
        map.insert("FREQD? 2".to_string(), "500".to_string());
        map.insert("PHASD? 2".to_string(), "0.00".to_string());
        map.insert("ISRCD? 2".to_string(), "0".to_string());
        map.insert("SENSD? 2".to_string(), "15".to_string());
        map.insert("OFLTD? 2".to_string(), "10".to_string());
        map.insert("OFSLD? 2".to_string(), "1".to_string());
        map.insert("HARMD? 2".to_string(), "1".to_string());
        map.insert(
            "RALL?".to_string(),
            "X=1.234E-3,Y=5.678E-4,R=1.357E-3,Theta=26.56,Freq=500.00,Noise=1.234E-5".to_string(),
        );
        map
    }
}

impl FakeDevice for FakeOe1022d {
    fn send_command(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        let trimmed = cmd.trim();
        // OE1022D discovery phase does not send setters; reject them for safety.
        Err(DeviceError::UnknownCommand(trimmed.to_string()))
    }

    fn query(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        let trimmed = cmd.trim();
        match trimmed {
            "*IDN?" => Ok(DeviceResponse::Value(self.idn.clone())),
            _ => Err(DeviceError::UnknownCommand(trimmed.to_string())),
        }
    }

    fn idn(&self) -> &str {
        &self.idn
    }
}

// ---------------------------------------------------------------------------
// Fake MAYNUO M8812 (mag_axis)
// ---------------------------------------------------------------------------

/// A fake MAYNUO M8812 magnetic axis current source.
pub struct FakeMagAxis {
    id: DeviceId,
    status: DeviceStatus,
    idn: String,
    #[allow(dead_code)]
    axis: char,
}

impl FakeMagAxis {
    pub fn new(id: &str, sn: &str, axis: char) -> Self {
        Self {
            id: DeviceId::new(id),
            status: DeviceStatus {
                connected: true,
                error_queue_len: 0,
                busy: false,
            },
            idn: format!("MAYNUO,M8812,{},V2.7", sn),
            axis,
        }
    }
}

impl Device for FakeMagAxis {
    fn id(&self) -> &DeviceId {
        &self.id
    }

    fn kind(&self) -> DeviceKind {
        DeviceKind::MagnetXyz
    }

    fn status(&self) -> DeviceStatus {
        self.status.clone()
    }
}

impl FakeDevice for FakeMagAxis {
    fn send_command(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        let trimmed = cmd.trim();
        match trimmed {
            "SYST:REM" | "SYST:LOC" => Ok(DeviceResponse::Ack),
            _ if trimmed.starts_with("VOLT ") => Ok(DeviceResponse::Ack),
            _ if trimmed.starts_with("CURR ") => Ok(DeviceResponse::Ack),
            _ if trimmed.starts_with("OUTP ") => Ok(DeviceResponse::Ack),
            _ => Err(DeviceError::UnknownCommand(trimmed.to_string())),
        }
    }

    fn query(&mut self, cmd: &str) -> Result<DeviceResponse, DeviceError> {
        let trimmed = cmd.trim();
        match trimmed {
            "*IDN?" => Ok(DeviceResponse::Value(self.idn.clone())),
            "MEAS:CURR?" => Ok(DeviceResponse::Value("0.00000".to_string())),
            _ => Err(DeviceError::UnknownCommand(trimmed.to_string())),
        }
    }

    fn idn(&self) -> &str {
        &self.idn
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_smb100a_idn() {
        let mut dev = FakeSmb100a::new("smb100a.main");
        assert_eq!(
            dev.query("*IDN?").unwrap().to_string(),
            "Rohde&Schwarz,SMB100A,1406.6000k02/101623,3.1.19.15-3.20.390.24"
        );
    }

    #[test]
    fn fake_smb100a_rejects_setter() {
        let mut dev = FakeSmb100a::new("smb100a.main");
        // send_command only accepts safe disconnect commands, not general setters
        assert!(dev.send_command("OUTP ON").is_err());
        assert!(dev.send_command("MOD:STAT ON").is_err());
        assert!(dev.send_command("FREQ:MODE SWE").is_err());
    }

    #[test]
    fn fake_smb100a_safe_disconnect() {
        let mut dev = FakeSmb100a::with_output_on("smb100a.main");
        assert_eq!(dev.query("OUTP?").unwrap().to_string(), "1");
        dev.send_command("OUTP OFF").unwrap();
        assert_eq!(dev.query("OUTP?").unwrap().to_string(), "0");
    }

    #[test]
    fn fake_oe1022d_idn() {
        let mut dev = FakeOe1022d::new("oe1022d.main");
        assert_eq!(
            dev.query("*IDN?").unwrap().to_string(),
            "SSI LIA-OE1022D,SN:D6522078,Version:Ver6.3200831"
        );
    }

    #[test]
    fn fake_oe1022d_rejects_all_setters() {
        let mut dev = FakeOe1022d::new("oe1022d.main");
        assert!(dev.send_command("RALL?").is_err());
        assert!(dev.send_command("FMODD 2,0").is_err());
    }

    #[test]
    fn fake_mag_axis_idn() {
        let mut dev = FakeMagAxis::new("mag.x", "080020960220402020", 'x');
        assert_eq!(
            dev.query("*IDN?").unwrap().to_string(),
            "MAYNUO,M8812,080020960220402020,V2.7"
        );
    }

    #[test]
    fn fake_mag_axis_measures_zero() {
        let mut dev = FakeMagAxis::new("mag.x", "080020960220402020", 'x');
        assert_eq!(dev.query("MEAS:CURR?").unwrap().to_string(), "0.00000");
    }

    #[test]
    fn fake_smb100a_readonly_snapshot_has_all_queries() {
        let dev = FakeSmb100a::new("smb100a.main");
        let snap = dev.readonly_snapshot();
        assert!(snap.contains_key("*IDN?"));
        assert!(snap.contains_key("OUTP?"));
        assert!(snap.contains_key("FREQ?"));
        assert_eq!(snap["OUTP?"], "0");
    }

    #[test]
    fn fake_oe1022d_readonly_snapshot_has_all_queries() {
        let dev = FakeOe1022d::new("oe1022d.main");
        let snap = dev.readonly_snapshot();
        assert!(snap.contains_key("*IDN?"));
        assert!(snap.contains_key("RALL?"));
        assert!(snap.contains_key("FMODD? 2"));
        assert_eq!(snap["FMODD? 2"], "0");
    }
}
