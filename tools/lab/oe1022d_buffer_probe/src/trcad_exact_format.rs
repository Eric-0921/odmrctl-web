//! Test TRCAD? with exact spacing from the manual source text.

use std::time::Duration;

fn query_once(port_path: &str, baud: u32, cmd: &str, settle_ms: u64) -> (usize, Vec<u8>) {
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
    println!("=== TRCAD? Exact Format from Manual Source ===");

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
    let (len, raw) = query_once(port_path, baud, "SPTSD ? 2", 800);
    println!("SPTSD ? 2: len={}, text='{}'\n", len, String::from_utf8_lossy(&raw).trim());

    // Test TRCAD? with spacing variations from manual source text
    let variants = [
        // Manual source: "TRCAD ? i,j ,k,l"  (space before comma after j)
        "TRCAD ? 2,1 ,0,1",
        "TRCAD ? 2,1 ,0,5",
        // Manual source: "TRCAD ? i, j, k , l" (spaces everywhere)
        "TRCAD ? 2, 1, 0, 1",
        "TRCAD ? 2, 1, 0, 5",
        // Manual source: "j=1,2 3,4" (space instead of comma between 2 and 3)
        // Try buffer list with space
        "TRCAD ? 2,1 2 3 4,0,1",
        // Try without space after TRCAD but with spaced params
        "TRCAD? 2, 1, 0, 1",
        // Try TRCAD without question mark (just in case)
        "TRCAD 2,1,0,1",
        // Try TRACE instead of TRCAD
        "TRACE ? 2,1,0,1",
        "TRACE? 2,1,0,1",
        // Try TRAC instead of TRCAD
        "TRAC ? 2,1,0,1",
        "TRAC? 2,1,0,1",
        // Try READ instead of TRCAD
        "READ ? 2,1,0,1",
        "READ? 2,1,0,1",
    ];

    for v in &variants {
        print!("'{}' -> ", v);
        let (len, raw) = query_once(port_path, baud, v, 3000);
        let text = String::from_utf8_lossy(&raw).trim().to_string();
        if len == 0 {
            println!("NO RESPONSE (0 bytes)");
        } else {
            println!("len={}, text='{}', hex={}", len, text.chars().take(100).collect::<String>(),
                raw.iter().take(20).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
        }
    }

    send_once(port_path, baud, "RESTD 2");
    println!("\nDone");
}
