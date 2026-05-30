#!/usr/bin/env bash
set -euo pipefail

# verify-smb100a.sh — Interactive human-in-the-loop command verification for SMB100A.
#
# Safety architecture:
#   1. Query phase: auto-send read-only queries, record responses
#   2. Set phase: display command + context, ask "Execute? (y/n)"
#   3. Forbidden gate: any command matching the forbidden list is rejected
#
# Usage:
#   bash scripts/lab/verify-smb100a.sh [HOST] [PORT]
#   Default: 169.254.2.20 5025

HOST="${1:-169.254.2.20}"
PORT="${2:-5025}"
TIMEOUT=2

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUTPUT="${REPO_ROOT}/examples/verification/smb100a_observed_responses.jsonl"

# ---------------------------------------------------------------------------
# Forbidden command gate
# ---------------------------------------------------------------------------

FORBIDDEN_PATTERNS=(
    "OUTP ON"
    "MOD:STAT ON"
    "FM:STAT ON"
    "FREQ:MODE SWE"
    "SWE:EXEC"
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

    local response=""
    response=$(printf '%s\n' "$cmd" | nc -w "$TIMEOUT" "$HOST" "$PORT" 2>/dev/null || true)
    response=$(echo "$response" | tr -d '\r' | awk 'NF || printed {printed=1; print}' | head -c 500)

    local pass="null"
    if [ -n "$response" ]; then
        pass="\"pending\""
    else
        pass="\"timeout\""
    fi

    local json
    json=$(cat <<EOF
{"timestamp":"$ts","device":"smb100a.main","transport":"tcp_scpi:$HOST:$PORT","phase":"$phase","command":"$cmd","expected_response":"$expected","observed_response":$(jq -Rs . <<< "$response" | sed 's/^"//;s/"$//'),"pass_fail":$pass,"human_approved":$approved,"human_notes":$(jq -Rs . <<< "$notes")}
EOF
)
    echo "$json" >> "$OUTPUT"
    echo "  → Recorded to $OUTPUT"
}

# ---------------------------------------------------------------------------
# Query phase (auto)
# ---------------------------------------------------------------------------

run_query_phase() {
    echo "=== SMB100A Phase 1: Safe Query Phase ==="
    echo ""

    local queries=(
        "*IDN?|Rohde&Schwarz,SMB100A,..."
        "SYST:ERR?|0,\"No error\""
        "OUTP?|0"
        "MOD:STAT?|0"
        "FREQ?|~2.882E9"
        "POW?|~-15"
        "POW:ALC?|AUTO"
        "FM:STAT?|0"
        "FM:SOUR?|INT or EXT"
        "FM:DEV?|~4E6"
        "LFO?|1 or 0"
        "LFO:FREQ?|~500"
        "LFO:VOLT?|~0.137"
        "LFO:SHAP?|SQUARE"
        "FREQ:MODE?|CW"
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
    echo "=== SMB100A Phase 2: Safe Set Phase ==="
    echo ""
    echo "⚠️  WARNING: The following commands will change device state."
    echo "    Ensure RF output is OFF or safely terminated before proceeding."
    echo ""
    read -r -p "Press Enter to begin set phase (or Ctrl-C to abort)..."
    echo ""

    local sets=(
        "OUTP OFF|Ensure RF output is OFF"
        "MOD:STAT OFF|Turn off modulation global state"
        "FREQ:MODE CW|Set fixed-frequency CW mode"
        "FREQ 2.882GHz|Set RF frequency to 2.882 GHz"
        "POW -15dBm|Set RF power to -15 dBm"
        "POW:ALC AUTO|Set ALC to auto mode"
        "FM:STAT OFF|Turn off FM modulation"
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
{"timestamp":"$ts","device":"smb100a.main","transport":"tcp_scpi:$HOST:$PORT","phase":"safe_set","command":"$cmd","expected_response":"ACK","observed_response":null,"pass_fail":"skipped","human_approved":false,"human_notes":"skipped by operator"}
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

echo "=== SMB100A Human-in-the-Loop Command Verification ==="
echo "Target: $HOST:$PORT"
echo "Output: $OUTPUT"
echo ""

# Backup existing output
if [ -f "$OUTPUT" ]; then
    cp "$OUTPUT" "${OUTPUT}.bak.$(date +%s)"
    echo "Existing output backed up."
fi

run_query_phase
run_set_phase

echo "=== Verification Complete ==="
echo "Results appended to: $OUTPUT"
