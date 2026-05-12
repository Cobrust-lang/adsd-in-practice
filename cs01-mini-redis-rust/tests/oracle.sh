#!/usr/bin/env bash
# oracle.sh — round-trip our RESP server against a real Redis oracle.
#
# **M1.3 status: PLACEHOLDER**.  This script exits 0 without doing any
# real comparison.  ADR-0005 §"Decision" + cs01 CLAUDE.md §2 promise a
# real docker-backed F23-A oracle; ADR-0005 §"Notes" explicitly defers
# the real implementation to M1.4.
#
# When M1.4 wires this up, the script will:
#
#   1. `docker run --rm -d -p 6379:6379 --name redis-oracle redis:7-alpine`
#   2. `cargo run -p redis-server -- --port 6380 &` (background)
#   3. For each command in a fixed corpus (PING / SET / GET / DEL /
#      INCR / EXPIRE / ECHO / SELECT 0 / QUIT), send via `redis-cli`
#      to both ports and diff the responses.
#   4. Tear down both servers; exit non-zero on any divergence.
#
# See cs01-mini-redis-rust/CLAUDE.md §2 for the canonical corpus.

set -euo pipefail

echo "oracle.sh — M1.3 placeholder, real implementation tracked in M1.4"
echo "  (cs01 ADR-0005 §Notes: oracle wiring deferred so M1.3 stays focused on listener)"
exit 0
