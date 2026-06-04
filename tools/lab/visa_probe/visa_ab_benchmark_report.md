# VISA A/B Performance Benchmark Report

- **Device**: SMB100A @ 169.254.2.20
- **Iterations**: 100
- **Warmup**: 5
- **Date**: 2026-06-04T15:15:04.461296+00:00

## Results

| Method | Min (µs) | Max (µs) | Mean (µs) | Median (µs) | p99 (µs) |
|--------|----------|----------|-----------|-------------|----------|
| Raw TCP (5025) | 1052 | 1248 | 1141.1 | 1142 | 1248 |
| VXI-11 | 1986 | 2199 | 2102.3 | 2104 | 2199 |
| HiSLIP | 1271 | 2122 | 1385.3 | 1371 | 2122 |

## Per-Method Details

### Raw TCP (5025)

- **IDN**: Rohde&Schwarz,SMB100A,1406.6000k02/101623,3.1.19.15-3.20.390.24
- **Min**: 1052 µs
- **Max**: 1248 µs
- **Mean**: 1141.1 µs
- **Median**: 1142 µs
- **p99**: 1248 µs

| Latency Bucket (µs) | Count |
|---------------------|-------|
| ≤ 2000 | 100 |

### VXI-11

- **IDN**: Rohde&Schwarz,SMB100A,1406.6000k02/101623,3.1.19.15-3.20.390.24
- **Min**: 1986 µs
- **Max**: 2199 µs
- **Mean**: 2102.3 µs
- **Median**: 2104 µs
- **p99**: 2199 µs

| Latency Bucket (µs) | Count |
|---------------------|-------|
| ≤ 2000 | 1 |
| ≤ 5000 | 99 |

### HiSLIP

- **IDN**: Rohde&Schwarz,SMB100A,1406.6000k02/101623,3.1.19.15-3.20.390.24
- **Min**: 1271 µs
- **Max**: 2122 µs
- **Mean**: 1385.3 µs
- **Median**: 1371 µs
- **p99**: 2122 µs

| Latency Bucket (µs) | Count |
|---------------------|-------|
| ≤ 2000 | 99 |
| ≤ 5000 | 1 |

## Interpretation

- **Raw TCP** is typically fastest (no VISA overhead), but lacks device-lock and error-handling abstractions.
- **HiSLIP** is the modern VISA replacement for VXI-11, offering better performance and features like async I/O.
- **VXI-11** is the legacy RPC-based protocol; usually slower due to SUN RPC overhead.

For production use, HiSLIP is recommended if available; otherwise raw TCP for minimal latency.