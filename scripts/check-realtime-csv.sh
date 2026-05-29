#!/usr/bin/env bash
set -euo pipefail

# check-realtime-csv.sh — forbid CSV writers in real-time acquisition/executor paths.
#
# Per ADR-005: real-time phase must only write raw bin + index.jsonl + events.jsonl.
# CSV export is a post-run offline operation.
#
# Usage: bash scripts/check-realtime-csv.sh (run from repo root)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ERRORS=0
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

pass() { echo -e "  ${GREEN}PASS${NC} $1"; }
fail() { echo -e "  ${RED}FAIL${NC} $1"; ERRORS=$((ERRORS + 1)); }

echo "=== realtime CSV writer check ==="
echo ""

# Crates that must not contain CSV writing in hot paths
REALTIME_CRATES=(
    "crates/odmr-executor"
    "crates/odmr-oe1022d"
    "crates/odmr-logging"
    "crates/odmr-harness"
)

# Patterns that indicate CSV writing
CSV_PATTERNS=(
    'csv::Writer'
    'csv_writer'
    'write_csv'
    'to_csv'
    'CsvWriter'
    'csv::WriterBuilder'
    'serde_csv'
)

for crate_dir in "${REALTIME_CRATES[@]}"; do
    if [ ! -d "$crate_dir" ]; then
        continue
    fi
    for pat in "${CSV_PATTERNS[@]}"; do
        matches=$(find "$crate_dir/src" -name '*.rs' -print0 2>/dev/null | \
            xargs -0 grep -n "$pat" 2>/dev/null || true)
        if [ -n "$matches" ]; then
            fail "crate $crate_dir contains CSV pattern '$pat':\n$matches"
        fi
    done
done

if [ "$ERRORS" -eq 0 ]; then
    pass "no CSV writers found in realtime crates"
fi

echo ""
if [ "$ERRORS" -eq 0 ]; then
    echo -e "${GREEN}Realtime CSV check passed.${NC}"
    exit 0
else
    echo -e "${RED}$ERRORS violation(s) found.${NC}"
    exit 1
fi
