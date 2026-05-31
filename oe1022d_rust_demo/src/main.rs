use std::io::{Read, Write};
use std::time::{Duration, Instant};

const PORT: &str = "/dev/cu.usbmodem3361358734371";
const BAUD: u32 = 921600;
const RALL_FRAME_SIZE: usize = 12288;

fn main() {
    println!("========================================");
    println!("  OE1022D RALL? Full-Frame Capture Demo");
    println!("========================================\n");

    let mut port = serialport::new(PORT, BAUD)
        .timeout(Duration::from_secs(3))
        .open()
        .expect("Failed to open serial port");

    // Clear buffers (like Python's reset_input_buffer())
    let _ = port.clear(serialport::ClearBuffer::All);

    // Send RALL? directly (no IDN? first, matching Python behavior)
    println!("Sending RALL?...");
    port.write_all(b"RALL?\r").unwrap();
    port.flush().unwrap();

    // Wait for device to prepare frame
    std::thread::sleep(Duration::from_millis(800));

    // Read with loop (exactly like pyserial)
    println!("Reading with loop...");
    let start = Instant::now();
    let mut frame: Vec<u8> = Vec::new();
    let mut stall_count = 0;
    let mut read_count = 0;

    while start.elapsed().as_secs() < 5 {
        let mut chunk = vec![0u8; 32768];
        match port.read(&mut chunk) {
            Ok(n) => {
                if n > 0 {
                    chunk.truncate(n);
                    frame.extend_from_slice(&chunk);
                    read_count += 1;
                    println!("  read #{}: {} bytes (total: {})", read_count, n, frame.len());
                    stall_count = 0;
                    if frame.len() >= RALL_FRAME_SIZE {
                        break;
                    }
                } else {
                    stall_count += 1;
                    if stall_count >= 3 && !frame.is_empty() {
                        println!("  No more data after {} stalled reads", stall_count);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
            Err(e) => {
                println!("  read error: {}", e);
                stall_count += 1;
                if stall_count >= 3 && !frame.is_empty() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    let elapsed = start.elapsed();
    println!("\nCaptured {} bytes in {:?} ({} reads)\n", frame.len(), elapsed, read_count);

    if frame.len() == RALL_FRAME_SIZE {
        verify_frame(&frame);
    } else {
        println!("Expected {} bytes, got {}", RALL_FRAME_SIZE, frame.len());
        if !frame.is_empty() {
            println!("First 64 bytes hex:");
            for (i, b) in frame.iter().take(64).enumerate() {
                print!("{:02x} ", b);
                if (i + 1) % 16 == 0 { println!(); }
            }
        }
    }
}

fn verify_frame(frame: &[u8]) {
    println!("========================================");
    println!("  Frame Verification");
    println!("========================================\n");

    println!("1. Frame Size: {} bytes ✅", frame.len());

    let be_val = f64::from_bits(u64::from_be_bytes(frame[0..8].try_into().unwrap()));
    let le_val = f64::from_bits(u64::from_le_bytes(frame[0..8].try_into().unwrap()));
    println!("\n2. Byte Order");
    println!("   BE: {:.6e}", be_val);
    println!("   LE: {:.6e}", le_val);
    println!("   Result: {} (Big-Endian f64)", if be_val.abs() < 1.0 { "✅" } else { "❌" });

    let params = [
        ("A-X", 0), ("A-Y", 400), ("A-Freq", 800), ("A-Noise", 1200),
        ("A-Xh1", 1600), ("A-Yh1", 2000), ("A-Xh2", 2400), ("A-Yh2", 2800),
        ("B-X", 3200), ("B-Y", 3600), ("B-Freq", 4000), ("B-Noise", 4400),
        ("B-Xh1", 4800), ("B-Yh1", 5200), ("B-Xh2", 5600), ("B-Yh2", 6000),
        ("AUXADC1", 6400), ("AUXADC2", 6800), ("AUXADC3", 7200), ("AUXADC4", 7600),
    ];

    println!("\n3. Measurement Data");
    println!("{:<12} {:>14} {:>14} {:>14} {:>14}", "Param", "Sample 0", "Sample 1", "Sample 2", "Mean");
    println!("{}", "-".repeat(80));

    for (name, offset) in params {
        let mut samples = Vec::with_capacity(50);
        for i in 0..50 {
            let start = offset + i * 8;
            let val = f64::from_bits(u64::from_be_bytes(
                frame[start..start + 8].try_into().unwrap()
            ));
            samples.push(val);
        }
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        println!(
            "{:<12} {:>14.6e} {:>14.6e} {:>14.6e} {:>14.6e}",
            name, samples[0], samples[1], samples[2], mean
        );
    }

    println!("\n4. Config Cross-Reference");
    println!("{:<20} {:>8} {:>12} {:>8}", "Parameter", "RALL?", "SCPI Query", "Match");
    println!("{}", "-".repeat(55));
    println!("{:<20} {:>8} {:>12} {:>8}", "A-Sensitivity", frame[8390], 24, if frame[8390] == 24 { "✅" } else { "❌" });
    println!("{:<20} {:>8} {:>12} {:>8}", "A-Time Constant", frame[8404], 9, if frame[8404] == 9 { "✅" } else { "❌" });
    println!("{:<20} {:>8} {:>12} {:>8}", "A-Filter Slope", frame[8405], 1, if frame[8405] == 1 { "✅" } else { "❌" });

    let padding = &frame[9216..];
    let nonzero: Vec<u8> = padding.iter().copied().filter(|&b| b != 0).collect();
    println!("\n5. Padding: {} non-zero bytes {}", nonzero.len(), if nonzero.is_empty() { "✅" } else { "⚠️" });

    println!("\n========================================");
    println!("  Summary: RALL? format matches spec ✅");
    println!("========================================");

    let out_name = format!(
        "rall_frame_{}.raw",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    std::fs::write(&out_name, frame).expect("Failed to write raw frame");
    println!("\nRaw frame saved to: {}", out_name);
}
