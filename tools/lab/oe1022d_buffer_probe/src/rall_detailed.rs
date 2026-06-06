//! Detailed RALL? frame analysis with multiple parse attempts.

use std::io::{Read, Write};
use std::time::Duration;

fn read_rall_detailed(port_path: &str, baud: u32) -> Vec<u8> {
    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(5000))
        .open()
        .expect("open port");

    let _ = port.clear(serialport::ClearBuffer::Input);
    port.write_all(b"RALL?\r").unwrap();
    port.flush().unwrap();

    let mut collected = Vec::new();
    let mut read_log: Vec<(usize, usize)> = Vec::new(); // (attempt, bytes_read)

    // Fast-poll: RALL? returns 12288 bytes at ~49ms/frame on USB CDC.
    // macOS CDC driver delivers ~1020 bytes per read().
    for attempt in 0..50 {
        std::thread::sleep(Duration::from_millis(5));

        let mut buf = vec![0u8; 16384];
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                buf.truncate(n);
                collected.extend_from_slice(&buf);
                read_log.push((attempt, n));

                // If we got a full frame, stop early
                if collected.len() >= 12288 {
                    break;
                }

                // Try to drain all remaining
                loop {
                    let avail = port.bytes_to_read().unwrap_or(0) as usize;
                    if avail == 0 { break; }
                    let mut extra = vec![0u8; avail.min(16384)];
                    match port.read(&mut extra) {
                        Ok(m) if m > 0 => {
                            extra.truncate(m);
                            collected.extend_from_slice(&extra);
                            read_log.push((attempt, m));
                        }
                        _ => break,
                    }
                    if collected.len() >= 12288 {
                        break;
                    }
                }
                if collected.len() >= 12288 {
                    break;
                }
            }
            Ok(_) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_e) => {
                read_log.push((attempt, 0));
                if attempt > 10 && collected.len() > 0 {
                    break;
                }
            }
        }
    }

    println!("Read attempts:");
    for (attempt, n) in &read_log {
        println!("  attempt {}: {} bytes", attempt, n);
    }
    println!("Total collected: {} bytes", collected.len());

    collected
}

fn parse_f64_be(data: &[u8], count: usize) -> Vec<f64> {
    let mut values = Vec::new();
    for i in 0..count {
        let offset = i * 8;
        if offset + 8 <= data.len() {
            let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap_or_default();
            values.push(f64::from_be_bytes(bytes));
        }
    }
    values
}

fn parse_f64_le(data: &[u8], count: usize) -> Vec<f64> {
    let mut values = Vec::new();
    for i in 0..count {
        let offset = i * 8;
        if offset + 8 <= data.len() {
            let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap_or_default();
            values.push(f64::from_le_bytes(bytes));
        }
    }
    values
}

fn parse_f32_be(data: &[u8], count: usize) -> Vec<f32> {
    let mut values = Vec::new();
    for i in 0..count {
        let offset = i * 4;
        if offset + 4 <= data.len() {
            let bytes: [u8; 4] = data[offset..offset + 4].try_into().unwrap_or_default();
            values.push(f32::from_be_bytes(bytes));
        }
    }
    values
}

fn parse_f32_le(data: &[u8], count: usize) -> Vec<f32> {
    let mut values = Vec::new();
    for i in 0..count {
        let offset = i * 4;
        if offset + 4 <= data.len() {
            let bytes: [u8; 4] = data[offset..offset + 4].try_into().unwrap_or_default();
            values.push(f32::from_le_bytes(bytes));
        }
    }
    values
}

fn is_valid_float(v: f64) -> bool {
    v.is_finite() && v.abs() < 1e6 && v.abs() > 1e-15 || v == 0.0
}

fn is_valid_float_f32(v: f32) -> bool {
    v.is_finite() && v.abs() < 1e6 && v.abs() > 1e-15 || v == 0.0
}

pub fn run(port_path: &str, baud: u32) {
    println!("=== Detailed RALL? Frame Analysis ===");

    let raw = read_rall_detailed(port_path, baud);

    if raw.len() < 8 {
        println!("Not enough data received ({} bytes)", raw.len());
        return;
    }

    // Try different parse strategies on first 400 bytes (50 points)
    let test_len = raw.len().min(400);
    let test_data = &raw[..test_len];

    println!("\n--- First 32 bytes hex ---");
    for i in 0..4 {
        let offset = i * 8;
        let hex: String = test_data[offset..offset+8].iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        println!("  offset {}: {}", offset, hex);
    }

    println!("\n--- Parse as f64 BE (50 points) ---");
    let be64 = parse_f64_be(test_data, 50);
    let valid_be64 = be64.iter().filter(|&&v| is_valid_float(v)).count();
    println!("Valid values: {}/50", valid_be64);
    for (i, v) in be64.iter().take(10).enumerate() {
        println!("  [{}]: {:.6e} {}", i, v, if is_valid_float(*v) { "✓" } else { "✗" });
    }

    println!("\n--- Parse as f64 LE (50 points) ---");
    let le64 = parse_f64_le(test_data, 50);
    let valid_le64 = le64.iter().filter(|&&v| is_valid_float(v)).count();
    println!("Valid values: {}/50", valid_le64);
    for (i, v) in le64.iter().take(10).enumerate() {
        println!("  [{}]: {:.6e} {}", i, v, if is_valid_float(*v) { "✓" } else { "✗" });
    }

    println!("\n--- Parse as f32 BE (100 points) ---");
    let be32 = parse_f32_be(test_data, 100);
    let valid_be32 = be32.iter().filter(|&&v| is_valid_float_f32(v)).count();
    println!("Valid values: {}/100", valid_be32);
    for (i, v) in be32.iter().take(10).enumerate() {
        println!("  [{}]: {:.6e} {}", i, v, if is_valid_float_f32(*v) { "✓" } else { "✗" });
    }

    println!("\n--- Parse as f32 LE (100 points) ---");
    let le32 = parse_f32_le(test_data, 100);
    let valid_le32 = le32.iter().filter(|&&v| is_valid_float_f32(v)).count();
    println!("Valid values: {}/100", valid_le32);
    for (i, v) in le32.iter().take(10).enumerate() {
        println!("  [{}]: {:.6e} {}", i, v, if is_valid_float_f32(*v) { "✓" } else { "✗" });
    }

    // Try offset shifts in case there's a header
    for offset in [0, 1, 2, 3, 4] {
        if test_data.len() >= offset + 8 * 10 {
            let shifted = &test_data[offset..];
            let values = parse_f64_be(shifted, 10);
            let valid = values.iter().filter(|&&v| is_valid_float(v)).count();
            if valid > 0 {
                println!("\n--- f64 BE with {}-byte offset: {}/10 valid ---", offset, valid);
                for (i, v) in values.iter().enumerate() {
                    println!("  [{}]: {:.6e} {}", i, v, if is_valid_float(*v) { "✓" } else { "✗" });
                }
            }
        }
    }

    println!("\nDone");
}
