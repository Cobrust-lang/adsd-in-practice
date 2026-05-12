# ADR-0003 English abstract: Storage layout

> Full ADR: [docs/agent/adr/0003-storage-layout.md](../../agent/adr/0003-storage-layout.md).

## Decision

`redis-storage::Store` uses:

- `parking_lot::RwLock<hashbrown::HashMap<String, Entry>>` — shared mutable state
- `Entry { value: Vec<u8>, expires_at: Option<Instant> }`
- `tokio::time::DelayQueue` running in a dedicated task for **active expiration** (key is actually removed when TTL hits)

## Rejected alternatives

- **Lazy expiration (check on GET)**: F24 shortcut, **violates Redis semantics** (`KEYS *` would list expired keys)
- **Sharded hashtable**: premature optimization, defer to v0.2

## Status

`accepted` — 2026-05-12.
