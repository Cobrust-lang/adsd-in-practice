#!/usr/bin/env bash
set -e
set -o pipefail
cd "$(dirname "$0")/.."

echo "── cs04 bootstrap ──"

command -v uv >/dev/null 2>&1 || {
    echo "  uv 未装,尝试 install ..."
    curl -LsSf https://astral.sh/uv/install.sh | sh
}
echo "  ✓ uv $(uv --version 2>&1 | awk '{print $2}')"

uv sync --extra dev
uv run pytest -q

echo
echo "✓ cs04 bootstrap done"
echo
echo "下一步:"
echo "  echo 'x   = 1  ' | uv run pyfmt-mini -"
echo "  uv run pyfmt-mini --check src/"
