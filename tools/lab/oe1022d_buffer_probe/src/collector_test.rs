//! Integration test for RallCollector on real hardware.

use odmr_oe1022d::{CollectorConfig, RallCollector};
use std::time::Duration;

pub fn run(port_path: &str, baud: u32) {
    println!("=== RallCollector Integration Test ===");

    let config = CollectorConfig {
        port_path: port_path.to_string(),
        baud,
        read_interval_ms: 48,
        timeout_ms: 300,
    };

    let (collector, rx) = match RallCollector::start(config) {
        Ok((c, r)) => (c, r),
        Err(e) => {
            eprintln!("Failed to start collector: {}", e);
            return;
        }
    };

    println!("Collecting for 2 seconds...");
    let deadline = std::time::Instant::now() + Duration::from_secs(2);

    let mut total: u64 = 0;
    let mut dups: u64 = 0;
    let mut total_read_us: u64 = 0;
    let mut x0_values: Vec<(u64, f64)> = Vec::new();

    while std::time::Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(frame) => {
                let x0 = frame.frame.measurements.lockin_A_X_mv
                    .first().copied().unwrap_or(f64::NAN);
                total_read_us += frame.read_time_us;
                x0_values.push((frame.read_time_us, x0));
                total += 1;
                if frame.is_duplicate {
                    dups += 1;
                }
                // Show first 5 frames
                if total <= 5 {
                    println!(
                        "  frame {}: read={:.1}ms X[0]={:.6e} {}",
                        total,
                        frame.read_time_us as f64 / 1000.0,
                        x0,
                        if frame.is_duplicate { "DUP" } else { "NEW" }
                    );
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let stats = collector.stats();
    // Don't call collector.stop() — it joins the producer thread which may
    // be blocked on a serial read. The Drop impl will signal stop.
    drop(collector);

    let uniq = total - dups;
    let avg_read = if total > 0 {
        total_read_us as f64 / total as f64 / 1000.0
    } else {
        0.0
    };

    println!();
    println!("--- Results ---");
    println!("  Frames: {} total, {} unique, {} dup", total, uniq, dups);
    println!("  Effective: {:.1} fps, {:.0} unique pts/sec", uniq as f64 / 2.0, uniq as f64 / 2.0 * 50.0);
    println!("  Avg read time: {:.1}ms", avg_read);
    println!(
        "  Producer: {} captured, {} dup, {} errors, {} attempts",
        stats.frames_captured, stats.frames_duplicated,
        stats.frames_parse_error, stats.total_reads_attempted
    );
    println!("Done.");
}
