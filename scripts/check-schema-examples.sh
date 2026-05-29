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
echo "=== Checking schema files exist ==="
for schema in \
    "${WS_ROOT}/schemas/station.schema.json" \
    "${WS_ROOT}/schemas/recipe.schema.json" \
    "${WS_ROOT}/schemas/safety_limit.schema.json" \
    "${WS_ROOT}/schemas/resolved_recipe.schema.json" \
    "${WS_ROOT}/schemas/run_event.schema.json" \
    "${WS_ROOT}/schemas/dry_run_plan.schema.json" \
    "${WS_ROOT}/schemas/safety_report.schema.json"; do
    if [[ ! -f "$schema" ]]; then
        echo "ERROR: Missing schema file: $schema"
        exit 1
    fi
    echo "OK: $(basename "$schema")"
done

echo ""
echo "=== Schema example check complete ==="
