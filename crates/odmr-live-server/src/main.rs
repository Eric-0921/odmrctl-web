//! OE1022D live trace HTTP server.
//!
//! Starts RallCollector, consumes frames into a ring buffer (all 50 samples per
//! unique frame, preserving 1kHz-equivalent resolution), and serves the trace
//! over HTTP for a browser-based chart frontend.
//!
//! ```bash
//! cargo run -p odmr-live-server -- --port /dev/cu.usbmodem395D388533371
//! ```

mod buffer;
mod server;
mod types;

use buffer::TraceRingBuffer;
use clap::Parser;
use odmr_oe1022d::{CollectorConfig, RallCollector};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use types::TracePoint;

#[derive(Parser, Debug)]
#[command(about = "OE1022D live trace HTTP server")]
struct Cli {
    #[arg(long, default_value = "/dev/cu.usbmodem3361358734371")]
    port: String,

    #[arg(long, default_value = "921600")]
    baud: u32,

    #[arg(long, default_value = "9876")]
    http_port: u16,

    #[arg(long, default_value = "2000")]
    ring_capacity: usize,
}

fn main() {
    let cli = Cli::parse();

    let config = CollectorConfig {
        port_path: cli.port.clone(),
        baud: cli.baud,
        read_interval_ms: 48,
        timeout_ms: 300,
    };

    let (mut collector, rx) = match RallCollector::start(config) {
        Ok((c, r)) => (c, r),
        Err(e) => {
            eprintln!("Failed to start collector: {}", e);
            std::process::exit(1);
        }
    };

    let ring = Arc::new(Mutex::new(TraceRingBuffer::new(cli.ring_capacity)));
    let ring_clone = Arc::clone(&ring);

    // Consumer thread: reads captured frames, extracts all 50 B-channel
    // samples, and pushes them into the shared ring buffer.
    let consumer = std::thread::spawn(move || {
        let start = Instant::now();
        loop {
            match rx.recv() {
                Ok(frame) => {
                    let elapsed_s = start.elapsed().as_secs_f64();
                    let bx = &frame.frame.measurements.lockin_B_X_mv;
                    let by = &frame.frame.measurements.lockin_B_Y_mv;
                    let bf = &frame.frame.measurements.lockin_B_freq_hz;

                    // Extract all 50 samples per frame (1ms spacing, 1kHz-equivalent).
                    // Samples cover the prior 50ms: sample[0] at t-50ms, sample[49] at t-1ms.
                    let points: [TracePoint; 50] = std::array::from_fn(|i| {
                        TracePoint {
                            elapsed_s: elapsed_s - (50 - i) as f64 * 0.001,
                            bx_mv: bx.get(i).copied().unwrap_or(f64::NAN),
                            by_mv: by.get(i).copied().unwrap_or(f64::NAN),
                            freq_hz: bf.get(i).copied().unwrap_or(f64::NAN),
                        }
                    });

                    let mut buf = ring_clone.lock().unwrap();
                    buf.push_frame(&points, frame.is_duplicate, frame.read_time_us);
                }
                Err(std::sync::mpsc::RecvError) => {
                    break;
                }
            }
        }
    });

    println!("=== OE1022D Live Trace Server ===");
    println!("Port: {} @ {}", cli.port, cli.baud);
    println!("HTTP: http://127.0.0.1:{}/api/trace", cli.http_port);
    println!("Ring capacity: {} points (~{:.1}s at 1kHz)", cli.ring_capacity, cli.ring_capacity as f64 / 1000.0);
    println!();
    println!("Start the frontend: cd apps/desktop && pnpm tauri dev");
    println!("Then open Live Chart in the sidebar.");
    println!();

    // Run HTTP server on main thread (blocking)
    if let Err(e) = server::run_server(ring, cli.http_port) {
        eprintln!("HTTP server error: {}", e);
    }

    collector.signal_stop();
    drop(collector);
    let _ = consumer.join();
}
