# cs01-mini-redis-rust (English user guide)

## What this is

A Rust-from-scratch Redis-compatible subset with a SvelteKit monitoring console. The goal is to validate whether the ADSD methodology still works in the **network-service + protocol + storage + frontend** domain (vs. Cobrust's original compiler domain).

## Quick start

```bash
cd cs01-mini-redis-rust
bash scripts/bootstrap.sh
cargo run -p redis-server -- --port 6380
```

Monitor UI: `http://localhost:6380/_studio` (available after M3).

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

## Status

🚧 M0 scaffold. See "Status" section in the root [README](../../../README.md).

## License

Apache-2.0 + MIT dual.
