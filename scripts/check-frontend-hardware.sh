#!/usr/bin/env bash
set -euo pipefail

# check-frontend-hardware.sh — forbid frontend TypeScript/React code from accessing hardware directly.
#
# Rules (M5C-A updated):
# - apps/desktop/src (frontend TS/TSX) must not contain serial, usb, visa, scpi socket, tcp patterns
# - Tauri backend (src-tauri) is ALLOWED to use hardware crates and transport via typed commands
# - Frontend must NEVER import hardware driver crates or open raw sockets/serial
#
# Usage: bash scripts/check-frontend-hardware.sh (run from repo root)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ERRORS=0
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

pass() { echo -e "  ${GREEN}PASS${NC} $1"; }
fail() { echo -e "  ${RED}FAIL${NC} $1"; ERRORS=$((ERRORS + 1)); }

echo "=== frontend hardware access check ==="
echo ""

# Only check frontend TypeScript/React code, NOT the Tauri backend.
# M5C-A allows the Rust backend to use serialport / TCP / hardware crates.
FRONTEND_SRC="apps/desktop/src"

# Patterns that indicate direct hardware access in frontend TS/TSX code
PATTERNS=(
    '\bserial(port|_port)?\b'
    '\busb(device|_device)?\b'
    '\bvisa\b'
    'scpi.*socket|socket.*scpi|SCPI.*SOCKET'
    'TcpStream|tcp_connect|TcpListener'
    'write_serial|read_serial|open_port'
)

for pat in "${PATTERNS[@]}"; do
    matches=$(find "$FRONTEND_SRC" -type f \( -name '*.ts' -o -name '*.tsx' \) \
        -not -path '*/node_modules/*' -not -path '*/.git/*' -not -path '*/dist/*' -not -path '*/target/*' \
        -not -name 'AboutBoundariesPage.tsx' \
        -print0 2>/dev/null | \
        xargs -0 grep -iE "$pat" 2>/dev/null || true)
    if [ -n "$matches" ]; then
        fail "pattern '$pat' found in frontend TS/TSX:\n$matches"
    fi
done

# Check for direct imports of hardware driver crates in frontend source
IMPORT_PATTERNS=(
    'odmr-smb100a'
    'odmr-oe1022d'
    'odmr-device'
    'odmr-preflight'
    'serialport'
)

for pat in "${IMPORT_PATTERNS[@]}"; do
    matches=$(find "$FRONTEND_SRC" -type f \( -name '*.ts' -o -name '*.tsx' \) \
        -not -path '*/node_modules/*' -not -path '*/.git/*' -not -path '*/dist/*' \
        -print0 2>/dev/null | \
        xargs -0 grep -v '^\s*//' | grep -v '^\s*/\*' | grep -F "$pat" 2>/dev/null || true)
    if [ -n "$matches" ]; then
        fail "frontend imports hardware crate '$pat':\n$matches"
    fi
done

if [ "$ERRORS" -eq 0 ]; then
    pass "no forbidden hardware access patterns in frontend TS/TSX"
fi

echo ""
if [ "$ERRORS" -eq 0 ]; then
    echo -e "${GREEN}Frontend hardware access check passed.${NC}"
    exit 0
else
    echo -e "${RED}$ERRORS violation(s) found.${NC}"
    exit 1
fi
