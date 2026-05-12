#!/usr/bin/env bash
# Wave M4.3 lightweight Tauri gate (ADR-0013).
#
# Default mode avoids full bundle builds under low-disk constraints:
#   - verifies source/config files exist
#   - runs pnpm install --frozen-lockfile only when node_modules is missing
#   - runs pnpm check/test/build for the shared SvelteKit UI
#   - runs targeted cargo check for web/src-tauri only
#
# Set CS01_TAURI_FULL_BUILD=1 to opt into `pnpm tauri build --bundles app`
# for release readiness. Record `df -h .` before/after when doing that heavy step.
# The macOS DMG target is intentionally excluded from this gate because its
# Finder AppleScript layout step is environment-dependent; signing/notarization
# and installer packaging remain explicit release tasks.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CASE_DIR="${SCRIPT_DIR}/.."
WEB_DIR="${CASE_DIR}/web"
TAURI_DIR="${WEB_DIR}/src-tauri"

required=(
    "${TAURI_DIR}/Cargo.toml"
    "${TAURI_DIR}/build.rs"
    "${TAURI_DIR}/tauri.conf.json"
    "${TAURI_DIR}/src/main.rs"
)

for path in "${required[@]}"; do
    if [[ ! -f "${path}" ]]; then
        echo "tauri-gate: missing required source file: ${path}" >&2
        exit 1
    fi
done

cd "${WEB_DIR}"

if [[ ! -d node_modules ]]; then
    echo ">> pnpm install --frozen-lockfile (node_modules missing)"
    pnpm install --frozen-lockfile
else
    echo ">> node_modules present; skipping install"
fi

echo ">> pnpm check (shared SvelteKit UI)"
pnpm check

echo ">> pnpm test (vitest --run)"
pnpm test -- --run

echo ">> pnpm build (adapter-static frontendDist)"
pnpm build

if [[ ! -f build/index.html ]]; then
    echo "tauri-gate: build/index.html missing after pnpm build" >&2
    exit 1
fi

echo ">> cargo check --manifest-path src-tauri/Cargo.toml"
cargo check --manifest-path "${TAURI_DIR}/Cargo.toml"

if [[ "${CS01_TAURI_FULL_BUILD:-0}" == "1" ]]; then
    echo ">> CS01_TAURI_FULL_BUILD=1 set; running pnpm tauri build --bundles app"
    pnpm tauri build --bundles app
else
    echo ">> skipping full Tauri bundle (set CS01_TAURI_FULL_BUILD=1 to run)"
fi

echo "tauri-gate all green"
