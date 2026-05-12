#!/usr/bin/env bash
set -e
set -o pipefail
cd "$(dirname "$0")/.."

echo "── cs05 bootstrap ──"

need() { command -v "$1" >/dev/null 2>&1 || { echo "  缺工具:$1"; missing=1; }; }
missing=0
need cmake
need clang-format
need clang-tidy
need cppcheck

if [ "$missing" = "1" ]; then
    echo
    echo "请先装齐工具(macOS):"
    echo "  brew install cmake llvm cppcheck"
    exit 1
fi
echo "  ✓ cmake $(cmake --version | head -1 | awk '{print $3}')"
echo "  ✓ clang-format / clang-tidy / cppcheck"

# native build + test
echo "── native build (Release) ──"
cmake -B build-native -S core -DCMAKE_BUILD_TYPE=Release -DCMAKE_EXPORT_COMPILE_COMMANDS=ON > /dev/null
cmake --build build-native --parallel
ctest --test-dir build-native --output-on-failure --parallel

# wasm build(需要 emsdk;如果没装就跳过并提示)
if command -v emcmake >/dev/null 2>&1; then
    echo "── wasm build ──"
    bash scripts/build-wasm.sh
else
    echo "  emsdk 未装,跳过 wasm build。"
    echo "  装方法:git clone https://github.com/emscripten-core/emsdk && cd emsdk && ./emsdk install latest && ./emsdk activate latest"
fi

# web deps(可选)
if [ -f web/package.json ] && command -v pnpm >/dev/null 2>&1; then
    (cd web && pnpm install)
fi

echo
echo "✓ cs05 bootstrap done"
echo
echo "下一步:"
echo "  cd web && pnpm dev"
echo "  open http://localhost:5173"
