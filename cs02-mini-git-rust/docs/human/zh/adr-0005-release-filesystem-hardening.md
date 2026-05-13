# ADR-0005 中文摘要:M4 release filesystem hardening 与文档诚实

> 完整 ADR 见 [docs/agent/adr/0005-release-filesystem-hardening.md](../../agent/adr/0005-release-filesystem-hardening.md)。

## 决策

M4 作为 release-hardening sprint,不扩展新的 Git 功能,只加固 v0.1 已支持范围的本地文件系统与解析边界:

- loose object、index、ref 写入改为同目录 temp-then-rename,并在可行处拒绝覆盖 final-path symlink;
- index 更新增加最小 `.mg/index.lock`,避免并发 `mg add` / `mg commit` 静默交错写入;
- `mg add` 拒绝 staging `.mg/**` 与 `.git/**` 仓库内部路径;
- index reader 在分配 entry vector 前,先用文件长度约束 entry count;
- loose object zlib inflate 增加 decoded-size cap;
- index reader 拒绝 v0.1 不支持的 flags;
- SHA-1 public validation 只接受 lowercase 40 hex;
- README 与 human docs 改成声明 "v0.1 supported subset is Git-compatible",移除未实现的 `ls-files` 声明。

## 为什么

cs02 的核心域是 filesystem-backed Git state。M1-M3 已经通过功能 oracle,但 pre-release audit 发现写入原子性、symlink、并发、bounded parsing、仓库内部路径和文档过度声明问题。如果不先关闭这些问题就声明 `0.1.0` ready,会把 HIGH/MED audit evidence 变成 release debt。

## 状态

`accepted` — 2026-05-13。
