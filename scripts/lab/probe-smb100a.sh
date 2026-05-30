#!/usr/bin/env bash
set -euo pipefail

# probe-smb100a.sh — Read-only SMB100A probe over TCP port 5025.
#
# Safety: only sends *IDN?, SYST:ERR?, OUTP?, MOD:STAT?.
# No state-changing commands are present in this script.
#
# Usage: bash scripts/lab/probe-smb100a.sh [IP] [PORT]
#   Default IP:   169.254.2.20
#   Default PORT: 5025

HOST="${1:-169.254.2.20}"
PORT="${2:-5025}"
TIMEOUT=2

echo "=== SMB100A Probe ==="
echo "Target: ${HOST}:${PORT}"
echo ""

probe_query() {
    local query="$1"
    echo "Sending: ${query}"
    # Use printf to ensure newline; nc with timeout
    response=$(printf '%s\n' "$query" | nc -w "$TIMEOUT" "$HOST" "$PORT" 2>/dev/null || true)
    if [ -n "$response" ]; then
        echo "Response: ${response}"
    else
        echo "Response: (timeout or no response)"
    fi
    echo ""
}

probe_query "*IDN?"
probe_query "SYST:ERR?"
probe_query "OUTP?"
probe_query "MOD:STAT?"

echo "=== SMB100A Probe complete ==="
