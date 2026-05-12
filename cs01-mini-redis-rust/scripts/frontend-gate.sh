#!/usr/bin/env bash
# Wave M2.2 frontend gate (ADR-0008). Pinned to the case-local `web/`
# project so it never picks up an ambient pnpm workspace.
#
# Runs: install (frozen-lockfile) → svelte-check + tsc → vitest → build.
# Any non-zero exit fails the whole gate.

set -euo pipefail

# Resolve `web/` relative to this script (works regardless of CWD).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEB_DIR="${SCRIPT_DIR}/../web"

cd "${WEB_DIR}"

echo ">> pnpm install --frozen-lockfile"
pnpm install --frozen-lockfile

echo ">> pnpm check (svelte-check + tsc)"
pnpm check

echo ">> pnpm test (vitest --run)"
pnpm test -- --run

echo ">> pnpm build (adapter-static)"
pnpm build

if [[ ! -f build/index.html ]]; then
    echo "frontend-gate: build/index.html missing after pnpm build" >&2
    exit 1
fi

echo "frontend-gate all green"
