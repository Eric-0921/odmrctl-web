//! Long-duration RALL? stability test.
//!
//! Records every raw 12288-byte frame to a `.rall` binary file (concatenated
//! blocks, N × 12288 bytes) plus a metadata CSV. All 20 parameters × 50 samples
//! and the full 1216-byte config snapshot are preserved — parse later with
//! `odmr_oe1022d::parser::parse_rall_frame`.

use odmr_oe1022d::{CollectorConfig, RallCollector};
use std::io::Write;
use std::time::{Duration, Instant};

pub fn run(port_path: &str, baud: u32, duration_secs: u64) {
    let ts = chrono_now();
    let raw_path = format!("oe1022d_stability_{}.rall", ts);
    let csv_path = format!("oe1022d_stability_{}.csv", ts);

    let mut raw_file = match std::fs::File::create(&raw_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Cannot create {}: {}", raw_path, e);
            return;
        }
    };
    let mut csv = match std::fs::File::create(&csv_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Cannot create {}: {}", csv_path, e);
            return;
        }
    };

    // Metadata CSV header
    writeln!(
        csv,
        "frame_index,unix_ms,read_us,is_dup,pll_a,pll_b,overload_a,overload_b"
    )
    .unwrap();

    println!("=== RALL? Stability Test ===");
    println!("Port: {}, baud: {}", port_path, baud);
    println!("Duration: {} s ({:.0} min)", duration_secs, duration_secs as f64 / 60.0);
    println!("Raw:  {}", raw_path);
    println!("Meta: {}", csv_path);
    println!();

    let config = CollectorConfig {
        port_path: port_path.to_string(),
        baud,
        read_interval_ms: 48,
        timeout_ms: 300,
    };

    let (mut collector, rx) = match RallCollector::start(config) {
        Ok((c, r)) => (c, r),
        Err(e) => {
            eprintln!("Failed to start collector: {}", e);
            return;
        }
    };

    let deadline = Instant::now() + Duration::from_secs(duration_secs);
    let bucket_dur = Duration::from_secs(10);
    let mut next_bucket = Instant::now() + bucket_dur;

    // Per-bucket accumulators
    let mut bucket_frames: u64 = 0;
    let mut bucket_dups: u64 = 0;
    let mut bucket_read_total_us: u64 = 0;
    let mut bucket_read_min_us: u64 = u64::MAX;
    let mut bucket_read_max_us: u64 = 0;
    let mut bucket_pll_lost: bool = false;
    let mut bucket_overload: bool = false;

    // Run accumulators
    let mut total_frames: u64 = 0;
    let mut total_dups: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut total_read_us: u64 = 0;
    let mut min_read_us: u64 = u64::MAX;
    let mut max_read_us: u64 = 0;
    let mut prev_ts_ms: Option<u64> = None;
    let mut intervals: Vec<f64> = Vec::new();
    let mut pll_lost_ever = false;
    let mut overload_ever = false;

    let mut bucket_idx: u32 = 1;

    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }

        let remaining = deadline - now;
        let recv_timeout = remaining.min(Duration::from_millis(500));

        match rx.recv_timeout(recv_timeout) {
            Ok(frame) => {
                // Write raw 12288-byte frame to binary file
                raw_file.write_all(&frame.raw).unwrap();
                total_bytes += frame.raw.len() as u64;

                // Write metadata CSV row
                let pll_a = frame.frame.config.a_pll_locked.unwrap_or(false);
                let pll_b = frame.frame.config.b_pll_locked.unwrap_or(false);
                let ov_a = frame.frame.config.a_input_overload.unwrap_or(false)
                    || frame.frame.config.a_gain_overload.unwrap_or(false);
                let ov_b = frame.frame.config.b_input_overload.unwrap_or(false)
                    || frame.frame.config.b_gain_overload.unwrap_or(false);

                writeln!(
                    csv,
                    "{},{},{},{},{},{},{},{}",
                    frame.frame_index,
                    frame.timestamp_unix_ms,
                    frame.read_time_us,
                    frame.is_duplicate as u8,
                    pll_a as u8,
                    pll_b as u8,
                    ov_a as u8,
                    ov_b as u8,
                )
                .unwrap();

                // Track frame intervals
                if let Some(prev) = prev_ts_ms {
                    let dt = frame.timestamp_unix_ms as f64 - prev as f64;
                    if dt > 0.0 && dt < 500.0 {
                        intervals.push(dt);
                    }
                }
                prev_ts_ms = Some(frame.timestamp_unix_ms);

                // Accumulators
                let rt = frame.read_time_us;
                bucket_frames += 1;
                bucket_read_total_us += rt;
                bucket_read_min_us = bucket_read_min_us.min(rt);
                bucket_read_max_us = bucket_read_max_us.max(rt);
                if frame.is_duplicate {
                    bucket_dups += 1;
                }
                if !pll_a || !pll_b {
                    bucket_pll_lost = true;
                    pll_lost_ever = true;
                }
                if ov_a || ov_b {
                    bucket_overload = true;
                    overload_ever = true;
                }

                total_frames += 1;
                total_read_us += rt;
                min_read_us = min_read_us.min(rt);
                max_read_us = max_read_us.max(rt);
                if frame.is_duplicate {
                    total_dups += 1;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("Producer disconnected unexpectedly.");
                break;
            }
        }

        // Print bucket stats every 10s
        if Instant::now() >= next_bucket {
            let elapsed = bucket_idx * 10;
            let uniq = bucket_frames.saturating_sub(bucket_dups);
            let avg_read = if bucket_frames > 0 {
                bucket_read_total_us as f64 / bucket_frames as f64 / 1000.0
            } else {
                0.0
            };
            let dup_pct = if bucket_frames > 0 {
                bucket_dups as f64 / bucket_frames as f64 * 100.0
            } else {
                0.0
            };
            let min_r = if bucket_read_min_us != u64::MAX {
                bucket_read_min_us as f64 / 1000.0
            } else {
                0.0
            };
            let max_r = bucket_read_max_us as f64 / 1000.0;

            let flags = match (bucket_pll_lost, bucket_overload) {
                (true, true) => " [!PLL,!OVL]",
                (true, false) => " [!PLL]",
                (false, true) => " [!OVL]",
                (false, false) => "",
            };

            let elapsed_min = elapsed as f64 / 60.0;
            println!(
                "  t={:>4}s ({:>5.1}min)  frames={:>4}  uniq={:>4}  dup={:>5.1}%  \
                 read={:>5.1}ms (min={:>5.1}, max={:>5.1}){}",
                elapsed, elapsed_min, bucket_frames, uniq, dup_pct, avg_read, min_r, max_r, flags
            );

            bucket_frames = 0;
            bucket_dups = 0;
            bucket_read_total_us = 0;
            bucket_read_min_us = u64::MAX;
            bucket_read_max_us = 0;
            bucket_pll_lost = false;
            bucket_overload = false;
            bucket_idx += 1;
            next_bucket = Instant::now() + bucket_dur;
        }
    }

    collector.signal_stop();
    drop(collector);
    raw_file.flush().unwrap();
    csv.flush().unwrap();

    // --- Final summary ---
    let uniq = total_frames.saturating_sub(total_dups);
    let avg_read = if total_frames > 0 {
        total_read_us as f64 / total_frames as f64 / 1000.0
    } else {
        0.0
    };
    let dup_pct = if total_frames > 0 {
        total_dups as f64 / total_frames as f64 * 100.0
    } else {
        0.0
    };
    let eff_fps = uniq as f64 / duration_secs as f64;
    let eff_pts = eff_fps * 50.0;
    let raw_mb = total_bytes as f64 / 1_048_576.0;

    // Interval stats
    let (int_min, int_med, int_max, int_avg) = if intervals.is_empty() {
        (0.0, 0.0, 0.0, 0.0)
    } else {
        let mut sorted = intervals.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let min = sorted.first().copied().unwrap_or(0.0);
        let max = sorted.last().copied().unwrap_or(0.0);
        let med = sorted[sorted.len() / 2];
        let avg = sorted.iter().sum::<f64>() / sorted.len() as f64;
        (min, med, max, avg)
    };

    println!();
    println!("=== Final Summary ===");
    println!("  Duration: {} s ({:.0} min)", duration_secs, duration_secs as f64 / 60.0);
    println!(
        "  Frames: {} total, {} unique, {} dup ({:.1}%)",
        total_frames, uniq, total_dups, dup_pct
    );
    println!(
        "  Effective: {:.1} fps, {:.0} unique pts/sec",
        eff_fps, eff_pts
    );
    println!(
        "  Read time: avg={:.1}ms, min={:.1}ms, max={:.1}ms",
        avg_read,
        min_read_us as f64 / 1000.0,
        max_read_us as f64 / 1000.0
    );
    println!(
        "  Frame interval: min={:.1}ms, med={:.1}ms, avg={:.1}ms, max={:.1}ms",
        int_min, int_med, int_avg, int_max
    );
    println!(
        "  Raw data: {:.1} MB ({} frames × 12288 bytes)",
        raw_mb, total_frames
    );

    let mut anomalies: Vec<&str> = Vec::new();
    if pll_lost_ever {
        anomalies.push("PLL lost (no external reference)");
    }
    if overload_ever {
        anomalies.push("input/gain overload");
    }
    if dup_pct > 5.0 {
        anomalies.push("high duplicate rate");
    }
    if anomalies.is_empty() {
        println!("  Anomalies: none");
    } else {
        println!("  Anomalies: {}", anomalies.join(", "));
    }

    println!();
    println!("  Raw:  {}", raw_path);
    println!("  Meta: {}", csv_path);
    println!("Done.");
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let days = secs / 86400;
    let day_secs = secs % 86400;
    let h = day_secs / 3600;
    let m = (day_secs % 3600) / 60;
    let s = day_secs % 60;

    let mut y = 1970;
    let mut d = days;
    loop {
        let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        let year_days = if leap { 366 } else { 365 };
        if d < year_days {
            break;
        }
        d -= year_days;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut mo = 1;
    for md in &month_days {
        if d < *md {
            break;
        }
        d -= *md;
        mo += 1;
    }
    format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}",
        y, mo, d + 1, h, m, s
    )
}
