# Contributing to ODMR Automation

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain with `rustfmt` and `clippy`)
- [Git](https://git-scm.com/)

## Local Development

### Enable pre-commit hooks

```bash
git config core.hooksPath .githooks
```

### Run all mechanical checks

```bash
bash scripts/check-consistency.sh
bash scripts/check-docs-links.sh
bash scripts/check-prd-adr-index.sh
bash scripts/check-frontend-hardware.sh
bash scripts/check-realtime-csv.sh
bash scripts/check-agents-md.sh
bash scripts/check-schema-examples.sh
bash scripts/check-command-catalog.sh
```

Or run the full CI locally:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Check Descriptions

| Script | What it protects |
|--------|-----------------|
| `check-consistency.sh` | Crate directories match `Cargo.toml`, PRD/ADR numbering is contiguous, every crate has `README.md` |
| `check-docs-links.sh` | Internal markdown links in `docs/` point to existing files |
| `check-prd-adr-index.sh` | `docs/prd/` and `docs/adr/` have index files; every PRD/ADR is referenced by another doc |
| `check-frontend-hardware.sh` | `apps/desktop/` does not contain serial/USB/VISA/SCPI socket/TCP patterns or import hardware crates |
| `check-realtime-csv.sh` | Realtime crates (`odmr-executor`, `odmr-oe1022d`, `odmr-logging`, `odmr-harness`) do not contain CSV writers |
| `check-agents-md.sh` | Important directories (`crates/`, `apps/`, `python/`, `docs/`, repo root) contain `AGENTS.md` |
| `check-schema-examples.sh` | All JSON examples under `examples/` conform to schema and are covered by tests |
| `check-command-catalog.sh` | Device command catalogs compile, pass tests, and match manual-derived JSON sources |

## CI

GitHub Actions runs all checks on every push and pull request. See `.github/workflows/ci.yml`.

## Architecture Rules (hard constraints)

- **No frontend hardware access**: GUI code must not open serial ports, TCP sockets, or USB devices.
- **No realtime CSV**: The acquisition/executor hot path must only write `raw bin` + `index.jsonl` + `events.jsonl`.
- **PRD-first**: Every implementation task must reference a PRD section.
- **No AI live hardware control**: See `docs/adr/ADR-004-no-ai-live-hardware.md`.
