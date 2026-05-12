# ADR-0002 中文摘要:对象身份与 loose object store

> 完整 ADR 见 [docs/agent/adr/0002-object-identity-loose-store.md](../../agent/adr/0002-object-identity-loose-store.md)。

## 决策

M1 直接实现与 Git 兼容的 blob 对象身份和 loose object store:

- 对象字节流固定为 `"<kind> <size>\0<payload>"`。
- v0.1.0 使用 SHA-1 计算对象 ID, SHA-256 只保留抽象升级口。
- `mg hash-object -w` 写入 zlib 压缩后的 `.mg/objects/aa/bb...` loose object。
- `mg cat-file -p` 先支持读取 blob payload。
- M1 允许实现最小 `mg init` 创建 `.mg/objects`;完整 HEAD/ref/index/commit 语义仍留到 M3。

## 为什么

如果 M1 只做库函数或 mg-only 临时格式,oracle 就会变成自己测自己,无法证明真 `git` 能读我们的对象。把最小 object database 初始化前移,可以让 `git cat-file -p` 从第一波就成为外部验收标准。

## 状态

`accepted` — 2026-05-13。
