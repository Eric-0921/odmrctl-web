#!/usr/bin/env bash
set -euo pipefail

# discover-macos.sh — macOS hardware discovery wrapper for ODMR lab bring-up.
#
# This script lists local network interfaces and serial ports, then invokes
# the Rust discovery tool (tools/discover) if available, or falls back to
# raw shell commands.
#
# Safety: this script only sends read-only queries (*IDN?, SYST:ERR?, OUTP?, MOD:STAT?).
# No RF output, modulation, or sweep commands are present.
#
# Usage: bash scripts/lab/discover-macos.sh

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

echo "=== ODMR Lab Hardware Discovery (macOS) ==="
echo ""

echo "--- Network Interfaces ---"
ifconfig || true
echo ""

echo "--- Hardware Ports ---"
networksetup -listallhardwareports || true
echo ""

echo "--- ARP Table ---"
arp -a || true
echo ""

echo "--- Serial Ports (/dev/cu.*) ---"
ls /dev/cu.* 2>/dev/null || true
echo ""

echo "--- Serial Ports (/dev/tty.*) ---"
ls /dev/tty.* 2>/dev/null || true
echo ""

# Prefer the Rust tool if it has been built
DISCOVER_BIN="${REPO_ROOT}/tools/discover/target/debug/odmr-discover"
if [ -x "$DISCOVER_BIN" ]; then
    echo "--- Probing with Rust discovery tool ---"
    "$DISCOVER_BIN" report --output "${REPO_ROOT}/docs/lab-bringup/discovery_report.md"
    echo "Report written to docs/lab-bringup/discovery_report.md"
else
    echo "--- Rust discovery tool not built ---"
    echo "Run: cd tools/discover && cargo build"
    echo ""
    echo "--- Manual probe helpers ---"
    echo "SMB100A (example):  echo '*IDN?' | nc -w 2 169.254.2.20 5025"
    echo "OE1022D (example):  stty -f /dev/cu.usbmodemXXXX speed 115200 && echo '*IDN?' > /dev/cu.usbmodemXXXX"
fi

echo ""
echo "=== Discovery complete ==="
