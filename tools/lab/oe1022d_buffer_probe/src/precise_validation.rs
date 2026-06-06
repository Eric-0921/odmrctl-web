//! Precise OE1022D buffer validation with fixed configuration and timing.

use odmr_oe1022d::commands::{
    query_stored_point_count, query_trace_data,
};
use std::io::{Read, Write};
use std::time::{Duration, Instant};

pub fn run(port_path: &str, baud: u32) {
    println!("=== Precise OE1022D Buffer Validation ===");
    println!("Port: {} @ {}", port_path, baud);

    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(3000))
        .open()
        .expect("open port");

    let mut query = |cmd: &str, settle_ms: u64| -> String {
        let _ = port.clear(serialport::ClearBuffer::Input);
        let line = format!("{}\r", cmd);
        port.write_all(line.as_bytes()).unwrap();
        port.flush().unwrap();

        let mut collected = Vec::new();
        for attempt in 0..5 {
            let wait_ms = if attempt == 0 { settle_ms } else { 200 };
            std::thread::sleep(Duration::from_millis(wait_ms));

            let mut buf = vec![0u8; 4096];
            match port.read(&mut buf) {
                Ok(n) if n > 0 => {
                    buf.truncate(n);
                    collected.extend_from_slice(&buf);
                    // Try to drain remaining
                    loop {
                        let avail = port.bytes_to_read().unwrap_or(0) as usize;
                        if avail == 0 { break; }
                        let mut extra = vec![0u8; avail.min(4096)];
                        match port.read(&mut extra) {
                            Ok(m) if m > 0 => { extra.truncate(m); collected.extend_from_slice(&extra); }
                            _ => break,
                        }
                    }
                    break;
                }
                _ => continue,
            }
        }

        let s = String::from_utf8_lossy(&collected)
            .replace('\0', "")
            .replace('\r', "\n")
            .trim()
            .to_string();
        // If there are multiple lines, take the last non-empty one
        let last_line = s.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("").to_string();
        last_line
    };

    // 1. Identity
    println!("\n1. Identity");
    let idn = query("*IDN?", 500);
    println!("   {}", idn);

    // 2. Fixed configuration
    println!("\n2. Fixed configuration");
    println!("   SRATD 2,0.001 -> {}", query("SRATD 2,0.001", 200));
    println!("   SRATD? 2 = {}", query("SRATD? 2", 500));

    println!("   SLEND 2,1000 -> {}", query("SLEND 2,1000", 200));
    println!("   SLEND? 2 = {}", query("SLEND? 2", 500));

    println!("   SSLED 2,1,1 -> {}", query("SSLED 2,1,1", 200));
    println!("   SSLED? 2,1 = {}", query("SSLED? 2,1", 500));

    println!("   STRGD 2,0 -> {}", query("STRGD 2,0", 200));
    println!("   STRGD? 2 = {}", query("STRGD? 2", 500));

    // Test both single and loop modes
    for mode in [0u8, 1u8] {
        let mode_name = if mode == 0 { "Single" } else { "Loop" };
        println!("\n3. {} mode validation", mode_name);

        println!("   SPRMD 2,{} -> {}", mode, query(&format!("SPRMD 2,{}\r", mode).trim(), 200));
        println!("   SPRMD? 2 = {}", query("SPRMD? 2", 500));

        // Reset and start
        println!("   RESTD 2 -> {}", query("RESTD 2", 200));
        std::thread::sleep(Duration::from_millis(100));
        println!("   STRDD 2 -> {}", query("STRDD 2", 200));

        // Poll with precise timing
        let start = Instant::now();
        let mut polls: Vec<(u64, u32)> = Vec::new();

        for _ in 0..12 {
            std::thread::sleep(Duration::from_millis(500));
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let resp = query(&query_stored_point_count(2), 500);
            let count = resp.trim().parse::<u32>().unwrap_or(0);
            polls.push((elapsed_ms, count));
            println!("   t={:>4}ms, SPTSD? = '{}'", elapsed_ms, resp);
        }

        // Try TRCAD? after we have some data
        let final_count = polls.last().map(|p| p.1).unwrap_or(0);
        let read_len = final_count.min(100);
        if read_len > 0 {
            std::thread::sleep(Duration::from_millis(200));
            println!("   TRCAD? 2,1,0,{} ->", read_len);
            let trace = query(&query_trace_data(2, 1, 0, read_len), 1000);
            println!("   Raw (len={}): '{}'", trace.len(), trace.chars().take(200).collect::<String>());

            // Parse
            let values: Vec<f64> = trace
                .split(',')
                .filter_map(|s| s.trim().parse::<f64>().ok())
                .collect();
            println!("   Parsed {} values, first 5: {:?}", values.len(), values.iter().take(5).collect::<Vec<_>>());
        }

        // Pause and check if points stop growing
        println!("   PAUSD 2 -> {}", query("PAUSD 2", 200));
        std::thread::sleep(Duration::from_millis(500));
        let paused_count = query("SPTSD? 2", 500);
        println!("   After pause: SPTSD? = {}", paused_count);
        std::thread::sleep(Duration::from_millis(500));
        let paused_count2 = query("SPTSD? 2", 500);
        println!("   After 500ms: SPTSD? = {}", paused_count2);

        // Calculate growth rate
        if polls.len() >= 2 {
            let first = polls.first().unwrap();
            let last = polls.last().unwrap();
            let dt_s = (last.0 - first.0) as f64 / 1000.0;
            let dp = if last.1 >= first.1 {
                last.1 - first.1
            } else {
                // Wrapped or reset — use positive segment only
                last.1
            };
            if dt_s > 0.0 {
                println!("   Growth rate: {:.1} points/s over {:.1}s ({} -> {} points)",
                    dp as f64 / dt_s, dt_s, first.1, last.1);
            }
        }
    }

    // Cleanup
    println!("\n4. Cleanup");
    println!("   RESTD 2 -> {}", query("RESTD 2", 200));
    println!("\n=== Done ===");
}
