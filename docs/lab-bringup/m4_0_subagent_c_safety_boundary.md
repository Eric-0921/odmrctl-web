# M4.0 Subagent C — Safety Boundary Review

## References

- ADR-004: No AI Live Hardware (forbids direct hardware control by AI)
- ADR-006: GUI M0 Mock-Only Boundary (constrains what GUI pages can do)

## What M4.0 Must NOT Do

### Forbidden GUI Actions
- Connect to any instrument (SMB100A, OE1022D, Maynuo, laser)
- Open serial ports
- Open TCP sockets to instruments
- Send SCPI commands
- Execute recipes
- Invoke executor (`start_run`, `apply_step`, `send_command`)
- Write run artifacts
- Modify analysis files
- Create CSV output
- Control magnetic hardware (coils, current sources)

### Forbidden Backend APIs (Tauri commands must NOT)
- Import `odmr-smb100a`, `odmr-oe1022d`, `odmr-mag`, `odmr-maynuo-m8812`
- Import `odmr-executor`, `odmr-compiler`, `odmr-safety`
- Import `serialport`, `std::net::TcpStream` for instrument use
- Open files with write/create/append/truncate modes
- Execute shell commands
- Access network (except localhost Tauri IPC)

### Forbidden Tauri Command Names
- `connect_smb100a`, `send_scpi`, `run_recipe`, `execute_recipe`
- `connect_oe1022d`, `connect_magnetic`, `open_serial`, `open_tcp_instrument`
- `start_run`, `stop_run`, `emergency_stop`
- `set_current`, `set_frequency`, `set_power`
- Any command containing "hardware", "device", "instrument", "scpi", "recipe"

## What M4.0 IS Allowed to Do

### Allowed GUI Actions
- Display analysis results read from disk
- Render charts (frequency vs signal)
- Show quality flags, metadata, source run info
- Open native folder picker to select analysis directory
- Navigate between existing pages

### Allowed Backend APIs
- `std::fs::read_to_string` — read-only file access
- `std::fs::read_dir` — directory listing
- `std::path::Path` — path validation
- `serde_json::from_str` — JSON parsing
- `tauri-plugin-dialog` — native folder picker (allowed by ADR-006)

### Allowed Tauri Command Names
- `read_analysis_directory` — reads analysis artifacts from disk
- `pick_analysis_directory` — opens native folder picker

## How to Prove No Hardware Access

### Static verification (after implementation)
```bash
# Check no hardware crate imports in Tauri backend
grep -r "odmr-smb100a\|odmr-oe1022d\|odmr-mag\|odmr-executor\|odmr-compiler\|odmr-safety\|serialport" apps/desktop/src-tauri/

# Check no forbidden command names in Tauri handler
grep -r "#\[tauri::command\]" apps/desktop/src-tauri/src/ -A 5

# Check no file write operations in new commands
grep -r "write\|create\|append\|truncate\|File::create\|OpenOptions" apps/desktop/src-tauri/src/
```

### Runtime verification
- Run app in dev mode, check terminal for connection logs
- Confirm no SMB100A, OE1022D, or magnetic messages appear
- Confirm no serial port enumeration attempts
- Attempt to trigger hardware action via UI — should be impossible (no buttons)

## Magnetic Isolation

- M4.0 may display `no_magnetic: true` and `magnetic_command_present: false` from quality_flags.json
- M4.0 must not import `odmr-mag` or `odmr-maynuo-m8812` crates
- M4.0 must not reference Maynuo M8812, axis IDs, or current setpoints in any command or UI action
- If analysis data contains `magnetic_command_present: true`, display it but do NOT connect to magnetic hardware

## Boundary Banner Text

Every page must show:
> M4.0 READ-ONLY VIEWER — No hardware connection. No recipe execution. No magnetic control.

This should appear prominently on the AnalysisViewerPage.

## What If Quality Flags Say "unsafe"?

Display the unsafe state clearly (red badge, warning icon) but do NOT:
- Attempt to send emergency shutdown commands
- Attempt to connect and verify current instrument state
- Offer to "fix" or "retry" the run

The viewer is strictly read-only — even for safety-critical information.

## Test Checklist

- [ ] No hardware crate imports in `src-tauri/Cargo.toml`
- [ ] No forbidden Tauri command names in `generate_handler![]`
- [ ] All file operations use read-only mode
- [ ] Banner displays on AnalysisViewerPage
- [ ] No magnetic reference in UI actions or Tauri commands
- [ ] `cargo test --workspace` passes (no regression from new Tauri code)
