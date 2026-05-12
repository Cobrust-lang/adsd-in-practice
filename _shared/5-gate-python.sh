#!/usr/bin/env bash
# 5-gate-python.sh — ADSD 五道闸门(Python 适配)
#
# 工具栈:ruff (format + lint) + mypy (type check) + pytest + coverage + doc-coverage
# 要求 Python ≥ 3.11(对齐 cobrust 的 modern toolchain 选择)。
set -e
set -o pipefail

cd "$(pwd)"

echo "════════════════════════════════════════"
echo "  ADSD 5-gate (Python) for $(basename "$PWD")"
echo "════════════════════════════════════════"

# Gate 1: format(ruff format,等价 cargo fmt)
echo
echo "▶ Gate 1/5: ruff format --check"
ruff format --check .
echo "  ✓ format clean"

# Gate 2: lint(ruff + mypy 合并)
echo
echo "▶ Gate 2/5: ruff check + mypy"
ruff check .
mypy --strict src/
echo "  ✓ lint + type clean"

# Gate 3: build(import 通过 = build pass for Python)
echo
echo "▶ Gate 3/5: build / package import"
python -c "import sys; sys.path.insert(0, 'src'); import $(basename "$PWD" | tr '-' '_')" 2>/dev/null || {
    # 退化:试 src/ 下任意 package
    find src -maxdepth 2 -name "__init__.py" | head -1 | xargs dirname | xargs -I {} python -c "import sys; sys.path.insert(0, '$(dirname {})'); import $(basename {})"
}
echo "  ✓ package importable"

# Gate 4: test + coverage
echo
echo "▶ Gate 4/5: pytest --cov(≥80% lines)"
pytest --cov=src --cov-report=term-missing --cov-fail-under=80
echo "  ✓ tests pass + coverage ≥ 80%"

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
