//! odmr-maynuo-m8812 — Real serial transport for Maynuo M8812 DC current source.
//!
//! This crate is the real-hardware bridge for the magnetic-axis line.
//! Capabilities grow with each Mag milestone:
//!
//! Mag-M2A : enumerate ports, open, `*IDN?` query
//! Mag-M2B : `SYST:REM`, `VOLT 75`, `CURR 0.00000`,
//!           `OUTP 0|1`, `MEAS:CURR?`, `SYST:LOC`
//!
//! It does **not** implement nonzero current, zero-lock, executor, or GUI
//! integration — those remain in odmr-mag or future milestones.

use odmr_device::{Device, DeviceStatus};
use odmr_types::{DeviceId, DeviceKind};
use serde::{Deserialize, Serialize};
use serialport::{
    ClearBuffer, DataBits, FlowControl, Parity, SerialPort, SerialPortType, StopBits,
};
use std::fmt;
use std::io::{self, Write};
use std::time::Duration;

use std::sync::LazyLock;

fn is_allowed(cmd: &str) -> bool {
    let trimmed = cmd.trim();
    // Exact-match set commands and queries
    static EXACT: LazyLock<Vec<&str>> = LazyLock::new(|| {
        vec![
            "*IDN?",
            "SYST:REM",
            "SYST:LOC",
            "VOLT 75",
            "OUTP 0",
            "OUTP 1",
            "MEAS:CURR?",
        ]
    });
    if EXACT.contains(&trimmed) {
        return true;
    }
    // CURR pattern: "CURR <float>"
    if let Some(val) = trimmed.strip_prefix("CURR ") {
        return val.parse::<f64>().is_ok();
    }
    false
}

/// Serial configuration for Maynuo M8812 identity probing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaynuoSerialPortConfig {
    pub baudrate: u32,
    pub data_bits: u8,
    pub parity: String,
    pub stop_bits: u8,
    pub flow_control: String,
    pub dtr: bool,
    pub read_timeout_ms: u64,
}

impl Default for MaynuoSerialPortConfig {
    fn default() -> Self {
        Self {
            baudrate: 9600,
            data_bits: 8,
            parity: "none".into(),
            stop_bits: 1,
            flow_control: "none".into(),
            dtr: true,
            read_timeout_ms: 300,
        }
    }
}

/// Best-effort metadata from serial-port enumeration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaynuoPortMetadata {
    pub port_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb_serial_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb_vid: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb_pid: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
}

impl MaynuoPortMetadata {
    pub fn from_port_info(info: &serialport::SerialPortInfo) -> Self {
        let (port_type, usb_serial_number, usb_vid, usb_pid, manufacturer, product) =
            match &info.port_type {
                SerialPortType::UsbPort(usb) => (
                    Some("usb".into()),
                    usb.serial_number.clone(),
                    Some(usb.vid),
                    Some(usb.pid),
                    usb.manufacturer.clone(),
                    usb.product.clone(),
                ),
                SerialPortType::BluetoothPort => {
                    (Some("bluetooth".into()), None, None, None, None, None)
                }
                SerialPortType::PciPort => (Some("pci".into()), None, None, None, None, None),
                SerialPortType::Unknown => (Some("unknown".into()), None, None, None, None, None),
            };

        Self {
            port_path: info.port_name.clone(),
            port_type,
            usb_serial_number,
            usb_vid,
            usb_pid,
            manufacturer,
            product,
        }
    }
}

/// Structured probe errors for Maynuo identity discovery.
#[derive(Debug)]
pub enum MaynuoProbeError {
    UnsupportedCommand { command: String },
    EnumerateFailed(String),
    OpenFailed { port_path: String, message: String },
    ConfigureFailed { port_path: String, message: String },
    ClearFailed { port_path: String, message: String },
    WriteFailed { port_path: String, message: String },
    FlushFailed { port_path: String, message: String },
    ReadFailed { port_path: String, message: String },
    Timeout { port_path: String },
    EmptyResponse { port_path: String },
    NonAsciiResponse { port_path: String },
    ParseFloat { port_path: String, raw: String },
}

impl fmt::Display for MaynuoProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MaynuoProbeError::UnsupportedCommand { command } => {
                write!(
                    f,
                    "unsupported command for Maynuo identity probe: {command}"
                )
            }
            MaynuoProbeError::EnumerateFailed(message) => {
                write!(f, "failed to enumerate serial ports: {message}")
            }
            MaynuoProbeError::OpenFailed { port_path, message } => {
                write!(f, "open {port_path}: {message}")
            }
            MaynuoProbeError::ConfigureFailed { port_path, message } => {
                write!(f, "configure {port_path}: {message}")
            }
            MaynuoProbeError::ClearFailed { port_path, message } => {
                write!(f, "clear {port_path}: {message}")
            }
            MaynuoProbeError::WriteFailed { port_path, message } => {
                write!(f, "write {port_path}: {message}")
            }
            MaynuoProbeError::FlushFailed { port_path, message } => {
                write!(f, "flush {port_path}: {message}")
            }
            MaynuoProbeError::ReadFailed { port_path, message } => {
                write!(f, "read {port_path}: {message}")
            }
            MaynuoProbeError::Timeout { port_path } => write!(f, "read timeout on {port_path}"),
            MaynuoProbeError::EmptyResponse { port_path } => {
                write!(f, "empty response on {port_path}")
            }
            MaynuoProbeError::NonAsciiResponse { port_path } => {
                write!(f, "response on {port_path} is not valid ASCII")
            }
            MaynuoProbeError::ParseFloat { port_path, raw } => {
                write!(f, "cannot parse float from {port_path}: {raw}")
            }
        }
    }
}

impl std::error::Error for MaynuoProbeError {}

/// Real serial transport bound to one port for identity-only probing.
pub struct MaynuoM8812Transport {
    device_id: DeviceId,
    port_path: String,
    config: MaynuoSerialPortConfig,
    port: Box<dyn SerialPort>,
    status: DeviceStatus,
}

impl MaynuoM8812Transport {
    pub fn enumerate_ports() -> Result<Vec<MaynuoPortMetadata>, MaynuoProbeError> {
        let ports = serialport::available_ports()
            .map_err(|e| MaynuoProbeError::EnumerateFailed(e.to_string()))?;
        Ok(ports
            .iter()
            .map(MaynuoPortMetadata::from_port_info)
            .collect())
    }

    pub fn open(
        device_id: DeviceId,
        port_path: &str,
        config: MaynuoSerialPortConfig,
    ) -> Result<Self, MaynuoProbeError> {
        let builder = serialport::new(port_path, config.baudrate)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(Duration::from_millis(config.read_timeout_ms))
            .dtr_on_open(config.dtr);
        let port = builder.open().map_err(|e| MaynuoProbeError::OpenFailed {
            port_path: port_path.into(),
            message: e.to_string(),
        })?;
        Ok(Self {
            device_id,
            port_path: port_path.into(),
            config,
            port,
            status: DeviceStatus {
                connected: true,
                error_queue_len: 0,
                busy: false,
            },
        })
    }

    pub fn port_path(&self) -> &str {
        &self.port_path
    }

    pub fn config(&self) -> &MaynuoSerialPortConfig {
        &self.config
    }

    // ---- query methods (write-then-read) ----

    /// Send `*IDN?` and read the identity string.
    pub fn query_idn(&mut self) -> Result<String, MaynuoProbeError> {
        self.query_response_line("*IDN?")
    }

    /// Send `MEAS:CURR?` and parse the response as amperes (A).
    pub fn query_meas_current(&mut self) -> Result<f64, MaynuoProbeError> {
        let response = self.query_response_line("MEAS:CURR?")?;
        response
            .parse::<f64>()
            .map_err(|_| MaynuoProbeError::ParseFloat {
                port_path: self.port_path.clone(),
                raw: response,
            })
    }

    // ---- set (fire-and-forget) methods ----

    /// Send `SYST:REM` to put the device in remote mode.
    pub fn send_set_remote(&mut self) -> Result<(), MaynuoProbeError> {
        self.write_command("SYST:REM")
    }

    /// Send `VOLT {v}`. Only `VOLT 75` is in the M2B allowlist.
    pub fn send_set_voltage(&mut self, voltage_v: u16) -> Result<(), MaynuoProbeError> {
        self.write_command(&format!("VOLT {}", voltage_v))
    }

    /// Send `CURR {a:.5}`. Only `CURR 0.00000` is in the M2B allowlist.
    pub fn send_set_current(&mut self, current_a: f64) -> Result<(), MaynuoProbeError> {
        self.write_command(&format!("CURR {:.5}", current_a))
    }

    /// Send `OUTP 0` or `OUTP 1`.
    pub fn send_set_output(&mut self, on: bool) -> Result<(), MaynuoProbeError> {
        self.write_command(if on { "OUTP 1" } else { "OUTP 0" })
    }

    /// Send `SYST:LOC` to return the device to local mode.
    pub fn send_set_local(&mut self) -> Result<(), MaynuoProbeError> {
        self.write_command("SYST:LOC")
    }

    // ---- internal command dispatch ----

    /// Write a command without waiting for a response (for set commands).
    fn write_command(&mut self, command: &str) -> Result<(), MaynuoProbeError> {
        if !is_allowed(command) {
            return Err(MaynuoProbeError::UnsupportedCommand {
                command: command.into(),
            });
        }
        self.port
            .clear(ClearBuffer::Input)
            .map_err(|e| MaynuoProbeError::ClearFailed {
                port_path: self.port_path.clone(),
                message: e.to_string(),
            })?;
        self.port
            .write_all(format!("{}\n", command.trim()).as_bytes())
            .map_err(|e| MaynuoProbeError::WriteFailed {
                port_path: self.port_path.clone(),
                message: e.to_string(),
            })?;
        self.port
            .flush()
            .map_err(|e| MaynuoProbeError::FlushFailed {
                port_path: self.port_path.clone(),
                message: e.to_string(),
            })?;
        Ok(())
    }

    /// Write a command and read back a response line (for query commands).
    fn query_response_line(&mut self, command: &str) -> Result<String, MaynuoProbeError> {
        if !is_allowed(command) {
            return Err(MaynuoProbeError::UnsupportedCommand {
                command: command.into(),
            });
        }
        self.port
            .clear(ClearBuffer::Input)
            .map_err(|e| MaynuoProbeError::ClearFailed {
                port_path: self.port_path.clone(),
                message: e.to_string(),
            })?;
        self.port
            .write_all(format!("{}\n", command.trim()).as_bytes())
            .map_err(|e| MaynuoProbeError::WriteFailed {
                port_path: self.port_path.clone(),
                message: e.to_string(),
            })?;
        self.port
            .flush()
            .map_err(|e| MaynuoProbeError::FlushFailed {
                port_path: self.port_path.clone(),
                message: e.to_string(),
            })?;
        read_ascii_line(self.port.as_mut(), &self.port_path)
    }
}

impl Device for MaynuoM8812Transport {
    fn id(&self) -> &DeviceId {
        &self.device_id
    }

    fn kind(&self) -> DeviceKind {
        DeviceKind::MagnetXyz
    }

    fn status(&self) -> DeviceStatus {
        self.status.clone()
    }
}

fn read_ascii_line(port: &mut dyn SerialPort, port_path: &str) -> Result<String, MaynuoProbeError> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match port.read(&mut byte) {
            Ok(0) => continue,
            Ok(1) => {
                if byte[0] == b'\n' {
                    break;
                }
                if byte[0] != b'\r' {
                    buf.push(byte[0]);
                }
            }
            Ok(_) => unreachable!(),
            Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                if buf.is_empty() {
                    return Err(MaynuoProbeError::Timeout {
                        port_path: port_path.into(),
                    });
                }
                break;
            }
            Err(e) => {
                return Err(MaynuoProbeError::ReadFailed {
                    port_path: port_path.into(),
                    message: e.to_string(),
                })
            }
        }
    }
    if buf.is_empty() {
        return Err(MaynuoProbeError::EmptyResponse {
            port_path: port_path.into(),
        });
    }
    if !buf.is_ascii() {
        return Err(MaynuoProbeError::NonAsciiResponse {
            port_path: port_path.into(),
        });
    }
    let line = String::from_utf8(buf).expect("ascii buffer must decode");
    let trimmed = line.trim().to_string();
    if trimmed.is_empty() {
        return Err(MaynuoProbeError::EmptyResponse {
            port_path: port_path.into(),
        });
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io::Read;

    #[derive(Default)]
    struct FakePort {
        reads: VecDeque<Result<u8, io::ErrorKind>>,
        writes: Vec<u8>,
        flushed: bool,
        timeout: Duration,
    }

    impl Read for FakePort {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            match self.reads.pop_front() {
                Some(Ok(b)) => {
                    buf[0] = b;
                    Ok(1)
                }
                Some(Err(kind)) => Err(io::Error::from(kind)),
                None => Err(io::Error::from(io::ErrorKind::TimedOut)),
            }
        }
    }

    impl Write for FakePort {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flushed = true;
            Ok(())
        }
    }

    impl SerialPort for FakePort {
        fn name(&self) -> Option<String> {
            None
        }
        fn baud_rate(&self) -> serialport::Result<u32> {
            Ok(9600)
        }
        fn data_bits(&self) -> serialport::Result<DataBits> {
            Ok(DataBits::Eight)
        }
        fn flow_control(&self) -> serialport::Result<FlowControl> {
            Ok(FlowControl::None)
        }
        fn parity(&self) -> serialport::Result<Parity> {
            Ok(Parity::None)
        }
        fn stop_bits(&self) -> serialport::Result<StopBits> {
            Ok(StopBits::One)
        }
        fn timeout(&self) -> Duration {
            self.timeout
        }
        fn set_baud_rate(&mut self, _: u32) -> serialport::Result<()> {
            Ok(())
        }
        fn set_data_bits(&mut self, _: DataBits) -> serialport::Result<()> {
            Ok(())
        }
        fn set_flow_control(&mut self, _: FlowControl) -> serialport::Result<()> {
            Ok(())
        }
        fn set_parity(&mut self, _: Parity) -> serialport::Result<()> {
            Ok(())
        }
        fn set_stop_bits(&mut self, _: StopBits) -> serialport::Result<()> {
            Ok(())
        }
        fn set_timeout(&mut self, timeout: Duration) -> serialport::Result<()> {
            self.timeout = timeout;
            Ok(())
        }
        fn write_request_to_send(&mut self, _: bool) -> serialport::Result<()> {
            Ok(())
        }
        fn write_data_terminal_ready(&mut self, _: bool) -> serialport::Result<()> {
            Ok(())
        }
        fn read_clear_to_send(&mut self) -> serialport::Result<bool> {
            Ok(true)
        }
        fn read_data_set_ready(&mut self) -> serialport::Result<bool> {
            Ok(true)
        }
        fn read_ring_indicator(&mut self) -> serialport::Result<bool> {
            Ok(false)
        }
        fn read_carrier_detect(&mut self) -> serialport::Result<bool> {
            Ok(true)
        }
        fn bytes_to_read(&self) -> serialport::Result<u32> {
            Ok(self.reads.len() as u32)
        }
        fn bytes_to_write(&self) -> serialport::Result<u32> {
            Ok(0)
        }
        fn clear(&self, _: ClearBuffer) -> serialport::Result<()> {
            Ok(())
        }
        fn try_clone(&self) -> serialport::Result<Box<dyn SerialPort>> {
            Ok(Box::new(FakePort::default()))
        }
        fn set_break(&self) -> serialport::Result<()> {
            Ok(())
        }
        fn clear_break(&self) -> serialport::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn read_ascii_line_reads_single_line() {
        let mut port = FakePort {
            reads: b"MAYNUO,M8812,080020960220402020,V2.7\r\n"
                .iter()
                .copied()
                .map(Ok)
                .collect(),
            ..Default::default()
        };
        let line = read_ascii_line(&mut port, "fake").unwrap();
        assert_eq!(line, "MAYNUO,M8812,080020960220402020,V2.7");
    }

    #[test]
    fn read_ascii_line_returns_timeout_when_no_data() {
        let mut port = FakePort::default();
        let err = read_ascii_line(&mut port, "fake").unwrap_err();
        assert!(matches!(err, MaynuoProbeError::Timeout { .. }));
    }

    #[test]
    fn read_ascii_line_rejects_non_ascii() {
        let mut reads = VecDeque::new();
        reads.push_back(Ok(0xff));
        reads.push_back(Ok(b'\n'));
        let mut port = FakePort {
            reads,
            ..Default::default()
        };
        let err = read_ascii_line(&mut port, "fake").unwrap_err();
        assert!(matches!(err, MaynuoProbeError::NonAsciiResponse { .. }));
    }

    #[test]
    fn write_command_rejects_unknown_command() {
        let port = FakePort::default();
        let mut transport = MaynuoM8812Transport {
            device_id: DeviceId::new("mag_x"),
            port_path: "fake".into(),
            config: MaynuoSerialPortConfig::default(),
            port: Box::new(port),
            status: DeviceStatus::default(),
        };
        let err = transport.write_command("MEAS:VOLT?").unwrap_err();
        assert!(matches!(err, MaynuoProbeError::UnsupportedCommand { .. }));
    }

    #[test]
    fn write_command_sends_syst_rem() {
        let port = FakePort::default();
        let mut transport = MaynuoM8812Transport {
            device_id: DeviceId::new("mag_x"),
            port_path: "fake".into(),
            config: MaynuoSerialPortConfig::default(),
            port: Box::new(port),
            status: DeviceStatus::default(),
        };
        // SYST:REM is in the allowlist — write_command should succeed.
        assert!(transport.write_command("SYST:REM").is_ok());
    }

    #[test]
    fn write_command_sends_curr_zero() {
        let port = FakePort::default();
        let mut transport = MaynuoM8812Transport {
            device_id: DeviceId::new("mag_x"),
            port_path: "fake".into(),
            config: MaynuoSerialPortConfig::default(),
            port: Box::new(port),
            status: DeviceStatus::default(),
        };
        assert!(transport.write_command("CURR 0.00000").is_ok());
    }

    #[test]
    fn write_command_allows_nonzero_curr() {
        let port = FakePort::default();
        let mut transport = MaynuoM8812Transport {
            device_id: DeviceId::new("mag_x"),
            port_path: "fake".into(),
            config: MaynuoSerialPortConfig::default(),
            port: Box::new(port),
            status: DeviceStatus::default(),
        };
        // CURR 0.01000 = 10 mA — should be allowed by pattern match
        assert!(transport.write_command("CURR 0.01000").is_ok());
    }

    #[test]
    fn write_command_rejects_malformed_curr() {
        let port = FakePort::default();
        let mut transport = MaynuoM8812Transport {
            device_id: DeviceId::new("mag_x"),
            port_path: "fake".into(),
            config: MaynuoSerialPortConfig::default(),
            port: Box::new(port),
            status: DeviceStatus::default(),
        };
        let err = transport.write_command("CURR abc").unwrap_err();
        assert!(matches!(err, MaynuoProbeError::UnsupportedCommand { .. }));
    }

    #[test]
    fn write_command_rejects_outp_2() {
        let port = FakePort::default();
        let mut transport = MaynuoM8812Transport {
            device_id: DeviceId::new("mag_x"),
            port_path: "fake".into(),
            config: MaynuoSerialPortConfig::default(),
            port: Box::new(port),
            status: DeviceStatus::default(),
        };
        let err = transport.write_command("OUTP 2").unwrap_err();
        assert!(matches!(err, MaynuoProbeError::UnsupportedCommand { .. }));
    }

    #[test]
    fn write_command_sends_outp_on() {
        let port = FakePort::default();
        let mut transport = MaynuoM8812Transport {
            device_id: DeviceId::new("mag_x"),
            port_path: "fake".into(),
            config: MaynuoSerialPortConfig::default(),
            port: Box::new(port),
            status: DeviceStatus::default(),
        };
        assert!(transport.write_command("OUTP 1").is_ok());
    }

    #[test]
    fn query_meas_current_parses_response() {
        let mut reads = VecDeque::new();
        for b in b"0.00015\r\n" {
            reads.push_back(Ok(*b));
        }
        let port = FakePort {
            reads,
            ..Default::default()
        };
        let mut transport = MaynuoM8812Transport {
            device_id: DeviceId::new("mag_x"),
            port_path: "fake".into(),
            config: MaynuoSerialPortConfig::default(),
            port: Box::new(port),
            status: DeviceStatus::default(),
        };
        let current = transport.query_meas_current().unwrap();
        assert!((current - 0.00015).abs() < 1e-9);
    }

    #[test]
    fn query_meas_current_rejects_non_float() {
        let mut reads = VecDeque::new();
        for b in b"garbage\r\n" {
            reads.push_back(Ok(*b));
        }
        let port = FakePort {
            reads,
            ..Default::default()
        };
        let mut transport = MaynuoM8812Transport {
            device_id: DeviceId::new("mag_x"),
            port_path: "fake".into(),
            config: MaynuoSerialPortConfig::default(),
            port: Box::new(port),
            status: DeviceStatus::default(),
        };
        let err = transport.query_meas_current().unwrap_err();
        assert!(matches!(err, MaynuoProbeError::ParseFloat { .. }));
    }

    #[test]
    fn write_command_rejects_syst_rem_with_typo() {
        let port = FakePort::default();
        let mut transport = MaynuoM8812Transport {
            device_id: DeviceId::new("mag_x"),
            port_path: "fake".into(),
            config: MaynuoSerialPortConfig::default(),
            port: Box::new(port),
            status: DeviceStatus::default(),
        };
        let err = transport.write_command("SYSTEM:REM").unwrap_err();
        assert!(matches!(err, MaynuoProbeError::UnsupportedCommand { .. }));
    }
}
