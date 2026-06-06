//! Minimal command-by-command OE1022D buffer validation.
//! This module is compiled as part of the probe binary but invoked separately
//! to test individual command semantics.

use std::io::{Read, Write};
use std::time::Duration;

pub fn run(port_path: &str, baud: u32) {
    println!("=== Minimal OE1022D Command Test ===");
    println!("Port: {} @ {}", port_path, baud);

    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(3000))
        .open()
        .expect("open port");

    // Clear input buffer
    let _ = port.clear(serialport::ClearBuffer::Input);

    // Helper to send and read raw response
    let mut query = |cmd: &str, settle_ms: u64| -> String {
        let _ = port.clear(serialport::ClearBuffer::Input);
        let line = format!("{}\r", cmd);
        port.write_all(line.as_bytes()).unwrap();
        port.flush().unwrap();
        std::thread::sleep(Duration::from_millis(settle_ms));

        let mut buf = vec![0u8; 4096];
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                buf.truncate(n);
                let s = String::from_utf8_lossy(&buf).replace('\0', "").trim().to_string();
                println!("  [<-] '{}' ({} bytes)", s.escape_default(), n);
                s
            }
            Ok(_) => {
                println!("  [<-] (empty / 0 bytes)");
                String::new()
            }
            Err(e) => {
                println!("  [<-] ERR: {}", e);
                String::new()
            }
        }
    };

    // Test 1: Identity
    println!("\n1. Identity");
    println!("  [->] *IDN?");
    let idn = query("*IDN?", 500);
    println!("  IDN: {}", idn);

    // Test 2: SRATD with different values
    println!("\n2. SRATD step time tests");

    println!("  [->] SRATD 2,0.001");
    query("SRATD 2,0.001", 200);
    println!("  [->] SRATD? 2");
    let r = query("SRATD? 2", 500);
    println!("  Result: set 0.001, read back: '{}'", r);

    println!("  [->] SRATD 2,1");
    query("SRATD 2,1", 200);
    println!("  [->] SRATD? 2");
    let r = query("SRATD? 2", 500);
    println!("  Result: set 1, read back: '{}'", r);

    println!("  [->] SRATD 2,0.05");
    query("SRATD 2,0.05", 200);
    println!("  [->] SRATD? 2");
    let r = query("SRATD? 2", 500);
    println!("  Result: set 0.05, read back: '{}'", r);

    println!("  [->] SRATD 2,0.1");
    query("SRATD 2,0.1", 200);
    println!("  [->] SRATD? 2");
    let r = query("SRATD? 2", 500);
    println!("  Result: set 0.1, read back: '{}'", r);

    // Test 3: SLEND
    println!("\n3. SLEND length tests");

    println!("  [->] SLEND 2,128");
    query("SLEND 2,128", 200);
    println!("  [->] SLEND? 2");
    let r = query("SLEND? 2", 500);
    println!("  Result: set 128, read back: '{}'", r);

    println!("  [->] SLEND 2,1000");
    query("SLEND 2,1000", 200);
    println!("  [->] SLEND? 2");
    let r = query("SLEND? 2", 500);
    println!("  Result: set 1000, read back: '{}'", r);

    println!("  [->] SLEND 2,16384");
    query("SLEND 2,16384", 200);
    println!("  [->] SLEND? 2");
    let r = query("SLEND? 2", 500);
    println!("  Result: set 16384, read back: '{}'", r);

    // Test 4: SSLED
    println!("\n4. SSLED buffer selector tests");

    println!("  [->] SSLED 2,1,1");
    query("SSLED 2,1,1", 200);
    println!("  [->] SSLED? 2,1");
    let r = query("SSLED? 2,1", 500);
    println!("  Result: set buffer1-param1, read back: '{}'", r);

    println!("  [->] SSLED 2,1,0");
    query("SSLED 2,1,0", 200);
    println!("  [->] SSLED? 2,1");
    let r = query("SSLED? 2,1", 500);
    println!("  Result: set buffer1-param0, read back: '{}'", r);

    // Test 5: STRGD / SPRMD
    println!("\n5. Trigger and run mode");

    println!("  [->] STRGD 2,0");
    query("STRGD 2,0", 200);
    println!("  [->] STRGD? 2");
    let r = query("STRGD? 2", 500);
    println!("  Result: set 0(INT), read back: '{}'", r);

    println!("  [->] SPRMD 2,0");
    query("SPRMD 2,0", 200);
    println!("  [->] SPRMD? 2");
    let r = query("SPRMD? 2", 500);
    println!("  Result: set 0(Single), read back: '{}'", r);

    // Test 6: Sampling start + SPTSD?
    println!("\n6. Sampling start test");
    println!("  [->] RESTD 2");
    query("RESTD 2", 200);
    println!("  [->] STRDD 2");
    query("STRDD 2", 200);

    // Poll SPTSD? several times
    for i in 0..5 {
        std::thread::sleep(Duration::from_millis(300));
        println!("  [->] SPTSD? 2 (poll {})", i + 1);
        let r = query("SPTSD? 2", 500);
        println!("  Stored points: '{}'", r);
    }

    // Test 7: TRCAD?
    println!("\n7. TRCAD? read test");
    println!("  [->] TRCAD? 2,1,0,10");
    let trace = query("TRCAD? 2,1,0,10", 800);
    println!("  Raw trace: '{}'", trace);

    // Cleanup
    println!("\n8. Cleanup");
    println!("  [->] PAUSD 2");
    query("PAUSD 2", 200);
    println!("  [->] RESTD 2");
    query("RESTD 2", 200);

    println!("\n=== Done ===");
}
