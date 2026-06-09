# odmr-live-server

**Layer**: 3~4 (Runtime / Application API sidecar)

OE1022D real-time trace HTTP server. Provides live data streaming to browser-based chart frontends during active acquisition.

## Responsibilities

- Start `RallCollector` from `odmr-oe1022d` with configured serial port
- Consume captured frames in a background consumer thread
- Extract all 50 B-channel samples per frame (1ms spacing, 1kHz-equivalent resolution)
- Push samples into a shared ring buffer (`TraceRingBuffer`)
- Serve trace snapshots and collector stats over HTTP via Actix-web
- CORS-enabled for local development with web frontends

## HTTP Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/trace` | Full ring buffer snapshot (`TraceSnapshot`) |
| `GET` | `/api/stats` | Collector status (`CollectorStatus`) |

## Usage

```bash
cargo run -p odmr-live-server -- --port /dev/cu.usbmodem395D388533371 --http-port 9876
```

### CLI Options

| Flag | Default | Description |
|------|---------|-------------|
| `--port` | `/dev/cu.usbmodem3361358734371` | Serial port path |
| `--baud` | `921600` | Baud rate |
| `--http-port` | `9876` | HTTP server port |
| `--ring-capacity` | `2000` | Ring buffer capacity (points) |

## Architecture

```
RallCollector (producer thread)
  → mpsc channel(8)
    → Consumer thread (extract 50 samples/frame)
      → Arc<Mutex<TraceRingBuffer>>
        ← Actix-web handlers (/api/trace, /api/stats)
```

## Status

Phase 1 sidecar demo. Not part of the hot real-time acquisition path (that remains `odmr-executor → odmr-logging`). This crate is for visualization and diagnostics only.

## Dependencies

- `odmr-oe1022d` — RallCollector and frame types
- `actix-web`, `actix-cors` — HTTP server
