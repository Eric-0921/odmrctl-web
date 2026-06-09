# M5C-A ODMR Device Workbench

Tauri v2 + React + TypeScript desktop application for ODMR experiment control and visualization.

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

## Features

- **Station Workbench**: Load station profile, run unified preflight, view preflight report with device reachability / identity / safe-state status
- **Device Panels** (4 minimal panels with typed get/set + readback):
  - **SMB100A**: Frequency, power, output state, modulation state
  - **OE1022D**: Reference frequency, sensitivity, time constant, filter slope, input source
  - **Magnetic**: Per-axis current, output state, zero-lock, sequential run
  - **Laser**: Off-only preflight integration, power state readback
- **Recipe Dry-Run Viewer** (M4.1): Full 201-step dry-run plan visualization
- **System Scan Artifact Viewer** (M5B-B): 7-tab read-only artifact viewer for system scan recipes
- **Experiment Planning**: Field grid scan recipe generation (1D / 2D / 3D) with parameter validation and export
- **Experiment Plan Execution Launcher** (preview): Preview UI for launching plans from the workbench

## Boundary

This application follows the architecture constraint that **the frontend never accesses hardware directly**:

- No serial / USB / VISA / TCP / SCPI code in the frontend
- All device interactions go through typed Tauri Command APIs
- The Rust backend performs discover/identify/lock/get/set via `odmr-preflight` and dedicated driver crates
- All set operations are protected by safety gates

The frontend is allowed to:
- Display device state read back from the backend
- Send typed set commands through Tauri Commands (backend validates and executes)
- Generate and preview experiment plans (pure data manipulation)
- View read-only artifacts and mock data

The frontend is **not** allowed to:
- Open sockets or serial ports directly
- Send raw SCPI strings
- Bypass safety checks
- Spawn lab tools as shell subprocesses for real runs
