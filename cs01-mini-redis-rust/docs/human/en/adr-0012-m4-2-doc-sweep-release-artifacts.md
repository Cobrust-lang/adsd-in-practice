# ADR-0012 English abstract: M4.2 doc sweep + release artifacts

> Full ADR: [docs/agent/adr/0012-m4-2-doc-sweep-release-artifacts.md](../../agent/adr/0012-m4-2-doc-sweep-release-artifacts.md).

## Decision

M4.2 ships a holistic doc-sweep + release artifacts wave — 7 buckets, ~30 items, all audit-team-surfaced doc fixes:

**Bucket A — Release artifacts** (Sarah's hard blockers):
- `LICENSE-APACHE` + `LICENSE-MIT` at repo root (SPDX declared but license text missing)
- `CHANGELOG.md` / `CONTRIBUTING.md` / `SECURITY.md`
- `METHODOLOGY-STATUS.md` cs01 section populated

**Bucket B — README sediment** (closes public-readiness BLOCK + HIGH cluster):
- §Status updated to reflect M1-M4.1 progress
- `/pubsub` stub note removed → read-only dashboard
- Quick-start drops `--aof data/dump.aof` (ENOENT)
- Adds §"Supported commands" / §"Prerequisites" / §"Docs" / §"Why does this exist?" / §"Known behavioral deltas vs real Redis"

**Bucket C — bootstrap.sh** no longer claims "M1.0 scaffold"

**Bucket D — Bilingual finding mirrors**: m1-1 + m3-1 + m3-2 each get zh + en abstracts (closes top CLAUDE.md §1.1 invariant violation)

**Bucket E — `_shared/doc-coverage.sh` enforces finding bilingual** (prevents recurrence; closes LOW-2 root cause)

**Bucket F — ADR metadata sweep**:
- ADR-0005 `last_verified_commit` re-blessed
- ADR-0001 Done Criteria: rust-embed deferred to M4.3+
- ADR-0009 §Done Criteria adds "RESET" + §Implementation deltas note about `pubsub.rs` not existing
- cs01 CLAUDE.md §3 wave order gets closure markers

## Numeric targets

- Backend tests ≥ 284 (no change)
- Oracle 36/36 (no change)
- 2 CHANGELOG.md files (root + cs01)
- 6 new bilingual finding abstracts
- 5 new release-artifact files at repo root

## Status

`accepted` — 2026-05-12. Next: M4.3 v0.1.0 tag.
