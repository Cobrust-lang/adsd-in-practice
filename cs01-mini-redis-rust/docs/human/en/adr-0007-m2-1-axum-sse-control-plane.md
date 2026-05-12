# ADR-0007 English abstract: M2.1 Axum HTTP + SSE control plane (backend)

> Full ADR: [docs/agent/adr/0007-m2-1-axum-sse-control-plane.md](../../agent/adr/0007-m2-1-axum-sse-control-plane.md).

## Decision

Split M2 into **M2.1 backend + M2.2 frontend**. M2.1 ships:

1. **Separate HTTP listener on 6381** — no 6380 multiplex; protocol-sniff between RESP and HTTP is over-engineering
2. **`Store::metrics()` + `Store::sample_keys(n)`** in the storage crate (data owner exposes own observability)
3. **`AppState { store, conn_count, cmd_count, started }`** shared between RESP and HTTP listeners; `handle_conn` uses an RAII guard for connection count and increments `commands_total` after each `from_frame`
4. **SSE event format**: standard `event: <type>\ndata: <json>\n\n`, 1 Hz cadence
5. **`/api/stats`**: pushes `{connections_active, commands_total, keys_active, mem_value_bytes, uptime_secs}`
6. **`/api/keys`**: pushes top-100 KeyInfo (`{key, type, ttl_secs}`) — large keyspace gets truncated
7. **`tokio::sync::broadcast` fan-out**: one sampler task → N SSE clients sharing data; lagging clients are disconnected
8. **e2e tests via `reqwest`** connecting to the SSE stream, asserting schema + monotonic uptime
9. **F23-A is not applicable to the SSE control plane** (no reference impl to compare against); a finding documents the gap explicitly

## Rejected alternatives

- Single-port multiplex (6380 byte-sniff RESP/HTTP) — over-engineering
- Server tracking key count itself (dual data source = F1 sync-bug candidate)
- Per-connection sampler timer (N×lock for no gain)
- CloudEvents format (EventSource clients don't need it)
- CORS in M2.1 (defer to M2.2 frontend ADR)
- Playwright in M2.1 (too heavy; `reqwest` suffices for backend e2e)

## Test target

`cargo test --workspace` ≥ 195 (M1.4 baseline 179, M2.1 adds ~15-20).

## Status

`accepted` — 2026-05-12.
