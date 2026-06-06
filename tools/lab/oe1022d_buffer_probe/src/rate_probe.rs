//! Measure actual buffer fill rate with high-frequency polling.

use std::time::{Duration, Instant};

fn query_once(port_path: &str, baud: u32, cmd: &str, settle_ms: u64) -> String {
    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(2000))
        .open()
        .expect("open port");
    let _ = port.clear(serialport::ClearBuffer::Input);
    let line = format!("{}\r", cmd);
    port.write_all(line.as_bytes()).unwrap();
    port.flush().unwrap();
    std::thread::sleep(Duration::from_millis(settle_ms));

    let mut buf = vec![0u8; 4096];
    match port.read(&mut buf) {
        Ok(n) if n > 0 => {
            buf.truncate(n);
            String::from_utf8_lossy(&buf).replace('\0', "").trim().to_string()
        }
        _ => String::new(),
    }
}

fn send_once(port_path: &str, baud: u32, cmd: &str) {
    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(2000))
        .open()
        .expect("open port");
    let line = format!("{}\r", cmd);
    port.write_all(line.as_bytes()).unwrap();
    port.flush().unwrap();
    std::thread::sleep(Duration::from_millis(150));
}

pub fn run(port_path: &str, baud: u32) {
    println!("=== OE1022D Rate Probe (high-frequency polling) ===");

    // 1. Configure
    send_once(port_path, baud, "PAUSD 2");
    send_once(port_path, baud, "RESTD 2");
    send_once(port_path, baud, "SRATD 2,0.001");
    send_once(port_path, baud, "SLEND 2,128");
    send_once(port_path, baud, "SSLED 2,1,1");
    send_once(port_path, baud, "STRGD 2,0");
    send_once(port_path, baud, "SPRMD 2,0");

    println!("Config readback:");
    println!("  SRATD? 2 = '{}'", query_once(port_path, baud, "SRATD? 2", 500));
    println!("  SLEND? 2 = '{}'", query_once(port_path, baud, "SLEND? 2", 500));
    println!("  SPRMD? 2 = '{}'", query_once(port_path, baud, "SPRMD? 2", 500));

    // 2. Single mode: poll every 100ms for 2 seconds
    println!("\nSingle mode: poll every 100ms");
    send_once(port_path, baud, "RESTD 2");
    std::thread::sleep(Duration::from_millis(50));
    let start = Instant::now();
    send_once(port_path, baud, "STRDD 2");

    let mut single_points: Vec<(u64, u32)> = Vec::new();
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(100));
        let elapsed = start.elapsed().as_millis() as u64;
        let resp = query_once(port_path, baud, "SPTSD ? 2", 300);
        let count = resp.trim().parse::<u32>().unwrap_or(0);
        single_points.push((elapsed, count));
    }
    for (t, c) in &single_points {
        println!("  t={:>5}ms, points={}", t, c);
    }

    // Calculate rate
    if single_points.len() >= 2 {
        let first = single_points[0];
        let last = *single_points.last().unwrap();
        let dt = (last.0 - first.0) as f64 / 1000.0;
        let dp = if last.1 >= first.1 { last.1 - first.1 } else { last.1 };
        if dt > 0.0 {
            println!("  Growth rate: {:.1} Hz ({} points in {:.2}s)", dp as f64 / dt, dp, dt);
        }
    }

    // 3. Loop mode: poll every 100ms for 2 seconds
    println!("\nLoop mode: poll every 100ms");
    send_once(port_path, baud, "PAUSD 2");
    send_once(port_path, baud, "RESTD 2");
    send_once(port_path, baud, "SPRMD 2,1");
    std::thread::sleep(Duration::from_millis(50));
    let start = Instant::now();
    send_once(port_path, baud, "STRDD 2");

    let mut loop_points: Vec<(u64, u32)> = Vec::new();
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(100));
        let elapsed = start.elapsed().as_millis() as u64;
        let resp = query_once(port_path, baud, "SPTSD ? 2", 300);
        let count = resp.trim().parse::<u32>().unwrap_or(0);
        loop_points.push((elapsed, count));
    }
    for (t, c) in &loop_points {
        println!("  t={:>5}ms, points={}", t, c);
    }

    if loop_points.len() >= 2 {
        let first = loop_points[0];
        let last = *loop_points.last().unwrap();
        let dt = (last.0 - first.0) as f64 / 1000.0;
        let dp = if last.1 >= first.1 { last.1 - first.1 } else { last.1 + 128 - first.1 };
        if dt > 0.0 {
            println!("  Growth rate: {:.1} Hz ({} points in {:.2}s, with wrap)", dp as f64 / dt, dp, dt);
        }
    }

    // 4. TRCAD? test with fresh connection and small read
    println!("\nTRCAD? test");
    let pts = single_points.last().map(|p| p.1).unwrap_or(0);
    if pts > 0 {
        // Try different TRCAD formats
        let trace_a = query_once(port_path, baud, "TRCAD ? 2,1,0,10", 1000);
        println!("  'TRCAD ? 2,1,0,10' -> len={}, raw='{}'", trace_a.len(), trace_a.chars().take(100).collect::<String>());

        let trace_b = query_once(port_path, baud, "TRCAD? 2,1,0,10", 1000);
        println!("  'TRCAD? 2,1,0,10'  -> len={}, raw='{}'", trace_b.len(), trace_b.chars().take(100).collect::<String>());
    }

    // Cleanup
    send_once(port_path, baud, "PAUSD 2");
    send_once(port_path, baud, "RESTD 2");
    println!("\nDone");
}
