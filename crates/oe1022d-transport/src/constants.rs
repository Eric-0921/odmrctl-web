//! Physical constants and pitfall-aware defaults for OE1022D transport.
//!
//! See module-level docs in `lib.rs` for K1..K8.

use std::time::Duration;

/// K2: Baud rate is **921600**, not 115200.
///
/// `oe1022d_rust_demo/VERIFICATION_REPORT.md §4.3` records that
/// `03_oe1022d_acquisition_prd_v0.2.md §6.2` originally specified 115200
/// — that is **wrong**. The USB CDC link is fixed at 921600 by device
/// firmware; negotiation is not supported.
pub const OE1022D_BAUD_RATE: u32 = 921_600;

/// K3: `RALL?` returns **12288 bytes** of binary data with no terminator.
///
/// Format: 20 channels × 50 samples × 8 bytes (Big-Endian f64) for the
/// measurement region (bytes 0..8000), then 1216 bytes of config snapshot
/// (bytes 8000..9216), then 3072 bytes of padding (bytes 9216..12288).
pub const RALL_FRAME_BYTES: usize = 12_288;

/// RALL? measurement region size in bytes.
pub const RALL_MEASUREMENT_BYTES: usize = 8_000;

/// RALL? config snapshot region size in bytes.
pub const RALL_CONFIG_BYTES: usize = 1_216;

/// RALL? padding region size in bytes.
pub const RALL_PADDING_BYTES: usize = 3_072;

/// K4 hint: macOS returns ~1020 bytes per read. Use a buffer at least
/// this size to minimize the number of `read()` syscalls per frame.
pub const RALL_READ_BUFFER_BYTES: usize = 4_096;

/// K6: After issuing `RALL?`, the device needs ~800 ms to prepare the
/// 12288-byte frame. Below this threshold the read loop will busy-wait.
/// 900 ms is chosen to be safely above the empirical 805 ms typical
/// (M2.5 acquisition report, 2026-05-31).
pub const RALL_PREPARE_DELAY: Duration = Duration::from_millis(900);

/// Maximum time we will wait for a complete RALL? frame. Beyond this we
/// record a `frame_short` event and skip the frame — never pad with zeros.
pub const RALL_READ_DEADLINE: Duration = Duration::from_millis(950);

/// 8N1 — 8 data bits, no parity, 1 stop bit. OE1022D does not negotiate
/// other settings over USB CDC.
pub const OE1022D_DATA_BITS: serialport::DataBits = serialport::DataBits::Eight;
pub const OE1022D_PARITY: serialport::Parity = serialport::Parity::None;
pub const OE1022D_STOP_BITS: serialport::StopBits = serialport::StopBits::One;

/// Flow control must be OFF. Hardware (RTS/CTS) flow control is known
/// to deadlock at 921600 baud on macOS USB CDC, and XON/XOFF is not
/// supported by the device firmware.
pub const OE1022D_FLOW_CONTROL: serialport::FlowControl = serialport::FlowControl::None;

/// Read timeout for `*IDN?` probe. The probe is small (a few dozen
/// bytes) and the device answers within tens of ms. `serialport 4.x`
/// uses a single timeout for both read and write, so we set it to the
/// larger of the two reasonable values.
pub const IDN_READ_TIMEOUT: Duration = Duration::from_millis(300);

/// `*IDN?` command string with CR terminator.
///
/// Manual says OE1022D accepts CR or LF. We use CR to match the
/// pre-existing `oe1022d_rust_demo/src/main.rs` choice.
pub const IDN_COMMAND: &[u8] = b"*IDN?\r";

/// `RALL?` command string with CR terminator.
pub const RALL_COMMAND: &[u8] = b"RALL?\r";
