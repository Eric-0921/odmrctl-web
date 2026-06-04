# Audit C — GUI Boundary

> Audit date: 2026-05-31  
> Base commit: `4627980`

## 1. Tauri Command Inventory

All commands defined in `apps/desktop/src-tauri/src/main.rs` (588 lines, single file). No other `.rs` files in Tauri src directory.

| # | Command Name | Classification | Hardware-Touching? | Notes |
|---|-------------|----------------|-------------------|-------|
| 1 | `app_metadata` | Read-only | **No** | Static JSON |
| 2 | `read_analysis_directory` | Read-only | **No** | `fs::read_to_string` only |
| 3 | `pick_analysis_directory` | Read-only | **No** | `tauri_plugin_dialog` folder picker |
| 4 | `read_recipe_file` | Read-only | **No** | `std::fs::read_to_string` |
| 5 | `pick_recipe_file` | Read-only | **No** | `tauri_plugin_dialog` file picker |
| 6 | `read_m5a_run_directory` | Read-only | **No** | Parses M5A artifacts from disk |
| 7 | `pick_m5a_run_directory` | Read-only | **No** | `tauri_plugin_dialog` folder picker |

**Total: 7 commands. All read-only. None write/mutate. None touch hardware.**

## 2. M5A Artifact Viewer Boundary

`read_m5a_run_directory` and `pick_m5a_run_directory` are **pure read-only file parsers**:
- Only call `fs::read_to_string` on expected artifact paths
- Return structured `M5aRunData` with warnings for missing/parse-failed files
- Never write to disk
- Never touch hardware

`apps/desktop/src/types/m5aRun.ts` is pure TypeScript interface definitions with zero runtime I/O.

**✅ Boundary clean.**

## 3. Forbidden Dependency Check

`apps/desktop/src-tauri/Cargo.toml` analysis:

| Forbidden Crate | Found? | Notes |
|-----------------|--------|-------|
| `odmr-executor` | ❌ No | Listed only in comment |
| `odmr-smb100a` | ❌ No | Listed only in comment |
| `odmr-oe1022d` | ❌ No | Listed only in comment |
| `odmr-device` | ❌ No | Listed only in comment |
| `odmr-compiler` | ❌ No | Listed only in comment |
| `odmr-safety` | ❌ No | Listed only in comment |
| `odmr-logging` | ❌ No | Listed only in comment |
| `odmr-maynuo-m8812` | ❌ No | Not mentioned |
| `serialport` | ❌ No | Not present |
| `rsvisa` / `visa-rs` | ❌ No | Not present |

**Actual dependencies:** `tauri`, `tauri-plugin-shell`, `tauri-plugin-dialog`, `serde`, `serde_json`.

**Minor observation:** `tauri-plugin-shell` is initialized in `main()` but **never invoked**. Low-risk but could be removed to reduce attack surface.

## 4. Shell Subprocess Spawning Check

| Method | Found? | Evidence |
|--------|--------|----------|
| `std::process::Command` | ❌ No | Not used |
| `tauri_plugin_shell` API calls | ❌ No | Initialized but never invoked |

**No GUI command spawns shell subprocesses to lab tools.** All data loaded by direct file parsing in Rust.

## 5. Frontend Hardware Access Check

Grep of `apps/desktop/src/` for hardware patterns:
- Only mock data references
- Disabled UI buttons with explicit "requires executor backend" / "no hardware authority" labels
- `AboutBoundariesPage.tsx`, `MockOnlyBanner.tsx` explicitly state "No serial / USB / VISA / TCP socket access"

**✅ No frontend hardware access.**

## 6. `common_preflight` Future Integration Assessment

| Aspect | Assessment |
|--------|------------|
| Hardware access? | **YES** — opens real serial ports, probes LAN/SCPI |
| File writes? | **YES** — writes lock files, station ledger |
| Blocking? | **YES** — synchronous I/O with timeouts |
| Safe for Tauri command? | **NO** — must NOT be called directly |

**Correct future architecture:**
- GUI sends intent via Tauri command to a **dedicated backend service/executor** (Layer 3/4)
- Backend service runs preflight logic on a dedicated thread/process
- Results streamed back as events/status updates

Direct GUI → `common_preflight` would violate:
- ADR-004 (hardware isolation)
- Layer dependency direction (Layer 5 → Layer 1)
- Would require `serialport` in `apps/desktop/src-tauri/Cargo.toml`

## 7. GUI Status Summary

| Checkpoint | Status |
|------------|--------|
| All Tauri commands read-only | ✅ PASS |
| No hardware-touching commands | ✅ PASS |
| M5A viewer does not touch hardware | ✅ PASS |
| Forbidden crate imports absent | ✅ PASS |
| No shell subprocess spawning | ✅ PASS |
| `common_preflight` not exposed to GUI | ✅ PASS (not integrated; must stay out) |
| Frontend hardware isolation | ✅ PASS |

## 8. Decision

**GUI boundary is clean. No violations detected.**

**Explicit rule:** GUI must not call lab tools as shell subprocesses for real runs. Future GUI run launcher (M4.2) must call a typed Rust runtime API (executor layer), not direct preflight invocation.
