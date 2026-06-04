# visa_probe

VISA A/B Performance Benchmark for SMB100A.

## Purpose

Compare three transport methods for SCPI `*IDN?` round-trip latency:

| Method | Resource String | Notes |
|--------|----------------|-------|
| Raw TCP | `169.254.2.20:5025` | No VISA overhead |
| VISA VXI-11 | `TCPIP::169.254.2.20::INSTR` | Legacy RPC-based |
| VISA HiSLIP | `TCPIP::169.254.2.20::hislip0` | Modern replacement for VXI-11 |

## Usage

```bash
cargo run
```

Output includes a Markdown report (`visa_ab_benchmark_report.md`) with
min/max/mean/median/p99 statistics per method.

## Requirements

- R&S VISA installed at `/Library/Frameworks/RsVisa.framework` (macOS)
- SMB100A reachable at `169.254.2.20`

> **Build note:** `visa-sys` defaults to linking `framework=VISA` on macOS, but R&S VISA installs as `RsVisa.framework`. This project includes `.cargo/config.toml` that sets `LIB_VISA_NAME="framework=RsVisa"` automatically. If you build outside of this directory, set the environment variable manually:
> ```bash
> LIB_VISA_NAME="framework=RsVisa" cargo run
> ```

## Safety

This tool only sends read-only `*IDN?` queries. No RF output is enabled.
