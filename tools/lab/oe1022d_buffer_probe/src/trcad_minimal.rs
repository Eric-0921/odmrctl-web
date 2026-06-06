//! Minimal TRCAD? format test — try every possible variation.

use std::time::Duration;

fn query_once(port_path: &str, baud: u32, cmd: &str, settle_ms: u64) -> String {
    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(3000))
        .open()
        .expect("open port");
    let _ = port.clear(serialport::ClearBuffer::Input);
    let line = format!("{}\r", cmd);
    port.write_all(line.as_bytes()).unwrap();
    port.flush().unwrap();
    std::thread::sleep(Duration::from_millis(settle_ms));

    let mut buf = vec![0u8; 8192];
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
        .timeout(Duration::from_millis(3000))
        .open()
        .expect("open port");
    let line = format!("{}\r", cmd);
    port.write_all(line.as_bytes()).unwrap();
    port.flush().unwrap();
    std::thread::sleep(Duration::from_millis(200));
}

pub fn run(port_path: &str, baud: u32) {
    println!("=== Minimal TRCAD? Format Test ===");

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

    let pts = query_once(port_path, baud, "SPTSD ? 2", 800);
    println!("Buffer points: '{}'", pts);

    // Sanity: OUTPD?
    println!("\nOUTPD? 2,1 = '{}'", query_once(port_path, baud, "OUTPD? 2,1", 1000));

    // TRCAD variants
    let variants = [
        "TRCAD ? 2,1,0,1",
        "TRCAD? 2,1,0,1",
        "TRCAD ? 2,1,1,1",
        "TRCAD ? 2,1,0,2",
        "TRCAD ? 2,1,0,10",
        "TRCAD ? 1,1,0,1",
        "TRCAD ? 2,2,0,1",
        "TRCAD ? 2,1,0,50",
    ];

    for v in &variants {
        println!("\n'{}'", v);
        let r = query_once(port_path, baud, v, 2000);
        println!("  -> len={}, raw='{}'", r.len(), r.chars().take(100).collect::<String>());
    }

    // Cleanup
    send_once(port_path, baud, "RESTD 2");
    println!("\nDone");
}
