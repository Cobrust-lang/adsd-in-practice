#!/usr/bin/env bash
# Build the WebAssembly binding via Emscripten + copy artifacts to web/static/.
set -e
set -o pipefail
cd "$(dirname "$0")/.."

if ! command -v emcmake >/dev/null 2>&1; then
    echo "emsdk not installed. See bootstrap.sh."
    exit 1
fi

mkdir -p build-wasm
emcmake cmake -B build-wasm -S core -DCMAKE_BUILD_TYPE=Release > /dev/null
emmake make -C build-wasm -j

# Copy artifact to web/static/
mkdir -p web/static
cp build-wasm/phys2d.js web/static/
cp build-wasm/phys2d.wasm web/static/
echo "✓ wasm artifact: web/static/phys2d.{js,wasm}"
