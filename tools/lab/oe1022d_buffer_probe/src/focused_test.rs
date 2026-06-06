//! Focused OE1022D buffer test: one command per fresh connection.
//! Mimics the original probe tool's pattern to avoid port state issues.

use std::io::{Read, Write};
use std::time::{Duration, Instant};

fn send_cmd(port_path: &str, baud: u32, cmd: &str) {
    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(2000))
        .open()
        .expect("open port");
    let line = format!("{}\r", cmd);
    port.write_all(line.as_bytes()).unwrap();
    port.flush().unwrap();
    std::thread::sleep(Duration::from_millis(150));
}

fn query_cmd(port_path: &str, baud: u32, cmd: &str, settle_ms: u64, max_bytes: usize) -> String {
    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(2000))
        .open()
        .expect("open port");
    let _ = port.clear(serialport::ClearBuffer::Input);
    let line = format!("{}\r", cmd);
    port.write_all(line.as_bytes()).unwrap();
    port.flush().unwrap();

    let mut collected = Vec::new();
    for attempt in 0..5 {
        let wait_ms = if attempt == 0 { settle_ms } else { 200 };
        std::thread::sleep(Duration::from_millis(wait_ms));

        let remaining = max_bytes.saturating_sub(collected.len());
        if remaining == 0 { break; }

        let mut buf = vec![0u8; remaining.min(8192)];
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                buf.truncate(n);
                collected.extend_from_slice(&buf);
                loop {
                    let avail = port.bytes_to_read().unwrap_or(0) as usize;
                    if avail == 0 || collected.len() >= max_bytes { break; }
                    let chunk_len = avail.min(max_bytes - collected.len()).min(8192);
                    let mut extra = vec![0u8; chunk_len];
                    match port.read(&mut extra) {
                        Ok(m) if m > 0 => { extra.truncate(m); collected.extend_from_slice(&extra); }
                        _ => break,
                    }
                }
                break;
            }
            Ok(_) => continue,
            Err(_) => continue,
        }
    }

    String::from_utf8_lossy(&collected)
        .replace('\0', "")
        .trim()
        .to_string()
}

pub fn run(port_path: &str, baud: u32) {
    println!("=== Focused OE1022D Buffer Test (fresh connection per command) ===");

    // 1. Identity
    let idn = query_cmd(port_path, baud, "*IDN?", 500, 4096);
    println!("1. IDN: {}", idn);

    // 2. Configure (send-only)
    println!("\n2. Configure");
    send_cmd(port_path, baud, "PAUSD 2");
    send_cmd(port_path, baud, "RESTD 2");
    send_cmd(port_path, baud, "SRATD 2,0.001");
    send_cmd(port_path, baud, "SLEND 2,128");
    send_cmd(port_path, baud, "SSLED 2,1,1");
    send_cmd(port_path, baud, "STRGD 2,0");
    send_cmd(port_path, baud, "SPRMD 2,0");

    // 3. Readback
    println!("\n3. Readback");
    println!("   SRATD? 2 = '{}'", query_cmd(port_path, baud, "SRATD? 2", 500, 1024));
    println!("   SLEND? 2 = '{}'", query_cmd(port_path, baud, "SLEND? 2", 500, 1024));
    println!("   SSLED? 2,1 = '{}'", query_cmd(port_path, baud, "SSLED? 2,1", 500, 1024));
    println!("   STRGD? 2 = '{}'", query_cmd(port_path, baud, "STRGD? 2", 500, 1024));
    println!("   SPRMD? 2 = '{}'", query_cmd(port_path, baud, "SPRMD? 2", 500, 1024));

    // 4. Single mode: start and poll
    println!("\n4. Single mode sampling");
    send_cmd(port_path, baud, "RESTD 2");
    std::thread::sleep(Duration::from_millis(100));
    send_cmd(port_path, baud, "STRDD 2");

    let start = Instant::now();
    let mut single_polls: Vec<(u64, String)> = Vec::new();

    for _ in 0..10 {
        std::thread::sleep(Duration::from_millis(1000));
        let elapsed = start.elapsed().as_millis() as u64;
        let resp = query_cmd(port_path, baud, "SPTSD ? 2", 800, 1024);
        single_polls.push((elapsed, resp.clone()));
        println!("   t={:>4}ms, SPTSD? = '{}'", elapsed, resp);
    }

    // 5. TRCAD? read
    println!("\n5. TRCAD? read");
    let pts = single_polls.last().map(|p| p.1.clone()).unwrap_or_default();
    let pts_num = pts.trim().parse::<u32>().unwrap_or(0);
    let read_len = pts_num.min(50);
    if read_len > 0 {
        let trace = query_cmd(port_path, baud, &format!("TRCAD ? 2,1,0,{}\r", read_len).trim(), 1500, 64 * 1024);
        println!("   TRCAD? 2,1,0,{} -> len={}, raw='{}'", read_len, trace.len(),
            trace.chars().take(200).collect::<String>());
        let values: Vec<f64> = trace.split(',').filter_map(|s| s.trim().parse::<f64>().ok()).collect();
        println!("   Parsed {} values, first 5: {:?}", values.len(), values.iter().take(5).collect::<Vec<_>>());
    } else {
        println!("   No points to read (SPTSD? = '{}')", pts);
    }

    // 6. Pause
    println!("\n6. Pause");
    send_cmd(port_path, baud, "PAUSD 2");
    std::thread::sleep(Duration::from_millis(500));
    println!("   After pause: SPTSD? = '{}'", query_cmd(port_path, baud, "SPTSD ? 2", 800, 1024));

    // 7. Loop mode
    println!("\n7. Loop mode sampling");
    send_cmd(port_path, baud, "RESTD 2");
    send_cmd(port_path, baud, "SPRMD 2,1");
    std::thread::sleep(Duration::from_millis(100));
    send_cmd(port_path, baud, "STRDD 2");

    let start = Instant::now();
    let mut loop_polls: Vec<(u64, String)> = Vec::new();

    for _ in 0..10 {
        std::thread::sleep(Duration::from_millis(1000));
        let elapsed = start.elapsed().as_millis() as u64;
        let resp = query_cmd(port_path, baud, "SPTSD ? 2", 800, 1024);
        loop_polls.push((elapsed, resp.clone()));
        println!("   t={:>4}ms, SPTSD? = '{}'", elapsed, resp);
    }

    // 8. TRCAD? in loop mode
    println!("\n8. TRCAD? in loop mode");
    let pts2 = loop_polls.last().map(|p| p.1.clone()).unwrap_or_default();
    let pts2_num = pts2.trim().parse::<u32>().unwrap_or(0);
    let read_len2 = pts2_num.min(50);
    if read_len2 > 0 {
        let trace = query_cmd(port_path, baud, &format!("TRCAD ? 2,1,0,{}\r", read_len2).trim(), 1500, 64 * 1024);
        println!("   TRCAD? 2,1,0,{} -> len={}, raw='{}'", read_len2, trace.len(),
            trace.chars().take(200).collect::<String>());
        let values: Vec<f64> = trace.split(',').filter_map(|s| s.trim().parse::<f64>().ok()).collect();
        println!("   Parsed {} values, first 5: {:?}", values.len(), values.iter().take(5).collect::<Vec<_>>());
    } else {
        println!("   No points to read (SPTSD? = '{}')", pts2);
    }

    // Cleanup
    println!("\n9. Cleanup");
    send_cmd(port_path, baud, "PAUSD 2");
    send_cmd(port_path, baud, "RESTD 2");
    println!("   Done");
}
