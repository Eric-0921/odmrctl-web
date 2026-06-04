//! CNI Laser off-only preflight probe (M2).
//!
//! **SAFETY**: This probe ONLY sends `laser_off` frames.
//! It NEVER sends `laser_on` or `set_power`.
//!
//! Protocol: binary frame over serial, 9600 8N1.
//! See `docs/equipment_manual/CNI Laser psu-sr/RS232语言协议_恒功率.md`

use crate::error::PreflightError;
use crate::types::{DeviceConfig, DevicePreflightReport, SafeState};
use cni_laser_fake_driver::protocol::CniFrame;
use std::io::{Read, Write};
use std::time::Duration;

/// Probe a CNI laser via serial port.
///
/// Steps:
/// 1. Open serial port at 9600 baud
/// 2. Send `laser_off` frame
/// 3. Verify frame was written successfully (no response expected from device)
/// 4. Mark safe_state as confirmed = true (we forced it off)
pub fn probe(device: &DeviceConfig) -> Result<DevicePreflightReport, PreflightError> {
    let timeout_ms = device.timeout_ms.unwrap_or(5000);
    let port_path = device.address.clone();

    // 1. Open serial port
    let mut port = serialport::new(&port_path, 9600)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| PreflightError::PhysicalUnreachable {
            device_id: device.device_id.clone(),
            detail: format!("open serial {}: {}", port_path, e),
        })?;

    // 2. Clear input buffer
    let _ = port.clear(serialport::ClearBuffer::Input);

    // 3. Send laser_off frame
    let off_frame = CniFrame::laser_off();
    let off_bytes = off_frame.to_bytes();

    port.write_all(&off_bytes)
        .and_then(|_| port.flush())
        .map_err(|e| PreflightError::SerialError {
            device_id: device.device_id.clone(),
            detail: format!("write laser_off: {}", e),
        })?;

    // Small delay to let device process
    std::thread::sleep(Duration::from_millis(100));

    // 4. Attempt to read any response (protocol does not specify responses,
    //    but some firmware versions may echo or send ack)
    let mut resp_buf = [0u8; 32];
    let _resp_len: usize = port.read(&mut resp_buf).unwrap_or_default();

    // Identity: CNI lasers have no *IDN? equivalent.
    // We use the port path + "CNI Laser" as a synthetic identity.
    let identity_display = format!("CNI Laser @ {}", port_path);

    // Safe state: we just sent OFF, so it should be safe.
    let safe_state = SafeState {
        confirmed: true,
        rf_output: None,
        modulation: None,
        fm: None,
        magnetic_output: None,
        magnetic_current_ma: None,
    };

    Ok(DevicePreflightReport {
        device_id: device.device_id.clone(),
        kind: device.kind.clone(),
        reachability: true,
        identity_raw: Some(off_bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")),
        identity_display: Some(identity_display),
        error_queue: vec![],
        safe_state: Some(safe_state),
        warnings: vec!["No response from laser (expected for this protocol)".into()],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_laser_off_frame_bytes() {
        let frame = CniFrame::laser_off();
        let bytes = frame.to_bytes();
        assert_eq!(bytes, vec![0x55, 0xAA, 0x03, 0x00, 0x03]);
    }
}
