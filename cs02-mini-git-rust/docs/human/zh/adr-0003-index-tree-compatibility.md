# ADR-0003 中文摘要:Index v2 与 canonical tree 兼容

> 完整 ADR 见 [docs/agent/adr/0003-index-tree-compatibility.md](../../agent/adr/0003-index-tree-compatibility.md)。

## 决策

M2 直接实现 Git-compatible 的 staging + tree 边界:

- `.mg/index` 使用 Git index v2 binary format,包含 `DIRC` header、entry metadata、路径、8-byte padding 和 trailing SHA-1 checksum。
- `mg add <path>` 写 blob loose object,并把 regular file stage 到 index。
- `mg write-tree` 从 index 生成 canonical tree payload:`<mode> <name>\0<raw 20-byte object id>`。
- M2 先接受 flat regular files 作为最小切片;递归目录可以后续扩展,但 oracle 必须诚实标注边界。

## 为什么

Git 的 index 和 tree 都是 binary 兼容边界。用 JSON/text/sqlite 临时格式会让 M2 变成自己测自己,后面 commit/log 再返工。直接实现 index v2 子集,可以从 M2 开始用 `git ls-files --stage`、`git write-tree`、`git cat-file -p` 验证。

## 状态

`accepted` — 2026-05-13。
