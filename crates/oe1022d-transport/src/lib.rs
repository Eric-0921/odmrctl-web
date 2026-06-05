//! oe1022d-transport — OE1022D serial transport layer.
//!
//! ## Pitfall coverage (K1..K8) — verified against
//! `oe1022d_rust_demo/VERIFICATION_REPORT.md` (2026-05-31) and
//! `docs/lab-bringup/device_connection_initialization_audit.md` (2026-06-04).
//!
//! | ID  | Pitfall                                                   | Handled in                                    |
//! |-----|-----------------------------------------------------------|-----------------------------------------------|
//! | K1  | `read_exact()` reads prior residue from input buffer     | [`idn::probe`] — uses `port.clear(Input)` first |
//! | K2  | Baud rate must be **921600**, not 115200                 | [`constants::OE1022D_BAUD_RATE`]              |
//! | K3  | RALL? has **no terminator**; fixed 12288 bytes            | (C4 will use this; declared here as constant) |
//! | K4  | macOS returns ~1020 bytes per read (~13 reads per frame)  | (C4) — loop-read until 12288                  |
//! | K5  | IDN? tail is `\0`-padded, not a comms failure            | [`idn::parse_idn`] — tolerates trailing NULs   |
//! | K6  | RALL? needs ~800 ms after RALL? before frame is ready     | (C4) — sleep 800 ms before reading            |
//! | K7  | `/dev/cu.usbmodem*` is USB CDC, not RS232                 | [`constants`] documents this; no special case  |
//! | K8  | Single-process device lock to avoid concurrent takeover   | [`guard::DeviceLock`]                          |
//!
//! C3 scope: serial enumeration, IDN? probe, IDN parsing, pitfall constants,
//! single-process device lock. RALL? reader is C4.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod constants;
pub mod discover;
pub mod guard;
pub mod idn;
pub mod port;
pub mod rall;
pub mod serial;

pub use constants::{OE1022D_BAUD_RATE, RALL_FRAME_BYTES};
pub use discover::{discover_oe1022d, DiscoverError, DiscoveredOe1022d};
pub use guard::{DeviceLock, DeviceLockError};
pub use idn::{IdnProbeError, IdnResponse, probe_idn};
pub use port::{enumerate_ports, PortInfo, PortKind};
pub use rall::{
    spawn_continuous_rall_loop, ContinuousRallHandle, MockFrameSource, MockRallLink,
    RallLink, RallLinkError, RallReader, RawFrameEnvelope, TransportStatus,
};
pub use serial::{pin_current_thread_to_core, SerialLinkError, SerialRallLink};

/// Marker for the C1 scaffold: lets `cargo build` produce something
/// useful and gives a single sanity test target.
pub const SCAFFOLD_VERSION: &str = "0.1.0-c3";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_compiles() {
        assert_eq!(SCAFFOLD_VERSION, "0.1.0-c3");
    }

    #[test]
    fn k2_baud_rate_constant() {
        // K2: must be 921600, not 115200
        assert_eq!(OE1022D_BAUD_RATE, 921_600);
    }

    #[test]
    fn k3_rall_frame_size_constant() {
        // K3: RALL? returns fixed 12288 bytes
        assert_eq!(RALL_FRAME_BYTES, 12_288);
    }
}
