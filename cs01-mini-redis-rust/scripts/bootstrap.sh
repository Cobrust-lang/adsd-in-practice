#!/usr/bin/env bash
# bootstrap.sh — 一键验证 cs01-mini-redis-rust 开发环境
set -e
set -o pipefail

cd "$(dirname "$0")/.."

echo "── cs01 bootstrap ──"

need() {
    command -v "$1" >/dev/null 2>&1 || { echo "缺工具:$1"; return 1; }
}

warn_missing() {
    if command -v "$1" >/dev/null 2>&1; then
        echo "  ok $1 $($1 --version 2>/dev/null | head -n 1)"
    else
        echo "  warn optional tool missing:$1 ($2)"
    fi
}

# 1. 验证后端工具链
need cargo
need rustc
echo "  ok cargo $(cargo --version | awk '{print $2}'), rustc $(rustc --version | awk '{print $2}')"

# 2. 前端工具软检查(M2.2+)
warn_missing node "needed for web/ frontend gate"
warn_missing pnpm "needed for web/ frontend gate"

# 3. 拉依赖
echo "── cargo fetch ──"
cargo fetch --locked || cargo fetch

# 4. 第一次 build(debug 即可)
echo "── cargo build ──"
cargo build --workspace --all-targets

# 5. 跑后端 smoke tests
echo "── cargo test(smoke) ──"
cargo test --workspace --lib --quiet

# 6. 提示
echo
echo "cs01 bootstrap done"
echo
echo "Run the RESP server:"
echo "  cargo run -p redis-server -- --port 6380"
echo
echo "Optional AOF persistence:"
echo "  mkdir -p data"
echo "  cargo run -p redis-server -- --port 6380 --aof data/dump.aof"
echo
echo "Browser dev UI (optional, requires node + pnpm):"
echo "  cargo run -p redis-server -- --port 6380 --http-port 6381"
echo "  (another terminal) cd web && pnpm install && pnpm dev"
echo
echo "Gates:"
echo "  cargo fmt --all -- --check"
echo "  cargo clippy --workspace --all-targets --locked -- -D warnings"
echo "  cargo build --workspace --all-targets --locked"
echo "  cargo test --workspace --locked"
echo "  bash ../_shared/doc-coverage.sh"
echo "  bash scripts/frontend-gate.sh  # if node/pnpm are available"
