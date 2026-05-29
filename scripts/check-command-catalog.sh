#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${WS_ROOT}"

echo "=== cargo fmt ==="
cargo fmt --all

echo ""
echo "=== cargo clippy ==="
cargo clippy --workspace --all-targets -- -D warnings

echo ""
echo "=== cargo test (workspace) ==="
cargo test --workspace

echo ""
echo "=== Command catalog check complete ==="
