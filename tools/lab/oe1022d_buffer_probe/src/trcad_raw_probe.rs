//! Distinguish "no response" vs "0 response" vs "other response" for TRCAD?.

use std::time::Duration;

fn raw_query(port_path: &str, baud: u32, cmd: &str, settle_ms: u64) -> (usize, Vec<u8>) {
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
        let wait_ms = if attempt == 0 { settle_ms } else { 500 };
        std::thread::sleep(Duration::from_millis(wait_ms));

        let mut buf = vec![0u8; 8192];
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                buf.truncate(n);
                collected.extend_from_slice(&buf);
                loop {
                    let avail = port.bytes_to_read().unwrap_or(0) as usize;
                    if avail == 0 { break; }
                    let mut extra = vec![0u8; avail.min(8192)];
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

    (collected.len(), collected)
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
    println!("=== TRCAD? Raw Response Probe ===");

    // Setup and fill buffer
    send_once(port_path, baud, "PAUSD 2");
    send_once(port_path, baud, "RESTD 2");
    send_once(port_path, baud, "SRATD 2,0.001");
    send_once(port_path, baud, "SLEND 2,50");
    send_once(port_path, baud, "SSLED 2,1,1");
    send_once(port_path, baud, "STRGD 2,0");
    send_once(port_path, baud, "SPRMD 2,0");
    send_once(port_path, baud, "RESTD 2");
    send_once(port_path, baud, "STRDD 2");
    std::thread::sleep(Duration::from_millis(2000));
    send_once(port_path, baud, "PAUSD 2");
    std::thread::sleep(Duration::from_millis(500));

    // Verify buffer has data
    let (len, raw) = raw_query(port_path, baud, "SPTSD ? 2", 800);
    println!("SPTSD ? 2: len={}, bytes={:?}, text='{}'", len, raw, String::from_utf8_lossy(&raw).trim());

    // Verify OUTPD works
    let (len, raw) = raw_query(port_path, baud, "OUTPD? 2,1", 1000);
    println!("OUTPD? 2,1: len={}, bytes={:?}, text='{}'", len, raw, String::from_utf8_lossy(&raw).trim());

    // Now test TRCAD? with raw byte capture
    println!("\n--- TRCAD? raw responses ---");

    let test_cases = [
        "TRCAD ? 2,1,0,1",
        "TRCAD? 2,1,0,1",
        "TRCAD ? 2,1,0,5",
        "TRCAD ? 2,1,1,1",
        "TRCAD ? 2,1,0,50",
    ];

    for cmd in &test_cases {
        let (len, raw) = raw_query(port_path, baud, cmd, 3000);
        let hex: String = raw.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
        let text = String::from_utf8_lossy(&raw);
        println!("'{}':", cmd);
        println!("  len={}", len);
        println!("  hex={}", if hex.is_empty() { "(empty)" } else { &hex });
        println!("  text='{}'", text.trim());
        println!();
    }

    send_once(port_path, baud, "RESTD 2");
    println!("Done");
}
