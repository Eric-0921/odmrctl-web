#!/usr/bin/env bash
set -euo pipefail

# check-docs-links.sh — verify that internal markdown links point to existing files.
# Usage: bash scripts/check-docs-links.sh (run from repo root)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ERRORS=0
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

pass() { echo -e "  ${GREEN}PASS${NC} $1"; }
fail() { echo -e "  ${RED}FAIL${NC} $1"; ERRORS=$((ERRORS + 1)); }

echo "=== docs link check ==="
echo ""

BROKEN_FILE=$(mktemp)
trap 'rm -f "$BROKEN_FILE"' EXIT

# Find all markdown files under docs/ and check their internal links.
while IFS= read -r mdfile; do
    dir=$(dirname "$mdfile")
    # Extract link targets from markdown [text](path) syntax
    (
        grep -oE '\[([^]]+)\]\(([^)]+)\)' "$mdfile" 2>/dev/null || true
    ) | (
        grep -oE '\]\([^)]+\)' || true
    ) | (
        sed 's/^](//;s/)$//' || true
    ) | while read -r link; do
        # Skip external URLs and anchors-only
        if [[ "$link" == http* ]] || [[ "$link" == "#"* ]]; then
            continue
        fi
        # Remove anchor fragment
        target="${link%%#*}"
        if [ -z "$target" ]; then
            continue
        fi
        # Resolve relative to the markdown file's directory
        if [ -f "$dir/$target" ]; then
            : # exists
        elif [ -f "$target" ]; then
            : # exists from repo root
        else
            echo "broken link in $mdfile -> '$link' (resolved as '$dir/$target')" >> "$BROKEN_FILE"
        fi
    done
done < <(find docs -name '*.md' -type f)

if [ -s "$BROKEN_FILE" ]; then
    while IFS= read -r line; do
        fail "$line"
    done < "$BROKEN_FILE"
fi

echo ""
if [ "$ERRORS" -eq 0 ]; then
    echo -e "${GREEN}All docs links OK.${NC}"
    exit 0
else
    echo -e "${RED}$ERRORS broken link(s) found.${NC}"
    exit 1
fi
