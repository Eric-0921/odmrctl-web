#!/usr/bin/env bash
set -euo pipefail

# verify-oe1022d.sh — Interactive human-in-the-loop command verification for OE1022D.
#
# Safety architecture:
#   1. Query phase: auto-send read-only queries, record responses
#   2. Set phase: display command + context, ask "Execute? (y/n)"
#   3. Forbidden gate: any command matching the forbidden list is rejected
#
# Usage:
#   bash scripts/lab/verify-oe1022d.sh [PORT] [BAUD]
#   Default: /dev/cu.usbmodem3361358734371 115200

PORT="${1:-/dev/cu.usbmodem3361358734371}"
BAUD="${2:-115200}"
TIMEOUT_MS=2000

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUTPUT="${REPO_ROOT}/examples/verification/oe1022d_observed_responses.jsonl"

# ---------------------------------------------------------------------------
# Forbidden command gate
# ---------------------------------------------------------------------------

FORBIDDEN_PATTERNS=(
    "SWRMD 2,1"
    "SWRMD 2,2"
    "APHSD 2"
    "*RST"
    "RST"
)

check_forbidden() {
    local cmd="$1"
    for pat in "${FORBIDDEN_PATTERNS[@]}"; do
        if [[ "$cmd" == *"$pat"* ]]; then
            echo "ERROR: Forbidden command detected: '$pat'"
            echo "Command '$cmd' was NOT sent. Aborting."
            exit 1
        fi
    done
}

# ---------------------------------------------------------------------------
# Send and record
# ---------------------------------------------------------------------------

send_and_record() {
    local phase="$1"
    local cmd="$2"
    local expected="$3"
    local approved="${4:-null}"
    local notes="${5:-null}"

    check_forbidden "$cmd"

    local ts
    ts=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    if [ ! -c "$PORT" ]; then
        echo "ERROR: Serial port $PORT is not available."
        exit 1
    fi

    # Configure port
    stty -f "$PORT" "$BAUD" cs8 -cstopb -parenb -echo min 0 time "$(($TIMEOUT_MS / 100))" 2>/dev/null || true

    # Send command
    printf '%s\r' "$cmd" > "$PORT"
    sleep 0.3

    # Read response
    local response=""
    response=$(cat "$PORT" 2>/dev/null || true)
    response=$(echo "$response" | tr -d '\r' | sed '/^$/d' | head -c 500)

    local pass="null"
    if [ -n "$response" ]; then
        pass="\"pending\""
    else
        pass="\"timeout\""
    fi

    local json
    json=$(cat <<EOF
{"timestamp":"$ts","device":"oe1022d.main","transport":"serial:$PORT:$BAUD","phase":"$phase","command":"$cmd","expected_response":"$expected","observed_response":$(jq -Rs . <<< "$response" | sed 's/^"//;s/"$//'),"pass_fail":$pass,"human_approved":$approved,"human_notes":$(jq -Rs . <<< "$notes")}
EOF
)
    echo "$json" >> "$OUTPUT"
    echo "  → Recorded to $OUTPUT"
}

# ---------------------------------------------------------------------------
# Query phase (auto)
# ---------------------------------------------------------------------------

run_query_phase() {
    echo "=== OE1022D Phase 1: Safe Query Phase ==="
    echo "Port: $PORT @ $BAUD baud"
    echo ""

    local queries=(
        "*IDN?|SSI LIA-OE1022D,SN:D6522078,..."
        "FMODD? 2|0, 1, or 2"
        "RSLPD? 2|0 (TTL) or 1 (Sine)"
        "FREQD? 2|reference frequency in Hz"
        "PHASD? 2|phase offset in degrees"
        "ISRCD? 2|0~3"
        "SENSD? 2|sensitivity index 1~27"
        "OFLTD? 2|time constant index 0~19"
        "OFSLD? 2|filter slope index 0~3"
        "HARMD? 2|harmonic number 1~99"
        "RALL?|all display values (multi-field)"
    )

    for item in "${queries[@]}"; do
        local cmd="${item%%|*}"
        local expected="${item##*|}"
        echo "Query: $cmd"
        send_and_record "query" "$cmd" "$expected" "null" "null"
        echo ""
    done
}

# ---------------------------------------------------------------------------
# Set phase (human approval required)
# ---------------------------------------------------------------------------

run_set_phase() {
    echo "=== OE1022D Phase 2: Safe Set Phase ==="
    echo ""
    echo "⚠️  WARNING: The following commands will change device settings."
    echo "    Ensure current settings are backed up or can be restored manually."
    echo ""
    read -r -p "Press Enter to begin set phase (or Ctrl-C to abort)..."
    echo ""

    local sets=(
        "FMODD 2,0|Set Ch-B reference source to External"
        "RSLPD 2,0|Set Ch-B external trigger to TTL Rising Edge"
        "PHASD 2,0|Set Ch-B phase offset to 0 degrees"
        "ISRCD 2,0|Set Ch-B input source to A (single-ended voltage)"
    )

    for item in "${sets[@]}"; do
        local cmd="${item%%|*}"
        local context="${item##*|}"
        echo "Command: $cmd"
        echo "Context: $context"
        read -r -p "Execute? (y/n) " answer
        if [ "$answer" = "y" ] || [ "$answer" = "Y" ]; then
            send_and_record "safe_set" "$cmd" "ACK" "true" "approved by operator"
        else
            echo "  → Skipped by operator"
            local ts
            ts=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
            local json
            json=$(cat <<EOF
{"timestamp":"$ts","device":"oe1022d.main","transport":"serial:$PORT:$BAUD","phase":"safe_set","command":"$cmd","expected_response":"ACK","observed_response":null,"pass_fail":"skipped","human_approved":false,"human_notes":"skipped by operator"}
EOF
)
            echo "$json" >> "$OUTPUT"
        fi
        echo ""
    done
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

echo "=== OE1022D Human-in-the-Loop Command Verification ==="
echo "Port: $PORT @ $BAUD baud"
echo "Output: $OUTPUT"
echo ""

if [ ! -c "$PORT" ]; then
    echo "ERROR: Serial port $PORT is not available."
    echo "Available ports:"
    ls /dev/cu.* 2>/dev/null || true
    exit 1
fi

# Backup existing output
if [ -f "$OUTPUT" ]; then
    cp "$OUTPUT" "${OUTPUT}.bak.$(date +%s)"
    echo "Existing output backed up."
fi

run_query_phase
run_set_phase

echo "=== Verification Complete ==="
echo "Results appended to: $OUTPUT"
