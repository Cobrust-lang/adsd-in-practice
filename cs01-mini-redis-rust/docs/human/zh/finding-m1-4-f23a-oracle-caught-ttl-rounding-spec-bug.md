# Finding M1.4 中文摘要(正面案例):F23-A oracle 抓到 TTL rounding spec 错

> 完整 finding 见 [docs/agent/findings/m1-4-f23a-oracle-caught-ttl-rounding-spec-bug.md](../../agent/findings/m1-4-f23a-oracle-caught-ttl-rounding-spec-bug.md)。

## 一句话

ADR-0006 §"TTL 整数语义" CTO 凭直觉规定 `floor((expires_at - now).as_secs())`。P9 实现后跑同 sprint 接入的 docker oracle(F23-A),发现真 Redis 7 用 **round-half-up**(`(pttl_ms + 500) / 1000`,见 `src/expire.c`),不是 floor。**oracle 在 sprint 内当场抓住 ADR-spec-vs-real-Redis 偏差**。

P9 自修 commit `0800d86`,oracle 22/22 match。CTO 后补 ADR addendum + 本 finding。

## 关键数字

- F23-A oracle 增加工作量:~30 min P9 sprint(写 oracle.sh)
- 救回成本:1 个 P1 wire-bug ≈ 3-4h(user 报 + debug + fix + release notes 公关)
- **杠杆 ~6×**

## 结论

ADSD F23-A 第一次在 sprint 内**可量化**展现 ROI。建议:
- ADR Phase 1 §"选" 项 cite upstream source line,不要拍脑袋
- F23-A oracle 在 sprint 内就接入,不要留到 release readiness
- 正面 finding 也要写 — 这是 ADSD 方法论生效的证据

## 状态

`P3 (positive)`,closed in-sprint。**回灌 ADSD upstream `case-study/` 候选**。
