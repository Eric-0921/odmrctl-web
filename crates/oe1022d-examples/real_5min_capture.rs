//! Real-device 5-minute capture for the v0.1 acceptance test.
//! Auto-discovers the OE1022D, opens the port, runs the
//! continuous RALL? loop for 5 minutes, writes a complete
//! ndjson + events run to ./runs/2026-06-05_real_5min/.
//!
//! Run with: cargo run --release --example real_5min_capture

use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use oe1022d_transport::{
    discover_oe1022d, expand_to_samples, parse_envelope,
    spawn_continuous_rall_loop_pinned, SerialRallLink, SampleField, TransportStatus,
};
use oe1022d_acquisition::{RunConfig, RunWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[real-5min] scanning for OE1022D...");
    let devices = discover_oe1022d(Duration::from_millis(300), Duration::from_secs(15))?;
    let device = devices.into_iter().next().expect("at least one device");
    println!(
        "[real-5min] found {} on {}",
        device.idn.device_id(),
        device.port.name
    );

    let port = device.port.name.clone();
    let device_id = device.idn.device_id();
    let link = SerialRallLink::open(&port)?;
    let (tx, rx) = unbounded();
    let _handle = spawn_continuous_rall_loop_pinned(link, device_id.clone(), tx, Some(0));

    let run_dir = std::path::PathBuf::from("./runs/2026-06-05_real_5min");
    let writer = RunWriter::create(RunConfig {
        run_dir: run_dir.clone(),
        run_id: "2026-06-05_real_5min".to_string(),
        fields: vec![SampleField::BX, SampleField::BY, SampleField::BFreq],
        buffer_bytes: 8 * 1024,
    })?;

    let start = Instant::now();
    let stop_at = Duration::from_secs(300);
    let mut total_samples: u64 = 0;
    let mut total_frames: u64 = 0;
    let mut short_frames: u64 = 0;
    let mut warmup_frames: u64 = 0;

    println!("[real-5min] capturing for 5 min...");
    while start.elapsed() < stop_at {
        match rx.recv_timeout(Duration::from_millis(1000)) {
            Ok(env) => {
                total_frames += 1;
                if matches!(env.transport_status, TransportStatus::FrameShort { .. }) {
                    short_frames += 1;
                }
                if env.raw.len() > 12288 {
                    warmup_frames += 1;
                }
                match parse_envelope(&env) {
                    Ok(report) => {
                        for field in [SampleField::BX, SampleField::BY, SampleField::BFreq] {
                            let samples = expand_to_samples(&env, &report, field);
                            for s in &samples {
                                if let Err(e) = writer.write_sample(s) {
                                    eprintln!("[real-5min] write_sample error: {e}");
                                }
                                total_samples += 1;
                            }
                        }
                    }
                    Err(e) => eprintln!("[real-5min] parse error: {e}"),
                }
                if total_frames % 10 == 0 {
                    println!(
                        "[real-5min] t={:?} frames={} samples={} short={} warmup={}",
                        start.elapsed(),
                        total_frames,
                        total_samples,
                        short_frames,
                        warmup_frames
                    );
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                eprintln!("[real-5min] recv timeout");
            }
            Err(e) => {
                eprintln!("[real-5min] recv error: {e}");
                break;
            }
        }
    }

    writer.shutdown()?;
    println!(
        "[real-5min] DONE: {} frames, {} samples, {} short, {} warmup, in {:?}",
        total_frames,
        total_samples,
        short_frames,
        warmup_frames,
        start.elapsed()
    );
    println!(
        "[real-5min] ndjson: {}",
        run_dir.join("samples.ndjson").display()
    );
    Ok(())
}
