# cs01-mini-redis-rust (English user guide)

## What this is

A Rust-from-scratch Redis-compatible subset with a SvelteKit browser monitoring console. The goal is to validate whether ADSD still works in the network-service + persistence + frontend domain.

## Quick start

```bash
cd cs01-mini-redis-rust
bash scripts/bootstrap.sh
cargo run -p redis-server -- --port 6380
```

Monitor UI: browser dev mode uses `cd web && pnpm dev` and opens `http://localhost:5173`.

AOF persistence requires an explicit directory:

```bash
mkdir -p data
cargo run -p redis-server -- --port 6380 --aof data/dump.aof
```

## Supported commands

- `PING / ECHO / SELECT 0 / QUIT`
- `SET key val [EX seconds] / GET key / DEL key... / EXISTS key...`
- `INCR / DECR / INCRBY / DECRBY`
- `EXPIRE key seconds / TTL key / PERSIST key`
- `TYPE key / KEYS pattern`
- `SUBSCRIBE / UNSUBSCRIBE / PUBLISH`

## Compatibility with real Redis

We run round-trip tests against `redis:7-alpine` (docker); see `tests/oracle.sh`. Known divergences are documented in findings, especially lagging Pub/Sub subscriber disconnect behavior and AOF corrupt-tail replay handling.

## ADR index

See full ADRs in [`docs/agent/adr/`](../../agent/adr/). English abstracts:

- [ADR-0001 Stack choice](./adr-0001-stack-choice.md): tokio + Axum + hashbrown + SvelteKit/browser UI
- [ADR-0002 RESP framing](./adr-0002-resp-framing.md): RESP parse / serialize strategy
- [ADR-0003 Storage layout](./adr-0003-storage-layout.md): in-memory storage layout
- [ADR-0004 Command routing](./adr-0004-command-routing.md): command routing
- [ADR-0005 TCP listener](./adr-0005-tcp-listener.md): RESP TCP accept loop
- [ADR-0006 M1.4 commands](./adr-0006-m1-4-commands-and-hardening.md): command expansion and hardening
- [ADR-0007 Axum SSE control plane](./adr-0007-m2-1-axum-sse-control-plane.md): HTTP/SSE control plane
- [ADR-0008 SvelteKit UI](./adr-0008-m2-2-sveltekit-ui.md): frontend dashboard
- [ADR-0009 Pub/Sub](./adr-0009-m3-1-pubsub.md): SUBSCRIBE / UNSUBSCRIBE / PUBLISH
- [ADR-0010 AOF persistence](./adr-0010-m3-2-aof-persistence.md): append-only persistence
- [ADR-0011 M4.1 critical fixes](./adr-0011-m4-1-critical-fixes.md): pre-release critical fixes
- [ADR-0012 M4.2 doc sweep](./adr-0012-m4-2-doc-sweep-release-artifacts.md): release artifacts + sediment cleanup

## Finding abstracts

- [M1.1 P9 missed shared doc-coverage](./finding-m1-1-p9-missed-shared-doc-coverage.md)
- [M1.3 CTO wrote code instead of dispatching](./finding-m1-3-cto-wrote-code-instead-of-dispatching.md)
- [M1.4 F23-A oracle caught TTL rounding](./finding-m1-4-f23a-oracle-caught-ttl-rounding-spec-bug.md)
- [M2.1 no F23-A on control plane](./finding-m2-1-no-f23a-on-control-plane.md)
- [M3.1 lagging subscriber disconnect](./finding-m3-1-lagging-subscriber-disconnect.md)
- [M3.2 AOF replay corruption handling](./finding-m3-2-aof-replay-corruption-handling.md)
- [M4 pre-release audit aggregation](./finding-m4-pre-release-audit-team-aggregation.md)
- [M4.4 cross-session Tauri contamination](./finding-m4-4-cross-session-tauri-contamination.md)

## Status

M1 backend MVP, M2 frontend/control-plane MVP, M3 Pub/Sub + AOF, M4.1 critical fixes, and M4.2 doc sweep + release artifacts are shipped. Tauri desktop/sidecar packaging is not a requirement for this project; earlier Tauri references came from a cross-session mix-up and are withdrawn.

## License

Apache-2.0 + MIT dual.
