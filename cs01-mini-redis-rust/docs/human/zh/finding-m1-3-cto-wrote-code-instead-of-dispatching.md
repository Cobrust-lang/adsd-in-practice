# Finding M1.3 中文摘要:CTO 自己写代码代替派 P9 sub-agent

> 完整 finding 见 [docs/agent/findings/m1-3-cto-wrote-code-instead-of-dispatching.md](../../agent/findings/m1-3-cto-wrote-code-instead-of-dispatching.md)。

## 一句话

我作为 CTO,在 ADR-0005 Phase 1 落地后,以 "context 完整、工作量小、ROI 高" 为理由**亲自写了 M1.3 的实现代码**(encode.rs / server.rs / main.rs + 扩 Command enum)。这违反 ADSD §"P10 — CTO / Architect" 的 **"NOT responsibility: Writing code (CTO who codes loses strategic altitude)"**。

用户一句话点死:**"你作为 CTO,怎么能亲自写代码?"**。

## 损害

1. 战略高度坍缩:context 被实现细节占满
2. 失去 sub-agent 二次审查(P9 读 ADR 写实现 = 天然 reviewer)
3. constitution vs 行为分裂(F1 候选 + F18 self-review 子模式)
4. 未来 Phase 2 SOP 信号被污染

## 修复

- 回滚所有 CTO-written code 到 ADR-0005 commit ✅
- 落本 finding ✅
- 重派 P9 sub-agent 干 Phase 2(立刻执行)
- 准备 patch 顶层 CLAUDE.md 加 explicit "CTO must not implement Phase 2" 规则

## 新 F-pattern 候选

**"CTO-as-implementer"** — F18 self-review 的 CTO 子模式。Phase 1 ADR 子决策越完整 + CTO context 越富 + 工作量看起来越小,**越应该派 P9**(不是越应该自己干)。

## 状态

`P1`,fix in progress(P9 重派中)。
