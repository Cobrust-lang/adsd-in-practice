# ADR-0009 English abstract: M3.1 Pub/Sub

> Full ADR: [docs/agent/adr/0009-m3-1-pubsub.md](../../agent/adr/0009-m3-1-pubsub.md).

## Decision (compact)

| Sub-item | Choice |
|---|---|
| Subscriber state | **`Inner.subscribers: HashMap<String, broadcast::Sender<Arc<Vec<u8>>>>`** (same parking_lot RwLock domain as KV) |
| Fan-out | **`tokio::sync::broadcast`** (same pattern as stats/keys per ADR-0007); `Arc<Vec<u8>>` shared payload to avoid N× copy |
| Channel matching | **Exact-match only in M3.1**; PSUBSCRIBE glob deferred to M3.2+ (F22 cadence) |
| Sub mode | **`handle_conn` holds `ConnState` enum** (`Normal` / `Subscribed { rxs }`); `tokio::select` on `(read_buf, rxs.recv)`; in sub mode GET/SET/... all reply with `-ERR Can't execute ... in this context` |
| PUBLISH return | **`Integer(receiver_count)`** (`broadcast::Sender::send` returns N) |
| Counters | sub mode keeps `commands_total` / `connections_active` incrementing — aligns with real Redis |
| `/api/pubsub` SSE | 1 Hz `{channels: [{name, subscribers}]}` snapshot; message firehose deferred to M4 |
| `/pubsub` UI | **Read-only dashboard** (channel/sub table + "Use redis-cli to publish/subscribe" banner); web→RESP bridge deferred to M4 |
| F23-A oracle | New `tests/oracle_pubsub.py` (Python redis-py + docker redis:7-alpine) + 6 stateful fixtures |

## Numeric targets

- Backend tests ≥ 220 (+20)
- Frontend vitest ≥ 28 (+3)
- Oracle 28/28 (M1.4 22 + M3.1 6 stateful)

## Accepted debt

- Subscriber senders not evicted (M4 release-readiness will add GC)
- PSUBSCRIBE deferred to M3.2
- UI is read-only (web→pub bridge deferred to M4)
- Lagging subscribers are disconnected (more aggressive than Redis; candidate finding)

## Status

`accepted` — 2026-05-12.
