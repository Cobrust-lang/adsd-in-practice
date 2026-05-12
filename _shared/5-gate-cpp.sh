#!/usr/bin/env bash
# 5-gate-cpp.sh — ADSD 五道闸门(C++ 适配)
#
# 工具栈:clang-format + clang-tidy + cmake build + ctest + cppcheck + doc-coverage
# 要求 C++17+ 工具链。
set -e
set -o pipefail

cd "$(pwd)"

echo "════════════════════════════════════════"
echo "  ADSD 5-gate (C++) for $(basename "$PWD")"
echo "════════════════════════════════════════"

BUILD_DIR=${BUILD_DIR:-build}

# Gate 1: format
echo
echo "▶ Gate 1/5: clang-format --dry-run"
find {include,src,tests,bench,core} -type f \( -name "*.cpp" -o -name "*.hpp" -o -name "*.h" -o -name "*.cc" \) 2>/dev/null \
    | xargs -r clang-format --dry-run --Werror -style=file
echo "  ✓ format clean"

# Gate 2: lint(clang-tidy + cppcheck 合并)
echo
echo "▶ Gate 2/5: clang-tidy + cppcheck"
if [ ! -f "$BUILD_DIR/compile_commands.json" ]; then
    cmake -B "$BUILD_DIR" -S . -DCMAKE_EXPORT_COMPILE_COMMANDS=ON > /dev/null
fi
find {include,src} -name "*.cpp" 2>/dev/null \
    | xargs -r clang-tidy -p "$BUILD_DIR" --warnings-as-errors='*'
cppcheck --enable=warning,style,performance --error-exitcode=1 --quiet \
    --suppress=missingIncludeSystem --suppress=unusedFunction \
    -I include {src,include} 2>&1
echo "  ✓ lint clean"

# Gate 3: build
echo
echo "▶ Gate 3/5: cmake build"
cmake --build "$BUILD_DIR" --parallel
echo "  ✓ build clean"

# Gate 4: test
echo
echo "▶ Gate 4/5: ctest"
ctest --test-dir "$BUILD_DIR" --output-on-failure --parallel
echo "  ✓ tests pass"

# Gate 5: doc-coverage
echo
echo "▶ Gate 5/5: doc-coverage"
if [ -f scripts/doc-coverage.sh ]; then
    bash scripts/doc-coverage.sh
elif [ -f ../_shared/doc-coverage.sh ]; then
    bash ../_shared/doc-coverage.sh
else
    echo "  WARN: doc-coverage.sh 不存在"
    exit 1
fi
echo "  ✓ doc-coverage clean"

echo
echo "════════════════════════════════════════"
echo "  ✓ All 5 gates green"
echo "════════════════════════════════════════"
