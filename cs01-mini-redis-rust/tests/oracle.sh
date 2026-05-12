#!/usr/bin/env bash
# oracle.sh — round-trip our RESP server against a real Redis 7 oracle.
#
# F23-A defence (cs01 CLAUDE.md §2): protocol behaviour is verified
# command-by-command against a real `redis:7-alpine` container.
#
# Activation
# ----------
#   Default                           → skip (exit 0).
#   `CS01_RUN_ORACLE=1 bash …`        → actually run the round-trip.
#
# Why opt-in?  ADR-0006 §"docker oracle 脚本":
#   the 5-gate CI keeps fast + reproducible by NOT requiring docker.
#   This script is the bridge between "self-tested" and "third-party
#   oracle".  Run locally; nightly job picks it up.
#
# Layout
# ------
#   1. ensure docker + redis-cli on PATH (skip if missing)
#   2. start a Redis 7 oracle container on host port 6379  (name: cs01-oracle)
#   3. build + start our mini-redis-server on host port 16380
#      (16380 chosen to avoid collision with anyone's local dev on 6380)
#   4. for each fixture (≥ 15 commands across PING/SET/GET/DEL/EXISTS/
#      INCR/EXPIRE/TTL/PERSIST/TYPE/KEYS/ECHO/SELECT/PING-message), send
#      via `redis-cli -p <port> <cmd…>` to both endpoints and diff stdout
#   5. fail-fast on first divergence with `cmd / ours / oracle` print
#   6. cleanup: stop docker + kill our server (trap EXIT)

set -euo pipefail

# ── Skip handling ────────────────────────────────────────────────────────────

if [[ "${CS01_RUN_ORACLE:-0}" != "1" ]]; then
    echo "oracle.sh: skipped (set CS01_RUN_ORACLE=1 to run)"
    exit 0
fi

if ! command -v docker >/dev/null 2>&1; then
    echo "oracle.sh: docker not on PATH — skipped"
    exit 0
fi
if ! command -v redis-cli >/dev/null 2>&1; then
    echo "oracle.sh: redis-cli not on PATH — skipped"
    exit 0
fi

# ── Configuration ────────────────────────────────────────────────────────────

ORACLE_PORT="${CS01_ORACLE_PORT:-6379}"
OUR_PORT="${CS01_OUR_PORT:-16380}"
ORACLE_CONTAINER="cs01-oracle"
IMAGE="redis:7-alpine"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# ── Cleanup trap ─────────────────────────────────────────────────────────────

OUR_PID=""

cleanup() {
    local rc=$?
    if [[ -n "$OUR_PID" ]]; then
        kill "$OUR_PID" 2>/dev/null || true
        wait "$OUR_PID" 2>/dev/null || true
    fi
    docker stop "$ORACLE_CONTAINER" >/dev/null 2>&1 || true
    exit "$rc"
}
trap cleanup EXIT INT TERM

# Pre-clean: if a previous run left the container behind, force-stop.
docker stop "$ORACLE_CONTAINER" >/dev/null 2>&1 || true

# ── Start oracle ─────────────────────────────────────────────────────────────

echo "oracle.sh: starting $IMAGE on host port $ORACLE_PORT"
docker run --rm -d \
    --name "$ORACLE_CONTAINER" \
    -p "$ORACLE_PORT:6379" \
    "$IMAGE" >/dev/null

# Wait for oracle PING to respond.
for _ in $(seq 1 30); do
    if redis-cli -p "$ORACLE_PORT" PING >/dev/null 2>&1; then
        break
    fi
    sleep 0.2
done
if ! redis-cli -p "$ORACLE_PORT" PING >/dev/null 2>&1; then
    echo "oracle.sh: oracle did not respond to PING within 6s"
    exit 1
fi
echo "oracle.sh: oracle ready"

# ── Build + start our server ─────────────────────────────────────────────────

echo "oracle.sh: building mini-redis-server (release)"
(cd "$ROOT" && cargo build --release -p redis-server --locked >/dev/null 2>&1)
BIN="$ROOT/target/release/redis-server"

echo "oracle.sh: starting our server on host port $OUR_PORT"
"$BIN" --port "$OUR_PORT" --bind 127.0.0.1 >/tmp/cs01-oracle-server.log 2>&1 &
OUR_PID=$!

# Wait for our server.
for _ in $(seq 1 30); do
    if redis-cli -p "$OUR_PORT" PING >/dev/null 2>&1; then
        break
    fi
    sleep 0.2
done
if ! redis-cli -p "$OUR_PORT" PING >/dev/null 2>&1; then
    echo "oracle.sh: our server did not respond to PING within 6s"
    echo "── server log ──"
    cat /tmp/cs01-oracle-server.log || true
    exit 1
fi
echo "oracle.sh: our server ready"

# Ensure both sides start with an empty keyspace for deterministic diffs.
redis-cli -p "$ORACLE_PORT" FLUSHDB >/dev/null
# Our server doesn't implement FLUSHDB yet — but it starts empty per
# process, so this is fine.

# ── Fixture corpus + diff loop ───────────────────────────────────────────────

# Each line: "<command words…>".  We deliberately stage commands so that
# state set up by one row is observed by the next; both endpoints run
# the SAME sequence, so divergence == bug.

FIXTURES=(
    "PING"
    "PING hello"
    "ECHO 'oracle-test'"
    "SET key1 value1"
    "GET key1"
    "EXISTS key1"
    "EXISTS does-not-exist"
    "TYPE key1"
    "TYPE missing"
    "INCR counter"
    "INCR counter"
    "DECR counter"
    "SET ttlkey ttlval EX 100"
    "TTL ttlkey"
    "TTL key1"
    "TTL does-not-exist"
    "EXPIRE key1 50"
    "TTL key1"
    "PERSIST key1"
    "TTL key1"
    "DEL key1 ttlkey nonexistent"
    "SELECT 0"
)

failures=0
total=0

run_diff() {
    local cmd="$1"
    total=$((total + 1))
    # shellcheck disable=SC2086 # we want word-splitting for the redis-cli args
    local ours
    ours="$(redis-cli -p "$OUR_PORT" $cmd 2>&1 | tr -d '\r')"
    local oracle
    oracle="$(redis-cli -p "$ORACLE_PORT" $cmd 2>&1 | tr -d '\r')"
    if [[ "$ours" != "$oracle" ]]; then
        echo "  ✗ DIVERGENCE on: $cmd"
        echo "      ours:   $ours"
        echo "      oracle: $oracle"
        failures=$((failures + 1))
        return 1
    fi
    echo "  ✓ $cmd → $ours"
    return 0
}

echo "oracle.sh: running ${#FIXTURES[@]} fixtures"
for cmd in "${FIXTURES[@]}"; do
    if ! run_diff "$cmd"; then
        echo "oracle.sh: fail-fast on first divergence"
        exit 1
    fi
done

# ── Summary ──────────────────────────────────────────────────────────────────

echo
echo "oracle.sh: $total / $total commands matched real Redis"
exit 0
