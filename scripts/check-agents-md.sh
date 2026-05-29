#!/usr/bin/env bash
set -euo pipefail

# check-agents-md.sh — verify that important directories contain AGENTS.md.
#
# Usage: bash scripts/check-agents-md.sh (run from repo root)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ERRORS=0
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

pass() { echo -e "  ${GREEN}PASS${NC} $1"; }
fail() { echo -e "  ${RED}FAIL${NC} $1"; ERRORS=$((ERRORS + 1)); }

echo "=== AGENTS.md presence check ==="
echo ""

# Directories that should have AGENTS.md
# The repo root AGENTS.md governs the entire project.
# Subdirectories may have their own AGENTS.md for more specific guidance.
REQUIRED_DIRS=(
    ""
)

OPTIONAL_DIRS=(
    "crates"
    "apps"
    "apps/desktop"
    "python"
    "docs"
)

for dir in "${REQUIRED_DIRS[@]}"; do
    target="${dir:-.}/AGENTS.md"
    if [ -f "$target" ]; then
        pass "$target exists"
    else
        fail "$target missing"
    fi
done

for dir in "${OPTIONAL_DIRS[@]}"; do
    target="${dir}/AGENTS.md"
    if [ -f "$target" ]; then
        pass "$target exists"
    else
        echo "  INFO $target missing (optional)"
    fi
done

echo ""
if [ "$ERRORS" -eq 0 ]; then
    echo -e "${GREEN}All AGENTS.md checks passed.${NC}"
    exit 0
else
    echo -e "${RED}$ERRORS missing AGENTS.md file(s).${NC}"
    exit 1
fi
