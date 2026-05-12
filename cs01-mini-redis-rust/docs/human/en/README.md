# cs01-mini-redis-rust (English user guide)

## What this is

A Rust-from-scratch Redis-compatible subset with a SvelteKit monitoring console. Starting in M4.3, the primary frontend release surface moves to a Tauri desktop app with a managed `redis-server` sidecar. The goal is to validate whether the ADSD methodology still works in the **network-service + protocol + storage + desktop-release** domain (vs. Cobrust's original compiler domain).

## Quick start

```bash
cd cs01-mini-redis-rust
bash scripts/bootstrap.sh
cargo run -p redis-server -- --port 6380
```

Monitor UI: M4.3 target is a Tauri desktop app; browser dev mode uses `cd web && pnpm dev` and opens `http://localhost:5173`.

## Supported commands (after M1)

- `PING / ECHO / QUIT`
- `SET key val [EX seconds] / GET key / DEL key... / EXISTS key...`
- `INCR / DECR / INCRBY / DECRBY`
- `EXPIRE key seconds / TTL key / PERSIST key`
- `TYPE key / KEYS pattern`

After M3: `SUBSCRIBE / UNSUBSCRIBE / PUBLISH`.

## Compatibility with real Redis

We run round-trip tests against `redis:7-alpine` (docker) — see `tests/oracle.sh` (available after M1.3).

## ADR index

See full ADRs in [`docs/agent/adr/`](../../agent/adr/). English abstracts:

- [ADR-0001 Stack choice](./adr-0001-stack-choice.md): tokio + Axum + hashbrown + rust-embed (aligned with Cobrust Studio stack).
- [ADR-0013 Tauri desktop frontend](./adr-0013-tauri-desktop-frontend.md): M4.3 frontend release surface moves to a Tauri desktop app + managed sidecar.

## Status

✅ M1-M3 shipped; ✅ M4.1 critical fixes; 🚧 M4.2 doc sweep; 🚧 M4.3 Tauri desktop frontend. See "Status" section in the root [README](../../../README.md).

## License

Apache-2.0 + MIT dual.
