# Audit A — Workspace / Dependency / CI

> Audit date: 2026-05-31  
> Base commit: `4627980` — "feat(lab): P6.1 preflight contract patch + GUI-M5A artifact viewer prep"

## 1. Workspace Membership

Root `Cargo.toml` defines **14 workspace crates** (all under `crates/`):

| # | Crate | Layer | Hardware I/O |
|---|-------|-------|-------------|
| 1 | `odmr-types` | 0 | No |
| 2 | `odmr-config` | 2 | No |
| 3 | `odmr-device` | 1 | No (trait only) |
| 4 | `odmr-safety` | 2 | No |
| 5 | `odmr-recipe` | 2 | No |
| 6 | `odmr-compiler` | 2 | No |
| 7 | `odmr-executor` | 3 | No (orchestration) |
| 8 | `odmr-logging` | 3 | No (file I/O only) |
| 9 | `odmr-replay` | 3 | No |
| 10 | `odmr-harness` | 3 | No (mock devices) |
| 11 | `odmr-smb100a` | 1 | TCP/SCPI |
| 12 | `odmr-oe1022d` | 1 | Serial |
| 13 | `odmr-mag` | 1 | No (mock-only models) |
| 14 | `odmr-maynuo-m8812` | 1 | Serial |

**NOT in workspace** (standalone projects):
- `apps/desktop/src-tauri/` — Tauri GUI app (separate workspace)
- `tools/discover/` — device discovery CLI
- All `tools/lab/*/` projects (~24 lab bring-up tools, each with its own `Cargo.toml`)

## 2. New Dependencies Since M4.1

| Dependency | Location | In Workspace? | Purpose |
|-----------|----------|---------------|---------|
| `serialport` | `crates/odmr-maynuo-m8812/Cargo.toml` | **Yes** | Maynuo M8812 serial transport |
| `serialport` | `tools/discover/Cargo.toml`, ~15 `tools/lab/*/Cargo.toml` | No | Lab tool serial I/O |
| `fs2` | `tools/lab/common_preflight/Cargo.toml` | No | Cross-process POSIX file lock (`DeviceLock`) |
| `tauri-plugin-dialog` | `apps/desktop/src-tauri/Cargo.toml` | No | Native folder picker for M5A viewer |
| `visa-rs` | `tools/lab/visa_probe/Cargo.toml` | No | R&S VISA benchmark tool |

**Notable absence:** `rsvisa` is **not found** in any `Cargo.toml` or `Cargo.lock` in the repo. Only `visa-rs` (a different crate) is used, and only in the standalone `visa_probe` tool.

## 3. VISA Isolation Status

| Aspect | Status |
|--------|--------|
| `rsvisa` in workspace | ❌ Absent entirely |
| `visa-rs` in workspace | ❌ Absent entirely |
| Feature-gated? | N/A — no VISA crate in workspace |
| VISA isolation verdict | ✅ **Clean** — all VISA usage quarantined to standalone `visa_probe` |

**Decision:** No feature-gate needed in workspace because no VISA dependency exists in the workspace. The workspace build cannot fail due to missing VISA libraries.

## 4. CI Risk Assessment

| Risk Item | Status |
|-----------|--------|
| `libudev-dev` in CI | ✅ Handled (`.github/workflows/ci.yml` line 44) |
| `serialport` build on Linux | ✅ Covered — CI installs `libudev-dev` before `cargo clippy` / `cargo test` |
| `cargo test --workspace` scope | ✅ Safe — only 14 workspace crates; no VISA/fs2 in scope |
| Tauri / lab tools built in CI | ✅ Not built — CI does not build `apps/desktop/src-tauri/` or `tools/lab/*/` |
| `cargo test --workspace` on clean Linux without `libudev-dev` | 🟡 **Developer risk** — will fail; documented in README but easy to miss |

## 5. Build Failure Risk Matrix

| Scenario | Risk | Mitigation |
|----------|------|------------|
| Missing RsVisa library | 🟢 **Zero** | No VISA dependency in workspace |
| Clean Linux without `libudev-dev` | 🟡 **Moderate** | CI handles it; developers need docs |
| macOS (any) | 🟢 **Low** | `serialport` uses IOKit; no extra deps |
| Building standalone `visa_probe` | 🟡 **Expected failure** | Requires R&S VISA installed; not a bug |

## 6. Recommendations

1. **No action on VISA feature-gating** — already fully isolated.
2. **Document `libudev-dev` requirement** more prominently for Linux developers (already in CI, add to README troubleshooting).
3. **Do not add `visa-rs` or `rsvisa` to workspace** without explicit ADR and feature-gate.
4. **Consider feature-gating `serialport` in `odmr-maynuo-m8812`** if the crate ever needs to compile in environments without serial support (e.g., headless servers). Not urgent today.
