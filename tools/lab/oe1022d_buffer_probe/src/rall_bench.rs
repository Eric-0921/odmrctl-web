//! Continuous RALL? benchmarking — measure real throughput without buffer clearing.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

fn read_exact_frame(port: &mut Box<dyn serialport::SerialPort>) -> Result<(Vec<u8>, u64), String> {
    let start = Instant::now();
    let mut buf = Vec::with_capacity(12288);
    let deadline = start + Duration::from_millis(5000);

    while buf.len() < 12288 && Instant::now() < deadline {
        let mut chunk = vec![0u8; 4096];
        match port.read(&mut chunk) {
            Ok(0) => {
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            Ok(n) => {
                chunk.truncate(n);
                buf.extend_from_slice(&chunk);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            Err(e) => return Err(format!("read error: {}", e)),
        }
    }

    let elapsed = start.elapsed().as_micros() as u64;
    Ok((buf, elapsed))
}

fn extract_first_f64s(data: &[u8], count: usize, offset: usize) -> Vec<f64> {
    (0..count)
        .filter_map(|i| {
            let pos = offset + i * 8;
            if pos + 8 <= data.len() {
                let bytes: [u8; 8] = data[pos..pos + 8].try_into().ok()?;
                Some(f64::from_be_bytes(bytes))
            } else {
                None
            }
        })
        .collect()
}

pub fn run(port_path: &str, baud: u32) {
    println!("=== RALL? Continuous Benchmark ===");
    println!();

    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(500))
        .open()
        .expect("open port");

    // Drain any stale data
    let _ = port.clear(serialport::ClearBuffer::Input);
    std::thread::sleep(Duration::from_millis(100));

    // --- Test A: Single frame timing (10 runs) ---
    println!("--- Test A: Single frame timing (10 runs) ---");
    let mut timings = Vec::new();
    for i in 0..10 {
        let _ = port.clear(serialport::ClearBuffer::Input);
        port.write_all(b"RALL?\r").unwrap();
        port.flush().unwrap();

        match read_exact_frame(&mut port) {
            Ok((raw, us)) => {
                let first_x = extract_first_f64s(&raw, 1, 0);
                let x0 = first_x.first().copied().unwrap_or(f64::NAN);
                timings.push((us, raw.len(), x0));
                println!(
                    "  frame {}: {:>5.1}ms, {} bytes, X[0]={:.6e}",
                    i + 1,
                    us as f64 / 1000.0,
                    raw.len(),
                    x0
                );
            }
            Err(e) => {
                println!("  frame {}: ERROR: {}", i + 1, e);
            }
        }
    }

    if timings.is_empty() {
        println!("No frames captured, aborting.");
        return;
    }

    let avg_us = timings.iter().map(|t| t.0).sum::<u64>() / timings.len() as u64;
    println!(
        "  Average: {:.1}ms, {:.1} fps",
        avg_us as f64 / 1000.0,
        1_000_000.0 / avg_us as f64
    );

    // --- Test B: Continuous read (NO clear between frames) ---
    println!();
    println!("--- Test B: Continuous read (20 frames, no input clear) ---");
    // for the first frame, clear and send RALL?
    let _ = port.clear(serialport::ClearBuffer::Input);
    port.write_all(b"RALL?\r").unwrap();
    port.flush().unwrap();
    let start = Instant::now();

    let mut frames: Vec<(f64, f64)> = Vec::new(); // (elapsed_ms, X[0])

    for _ in 0..20 {
        match read_exact_frame(&mut port) {
            Ok((raw, _us)) => {
                let elapsed = start.elapsed().as_millis() as f64;
                let x0 = extract_first_f64s(&raw, 1, 0)
                    .first()
                    .copied()
                    .unwrap_or(f64::NAN);
                frames.push((elapsed, x0));
                // Send next RALL? immediately after read completes
                port.write_all(b"RALL?\r").unwrap();
                port.flush().unwrap();
            }
            Err(e) => {
                eprintln!("  Frame {} error: {}", frames.len() + 1, e);
                break;
            }
        }
    }

    // Analyze
    let total_elapsed = frames.last().map(|f| f.0).unwrap_or(0.0);
    let count = frames.len();
    println!("  Captured {} frames in {:.0}ms", count, total_elapsed);
    println!(
        "  Effective rate: {:.1} fps, {:.0} pts/sec",
        count as f64 / (total_elapsed / 1000.0),
        count as f64 / (total_elapsed / 1000.0) * 50.0
    );

    // Frame intervals
    let mut intervals: Vec<f64> = Vec::new();
    for i in 1..frames.len() {
        intervals.push(frames[i].0 - frames[i - 1].0);
    }
    if !intervals.is_empty() {
        intervals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let min = intervals.first().copied().unwrap_or(0.0);
        let max = intervals.last().copied().unwrap_or(0.0);
        let med = intervals[intervals.len() / 2];
        let avg = intervals.iter().sum::<f64>() / intervals.len() as f64;
        println!("  Intervals: min={:.1}ms, med={:.1}ms, avg={:.1}ms, max={:.1}ms", min, med, avg, max);

        // Group into buckets
        let mut buckets: BTreeMap<i32, usize> = BTreeMap::new();
        for iv in &intervals {
            let bucket = (*iv / 10.0).floor() as i32 * 10;
            *buckets.entry(bucket).or_insert(0) += 1;
        }
        println!("  Interval distribution (10ms buckets):");
        for (bucket, count) in buckets.iter() {
            let bar = "█".repeat(*count);
            println!("    {}ms: {} {}", bucket, count, bar);
        }
    }

    // Frame content freshness
    println!();
    println!("  X[0] sequence (checking for duplicates):");
    let mut unique_count = 0;
    let mut prev_x = f64::NAN;
    for (i, (t, x)) in frames.iter().enumerate() {
        let changed = if i == 0 || (*x - prev_x).abs() > 1e-20 {
            if i > 0 {
                unique_count += 1;
            }
            "NEW"
        } else {
            "DUP"
        };
        println!("    t={:>6.0}ms  X[0]={:.6e}  {}", t, x, changed);
        prev_x = *x;
    }
    println!(
        "  Unique frames: {}/{} ({:.0}%)",
        unique_count + 1,
        count,
        (unique_count + 1) as f64 / count as f64 * 100.0
    );

    // --- Test C: As-fast-as-possible (pipeline mode) ---
    println!();
    println!("--- Test C: Pipeline mode (send next RALL? before current frame done) ---");
    // This tests if device can buffer multiple RALL? responses
    let _ = port.clear(serialport::ClearBuffer::Input);

    // Burst-send 5 RALL? commands back to back
    for _ in 0..5 {
        port.write_all(b"RALL?\r").unwrap();
    }
    port.flush().unwrap();

    // Now read them all
    let mut pipeline_frames: Vec<(usize, f64)> = Vec::new();
    let pipe_start = Instant::now();
    for _ in 0..5 {
        match read_exact_frame(&mut port) {
            Ok((raw, _)) => {
                let x0 = extract_first_f64s(&raw, 1, 0)
                    .first()
                    .copied()
                    .unwrap_or(f64::NAN);
                let elapsed = pipe_start.elapsed().as_millis() as f64;
                pipeline_frames.push((raw.len(), x0));
                println!(
                    "  t={:.0}ms: {} bytes, X[0]={:.6e}",
                    elapsed,
                    raw.len(),
                    x0
                );
            }
            Err(e) => {
                eprintln!("  Error: {}", e);
                break;
            }
        }
    }

    // Check if pipeline works (all frames same = pipelining works, only first = device ignores extras)
    let unique_x: std::collections::HashSet<i64> = pipeline_frames
        .iter()
        .map(|(_, x)| (*x * 1e12) as i64)
        .collect();
    println!(
        "  Frames: {}, unique X[0]: {}",
        pipeline_frames.len(),
        unique_x.len()
    );
    if unique_x.len() == pipeline_frames.len() && pipeline_frames.len() > 1 {
        println!("  -> Device DOES buffer RALL? responses (pipeline works)");
    } else if unique_x.len() == 1 && pipeline_frames.len() > 1 {
        println!("  -> Device returns same frame for queued RALL? (no pipeline benefit)");
    }

    println!();
    println!("Done.");
}
