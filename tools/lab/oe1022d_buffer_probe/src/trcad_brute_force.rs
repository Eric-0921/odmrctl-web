//! Brute-force probe for buffer-read command names.
//! If any command returns non-zero bytes (even garbage), it means the device
//! recognizes the command name and the issue is parameter format.

use std::time::Duration;

fn query_once(port_path: &str, baud: u32, cmd: &str, settle_ms: u64) -> (usize, Vec<u8>) {
    let mut port = serialport::new(port_path, baud)
        .timeout(Duration::from_millis(800))
        .open()
        .expect("open port");
    let _ = port.clear(serialport::ClearBuffer::Input);
    let line = format!("{}\r", cmd);
    port.write_all(line.as_bytes()).unwrap();
    port.flush().unwrap();

    let mut collected = Vec::new();
    for attempt in 0..3 {
        let wait_ms = if attempt == 0 { settle_ms } else { 100 };
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
    std::thread::sleep(Duration::from_millis(150));
}

pub fn run(port_path: &str, baud: u32) {
    println!("=== Brute-force buffer-read command probe ===");

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
    std::thread::sleep(Duration::from_millis(300));

    let (len, raw) = query_once(port_path, baud, "SPTSD ? 2", 500);
    println!("SPTSD ? 2 -> len={}, text='{}'\n", len, String::from_utf8_lossy(&raw).trim());

    // Build candidate list
    let names = [
        // Original + permutations
        "TRCAD", "TRACD", "TCRAD", "TCRDA", "TARCD", "TARDC",
        // OE1022 legacy (no D)
        "TRCA", "TRAC", "TCRA", "TCAR", "TARC", "TACR",
        // SCPI-ish
        "FETD", "FETC", "FETCD", "FETH",
        // Read data variants
        "READD", "READ", "REDA", "RDDD",
        // Get / Fetch
        "GETD", "GETCD", "GETC",
        // Data / Buffer
        "DATD", "DATAD", "DATA", "BUFF", "BUFD", "BUFDA",
        // Trace variants
        "TRAD", "TRDA", "TREAD", "TRDAD",
        // Sample data
        "SAMP", "SAMD", "SMPL", "SMPLD",
        // Output / Query
        "OUTD", "OUTPD", "QRYD", "QUER",
        // Other plausible 4-letter + D
        "SNPD", "SNAP", "SNPDD",
        // Try with extra D
        "TRCADD", "TRACDD",
    ];

    let params = [
        " ? 2,1,0,1",
        "? 2,1,0,1",
        " 2,1,0,1",
        " ? 2,1,0,5",
        "? 2,1,0,5",
    ];

    let total = names.len() * params.len();
    let mut tested = 0;
    let mut found_any = false;

    for name in &names {
        for param in &params {
            let cmd = format!("{}{}", name, param);
            let (len, raw) = query_once(port_path, baud, &cmd, 500);
            tested += 1;
            if len > 0 {
                let text = String::from_utf8_lossy(&raw).trim().to_string();
                let hex = raw.iter().take(20).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
                println!("FOUND! '{}' -> len={}, text='{}', hex={}", cmd, len, text, hex);
                found_any = true;
            }
            if tested % 10 == 0 {
                println!("  progress: {}/{}...", tested, total);
            }
        }
    }

    if !found_any {
        println!("\nNo response from any candidate command.");
        println!("Tested {} command names x {} param formats = {} combinations.",
            names.len(), params.len(), total);
    }

    send_once(port_path, baud, "RESTD 2");
    println!("\nDone");
}
