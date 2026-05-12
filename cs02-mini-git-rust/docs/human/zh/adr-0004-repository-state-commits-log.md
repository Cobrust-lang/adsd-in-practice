# ADR-0004 中文摘要:Repository state、commit 与 log

> 完整 ADR 见 [docs/agent/adr/0004-repository-state-commits-log.md](../../agent/adr/0004-repository-state-commits-log.md)。

## 决策

M3 把 cs02 从 object/index/tree 切片推进到最小可用仓库状态机:

- `mg-core::repo` 负责向上发现 `.mg`、初始化目录布局、读取/更新 HEAD 与 refs。
- `mg init` 创建 Git-compatible 的 `.mg/objects`、`.mg/refs/heads`、`.mg/HEAD`、`.mg/config`。
- `mg add <path>` 通过 repo discovery 计算 worktree-relative path,支持 slash-separated regular file paths。
- `mg write-tree` 从 index 写递归 tree object。
- `mg commit-tree` / `mg commit -m` 写 Git-compatible commit object,并在 porcelain commit 时推进 `refs/heads/main`。
- `mg log` 从 HEAD 做 first-parent traversal。

## 为什么

M2 为了让 real Git 读取 index,已经提前写了最小 HEAD/config;M3 必须把它收敛成正式 repository state 语义,不能继续散落在 CLI helper 里。同时 flat-only staging 对 v0.1.0 的 `mg add` / repo discovery 太显眼,应在 M3 关闭而不是拖到 release sweep。

## 状态

`accepted` — 2026-05-13。
