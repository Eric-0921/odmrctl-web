//! odmr-discover — Read-only hardware discovery CLI for macOS lab bring-up.
//!
//! **Safety invariant**: only queries are sent. All commands are checked against
//! a hard-coded allow-list before transmission.

use clap::{Parser, Subcommand};
use odmr_discover::{discover_serial_ports, generate_report, is_safe_query, probe_tcp_address};
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "odmr-discover")]
#[command(about = "Read-only discovery tool for ODMR lab hardware")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List serial ports and probe each with *IDN?
    Serial {
        /// Probe a specific port instead of auto-discovering
        #[arg(short, long)]
        port: Option<String>,
        /// Baud rate for probing
        #[arg(short, long, default_value = "115200")]
        baud: u32,
    },
    /// Probe SMB100A over TCP port 5025
    Lan {
        /// Target IP address or hostname
        #[arg(short, long, default_value = "169.254.2.20")]
        host: String,
        /// Target port
        #[arg(short, long, default_value = "5025")]
        port: u16,
        /// SCPI query to send
        #[arg(short, long, default_value = "*IDN?")]
        query: String,
    },
    /// Run full discovery and write report to stdout or file
    Report {
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serial { port, baud } => {
            if let Some(port_name) = port {
                let result = odmr_discover::probe_serial_port(&port_name, baud, "*IDN?", 2000);
                println!("{:#?}", result);
            } else {
                let results = discover_serial_ports();
                for r in &results {
                    println!(
                        "{} @ {} -> {} : {}",
                        r.port_name,
                        r.baud_rate,
                        r.response.as_deref().unwrap_or("(no response)"),
                        r.error.as_deref().unwrap_or("OK")
                    );
                }
            }
        }
        Commands::Lan { host, port, query } => {
            if !is_safe_query("smb100a", &query) {
                eprintln!("Error: query '{}' is not in the safe-command allow-list.", query);
                eprintln!("Permitted queries: *IDN?, SYST:ERR?, OUTP?, MOD:STAT?");
                std::process::exit(1);
            }
            let addr = SocketAddr::from_str(&format!("{}:{}", host, port)).unwrap();
            let result = probe_tcp_address(addr, &query, 2000);
            println!("{:#?}", result);
        }
        Commands::Report { output } => {
            let serial_results = discover_serial_ports();
            let tcp_results = vec![probe_tcp_address(
                SocketAddr::from_str("169.254.2.20:5025").unwrap(),
                "*IDN?",
                2000,
            )];
            let report = generate_report(&serial_results, &tcp_results);
            if let Some(path) = output {
                std::fs::write(&path, report).expect("failed to write report");
                println!("Report written to {}", path);
            } else {
                println!("{}", report);
            }
        }
    }
}
