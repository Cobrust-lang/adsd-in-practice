#!/usr/bin/env bash
set -e
set -o pipefail
cd "$(dirname "$0")/.."

echo "── cs02 bootstrap ──"
command -v cargo >/dev/null || { echo "缺 cargo"; exit 1; }
command -v git >/dev/null || { echo "缺 git(oracle 测试需要)"; exit 1; }

echo "  ✓ cargo $(cargo --version | awk '{print $2}')"
echo "  ✓ git   $(git --version | awk '{print $3}')"

cargo fetch --locked || cargo fetch
cargo build --workspace --all-targets
cargo test --workspace --lib --quiet

echo
echo "✓ cs02 bootstrap done"
echo
echo "下一步:"
echo "  cargo install --path crates/mg-cli"
echo "  mg --help"
