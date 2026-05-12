---
finding: m1-1-p9-missed-shared-doc-coverage
date: 2026-05-12
case: cs01-mini-redis-rust
severity: P2
specificity: high
related_adr: none
related_f: F17
last_verified_commit: live
---

# Finding: P9 漏跑 `_shared/doc-coverage.sh`,自报 "no doc-coverage script exists"

## Hypothesis

我(CTO)派发 P9 任务时,在 prompt 列了 5 个 gates:fmt / clippy / build / test / doc-coverage,**默认 P9 会自己找 doc-coverage 脚本的位置**。

## Method

派发完 P9 后,我自己跑全 5-gate 验证(ADSD §1.2 CTO review:smoke-check + cold rebuild + 5-gate)。Gate 1-4 全过(跟 P9 自报对得上)。

跑 Gate 5:

```bash
cd cs01-mini-redis-rust
bash ../_shared/doc-coverage.sh
```

## Result

- P9 自报:"Gate 5 doc-coverage: skip (no doc-coverage.sh script exists yet in this scaffold)"
- 实际跑:`✓ doc-coverage all green` 退出码 0
- `_shared/doc-coverage.sh` 是**顶层 repo 的脚本**,case 目录跑要用相对路径 `../_shared/doc-coverage.sh`,**P9 只看了 case 目录内部,没看顶层 _shared**

## Conclusion

**Hypothesis was wrong**:不能默认 P9 会自己探出跨目录的共享脚本。

根因:
1. dispatch prompt 写了 "5 gates" 但没显式说脚本在哪里
2. P9 prompt "Required reads" 列了 4 个 CLAUDE.md / ADR,没包含 `_shared/` 目录的存在性
3. P9 在 case 目录 `cs01-mini-redis-rust/` 内 ls 找 `scripts/doc-coverage.sh`,找不到,合理地汇报"不存在",**没主动 cd .. 看父目录**

这是 ADSD F17(sub-agent KPI self-report fidelity gap)的一个具体子模式:
**not lying, just incomplete search**。

## Fix / Mitigation

- **本 finding 后所有 P9 dispatch prompt 必加 explicit "Gate 5 doc-coverage 跑 `bash ../_shared/doc-coverage.sh`"**
- 更新顶层 `CLAUDE.md` 在 §5 sub-agent 操作指南加一行:"5-gate 的 doc-coverage 脚本在 `_shared/doc-coverage.sh`,从 case 目录用相对路径调用"
- (可选)在每个 case 目录里加一个 `scripts/run-5-gate.sh` 包装,把跨目录依赖隐藏

本 finding 不阻塞 M1.1 commit:CTO 自己跑过 Gate 5 green。

## Lessons / F-pattern mapping

- 这是 ADSD F17 的 **F17.x sub-case**:**sub-agent search depth bounded to assigned directory**
- 适合提到 ADSD F-catalog 作 F17.1 候选:*"Search-boundary self-truncation"*
- 提醒 CTO:**dispatch prompt 必须列出所有跨目录依赖的 explicit path**

## Notes

P9 的报告其它部分高度准确(24 tests / gates / decisions 都对得上 my own verification),**只这一项漏**。不是诚信问题,是搜索边界问题。
