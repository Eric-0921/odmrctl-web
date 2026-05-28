#!/usr/bin/env bash
set -euo pipefail

# check-consistency.sh — 守卫 odmrctl-web 仓库结构漂移
# 用法: bash scripts/check-consistency.sh (从仓库根目录执行)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ERRORS=0
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

pass() { echo -e "  ${GREEN}PASS${NC} $1"; }
fail() { echo -e "  ${RED}FAIL${NC} $1"; ERRORS=$((ERRORS + 1)); }

echo "=== odmrctl-web consistency check ==="
echo ""

# --- C1: crates/ 目录与 Cargo.toml workspace members 一致 ---
echo "C1: crate directories match Cargo.toml workspace members"
TOML_CRATES=$(grep -E '^\s*"crates/' Cargo.toml 2>/dev/null | sed 's/.*"crates\/\([^"]*\)".*/\1/' | sort || true)
DIR_CRATES=$(find crates -mindepth 1 -maxdepth 1 -type d ! -name '.gitkeep' -exec basename {} \; | sort)

if [ -z "$TOML_CRATES" ] && [ -z "$DIR_CRATES" ]; then
    pass "Cargo.toml and crates/ both empty (pre-workspace phase)"
elif [ "$TOML_CRATES" = "$DIR_CRATES" ]; then
    pass "workspace members match crate directories"
else
    diff <(echo "$TOML_CRATES") <(echo "$DIR_CRATES") 2>/dev/null || true
    fail "C1: crate directories != Cargo.toml members. Sync them."
fi

# --- C2: PRD 编号连续 ---
echo "C2: PRD files sequentially numbered"
PRD_FILES=$(find docs/prd -maxdepth 1 -name '[0-9]*_prd_*.md' -exec basename {} \; | sort)
PRD_NUMS=$(echo "$PRD_FILES" | sed 's/^0*//;s/_.*//;s/^$/0/' | sort -n)
C2_FAILED=0
expected=0
for n in $PRD_NUMS; do
    if [ -z "$n" ]; then n=0; fi
    if [ "$n" -ne "$((expected))" ]; then
        fail "C2: expected PRD-$expected but found PRD-$n (gap or duplicate)"
        C2_FAILED=1
    fi
    expected=$((expected + 1))
done
if [ "$C2_FAILED" -eq 0 ]; then
    pass "PRD numbering is contiguous (0..$((expected - 1)))"
fi

# --- C3: PRD count in README matches actual ---
echo "C3: PRD count in README matches docs/prd/"
ACTUAL_PRD=$(find docs/prd -maxdepth 1 -name '[0-9]*_prd_*.md' | wc -l | tr -d ' ')
README_PRD=$(perl -ne 'if (/prd.*?(\d+)/i) { print $1; exit }' README.md || echo "0")
if [ "$ACTUAL_PRD" -eq "${README_PRD:-0}" ] 2>/dev/null; then
    pass "PRD count: $ACTUAL_PRD (README matches)"
else
    fail "C3: README says ~${README_PRD:-?} PRDs, actual=$ACTUAL_PRD"
fi

# --- C4: ADR count in README matches actual ---
echo "C4: ADR count in README matches docs/adr/"
ACTUAL_ADR=$(find docs/adr -maxdepth 1 -name 'ADR-*.md' | wc -l | tr -d ' ')
README_ADR=$(perl -ne 'if (/adr.*?(\d+)/i) { print $1; exit }' README.md || echo "0")
if [ "$ACTUAL_ADR" -eq "${README_ADR:-0}" ] 2>/dev/null; then
    pass "ADR count: $ACTUAL_ADR (README matches)"
else
    fail "C4: README says ~${README_ADR:-?} ADRs, actual=$ACTUAL_ADR"
fi

# --- C5: Each crate directory has a README.md ---
echo "C5: every crate has a README.md"
C5_FAILED=0
for d in crates/*/; do
    name=$(basename "$d")
    if [ "$name" = ".gitkeep" ]; then continue; fi
    if [ ! -f "$d/README.md" ]; then
        fail "C5: crates/$name/ missing README.md"
        C5_FAILED=1
    fi
done
if [ "$C5_FAILED" -eq 0 ]; then
    pass "all crate directories have README.md"
fi

# --- Summary ---
echo ""
if [ "$ERRORS" -eq 0 ]; then
    echo -e "${GREEN}All consistency checks passed.${NC}"
    exit 0
else
    echo -e "${RED}$ERRORS check(s) failed.${NC}"
    exit 1
fi
