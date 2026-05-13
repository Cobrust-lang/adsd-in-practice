# Finding 中文摘要:M4 pre-release filesystem hardening

> 完整 finding 见 [docs/agent/findings/m4-pre-release-filesystem-hardening.md](../../agent/findings/m4-pre-release-filesystem-hardening.md)。

## 观察

cs02 M1-M3 已经实现支持子集的 Git 兼容性,但 M4 pre-release 审计发现本地文件系统领域的 release-readiness 缺口:

- loose object、index、ref 写入缺少足够的 symlink/原子写/并发保护;
- `.mg/index` 的 entry count 可能在校验前造成过大分配;
- loose object 解压缺少 decoded-size 上限;
- `mg add` 可以 staging `.mg/**` / `.git/**` 仓库内部路径;
- human docs 对 `.mg`/`.git` 兼容性表述过强,并列出了未实现的 `ls-files`。

## 处理

接受为 M4 release-hardening sprint 输入,由 ADR-0005 统一关闭。目标是先修本地文件系统 hardening 和文档 honesty,再声明 `0.1.0` readiness。

## 状态

`accepted` — 2026-05-13。
