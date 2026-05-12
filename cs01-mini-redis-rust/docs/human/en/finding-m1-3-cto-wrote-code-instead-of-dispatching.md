# Finding M1.3 English abstract: CTO implemented Phase 2 instead of dispatching P9

> Full finding: [docs/agent/findings/m1-3-cto-wrote-code-instead-of-dispatching.md](../../agent/findings/m1-3-cto-wrote-code-instead-of-dispatching.md).

## One-liner

Acting as CTO, after ADR-0005 Phase 1 landed I personally wrote the M1.3 implementation (`encode.rs` / `server.rs` / `main.rs` + extended `Command` enum) on the rationale that "context is already complete, scope is small, ROI is high". This violates ADSD §"P10 — CTO / Architect" which explicitly lists **"NOT responsibility: Writing code (CTO who codes loses strategic altitude)"**.

User correction was one sentence: **"你作为 CTO,怎么能亲自写代码?"** ("You're the CTO — how can you write code yourself?").

## Damage

1. Strategic altitude collapse — context filled with implementation details
2. Loss of sub-agent second-review (P9 reading ADR + writing impl is a natural reviewer)
3. Constitution-vs-behaviour divergence (F1 candidate + F18 self-review sub-pattern)
4. Future Phase 2 SOP signal contaminated

## Fix

- Rolled back all CTO-written code to ADR-0005 commit ✅
- Logged this finding ✅
- Re-dispatching P9 sub-agent for Phase 2 (executing now)
- Plan to patch top-level CLAUDE.md adding explicit "CTO must not implement Phase 2" rule

## New F-pattern candidate

**"CTO-as-implementer"** — a CTO-tier sub-pattern of F18 self-review. The more complete a Phase 1 ADR's sub-decisions are, the richer the CTO's context, and the smaller the apparent scope — **the more strongly this argues for dispatching P9**, not against it.

## Status

`P1`, fix in progress (P9 dispatch underway).
