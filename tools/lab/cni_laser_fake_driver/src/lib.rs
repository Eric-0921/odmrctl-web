//! CNI Laser Fake Driver — M1 Protocol Implementation
//!
//! Implements the binary frame protocol for CNI PSU-SR series lasers
//! (e.g., MSL-U-532-300mW) used in ODMR optical excitation.
//!
//! ## Safety Invariants
//! - `MAX_POWER_MW = 150` (conservative, device label limit)
//! - `FakeCniLaser::handle_frame` rejects `laser_on` when setpoint is 0
//! - No real serial I/O in this crate — purely in-memory simulation
//!
//! ## Protocol Reference
//! - `docs/equipment_manual/CNI Laser psu-sr/RS232语言协议_恒功率.md`
//! - Baud: 9600, 8 data bits, 1 stop bit, no parity
//! - Frame: `[0x55] [0xAA] [Command] [Data...] [Checksum]`

pub mod fake;
pub mod protocol;

pub use fake::FakeCniLaser;
pub use protocol::{CniFrame, MAX_POWER_MW, HEADER, CMD_SET_POWER, CMD_OUTPUT};
