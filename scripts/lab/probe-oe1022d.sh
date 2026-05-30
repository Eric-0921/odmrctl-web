#!/usr/bin/env bash
set -euo pipefail

# probe-oe1022d.sh — Read-only OE1022D probe over serial port.
#
# Safety: only sends *IDN?.
# No state-changing commands are present in this script.
#
# Usage: bash scripts/lab/probe-oe1022d.sh [PORT] [BAUD]
#   Default PORT: /dev/cu.usbmodem3361358734371
#   Default BAUD: 115200

PORT="${1:-/dev/cu.usbmodem3361358734371}"
BAUD="${2:-115200}"
TIMEOUT_MS=2000

echo "=== OE1022D Probe ==="
echo "Port: ${PORT} @ ${BAUD} baud"
echo ""

if [ ! -c "$PORT" ]; then
    echo "Error: ${PORT} is not a character device."
    echo "Available ports:"
    ls /dev/cu.* 2>/dev/null || true
    exit 1
fi

# Configure serial port (macOS stty syntax)
stty -f "$PORT" "${BAUD}" cs8 -cstopb -parenb -echo min 0 time "$(($TIMEOUT_MS / 100))" 2>/dev/null || true

echo "Sending: *IDN?"
printf '*IDN?\r' > "$PORT"

# Brief delay to let device respond
sleep 0.5

# Read response (best-effort; may need manual retry)
response=$(cat "$PORT" 2>/dev/null || true)
if [ -n "$response" ]; then
    echo "Response: ${response}"
else
    echo "Response: (timeout or no response)"
    echo "Tip: try running 'screen ${PORT} ${BAUD}' and type *IDN? manually"
fi

echo ""
echo "=== OE1022D Probe complete ==="
