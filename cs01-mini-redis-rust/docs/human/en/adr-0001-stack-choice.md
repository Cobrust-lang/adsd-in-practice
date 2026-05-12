# ADR-0001 English abstract: Stack choice

> This is the human-readable abstract of [docs/agent/adr/0001-stack-choice.md](../../agent/adr/0001-stack-choice.md). The full ADR (with 3 alternatives compared, irreversibility analysis, and done criteria) is in the agent version.

## Decision

CS-01 mini-redis-rust adopts:

- **Async runtime**: `tokio`
- **HTTP framework**: `Axum`
- **In-memory KV**: `hashbrown::HashMap`
- **Single-binary embedding**: `rust-embed`
- **TCP framing**: `tokio_util::codec` + `bytes::BytesMut`

## Why

- **Aligned with Cobrust Studio stack** — sub-agent experience, tools, and snippets reuse across projects.
- tokio + Axum is the most mature Rust async ecosystem combo, lowest barrier for external contributors.
- rust-embed is the standard single-binary deployment path, already validated by Cobrust Studio M3.

## Alternatives rejected

- `async-std + tide`: ecosystem decline, long-tail maintenance risk.
- Pure hyper + custom router: heavy yak-shaving, not feasible in 5-day MVP.

## Consequences

- Performance ceiling bound by tokio's per-task connection model (vs io_uring zero-copy). Revisit in v0.2 with tokio-uring.
- hashbrown pinned to 0.15.x (RawTable API still unstable).

## Status

`accepted` — 2026-05-12.
