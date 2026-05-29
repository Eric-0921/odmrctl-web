#!/usr/bin/env bash
set -euo pipefail

# check-prd-adr-index.sh — verify PRD/ADR directories have index files.
# Usage: bash scripts/check-prd-adr-index.sh (run from repo root)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ERRORS=0
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

pass() { echo -e "  ${GREEN}PASS${NC} $1"; }
fail() { echo -e "  ${RED}FAIL${NC} $1"; ERRORS=$((ERRORS + 1)); }

echo "=== PRD/ADR index check ==="
echo ""

# Check docs/prd/ and docs/adr/ have at least one file
if [ -n "$(find docs/prd -maxdepth 1 -name '*.md' -type f 2>/dev/null)" ]; then
    pass "docs/prd/ contains PRD files"
else
    fail "docs/prd/ missing PRD files"
fi

if [ -n "$(find docs/adr -maxdepth 1 -name 'ADR-*.md' -type f 2>/dev/null)" ]; then
    pass "docs/adr/ contains ADR files"
else
    fail "docs/adr/ missing ADR files"
fi

# Optional: warn if index files are missing (not a hard failure)
if [ ! -f "docs/prd/README.md" ] && [ ! -f "docs/prd/index.md" ]; then
    echo "  WARN docs/prd/ missing README.md or index.md (optional)"
fi
if [ ! -f "docs/adr/README.md" ] && [ ! -f "docs/adr/index.md" ]; then
    echo "  WARN docs/adr/ missing README.md or index.md (optional)"
fi

# Optional: check that every PRD file is mentioned in at least one other docs file
# (simple heuristic: grep for the basename without extension)
echo ""
echo "Checking PRD/ADR file references (informational)..."
for f in docs/prd/*.md; do
    base=$(basename "$f" .md)
    refs=$( (grep -rl "$base" docs/ --include="*.md" 2>/dev/null || true) | (grep -v "$(basename "$f")" || true) | wc -l | tr -d ' ')
    if [ "$refs" -gt 0 ]; then
        pass "$base referenced in $refs other doc(s)"
    else
        echo "  INFO $base not referenced by any other doc (optional)"
    fi
done

echo ""
if [ "$ERRORS" -eq 0 ]; then
    echo -e "${GREEN}All PRD/ADR index checks passed.${NC}"
    exit 0
else
    echo -e "${RED}$ERRORS check(s) failed.${NC}"
    exit 1
fi
