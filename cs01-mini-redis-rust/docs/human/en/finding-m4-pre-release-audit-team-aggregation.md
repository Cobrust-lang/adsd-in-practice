# Finding M4 English abstract: Pre-M4 8-agent audit team aggregation

> Full finding: [docs/agent/findings/m4-pre-release-audit-team-aggregation.md](../../agent/findings/m4-pre-release-audit-team-aggregation.md).

## One-liner

At Wave M3 closure, per ADSD §"Self-applied multi-agent audit" + §"LLM-simulated user persona" + §"Deep-source-read", dispatched **8 read-only audit sub-agents** (4 internal dimensions + 3 personas + 1 deep-source). **~80 raw / ~50 unique findings**: 1 BLOCK / 12 cross-validated HIGH / 14 single-agent HIGH / 20 MED / 13 LOW.

## Key findings

**Constitution-vs-ADR drift** (new F1 sub-pattern):
- cs01 §1 explicitly bans `tokio::sync::broadcast` for Pub/Sub, but ADR-0009 chose broadcast (SA-1)
- cs01 §4 mandates single-direction crate layers, but M3.2 let `storage → protocol` (SA-2)

**F23-A oracle gap**: we accept `SET k v EX 60 GARBAGE`, real Redis rejects (SA-6) — only line-by-line deep-source caught it; dimension agents + oracle harness missed

**Real P0 issues**:
- `Frame::parse` array recursion has no depth limit (stack-overflow attack) (CV-8)
- AOF mpsc is `unbounded_channel` (slow disk → OOM) (SA-4)
- `--max-clients` cap missing (DoS) (CV-9)
- AOF `Always` policy is misleading (`Reply::Ok` returns before `sync_data`) (SA-3)
- AOF tail corruption silent re-replay → INCR/DECR counter drift across reboots (SA-7)

**Sediment** (F1):
- README §Status is two waves stale (CV-1)
- `bootstrap.sh` still says "M1.0 scaffold" (CV-4)
- Findings ledger missing m3-1 + m3-2 (CV-10)
- 3 findings lack zh/en bilingual abstracts (CV-11)
- LICENSE files missing (CV-5)

## Persona scores

- Mei (Python user): 4/5, BOOKMARK + RECOMMEND-ON-SLACK
- Aleksandr (Rust senior): 4/5, "would merge this"
- Sarah (OSS evaluator): 36/100, WATCH-FROM-DISTANCE (waiting for cs02 ship + LICENSE + v0.1.0 tag)

## Proposed M4 split

- **M4.1** ADR-0011 critical fixes (real bugs + constitution drift)
- **M4.2** ADR-0012 doc sweep + release artifacts (LICENSE + CHANGELOG + CONTRIBUTING + SECURITY + METHODOLOGY-STATUS + README sediment cleanup)
- **M4.3** v0.1.0 tag

## Status

`P1`, fix in progress (M4.1 / M4.2 incoming). **ADSD upstream `case-study/` candidate**: first quantified 8-agent audit-team leverage data point (≈ 6-8× over single-CTO self-review).
