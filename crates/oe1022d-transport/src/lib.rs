//! oe1022d-transport — OE1022D serial transport layer.
//!
//! Responsibilities (C3+ will fill in):
//! 1. Enumerate available serial ports (`serialport::available_ports`).
//! 2. Probe a port with `*IDN?` and parse the response.
//! 3. Open a port at 921600 baud, 8N1, no flow control.
//! 4. Issue `RALL?\r` and read exactly 12288 bytes, handling the macOS
//!    chunked-read pitfall (K2) and the read_exact residue pitfall (K1).
//! 5. Continuously cycle the above with jitter < 10 ms (D1).
//!
//! This file is the C1 placeholder. Real code lands in C3 and C4.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

/// Marker for the C1 scaffold: lets `cargo build` produce something
/// useful and gives a single sanity test target.
pub const SCAFFOLD_VERSION: &str = "0.1.0-c1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_compiles() {
        // Trivial assertion to confirm the crate is wired into the workspace.
        assert_eq!(SCAFFOLD_VERSION, "0.1.0-c1");
    }
}
