//! Test continuous RALL? polling to see if higher-frequency querying
//! yields more data or just duplicates.

use std::io::{Read, Write};
use std::time::{Duration, Instant};

fn query_rall(port: &mut Box<dyn serialport::SerialPort>, settle_ms: u64) -> (usize, Vec<u8>) {
    let _ = port.clear(serialport::ClearBuffer::Input);
    port.write_all(b"RALL?\r").unwrap();
    port.flush().unwrap();

    let mut collected = Vec::new();
    for attempt in 0..10 {
        let wait_ms = if attempt == 0 { settle_ms } else { 100 };
        std::thread::sleep(Duration::from_millis(wait_ms));

        let mut buf = vec![0u8; 16384];
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                buf.truncate(n);
                collected.extend_from_slice(&buf);
                if collected.len() >= 12288 {
                    break;
                }
            }
            Ok(_) => continue,
            Err(_) => continue,
        }
    }

    (collected.len(), collected)
}

fn parse_rall_frame(data: &[u8]) -> Vec<f64> {
    // Parse first 400 bytes = 50 points of Ch-A X values (f64 BE)
    let mut values = Vec::new();
    for i in 0..50.min(data.len() / 8) {
        let offset = i * 8;
        if offset + 8 <= data.len() {
            let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap_or_default();
            let val = f64::from_be_bytes(bytes);
            values.push(val);
        }
    }
    values
}

pub fn run(port_path: &str, baud: u32) {
    println!("=== Continuous RALL? Test ===");

    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(5000))
        .open()
        .expect("open port");

    // Test 1: Query every 50ms for 1 second (20 queries)
    println!("\n1. Query every 50ms for 1 second");
    let start = Instant::now();
    let mut results: Vec<(u64, usize, f64)> = Vec::new();

    for i in 0..20 {
        let elapsed = start.elapsed().as_millis() as u64;
        let (len, raw) = query_rall(&mut port, 150);
        let x_values = parse_rall_frame(&raw);
        let first_x = x_values.first().copied().unwrap_or(f64::NAN);
        results.push((elapsed, len, first_x));

        let target_time = (i + 1) as u64 * 50;
        let elapsed_now = start.elapsed().as_millis() as u64;
        if elapsed_now < target_time {
            std::thread::sleep(Duration::from_millis(target_time - elapsed_now));
        }
    }

    for (t, len, first_x) in &results {
        println!("  t={:>4}ms, len={:>5}, first_X={:.6e}", t, len, first_x);
    }

    // Check for duplicates
    let unique_lens: std::collections::HashSet<_> = results.iter().map(|r| r.1).collect();
    let unique_x: std::collections::HashSet<_> = results
        .iter()
        .map(|r| (r.2 * 1e12) as i64) // Quantize for comparison
        .collect();
    println!("  Unique frame lengths: {:?}", unique_lens);
    println!("  Unique first-X count: {}", unique_x.len());

    // Test 2: Query as fast as possible for 1 second
    println!("\n2. Query as fast as possible for 1 second");
    let start = Instant::now();
    let mut fast_results: Vec<(u64, usize, f64)> = Vec::new();

    while start.elapsed().as_millis() < 1000 {
        let elapsed = start.elapsed().as_millis() as u64;
        let (len, raw) = query_rall(&mut port, 50);
        let x_values = parse_rall_frame(&raw);
        let first_x = x_values.first().copied().unwrap_or(f64::NAN);
        fast_results.push((elapsed, len, first_x));
    }

    println!("  Total queries: {}", fast_results.len());
    let unique_lens_fast: std::collections::HashSet<_> =
        fast_results.iter().map(|r| r.1).collect();
    let unique_x_fast: std::collections::HashSet<_> = fast_results
        .iter()
        .map(|r| (r.2 * 1e12) as i64)
        .collect();
    println!("  Unique frame lengths: {:?}", unique_lens_fast);
    println!("  Unique first-X count: {}", unique_x_fast.len());

    // Show first 10 results
    for (t, len, first_x) in fast_results.iter().take(10) {
        println!("  t={:>4}ms, len={:>5}, first_X={:.6e}", t, len, first_x);
    }

    // Test 3: Check frame content freshness
    println!("\n3. Frame content freshness check");
    let mut prev_frame: Option<Vec<f64>> = None;
    let mut same_count = 0;
    let mut diff_count = 0;

    for _ in 0..10 {
        let (_, raw) = query_rall(&mut port, 150);
        let x_values = parse_rall_frame(&raw);
        if let Some(ref prev) = prev_frame {
            if x_values == *prev {
                same_count += 1;
            } else {
                diff_count += 1;
            }
        }
        prev_frame = Some(x_values);
        std::thread::sleep(Duration::from_millis(50));
    }
    println!("  Same frames: {}, Different frames: {}", same_count, diff_count);

    println!("\nDone");
}
