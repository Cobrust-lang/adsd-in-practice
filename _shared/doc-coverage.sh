#!/usr/bin/env bash
# doc-coverage.sh — 双语 + ADR 完整性强制脚本
#
# 三个核心检查:
#   1. 每个 docs/agent/adr/NNNN-*.md 在 docs/human/zh/ 和 docs/human/en/ 都有对应说明
#   2. 每个 ADR / finding 有 frontmatter 必填字段
#   3. README.md / CLAUDE.md 存在且不为空
#
# 退出码:0 = 通过,非 0 = 失败
set -e
set -o pipefail

errors=0

# Helper
fail() { echo "  ✗ $1"; errors=$((errors + 1)); }
ok() { echo "  ✓ $1"; }

echo "── doc-coverage ──"

# 1. README + CLAUDE.md 存在且非空
for f in README.md CLAUDE.md; do
    if [ ! -f "$f" ] || [ ! -s "$f" ]; then
        fail "$f 缺失或为空"
    fi
done

# 2. 双语目录存在
for d in docs/human/zh docs/human/en docs/agent/adr docs/agent/findings; do
    if [ ! -d "$d" ]; then
        fail "$d 目录缺失"
    fi
done

# 3. 每个 ADR 有 frontmatter 必填字段
for adr in docs/agent/adr/[0-9]*.md; do
    [ -f "$adr" ] || continue
    [ "$(basename "$adr")" = "README.md" ] && continue
    for field in adr title status date case; do
        if ! grep -q "^$field:" "$adr"; then
            fail "$adr 缺 frontmatter '$field'"
        fi
    done
done

# 4. 每个 finding 有 frontmatter 必填字段
for f in docs/agent/findings/*.md; do
    [ -f "$f" ] || continue
    [ "$(basename "$f")" = "README.md" ] && continue
    for field in finding date case severity; do
        if ! grep -q "^$field:" "$f"; then
            fail "$f 缺 frontmatter '$field'"
        fi
    done
done

# 5. 每个 ADR 在 docs/human/zh/ 和 docs/human/en/ 都被引用
#    (引用 = 文件名出现在 human/{zh,en}/ 下任意 .md)
for adr in docs/agent/adr/[0-9]*.md; do
    [ -f "$adr" ] || continue
    name=$(basename "$adr" .md)
    for lang in zh en; do
        if ! grep -rq "$name" "docs/human/$lang/" 2>/dev/null; then
            fail "$adr 未在 docs/human/$lang/ 引用"
        fi
    done
done

# 总结
echo
if [ "$errors" -eq 0 ]; then
    ok "doc-coverage all green"
    exit 0
else
    echo "  ✗ doc-coverage failed: $errors error(s)"
    exit 1
fi
