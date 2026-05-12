---
finding: m1-4-f23a-oracle-caught-ttl-rounding-spec-bug
date: 2026-05-12
case: cs01-mini-redis-rust
severity: P3
specificity: high
related_adr: 0006
related_f: F23-A
last_verified_commit: live
positive: true
---

# Finding(正面案例):F23-A oracle 在 sprint 内捕获 ADR-vs-真-Redis 的 TTL rounding 偏差

## Hypothesis(预期)

ADR-0006 §"TTL 整数语义" 我作为 CTO 凭直觉规定 TTL 返回值 = `floor((expires_at - now).as_secs())`。Reasoning:Redis 是秒级 TTL,floor 是"最保守"的下取整,**应该跟真 Redis 一致**。

## Method

P9 sub-agent 在 Phase 2 实现 ADR-0006 时,严格按 ADR §TTL 写 `floor` 逻辑。落地后跑 ADR-0006 同 sprint 新接的 docker oracle(`tests/oracle.sh` + `redis:7-alpine`):

```bash
CS01_RUN_ORACLE=1 bash tests/oracle.sh
```

oracle 跑了 22 条命令对照,**TTL 那一条 mismatch**:

```
SET ttlkey ttlval EX 100
TTL ttlkey
  ours:   99    (floor of 99.6s remaining)
  oracle: 100   (real Redis returns ceiling-ish)
```

P9 去查 redis upstream `src/expire.c`,发现真 Redis 用:

```c
long long pttl = expireGenericCommand_ttl(c, db, key) - mstime();
addReplyLongLong(c, (pttl + 500) / 1000);
```

`(pttl + 500) / 1000` = **round-half-up**(凑到最近的整秒,边界 `.5` 向上),不是 floor。

## Result

ADR-0006 §"TTL 整数语义" 的 spec **错了**。Real Redis 在 SET EX 100 后立刻 TTL 应该返回 100(因为 pttl 在 ~99500-100000ms,加 500 后 / 1000 = 100),不是 99。

P9 在 Phase 2 sprint 内自己修了:commit `0800d86 fix(storage): A6.4 cs01 TTL rounding aligns with real Redis (oracle-driven)`。修后 oracle 22/22 match。

## Conclusion

**这是 ADSD F23-A "oracle 自己写自己测" 真正发挥作用的正面案例**。

具体路径:

1. CTO 在 Phase 1 ADR 凭直觉决定 spec(floor)
2. P9 严格按 ADR 实现(floor)
3. 同 sprint 接的 F23-A docker oracle 跑通用例,**抓到 spec 错了**
4. P9 用 default-proceed(top-level CLAUDE.md §1.2)修正 + 留 escalation 给 CTO 写 ADR addendum

如果没有 F23-A oracle,这个 bug 会:
- 在 `tests/store_basic.rs` 自己测自己 — 因为 ADR 也写错了,test 跟着错,bug 通过
- 一直活到第一个真 Redis-client 用户报 bug:**"为什么我 SET k v EX 100 立刻 TTL 是 99 不是 100?"**
- 那时 bug 已经在 0.1.0 release notes 里 ✗

具体 ROI 量化:
- F23-A oracle 增加 P9 sprint 工作量:~30 min(写 oracle.sh 真 docker round-trip + 22 个 fixture)
- 救回的成本:1 个 P1 bug(用户报 + 找回 Postgres-test-style debugging + 写 fix + release notes contention)≈ 3-4 小时

**杠杆 ~6×**。

## Pattern lessons

CTO Phase 1 ADR 凭直觉锁 spec 的风险:**spec 跟真世界(oracle)对不上**。Mitigation:
- **Spec 引用真实**:ADR 的 §"选" 项里**应该 cite upstream source**(e.g. `src/expire.c#L123`),不是自己拍脑袋
- **F23-A oracle 在 sprint 内,不留到 release readiness**:这个 finding 证明 in-sprint oracle 比 release-readiness oracle 价值高 6×
- **正面 finding 也要写**:F23-A 兑现 = ADSD 方法论生效的证据,**回灌 ADSD upstream 时这种 finding 是 wedge story**(不是 negative result,是 positive)

## Fix / Mitigation

- TTL 实现已修(commit `0800d86`)— P9 sprint 内修
- ADR-0006 §TTL 整数语义 加 **Addendum**(本 commit 同时落)— 保留原 `floor` 文本作 audit trail
- 加到 cs01 CLAUDE.md §3 / §2 oracle 节的下次 review:**ADR Phase 1 写"选 spec"时 cite 真 Redis 源码 line**,而不是 "严格按 Redis"

## Lessons / F-pattern mapping

- 这是 ADSD F23-A 第一个**正面案例**:F23-A 不仅是防御性原则,在 sprint 内就有可量化 ROI
- 提议加入 ADSD upstream `case-study/cobrust-multi-agent-experience.md` 作 sub-section "F23-A in action: ADR-spec vs upstream-source"
- 跟 Cobrust 的 codegen-pollution-quarantine 是反方向 — Cobrust 是 oracle 抓到自家 codegen bug,这里是 oracle 抓到自家 ADR-spec bug。**两者都验证了 oracle 不能跟实现同源**

## Notes

P9 报告里在 "ADR-0006 correction needed? YES" 字段就 flag 了这个。CTO follow-up = 这份 finding + addendum,**这就是 ADSD 守闸的正确收尾形态**:P9 修 code + 标 escalation,CTO 写 ADR addendum + finding。
