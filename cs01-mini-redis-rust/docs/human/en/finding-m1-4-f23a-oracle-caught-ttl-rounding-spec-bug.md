# Finding M1.4 English abstract (positive case): F23-A oracle caught TTL rounding spec bug

> Full finding: [docs/agent/findings/m1-4-f23a-oracle-caught-ttl-rounding-spec-bug.md](../../agent/findings/m1-4-f23a-oracle-caught-ttl-rounding-spec-bug.md).

## One-liner

ADR-0006 §"TTL 整数语义" had the CTO specify `floor((expires_at - now).as_secs())` by intuition. P9 implemented per ADR and ran the docker oracle (F23-A) connected in the same sprint — found that real Redis 7 uses **round-half-up** (`(pttl_ms + 500) / 1000`, per `src/expire.c`), not floor. **The oracle caught the ADR-spec-vs-real-Redis divergence within the same sprint.**

P9 self-fixed in commit `0800d86`; oracle now 22/22 match. CTO followed up with the ADR addendum + this finding.

## Key numbers

- F23-A oracle marginal cost: ~30 min P9 sprint time (writing oracle.sh)
- Prevented cost: 1 P1 wire-bug ≈ 3-4h (user report + debug + fix + release-notes triage)
- **Leverage ~6×**

## Conclusion

ADSD F23-A's first **quantified positive in-sprint case**. Recommendations:
- ADR Phase 1 §"choice" rows should cite upstream source lines, not rely on intuition
- F23-A oracle should be wired up *during* a sprint, not deferred to release-readiness
- Positive findings must be written too — they're evidence the methodology pays off

## Status

`P3 (positive)`, closed in-sprint. Candidate for backport to ADSD upstream `case-study/`.
