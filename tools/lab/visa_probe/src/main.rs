use std::io::{Read, Write};
use visa_rs::{DefaultRM, AsResourceManager};

fn try_open(rm: &DefaultRM, addr_str: &str) {
    println!("\n--- Trying: {} ---", addr_str);
    let addr = visa_rs::VisaString::from_string(addr_str.to_string())
        .expect("valid address");
    
    match rm.open(&addr, visa_rs::flags::AccessMode::NO_LOCK, std::time::Duration::from_millis(3000)) {
        Ok(mut instr) => {
            println!("OPEN OK");
            
            let write_buf = b"*IDN?\n";
            match instr.write(write_buf) {
                Ok(n) => println!("Wrote {} bytes", n),
                Err(e) => println!("Write error: {:?}", e),
            }
            
            let mut read_buf = [0u8; 256];
            match instr.read(&mut read_buf) {
                Ok(n) => {
                    let resp = String::from_utf8_lossy(&read_buf[..n]);
                    println!("IDN ({} bytes): {}", n, resp.trim());
                }
                Err(e) => println!("Read error: {:?}", e),
            }
        }
        Err(e) => println!("OPEN FAILED: {:?}", e),
    }
}

fn main() {
    println!("=== VISA Probe for SMB100A ===");
    
    let rm = match DefaultRM::new() {
        Ok(rm) => {
            println!("Resource manager OK");
            rm
        }
        Err(e) => {
            eprintln!("RM failed: {:?}", e);
            std::process::exit(1);
        }
    };
    
    // R&S VISA format (from VB.NET example)
    try_open(&rm, "TCPIP::169.254.2.20::INSTR");        // VXI-11
    try_open(&rm, "TCPIP::169.254.2.20::hislip0");      // HiSLIP
    try_open(&rm, "TCPIP::169.254.2.20::5025::SOCKET"); // Raw socket
    
    println!("\n=== Done ===");
}
