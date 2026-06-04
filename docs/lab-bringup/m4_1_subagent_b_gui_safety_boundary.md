# Subagent B — GUI Safety Boundary

## 1. Allowed Read-Only Commands

| Command | Purpose | Risk Level |
|---------|---------|------------|
| `app_metadata()` | Static app info | None |
| `read_analysis_directory(path)` | Read M3.6 analysis artifacts | None (read-only fs) |
| `pick_analysis_directory()` | Native folder picker | None (dialog only) |
| `read_recipe_file(path)` | Read recipe JSON text | None (read-only fs) |
| `pick_recipe_file()` | Native file picker | None (dialog only) |

## 2. Forbidden Hardware APIs

No Tauri command may:
- Open TCP sockets
- Open serial ports
- Send SCPI commands
- Call `odmr-smb100a`, `odmr-oe1022d`, `odmr-mag`, `odmr-device`, `odmr-executor` crates
- Execute recipe on hardware
- Connect to instruments

Forbidden command name patterns:
- `connect_*`
- `send_*`
- `open_serial*`
- `open_tcp*`
- `execute_*`
- `run_hardware*`
- `run_real*`
- `start_executor*`

## 3. How to Verify No TCP/Serial/SCPI Path Exists in GUI

```bash
# Check Tauri main.rs for forbidden imports
grep -n "odmr-smb100a\|odmr-oe1022d\|odmr-mag\|odmr-device\|odmr-executor\|TcpStream\|SerialPort\|serialport\|scpi\|VISA" apps/desktop/src-tauri/src/main.rs

# Check for forbidden command names
grep -n "connect_\|send_\|open_serial\|open_tcp\|execute_\|run_hardware\|run_real\|start_executor" apps/desktop/src-tauri/src/main.rs

# Check frontend for direct hardware access patterns
grep -rn "navigator.serial\|navigator.usb\|WebSocket\|fetch.*5025\|fetch.*5024\|SCPI\|VISA" apps/desktop/src/
```

Expected result: all commands return empty or only match in comments.

## 4. How to Display "Dry-Run Only" Clearly

### Page-level banner
- Blue/primary banner at top of `/recipe-viewer`
- Text: "Dry-run viewer only. No hardware connection. No recipe execution."
- Collapsible (like M4.0 banner)

### Validation panel
- Show "Operator approval required" as info (not actionable button)
- No "Approve and Run" button

### Command plan panel
- Explicit note: "This is a preview. No commands are sent to instruments."

### Sidebar nav
- "Recipe Viewer" label (not "Run Recipe" or "Execute")

## 5. What to Test

### Automated
1. `check-frontend-hardware.sh` passes (known false positives documented)
2. `pnpm tsc --noEmit` passes
3. `cargo test --workspace` passes
4. No forbidden command names in `main.rs`

### Manual
1. Load `/recipe-viewer` — confirm no network/serial activity
2. Load example recipe — confirm no file writes
3. Edit JSON — confirm changes stay in browser memory
4. Check browser DevTools Network tab — confirm no external requests
5. Check Tauri logs — confirm no hardware crate initialization
