# ODMR GUI-M0 Mock Viewer

Mock-only Tauri + React + TypeScript viewer for ODMR Automation system.

## Prerequisites

- Node.js >= 18
- pnpm >= 8
- Rust >= 1.70 (via [rustup](https://rustup.rs/))

## Local Development

```bash
cd apps/desktop
pnpm install
pnpm tauri dev
```

The app will open in a desktop window. Frontend runs on `http://localhost:1420`.

## Build

```bash
cd apps/desktop
pnpm tauri build
```

## Boundary

This application is **mock-only**:

- No hardware access
- No executor connection
- No SCPI / serial / USB / VISA / socket code
- No raw binary parsing
- No experiment data writing

All controls that would trigger real actions are disabled and labeled with reasons.
