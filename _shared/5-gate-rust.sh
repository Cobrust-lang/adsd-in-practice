#!/usr/bin/env bash
# 5-gate-rust.sh — ADSD 五道闸门(Rust 适配)
#
# 用法:在某个 Rust case 目录下执行
#   bash ../_shared/5-gate-rust.sh
# 或者从 CI 调用。
#
# 5 道闸门必须全过:任何一道失败 = 不准 merge。
set -e
set -o pipefail

cd "$(pwd)"

echo "════════════════════════════════════════"
echo "  ADSD 5-gate (Rust) for $(basename "$PWD")"
echo "════════════════════════════════════════"

# Gate 1: format
echo
echo "▶ Gate 1/5: cargo fmt"
cargo fmt --all -- --check
echo "  ✓ format clean"

# Gate 2: lint
echo
echo "▶ Gate 2/5: cargo clippy"
cargo clippy --workspace --all-targets --locked -- -D warnings
echo "  ✓ clippy clean"

# Gate 3: build
echo
echo "▶ Gate 3/5: cargo build"
cargo build --workspace --all-targets --locked
echo "  ✓ build clean"

# Gate 4: test
echo
echo "▶ Gate 4/5: cargo test"
cargo test --workspace --locked
echo "  ✓ tests pass"

# Gate 5: doc-coverage(双语 + ADR 完整性)
echo
echo "▶ Gate 5/5: doc-coverage"
if [ -f scripts/doc-coverage.sh ]; then
    bash scripts/doc-coverage.sh
elif [ -f ../_shared/doc-coverage.sh ]; then
    bash ../_shared/doc-coverage.sh
else
    echo "  WARN: doc-coverage.sh 不存在,跳过(但不算通过)"
    exit 1
fi
echo "  ✓ doc-coverage clean"

echo
echo "════════════════════════════════════════"
echo "  ✓ All 5 gates green"
echo "════════════════════════════════════════"
