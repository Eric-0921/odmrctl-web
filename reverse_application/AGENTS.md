# reverse_application — Read-Only Reference

This directory contains reverse-engineering results from the legacy
磁场线圈控制软件 (magnetic coil control software).

## Rules for AI agents

- **Read only.** Do not modify files in this directory.
- **Do not copy architecture.** Do not use its GUI patterns, serial-port
  patterns, or threading model as a template for the new Rust/Tauri system.
- **Do not import code.** Do not copy source files or algorithms into
  `crates/`, `apps/`, or `python/`.
- **Allowed use:** Extract hardware/protocol clues (register maps,
  command sequences, timing constants) **only when explicitly asked**.
