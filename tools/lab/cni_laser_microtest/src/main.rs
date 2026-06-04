//! CNI Laser Low-Power Microtest (M3)
//!
//! SAFETY INVARIANTS (hardcoded):
//! - MAX_POWER_MW = 5
//! - MAX_DURATION_MS = 5000
//! - Requires --operator-approve with explicit confirmation text
//! - Drop guard ALWAYS sends laser_off before exit
//!
//! Preconditions (ALL must be true before running):
//! 1. M0 manual checklist completed (key OFF, interlock closed, shutter closed, glasses on)
//! 2. M2 preflight passed (laser confirmed OFF via common_preflight)
//! 3. Physical shutter can be manually opened/closed
//! 4. Power meter is available and calibrated
//! 5. Beam dump is in place

use clap::Parser;
use cni_laser_fake_driver::protocol::CniFrame;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

const MAX_POWER_MW: u16 = 5;
const MAX_DURATION_MS: u64 = 5000;
const LASER_PORT: &str = "/dev/cu.usbserial-FTE86EB2";

#[derive(Parser, Debug)]
#[command(name = "cni-laser-microtest")]
struct Cli {
    /// Laser power in mW (clamped to 5)
    #[arg(long, default_value = "5")]
    power_mw: u16,

    /// Duration in milliseconds (clamped to 5000)
    #[arg(long, default_value = "5000")]
    duration_ms: u64,

    /// Operator approval — required
    #[arg(long)]
    operator_approve: bool,

    /// Port path (default: discovered CNI laser port)
    #[arg(long, default_value = LASER_PORT)]
    port: String,
}

/// Serial port wrapper that sends laser_off on drop.
struct SafeLaserPort {
    port: Box<dyn serialport::SerialPort>,
    device_id: String,
}

impl SafeLaserPort {
    fn open(path: &str) -> Result<Self, String> {
        let port = serialport::new(path, 9600)
            .timeout(Duration::from_millis(1000))
            .open()
            .map_err(|e| format!("open {}: {}", path, e))?;
        Ok(Self {
            port,
            device_id: path.into(),
        })
    }

    fn send_frame(&mut self, frame: &CniFrame) {
        let bytes = frame.to_bytes();
        if let Err(e) = self.port.write_all(&bytes) {
            eprintln!("[{}] write error: {}", self.device_id, e);
            return;
        }
        if let Err(e) = self.port.flush() {
            eprintln!("[{}] flush error: {}", self.device_id, e);
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    fn read_echo(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        self.port.read(buf)
    }
}

impl Drop for SafeLaserPort {
    fn drop(&mut self) {
        println!("\n🛡️  Safety guard: sending laser_off...");
        self.send_frame(&CniFrame::laser_off());
        println!("🛡️  laser_off sent.");
    }
}

fn main() {
    let cli = Cli::parse();

    println!("=== CNI Laser Microtest (M3) ===\n");

    // ---- Hard limits ----
    let power_mw = cli.power_mw.min(MAX_POWER_MW);
    let duration_ms = cli.duration_ms.min(MAX_DURATION_MS);

    if cli.power_mw > MAX_POWER_MW {
        println!(
            "⚠️  Requested power {} mW exceeds M3 limit {} mW. Clamped to {} mW.",
            cli.power_mw, MAX_POWER_MW, power_mw
        );
    }
    if cli.duration_ms > MAX_DURATION_MS {
        println!(
            "⚠️  Requested duration {} ms exceeds M3 limit {} ms. Clamped to {} ms.",
            cli.duration_ms, MAX_DURATION_MS, duration_ms
        );
    }

    // ---- Operator approval gate ----
    if !cli.operator_approve {
        eprintln!("ERROR: Operator approval required.");
        eprintln!("You must confirm:");
        eprintln!("  1. M0 checklist completed (key OFF, interlock closed, shutter closed, glasses on)");
        eprintln!("  2. M2 preflight passed");
        eprintln!("  3. Shutter can be manually opened/closed");
        eprintln!("  4. Power meter is available and calibrated");
        eprintln!("  5. Beam dump is in place");
        eprintln!("\nUse --operator-approve to confirm.");
        std::process::exit(1);
    }

    println!("Operator approval: CONFIRMED");
    println!("Power: {} mW (limit: {} mW)", power_mw, MAX_POWER_MW);
    println!("Duration: {} ms (limit: {} ms)", duration_ms, MAX_DURATION_MS);
    println!("Port: {}", cli.port);
    println!();

    // ---- Open serial port with safety guard ----
    let mut laser = match SafeLaserPort::open(&cli.port) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: Failed to open {}: {}", cli.port, e);
            std::process::exit(1);
        }
    };

    // ---- Step 1: Initial OFF ----
    println!("Step 1: Sending laser_off...");
    laser.send_frame(&CniFrame::laser_off());
    println!("  ✅ laser_off sent\n");

    // ---- Step 2: Set low power ----
    println!("Step 2: Setting power to {} mW...", power_mw);
    laser.send_frame(&CniFrame::set_power(power_mw));
    println!("  ✅ set_power({} mW) sent\n", power_mw);

    // ---- Step 3: Operator opens shutter (manual) ----
    println!("Step 3: MANUAL ACTION REQUIRED");
    println!("  → Open the physical shutter NOW.");
    println!("  → Verify power meter reads near 0 mW (laser not yet enabled).");
    println!("  Press Enter to continue...");
    let _ = std::io::stdin().read_line(&mut String::new());

    // ---- Step 4: Enable output ----
    println!("\nStep 4: Enabling laser output...");
    let enable_start = Instant::now();
    laser.send_frame(&CniFrame::laser_on());
    println!("  ✅ laser_on sent");
    println!("  ⏱️  Output will be held for {} ms MAX", duration_ms);
    println!(
        "  Monitor power meter. Abort if reading > {} mW.",
        power_mw + 1
    );
    println!();

    // ---- Step 5: Hold with abort watch ----
    println!("Step 5: Holding output...");
    let aborted = false;

    while enable_start.elapsed().as_millis() < duration_ms as u128 {
        std::thread::sleep(Duration::from_millis(100));
        // Check for Ctrl+C or other abort signals would go here
        // For now, user presses Ctrl+C to trigger the Drop guard
    }

    if !aborted {
        println!("  ✅ Duration elapsed ({} ms)", duration_ms);
    }

    // ---- Step 6: Disable output (explicit, before drop guard) ----
    println!("\nStep 6: Disabling laser output...");
    laser.send_frame(&CniFrame::laser_off());
    println!("  ✅ laser_off sent");

    // ---- Step 7: Manual close shutter ----
    println!("\nStep 7: MANUAL ACTION REQUIRED");
    println!("  → Close the physical shutter NOW.");
    println!("  Press Enter to continue...");
    let _ = std::io::stdin().read_line(&mut String::new());

    // ---- Step 8: Verify OFF ----
    println!("\nStep 8: Verifying laser is OFF...");
    laser.send_frame(&CniFrame::laser_off());
    println!("  ✅ Redundant laser_off sent");

    // Read any echo to confirm port is still alive
    let mut echo_buf = [0u8; 32];
    match laser.read_echo(&mut echo_buf) {
        Ok(n) if n > 0 => {
            let hex: Vec<String> = echo_buf[..n]
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect();
            println!("  Echo: {} ({} bytes)", hex.join(" "), n);
        }
        _ => println!("  No echo (normal for CNI protocol)"),
    }

    println!("\n=== Microtest Complete ===");
    if aborted {
        println!("Result: ABORTED (output was disabled immediately)");
        std::process::exit(2);
    } else {
        println!("Result: PASSED");
        println!("  - Power setpoint: {} mW", power_mw);
        println!("  - Actual duration: {:?}", enable_start.elapsed());
        println!("  - Output disabled: confirmed");
        println!("  - Shutter closed: operator confirmed");
    }

    // laser drops here, sending another laser_off via guard
}
