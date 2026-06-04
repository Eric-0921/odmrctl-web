//! VISA A/B Performance Benchmark — SMB100A
//!
//! Compares three transport methods for SCPI *IDN? round-trip latency:
//! 1. Raw TCP socket (port 5025)
//! 2. VISA VXI-11  (TCPIP::addr::INSTR)
//! 3. VISA HiSLIP  (TCPIP::addr::hislip0)
//!
//! Output: Markdown report with min/max/mean/median/p99 statistics.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};
use visa_rs::{AsResourceManager, DefaultRM};

const SMB_ADDR: &str = "169.254.2.20";
const ITERATIONS: usize = 100;
const WARMUP: usize = 5;

fn main() {
    println!("=== VISA A/B Performance Benchmark ===\n");
    println!("Device: SMB100A @ {}\n", SMB_ADDR);

    let mut results: Vec<BenchmarkResult> = Vec::with_capacity(3);

    // 1. Raw TCP socket
    if let Ok(r) = benchmark_raw_socket() {
        results.push(r);
    }

    // 2. VISA VXI-11
    if let Ok(r) = benchmark_visa("VXI-11", &format!("TCPIP::{}::INSTR", SMB_ADDR)) {
        results.push(r);
    }

    // 3. VISA HiSLIP
    if let Ok(r) = benchmark_visa("HiSLIP", &format!("TCPIP::{}::hislip0", SMB_ADDR)) {
        results.push(r);
    }

    // Print report
    println!("\n=== Results ({} iterations after {} warmup) ===\n", ITERATIONS, WARMUP);
    print_markdown_table(&results);

    // Write markdown report
    let report = generate_markdown_report(&results);
    let report_path = "visa_ab_benchmark_report.md";
    if let Err(e) = std::fs::write(report_path, report) {
        eprintln!("Failed to write report: {}", e);
    } else {
        println!("\nReport written to: {}", report_path);
    }
}

#[derive(Debug)]
struct BenchmarkResult {
    method: String,
    idn: String,
    samples: Vec<u64>, // microseconds
}

fn benchmark_raw_socket() -> Result<BenchmarkResult, String> {
    let addr: SocketAddr = format!("{}:5025", SMB_ADDR)
        .parse()
        .map_err(|e| format!("parse addr: {}", e))?;

    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))
        .map_err(|e| format!("connect: {}", e))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| format!("timeout: {}", e))?;

    let idn = scpi_roundtrip(&mut stream, "*IDN?")
        .map_err(|e| format!("IDN failed: {}", e))?;

    // Warmup
    for _ in 0..WARMUP {
        let _ = scpi_roundtrip(&mut stream, "*IDN?");
    }

    let mut samples = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let _ = scpi_roundtrip(&mut stream, "*IDN?");
        samples.push(start.elapsed().as_micros() as u64);
    }

    Ok(BenchmarkResult {
        method: "Raw TCP (5025)".into(),
        idn,
        samples,
    })
}

fn benchmark_visa(name: &str, resource: &str) -> Result<BenchmarkResult, String> {
    let rm = DefaultRM::new().map_err(|e| format!("RM: {:?}", e))?;
    let addr = visa_rs::VisaString::from_string(resource.to_string())
        .ok_or_else(|| "invalid VISA resource string".to_string())?;
    let mut instr = rm
        .open(&addr, visa_rs::flags::AccessMode::NO_LOCK, Duration::from_secs(3))
        .map_err(|e| format!("open: {:?}", e))?;

    let idn = visa_roundtrip(&mut instr, "*IDN?")
        .map_err(|e| format!("IDN failed: {}", e))?;

    // Warmup
    for _ in 0..WARMUP {
        let _ = visa_roundtrip(&mut instr, "*IDN?");
    }

    let mut samples = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let _ = visa_roundtrip(&mut instr, "*IDN?");
        samples.push(start.elapsed().as_micros() as u64);
    }

    Ok(BenchmarkResult {
        method: name.into(),
        idn,
        samples,
    })
}

fn scpi_roundtrip(stream: &mut TcpStream, cmd: &str) -> Result<String, String> {
    stream
        .write_all(format!("{}\n", cmd).as_bytes())
        .map_err(|e| format!("write: {}", e))?;
    let mut buf = [0u8; 256];
    let n = stream
        .read(&mut buf)
        .map_err(|e| format!("read: {}", e))?;
    Ok(String::from_utf8_lossy(&buf[..n]).trim().to_string())
}

fn visa_roundtrip(instr: &mut visa_rs::Instrument, cmd: &str) -> Result<String, String> {
    instr.write_all(format!("{}\n", cmd).as_bytes())
        .map_err(|e| format!("write: {}", e))?;
    let mut buf = [0u8; 256];
    let n = instr.read(&mut buf)
        .map_err(|e| format!("read: {}", e))?;
    Ok(String::from_utf8_lossy(&buf[..n]).trim().to_string())
}

fn stats(samples: &[u64]) -> (u64, u64, f64, u64, u64) {
    let min = *samples.iter().min().unwrap_or(&0);
    let max = *samples.iter().max().unwrap_or(&0);
    let mean = samples.iter().sum::<u64>() as f64 / samples.len() as f64;
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let median = sorted[sorted.len() / 2];
    let p99_idx = (sorted.len() as f64 * 0.99) as usize;
    let p99 = sorted[p99_idx.min(sorted.len() - 1)];
    (min, max, mean, median, p99)
}

fn print_markdown_table(results: &[BenchmarkResult]) {
    println!("| Method | Min (µs) | Max (µs) | Mean (µs) | Median (µs) | p99 (µs) | IDN |");
    println!("|--------|----------|----------|-----------|-------------|----------|-----|");
    for r in results {
        let (min, max, mean, median, p99) = stats(&r.samples);
        let idn_short = if r.idn.len() > 30 {
            format!("{}...", &r.idn[..30])
        } else {
            r.idn.clone()
        };
        println!(
            "| {} | {} | {} | {:.1} | {} | {} | {} |",
            r.method, min, max, mean, median, p99, idn_short
        );
    }
}

fn generate_markdown_report(results: &[BenchmarkResult]) -> String {
    let mut lines = Vec::new();
    lines.push("# VISA A/B Performance Benchmark Report".to_string());
    lines.push("".to_string());
    lines.push(format!("- **Device**: SMB100A @ {}", SMB_ADDR));
    lines.push(format!("- **Iterations**: {}", ITERATIONS));
    lines.push(format!("- **Warmup**: {}", WARMUP));
    lines.push(format!("- **Date**: {}", chrono::Utc::now().to_rfc3339()));
    lines.push("".to_string());
    lines.push("## Results".to_string());
    lines.push("".to_string());
    lines.push("| Method | Min (µs) | Max (µs) | Mean (µs) | Median (µs) | p99 (µs) |".to_string());
    lines.push("|--------|----------|----------|-----------|-------------|----------|".to_string());

    for r in results {
        let (min, max, mean, median, p99) = stats(&r.samples);
        lines.push(format!(
            "| {} | {} | {} | {:.1} | {} | {} |",
            r.method, min, max, mean, median, p99
        ));
    }

    lines.push("".to_string());
    lines.push("## Per-Method Details".to_string());
    lines.push("".to_string());

    for r in results {
        let (min, max, mean, median, p99) = stats(&r.samples);
        lines.push(format!("### {}", r.method));
        lines.push("".to_string());
        lines.push(format!("- **IDN**: {}", r.idn));
        lines.push(format!("- **Min**: {} µs", min));
        lines.push(format!("- **Max**: {} µs", max));
        lines.push(format!("- **Mean**: {:.1} µs", mean));
        lines.push(format!("- **Median**: {} µs", median));
        lines.push(format!("- **p99**: {} µs", p99));
        lines.push("".to_string());

        // Histogram buckets (log scale)
        let mut buckets: std::collections::BTreeMap<u64, usize> = std::collections::BTreeMap::new();
        for &s in &r.samples {
            let bucket = match s {
                0..=99 => 100,
                100..=199 => 200,
                200..=499 => 500,
                500..=999 => 1000,
                1000..=1999 => 2000,
                2000..=4999 => 5000,
                5000..=9999 => 10000,
                _ => 20000,
            };
            *buckets.entry(bucket).or_insert(0) += 1;
        }
        lines.push("| Latency Bucket (µs) | Count |".to_string());
        lines.push("|---------------------|-------|".to_string());
        for (bucket, count) in &buckets {
            lines.push(format!("| ≤ {} | {} |", bucket, count));
        }
        lines.push("".to_string());
    }

    lines.push("## Interpretation".to_string());
    lines.push("".to_string());
    lines.push("- **Raw TCP** is typically fastest (no VISA overhead), but lacks device-lock and error-handling abstractions.".to_string());
    lines.push("- **HiSLIP** is the modern VISA replacement for VXI-11, offering better performance and features like async I/O.".to_string());
    lines.push("- **VXI-11** is the legacy RPC-based protocol; usually slower due to SUN RPC overhead.".to_string());
    lines.push("".to_string());
    lines.push("For production use, HiSLIP is recommended if available; otherwise raw TCP for minimal latency.".to_string());

    lines.join("\n")
}
