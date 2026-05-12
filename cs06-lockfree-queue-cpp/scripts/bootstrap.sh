#!/usr/bin/env bash
set -e
set -o pipefail
cd "$(dirname "$0")/.."

echo "── cs06 bootstrap ──"

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

# Release build + test
cmake -B build -S . -DCMAKE_BUILD_TYPE=Release -DCMAKE_EXPORT_COMPILE_COMMANDS=ON > /dev/null
cmake --build build --parallel
ctest --test-dir build --output-on-failure --parallel

echo
echo "✓ cs06 bootstrap done (Release)"
echo
echo "TSAN build(每个 PR 前必跑):"
echo "  cmake -B build-tsan -S . -DCMAKE_CXX_FLAGS=\"-fsanitize=thread -g -O1\""
echo "  cmake --build build-tsan && ctest --test-dir build-tsan"
echo
echo "Benchmark build:"
echo "  cmake -B build -S . -DLFQ_BENCH=ON && cmake --build build --target queue_bench"
echo "  ./build/queue_bench"
