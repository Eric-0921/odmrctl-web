//! Probe to determine exact command format accepted by OE1022D firmware.

use std::io::{Read, Write};
use std::time::Duration;

fn query(port: &mut Box<dyn serialport::SerialPort>, cmd: &str, settle_ms: u64) -> String {
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
    let last_line = s.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("").to_string();
    last_line
}

pub fn run(port_path: &str, baud: u32) {
    println!("=== OE1022D Command Format Probe ===");
    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(3000))
        .open()
        .expect("open port");

    // Identity baseline
    println!("\n1. Identity baseline");
    println!("   *IDN?    -> '{}'", query(&mut port, "*IDN?", 500));

    // 2. SPTSD format variants
    println!("\n2. SPTSD format variants");
    println!("   'SPTSD ? 2'   -> '{}'", query(&mut port, "SPTSD ? 2", 800));
    println!("   'SPTSD? 2'    -> '{}'", query(&mut port, "SPTSD? 2", 800));
    println!("   'SPTSD ?2'    -> '{}'", query(&mut port, "SPTSD ?2", 800));

    // 3. TRCAD format variants (first ensure some data exists)
    println!("\n3. Prepare buffer for TRCAD test");
    query(&mut port, "RESTD 2", 200);
    query(&mut port, "SRATD 2,0.001", 200);
    query(&mut port, "SLEND 2,100", 200);
    query(&mut port, "SSLED 2,1,1", 200);
    query(&mut port, "STRGD 2,0", 200);
    query(&mut port, "SPRMD 2,1", 200);
    query(&mut port, "STRDD 2", 200);
    std::thread::sleep(Duration::from_millis(2000));
    let pts = query(&mut port, "SPTSD? 2", 800);
    println!("   After 2s: SPTSD? = '{}'", pts);

    println!("\n4. TRCAD format variants");
    println!("   'TRCAD ? 2,1,0,5'  -> '{}'", query(&mut port, "TRCAD ? 2,1,0,5", 1000));
    println!("   'TRCAD? 2,1,0,5'   -> '{}'", query(&mut port, "TRCAD? 2,1,0,5", 1000));

    // 5. SRATD format variants
    println!("\n5. SRATD format variants");
    query(&mut port, "SRATD 2,0.001", 200);
    println!("   After 'SRATD 2,0.001': SRATD? 2 = '{}'", query(&mut port, "SRATD? 2", 500));
    query(&mut port, "SRATD 2,1", 200);
    println!("   After 'SRATD 2,1':    SRATD? 2 = '{}'", query(&mut port, "SRATD? 2", 500));

    // Cleanup
    query(&mut port, "PAUSD 2", 200);
    query(&mut port, "RESTD 2", 200);
    println!("\n=== Done ===");
}
