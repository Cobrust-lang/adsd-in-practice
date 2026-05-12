# Finding M2.1 English abstract (gap acknowledgement): no F23-A oracle for the HTTP/SSE control plane

> Full finding: [docs/agent/findings/m2-1-no-f23a-on-control-plane.md](../../agent/findings/m2-1-no-f23a-on-control-plane.md).

## One-liner

When cs01 wired up the Axum HTTP + SSE control plane in M2.1 (`/api/stats`, `/api/keys`), **no comparable reference implementation exists to serve as the F23-A oracle** — real Redis has no SSE control plane, and redis-stat / Redis Insight / redis_exporter each speak different wire protocols. **F23-A doesn't apply at this layer.** Accepted gap, mitigated with ADR-0007 schema lock + `tests/http_sse.rs` self-tests + cross-sprint contract (M2.2 frontend ADR-0008 must sync schema changes with the backend).

## Key calls

- F23-A is not universal: the RESP protocol layer has real Redis as a strong oracle; the HTTP/SSE control plane has **none**
- Not a failure — an **acceptance gap** (severity P4 + positive: false neutral tag)
- Mitigation: `StatsSnapshot` / `KeyJson` 5+3 fields are locked in ADR-0007 §Done Criteria; frontend ADR-0008 will cite this finding as the basis for the no-unilateral-rename rule

## Conclusion

**Boundary of F23-A**: in sub-domains without a reference implementation (SSE control planes, custom admin APIs, private RPC), F23-A doesn't apply — the correct response is to **explicitly mark the gap** (this finding's pattern). Mirrors the M1.4 positive F23-A finding: there, the oracle caught a bug; here, the missing oracle is explicitly acknowledged.

## Status

`P4`, acceptance gap. Candidate citation in M2.2 frontend ADR-0008.
