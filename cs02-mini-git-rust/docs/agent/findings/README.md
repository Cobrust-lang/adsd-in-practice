# Findings — cs02-mini-git-rust

> Negative result ledger。任何"我以为 X,结果是 Y"都进这里。

## Ledger

| File | Severity | Status | Related ADR | F-pattern | Date |
|---|---|---|---|---|---|
| [m4-pre-release-filesystem-hardening](m4-pre-release-filesystem-hardening.md) | High | accepted | ADR-0005 | filesystem hardening / docs honesty | 2026-05-13 |

## 预期会撞的 finding 模板

- `m1-sha1-empty-input-mismatch.md` — sha1("") 跟 git 不一致(应该没有,但 round-trip 测试时要确认)
- `m2-index-format-endianness.md` — git index 大端字节序坑
- `m3-zlib-flush-empty-blob.md` — 空 blob 的 zlib 流 flush 行为
- `oracle-roundtrip-divergence.md` — 跟真 git 双向兼容时撞出的差异
