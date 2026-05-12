#!/usr/bin/env bash
set -e
set -o pipefail
cd "$(dirname "$0")/.."

echo "── cs03 bootstrap ──"

# 1. 工具链
command -v uv >/dev/null 2>&1 || {
    echo "  uv 未装,尝试 install ..."
    curl -LsSf https://astral.sh/uv/install.sh | sh || {
        echo "  uv 自动装失败,请手动 install: https://docs.astral.sh/uv/"
        exit 1
    }
}
echo "  ✓ uv $(uv --version 2>&1 | awk '{print $2}')"

# 2. Python deps
uv sync --extra dev
echo "  ✓ Python deps installed"

# 3. 前端 deps(可选,M2 后才需要)
if [ -f web/package.json ]; then
    command -v pnpm >/dev/null || { echo "  pnpm 未装,跳过前端(M0 暂不需要)"; }
    if command -v pnpm >/dev/null; then
        cd web && pnpm install --frozen-lockfile 2>/dev/null || pnpm install
        cd ..
        echo "  ✓ web deps installed"
    fi
fi

# 4. 跑 smoke test
uv run pytest tests/test_smoke.py -q

echo
echo "✓ cs03 bootstrap done"
echo
echo "下一步:"
echo "  export ANTHROPIC_API_KEY=sk-ant-..."
echo "  uv run uvicorn taskboard.main:app --port 8000"
echo "  curl http://localhost:8000/healthz"
