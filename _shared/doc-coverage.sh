#!/usr/bin/env bash
# doc-coverage.sh — 双语 + ADR/finding 完整性强制脚本
#
# 核心检查:
#   1. README.md / CLAUDE.md 存在且不为空
#   2. 每个 docs/agent/adr/NNNN-*.md 有 frontmatter 必填字段
#   3. 每个 docs/agent/findings/*.md 有 frontmatter 必填字段
#   4. 每个 ADR 在 docs/human/zh/ 和 docs/human/en/ 有对应 human doc / 引用
#   5. 每个 finding 在 docs/human/zh/ 和 docs/human/en/ 有 matching abstract:
#      docs/human/{zh,en}/finding-<agent-finding-basename>.md
#
# 退出码:0 = 通过,非 0 = 失败
set -e
set -o pipefail

errors=0

fail() { echo "  ✗ $1"; errors=$((errors + 1)); }
ok() { echo "  ✓ $1"; }

has_glob() {
    local pattern="$1"
    compgen -G "$pattern" >/dev/null
}

check_case() {
    local case_dir="$1"

    echo "── doc-coverage: ${case_dir:-.} ──"

    if [ -n "$case_dir" ]; then
        pushd "$case_dir" >/dev/null
    fi

    for f in README.md CLAUDE.md; do
        if [ ! -f "$f" ] || [ ! -s "$f" ]; then
            fail "$PWD/$f 缺失或为空"
        fi
    done

    for d in docs/human/zh docs/human/en docs/agent/adr; do
        if [ ! -d "$d" ]; then
            fail "$PWD/$d 目录缺失"
        fi
    done

    if [ -d docs/agent/findings ]; then
        :
    else
        ok "$PWD/docs/agent/findings 不存在,跳过 finding 检查"
    fi

    if [ -d docs/agent/adr ]; then
        for adr in docs/agent/adr/[0-9]*.md; do
            [ -f "$adr" ] || continue
            [ "$(basename "$adr")" = "README.md" ] && continue
            for field in adr title status date case; do
                if ! grep -q "^$field:" "$adr"; then
                    fail "$PWD/$adr 缺 frontmatter '$field'"
                fi
            done
        done

        for adr in docs/agent/adr/[0-9]*.md; do
            [ -f "$adr" ] || continue
            local name
            name=$(basename "$adr" .md)
            for lang in zh en; do
                local direct="docs/human/$lang/adr-$name.md"
                if [ -f "$direct" ]; then
                    continue
                fi
                if ! grep -rq "$name" "docs/human/$lang/" 2>/dev/null; then
                    fail "$PWD/$adr 未在 docs/human/$lang/ 引用"
                fi
            done
        done
    fi

    if [ -d docs/agent/findings ]; then
        for finding in docs/agent/findings/*.md; do
            [ -f "$finding" ] || continue
            [ "$(basename "$finding")" = "README.md" ] && continue
            for field in finding date case severity; do
                if ! grep -q "^$field:" "$finding"; then
                    fail "$PWD/$finding 缺 frontmatter '$field'"
                fi
            done
        done

        for finding in docs/agent/findings/*.md; do
            [ -f "$finding" ] || continue
            [ "$(basename "$finding")" = "README.md" ] && continue
            local base
            base=$(basename "$finding" .md)
            for lang in zh en; do
                local mirror="docs/human/$lang/finding-$base.md"
                if [ ! -f "$mirror" ] || [ ! -s "$mirror" ]; then
                    fail "$PWD/$finding 缺 matching docs/human/$lang/finding-$base.md"
                fi
            done
        done
    fi

    if [ -n "$case_dir" ]; then
        popd >/dev/null
    fi
}

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/.." && pwd)

if [ -d docs/agent/adr ] || [ -d docs/agent/findings ]; then
    check_case ""
elif [ -d "$repo_root" ] && has_glob "$repo_root/cs[0-9][0-9]-*"; then
    for case_dir in "$repo_root"/cs[0-9][0-9]-*; do
        [ -d "$case_dir" ] || continue
        if [ -d "$case_dir/docs/agent/adr" ] || [ -d "$case_dir/docs/agent/findings" ]; then
            check_case "$case_dir"
        else
            echo "── doc-coverage: $case_dir ──"
            ok "无 docs/agent/adr 或 docs/agent/findings,跳过"
        fi
    done
else
    check_case ""
fi

echo
if [ "$errors" -eq 0 ]; then
    ok "doc-coverage all green"
    exit 0
else
    echo "  ✗ doc-coverage failed: $errors error(s)"
    exit 1
fi
