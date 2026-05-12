#!/usr/bin/env bash
# bootstrap.sh — 一键启动 cs01-mini-redis-rust 开发环境
set -e
set -o pipefail

cd "$(dirname "$0")/.."

echo "── cs01 bootstrap ──"

# 1. 验证工具链
need() {
    command -v "$1" >/dev/null 2>&1 || { echo "缺工具:$1"; return 1; }
}
need cargo
need rustc
echo "  ✓ cargo $(cargo --version | awk '{print $2}'), rustc $(rustc --version | awk '{print $2}')"

# 2. 拉依赖
echo "── cargo fetch ──"
cargo fetch --locked || cargo fetch

# 3. 第一次 build(release 不必要,debug 即可)
echo "── cargo build ──"
cargo build --workspace --all-targets

# 4. 跑测试确认 scaffold 没坏
echo "── cargo test(scaffold smoke)──"
cargo test --workspace --lib --quiet

# 5. 提示
echo
echo "✓ cs01 bootstrap done"
echo
echo "下一步:"
echo "  cargo run -p redis-server -- --port 6380"
echo "  (M1.0 是 scaffold,会打印一行就退出)"
echo
echo "5-gate:"
echo "  bash ../_shared/5-gate-rust.sh"
