#!/usr/bin/env bash
set -euo pipefail

# check-frontend-hardware.sh — forbid frontend code from accessing hardware directly.
#
# Rules:
# - apps/desktop must not contain serial, usb, visa, scpi socket, tcp patterns
# - frontend must not import hardware crates directly (odmr-smb100a, odmr-oe1022d, odmr-device)
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

FRONTEND_DIR="apps/desktop"

# Patterns that indicate direct hardware access in frontend code
PATTERNS=(
    'serial|SerialPort|serialport'
    'usb|UsbDevice|libusb'
    'visa|Visa|pyvisa|VISA'
    'scpi.*socket|socket.*scpi|SCPI.*SOCKET'
    'TcpStream|tcp_connect|TcpListener'
    'write_serial|read_serial|open_port'
)

for pat in "${PATTERNS[@]}"; do
    matches=$(find "$FRONTEND_DIR" -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.js' -o -name '*.tsx' -o -name '*.json' \) -print0 2>/dev/null | \
        xargs -0 grep -iE "$pat" 2>/dev/null || true)
    if [ -n "$matches" ]; then
        fail "pattern '$pat' found in frontend:\n$matches"
    fi
done

# Check for direct imports of hardware driver crates
IMPORT_PATTERNS=(
    'odmr-smb100a'
    'odmr-oe1022d'
    'odmr-device'
)

for pat in "${IMPORT_PATTERNS[@]}"; do
    matches=$(find "$FRONTEND_DIR" -type f \( -name '*.rs' -o -name 'Cargo.toml' \) -print0 2>/dev/null | \
        xargs -0 grep -F "$pat" 2>/dev/null || true)
    if [ -n "$matches" ]; then
        fail "frontend imports hardware crate '$pat':\n$matches"
    fi
done

if [ "$ERRORS" -eq 0 ]; then
    pass "no forbidden hardware access patterns in frontend"
fi

echo ""
if [ "$ERRORS" -eq 0 ]; then
    echo -e "${GREEN}Frontend hardware access check passed.${NC}"
    exit 0
else
    echo -e "${RED}$ERRORS violation(s) found.${NC}"
    exit 1
fi
