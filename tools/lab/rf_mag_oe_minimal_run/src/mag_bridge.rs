//! Maynuo M8812 magnetic transport wrapper for Mag-M5A.
//! Wraps odmr-maynuo-m8812 crate with identity probe, zero-baseline, recur, cleanup.

use crate::types::CommandAuditEntry;
use odmr_mag::{
    expected_sn_from_idn, parse_maynuo_idn, MaynuoAxisProfile,
};
use odmr_maynuo_m8812::{
    MaynuoM8812Transport, MaynuoPortMetadata, MaynuoProbeError, MaynuoSerialPortConfig,
};
use std::thread::sleep;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Real Maynuo transport wrapper
// ---------------------------------------------------------------------------

pub struct MaynuoTransport {
    inner: MaynuoM8812Transport,
}

impl MaynuoTransport {
    pub fn from_transport(inner: MaynuoM8812Transport) -> Self {
        MaynuoTransport { inner }
    }

    pub fn enumerate_ports() -> Result<Vec<MaynuoPortMetadata>, MaynuoProbeError> {
        MaynuoM8812Transport::enumerate_ports()
    }

    pub fn open(port_path: &str, config: &MaynuoSerialPortConfig) -> Result<Self, MaynuoProbeError> {
        let inner = MaynuoM8812Transport::open(
            odmr_types::DeviceId::new("maynuo_m8812"),
            port_path,
            config.clone(),
        )?;
        Ok(MaynuoTransport { inner })
    }

    pub fn query_idn(&mut self) -> Result<String, MaynuoProbeError> {
        self.inner.query_idn()
    }

    pub fn query_meas_current(&mut self) -> Result<f64, MaynuoProbeError> {
        self.inner.query_meas_current()
    }

    pub fn send_set_remote(&mut self) -> Result<(), MaynuoProbeError> {
        self.inner.send_set_remote()
    }

    pub fn send_set_voltage(&mut self, v: u16) -> Result<(), MaynuoProbeError> {
        self.inner.send_set_voltage(v)
    }

    pub fn send_set_current(&mut self, current_a: f64) -> Result<(), MaynuoProbeError> {
        self.inner.send_set_current(current_a)
    }

    pub fn send_set_output(&mut self, on: bool) -> Result<(), MaynuoProbeError> {
        self.inner.send_set_output(on)
    }

    pub fn send_set_local(&mut self) -> Result<(), MaynuoProbeError> {
        self.inner.send_set_local()
    }
}

// ---------------------------------------------------------------------------
// Fake Maynuo transport for testing
// ---------------------------------------------------------------------------

pub struct FakeMaynuoTransport {
    pub idn: String,
    pub output_on: bool,
    pub current_a: f64,
    pub remote_mode: bool,
    pub meas_noise_ma: f64,
    pub meas_counter: u64,
}

impl FakeMaynuoTransport {
    pub fn new(idn: &str) -> Self {
        FakeMaynuoTransport {
            idn: idn.into(),
            output_on: false,
            current_a: 0.0,
            remote_mode: false,
            meas_noise_ma: 0.001,
            meas_counter: 0,
        }
    }

    pub fn query_idn(&mut self) -> Result<String, String> {
        Ok(self.idn.clone())
    }

    pub fn query_meas_current(&mut self) -> Result<f64, String> {
        self.meas_counter += 1;
        // Return current in amperes with small noise
        let noise = (self.meas_counter as f64 * 0.0001).sin() * self.meas_noise_ma / 1000.0;
        Ok(self.current_a + noise)
    }

    pub fn send_set_remote(&mut self) -> Result<(), String> {
        self.remote_mode = true;
        Ok(())
    }

    pub fn send_set_voltage(&mut self, _v: u16) -> Result<(), String> {
        Ok(())
    }

    pub fn send_set_current(&mut self, current_a: f64) -> Result<(), String> {
        self.current_a = current_a;
        Ok(())
    }

    pub fn send_set_output(&mut self, on: bool) -> Result<(), String> {
        self.output_on = on;
        Ok(())
    }

    pub fn send_set_local(&mut self) -> Result<(), String> {
        self.remote_mode = false;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Identity probe helpers (shared between real and fake)
// ---------------------------------------------------------------------------

pub struct ProbeResult {
    pub port_path: String,
    pub idn_raw: Option<String>,
    pub sn: Option<String>,
    pub error: Option<String>,
}

pub fn probe_all_ports(
    ports: &[MaynuoPortMetadata],
    config: &MaynuoSerialPortConfig,
) -> Vec<ProbeResult> {
    let mut results = Vec::new();
    for port in ports {
        let mut result = ProbeResult {
            port_path: port.port_path.clone(),
            idn_raw: None,
            sn: None,
            error: None,
        };
        match MaynuoTransport::open(&port.port_path, config) {
            Ok(mut t) => match t.query_idn() {
                Ok(idn) => {
                    result.idn_raw = Some(idn.clone());
                    if let Ok(parsed) = parse_maynuo_idn(&idn) {
                        result.sn = Some(parsed.serial_number);
                    }
                }
                Err(e) => {
                    result.error = Some(format!("IDN query failed: {}", e));
                }
            },
            Err(e) => {
                result.error = Some(format!("Open failed: {}", e));
            }
        }
        results.push(result);
    }
    results
}

pub fn find_axis_port(
    axis_profile: &MaynuoAxisProfile,
    probes: &[ProbeResult],
) -> Result<(String, String), String> {
    let expected_sn = expected_sn_from_idn(&axis_profile.expected_idn)
        .map_err(|e| format!("parse expected_idn: {}", e))?;

    for probe in probes {
        if let Some(ref sn) = probe.sn {
            if sn == &expected_sn {
                if let Some(ref idn) = probe.idn_raw {
                    return Ok((probe.port_path.clone(), idn.clone()));
                }
            }
        }
    }

    Err(format!(
        "No port matched axis '{}' with expected SN '{}'",
        axis_profile.axis_id, expected_sn
    ))
}

// ---------------------------------------------------------------------------
// Zero-baseline + recur + cleanup helpers
// ---------------------------------------------------------------------------

pub fn run_zero_baseline(
    transport: &mut MaynuoTransport,
    settle_ms: u64,
    samples: u64,
    axis_id: &str,
    audit: &mut Vec<CommandAuditEntry>,
) -> Result<(f64, f64), String> {
    // SYST:REM
    transport
        .send_set_remote()
        .map_err(|e| format!("SYST:REM: {}", e))?;
    push_mag_audit(audit, axis_id, "SYST:REM", "set_remote", false, None);

    // VOLT 75
    transport
        .send_set_voltage(75)
        .map_err(|e| format!("VOLT 75: {}", e))?;
    push_mag_audit(audit, axis_id, "VOLT 75", "set_voltage", false, None);

    // CURR 0
    transport
        .send_set_current(0.0)
        .map_err(|e| format!("CURR 0: {}", e))?;
    push_mag_audit(audit, axis_id, "CURR 0.00000", "set_current", false, None);

    // OUTP 1
    transport
        .send_set_output(true)
        .map_err(|e| format!("OUTP 1: {}", e))?;
    push_mag_audit(audit, axis_id, "OUTP 1", "set_output", true, None);

    // Settle
    sleep(Duration::from_millis(settle_ms));

    // MEAS:CURR? x N
    let mut readings = Vec::new();
    for _ in 0..samples {
        let current_a = transport
            .query_meas_current()
            .map_err(|e| format!("MEAS:CURR?: {}", e))?;
        readings.push(current_a * 1000.0); // convert to mA
        push_mag_audit(
            audit,
            axis_id,
            "MEAS:CURR?",
            "query_current",
            false,
            Some(format!("{:.6}", current_a)),
        );
    }

    let mean = readings.iter().sum::<f64>() / readings.len() as f64;
    let variance = readings
        .iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>()
        / readings.len() as f64;
    let std = variance.sqrt();

    Ok((mean, std))
}

pub fn run_recur_setpoint(
    transport: &mut MaynuoTransport,
    recur_current_ma: f64,
    zero_current_ma: f64,
    settle_ms: u64,
    samples: u64,
    coil_nt_per_ma: f64,
    axis_id: &str,
    audit: &mut Vec<CommandAuditEntry>,
) -> Result<(f64, f64, f64), String> {
    let total_current_ma = zero_current_ma + recur_current_ma;
    let current_a = total_current_ma / 1000.0;

    // CURR {nonzero}
    transport
        .send_set_current(current_a)
        .map_err(|e| format!("CURR recur: {}", e))?;
    push_mag_audit(
        audit,
        axis_id,
        &format!("CURR {:.5}", current_a),
        "set_current",
        true,
        None,
    );

    // Settle
    sleep(Duration::from_millis(settle_ms));

    // MEAS:CURR? x N
    let mut readings = Vec::new();
    for _ in 0..samples {
        let current_a = transport
            .query_meas_current()
            .map_err(|e| format!("MEAS:CURR?: {}", e))?;
        readings.push(current_a * 1000.0); // mA
        push_mag_audit(
            audit,
            axis_id,
            "MEAS:CURR?",
            "query_current",
            false,
            Some(format!("{:.6}", current_a)),
        );
    }

    let total_mean = readings.iter().sum::<f64>() / readings.len() as f64;
    let measured_recur_ma = total_mean - zero_current_ma;
    let measured_recur_nt = measured_recur_ma * coil_nt_per_ma;

    Ok((total_mean, measured_recur_ma, measured_recur_nt))
}

pub fn run_cleanup(
    transport: &mut MaynuoTransport,
    axis_id: &str,
    audit: &mut Vec<CommandAuditEntry>,
) -> Result<(), String> {
    // CURR 0 first
    let _ = transport.send_set_current(0.0);
    push_mag_audit(audit, axis_id, "CURR 0.00000", "set_current", false, None);

    // OUTP 0
    let _ = transport.send_set_output(false);
    push_mag_audit(audit, axis_id, "OUTP 0", "set_output", true, None);

    // SYST:LOC
    let _ = transport.send_set_local();
    push_mag_audit(audit, axis_id, "SYST:LOC", "set_local", false, None);

    Ok(())
}

// ---------------------------------------------------------------------------
// Fake variants of the workflow helpers
// ---------------------------------------------------------------------------

pub fn fake_run_zero_baseline(
    transport: &mut FakeMaynuoTransport,
    settle_ms: u64,
    samples: u64,
    axis_id: &str,
    audit: &mut Vec<CommandAuditEntry>,
) -> Result<(f64, f64), String> {
    transport.send_set_remote().ok();
    push_mag_audit(audit, axis_id, "SYST:REM", "set_remote", false, None);
    transport.send_set_voltage(75).ok();
    push_mag_audit(audit, axis_id, "VOLT 75", "set_voltage", false, None);
    transport.send_set_current(0.0).ok();
    push_mag_audit(audit, axis_id, "CURR 0.00000", "set_current", false, None);
    transport.send_set_output(true).ok();
    push_mag_audit(audit, axis_id, "OUTP 1", "set_output", true, None);

    sleep(Duration::from_millis(settle_ms));

    let mut readings = Vec::new();
    for _ in 0..samples {
        let current_a = transport.query_meas_current()?;
        readings.push(current_a * 1000.0);
        push_mag_audit(
            audit,
            axis_id,
            "MEAS:CURR?",
            "query_current",
            false,
            Some(format!("{:.6}", current_a)),
        );
    }

    let mean = readings.iter().sum::<f64>() / readings.len() as f64;
    let variance = readings.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / readings.len() as f64;
    let std = variance.sqrt();

    Ok((mean, std))
}

pub fn fake_run_recur_setpoint(
    transport: &mut FakeMaynuoTransport,
    recur_current_ma: f64,
    zero_current_ma: f64,
    settle_ms: u64,
    samples: u64,
    coil_nt_per_ma: f64,
    axis_id: &str,
    audit: &mut Vec<CommandAuditEntry>,
) -> Result<(f64, f64, f64), String> {
    let total_current_ma = zero_current_ma + recur_current_ma;
    let current_a = total_current_ma / 1000.0;

    transport.send_set_current(current_a).ok();
    push_mag_audit(
        audit,
        axis_id,
        &format!("CURR {:.5}", current_a),
        "set_current",
        true,
        None,
    );

    sleep(Duration::from_millis(settle_ms));

    let mut readings = Vec::new();
    for _ in 0..samples {
        let current_a = transport.query_meas_current()?;
        readings.push(current_a * 1000.0);
        push_mag_audit(
            audit,
            axis_id,
            "MEAS:CURR?",
            "query_current",
            false,
            Some(format!("{:.6}", current_a)),
        );
    }

    let total_mean = readings.iter().sum::<f64>() / readings.len() as f64;
    let measured_recur_ma = total_mean - zero_current_ma;
    let measured_recur_nt = measured_recur_ma * coil_nt_per_ma;

    Ok((total_mean, measured_recur_ma, measured_recur_nt))
}

pub fn fake_run_cleanup(
    transport: &mut FakeMaynuoTransport,
    axis_id: &str,
    audit: &mut Vec<CommandAuditEntry>,
) {
    transport.send_set_current(0.0).ok();
    push_mag_audit(audit, axis_id, "CURR 0.00000", "set_current", false, None);
    transport.send_set_output(false).ok();
    push_mag_audit(audit, axis_id, "OUTP 0", "set_output", true, None);
    transport.send_set_local().ok();
    push_mag_audit(audit, axis_id, "SYST:LOC", "set_local", false, None);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn push_mag_audit(
    audit: &mut Vec<CommandAuditEntry>,
    axis_id: &str,
    command: &str,
    command_class: &str,
    safety_relevant: bool,
    response_preview: Option<String>,
) {
    audit.push(CommandAuditEntry {
        seq: audit.len() as u64,
        timestamp_unix_ms: now_ms(),
        device_id: format!("maynuo_{}", axis_id),
        command: command.into(),
        command_class: command_class.into(),
        allowed: true,
        sent_to_transport: true,
        rejection_reason: None,
        response_preview,
        transport_error: None,
        safety_relevant,
    });
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
