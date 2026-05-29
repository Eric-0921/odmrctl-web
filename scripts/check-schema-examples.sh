#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${WS_ROOT}"

echo "=== Running cargo test ==="
cargo test --workspace

echo ""
echo "=== Validating example JSON files ==="
# All integration tests in tests/recipe_integration_tests.rs already cover
# parsing of every .json under examples/.  We run them explicitly here.
cargo test --test recipe_integration_tests -- all_example_json_files_are_covered_by_tests

echo ""
echo "=== Schema example check complete ==="
