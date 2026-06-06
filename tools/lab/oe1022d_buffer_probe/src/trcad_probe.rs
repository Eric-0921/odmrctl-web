//! Focused TRCAD? troubleshooting probe.

use std::time::Duration;

fn query_once(port_path: &str, baud: u32, cmd: &str, settle_ms: u64, max_bytes: usize) -> String {
    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(5000))
        .open()
        .expect("open port");
    let _ = port.clear(serialport::ClearBuffer::Input);
    let line = format!("{}\r", cmd);
    port.write_all(line.as_bytes()).unwrap();
    port.flush().unwrap();

    let mut collected = Vec::new();
    for attempt in 0..10 {
        let wait_ms = if attempt == 0 { settle_ms } else { 300 };
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

fn send_once(port_path: &str, baud: u32, cmd: &str) {
    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(3000))
        .open()
        .expect("open port");
    let line = format!("{}\r", cmd);
    port.write_all(line.as_bytes()).unwrap();
    port.flush().unwrap();
    std::thread::sleep(Duration::from_millis(200));
}

pub fn run(port_path: &str, baud: u32) {
    println!("=== TRCAD? Troubleshooting Probe ===");

    // 1. Setup
    send_once(port_path, baud, "PAUSD 2");
    send_once(port_path, baud, "RESTD 2");
    send_once(port_path, baud, "SRATD 2,0.001");
    send_once(port_path, baud, "SLEND 2,100");
    send_once(port_path, baud, "SSLED 2,1,1");
    send_once(port_path, baud, "STRGD 2,0");
    send_once(port_path, baud, "SPRMD 2,0");

    // 2. Start sampling, wait for fill
    send_once(port_path, baud, "RESTD 2");
    send_once(port_path, baud, "STRDD 2");
    std::thread::sleep(Duration::from_millis(3000));

    let pts = query_once(port_path, baud, "SPTSD ? 2", 800, 1024);
    println!("1. After 3s: SPTSD? = '{}'", pts);

    // 3. Try TRCAD? while running
    println!("\n2. TRCAD? while running");
    let trace1 = query_once(port_path, baud, "TRCAD ? 2,1,0,5", 3000, 64 * 1024);
    println!("   'TRCAD ? 2,1,0,5'  -> len={}, hex={}, raw='{}'",
        trace1.len(),
        trace1.bytes().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "),
        trace1.chars().take(200).collect::<String>());

    // 4. Pause, then TRCAD?
    println!("\n3. TRCAD? after pause");
    send_once(port_path, baud, "PAUSD 2");
    std::thread::sleep(Duration::from_millis(500));
    let pts2 = query_once(port_path, baud, "SPTSD ? 2", 800, 1024);
    println!("   After pause: SPTSD? = '{}'", pts2);

    let trace2 = query_once(port_path, baud, "TRCAD ? 2,1,0,5", 3000, 64 * 1024);
    println!("   'TRCAD ? 2,1,0,5'  -> len={}, hex={}, raw='{}'",
        trace2.len(),
        trace2.bytes().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "),
        trace2.chars().take(200).collect::<String>());

    // 5. Try TRCAD? without space
    println!("\n4. TRCAD? without space after pause");
    let trace3 = query_once(port_path, baud, "TRCAD? 2,1,0,5", 3000, 64 * 1024);
    println!("   'TRCAD? 2,1,0,5'   -> len={}, hex={}, raw='{}'",
        trace3.len(),
        trace3.bytes().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "),
        trace3.chars().take(200).collect::<String>());

    // 6. Try different buffer / length
    println!("\n5. TRCAD? with l=1");
    let trace4 = query_once(port_path, baud, "TRCAD ? 2,1,0,1", 3000, 64 * 1024);
    println!("   'TRCAD ? 2,1,0,1'  -> len={}, raw='{}'", trace4.len(), trace4.chars().take(200).collect::<String>());

    // 7. Try reading from buffer 2, 3, 4
    println!("\n6. TRCAD? other buffers");
    for buf in [2, 3, 4] {
        let t = query_once(port_path, baud, &format!("TRCAD ? 2,{},0,1", buf), 3000, 64 * 1024);
        println!("   Buffer {}: len={}, raw='{}'", buf, t.len(), t.chars().take(50).collect::<String>());
    }

    // 8. Try OUTPD? as sanity check
    println!("\n7. OUTPD? sanity check");
    let outpd = query_once(port_path, baud, "OUTPD? 2,1", 1000, 1024);
    println!("   OUTPD? 2,1 = '{}'", outpd);

    // Cleanup
    send_once(port_path, baud, "RESTD 2");
    println!("\nDone");
}
