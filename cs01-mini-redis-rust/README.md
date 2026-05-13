<div align="center">

# CS-01 · mini-redis-rust

**Redis-compatible in-memory KV + browser monitoring dashboard · Rust implementation**

*ADSD case study #1 — network service + protocol parsing + persistence + live monitoring*

</div>

---

## Why this exists

This is a Redis-compatible subset built from zero with ADSD multi-agent methodology, deliberately far from Cobrust's compiler domain, to test whether the methodology still works for network services, persistence, and a frontend surface.

The product goal is a small, inspectable Redis-like server that works with `redis-cli` for the supported subset. The research goal is the evidence trail: ADRs, findings, bilingual docs, and pre-release multi-agent audits that show where ADSD works or breaks.

## What this is

CS-01 implements a real RESP TCP server, command router, in-memory storage, TTL handling, AOF persistence, Pub/Sub, Axum HTTP/SSE control plane, and SvelteKit browser monitoring UI.

It is not production Redis. It is a case-study `0.1.0` line for the supported Redis subset; desktop packaging, installers, signing, and notarization are not part of this case's release surface.

## Prerequisites

- Rust toolchain with Cargo (workspace currently targets the checked-in `Cargo.lock`; use the project toolchain if one is pinned locally).
- `redis-cli` for manual smoke tests.
- Optional: Docker for oracle comparison against `redis:7-alpine`.
- Optional frontend tooling: Node.js 20+ and pnpm 9+ for `web/` gates and browser dev mode.

## Quick start

```bash
cd cs01-mini-redis-rust
bash scripts/bootstrap.sh
cargo run -p redis-server -- --port 6380
# another terminal
redis-cli -p 6380 PING
```

The quick-start command intentionally does not enable AOF, so it works in a fresh checkout without pre-creating directories.

### Persistence (M3.2)

AOF append/replay is available when explicitly requested:

```bash
mkdir -p data
cargo run -p redis-server -- --port 6380 --aof data/dump.aof
```

AOF is a simplified append-only mode with no rewrite/compaction in `0.1.0`.

## Dev mode and UI

M2.2 shipped the SvelteKit browser frontend under `web/` with Dashboard, Keys, and Pub/Sub pages.

```bash
# Terminal 1 — backend (RESP :6380 + HTTP/SSE :6381)
cargo run -p redis-server -- --port 6380 --http-port 6381

# Terminal 2 — frontend (vite :5173, proxy /api → 6381)
cd web
pnpm install
pnpm dev
# open http://localhost:5173
```

### Pub/Sub page is a read-only dashboard (M3.1)

`/_studio/pubsub` is no longer an M2 placeholder. It displays live channel/subscriber counts from `/api/pubsub` SSE and intentionally does not publish or subscribe on behalf of the browser. Use a RESP client such as `redis-cli -p 6380` for `SUBSCRIBE`, `UNSUBSCRIBE`, and `PUBLISH`.

### Frontend gate

```bash
bash scripts/frontend-gate.sh
```

The gate runs install/check/test/build for `web/`. If Node or pnpm is unavailable, treat that as an environment skip and report the exact missing tool.

## Supported commands

Supported in the current committed `0.1.0` line:

- Connection / utility: `PING`, `ECHO`, `SELECT 0`, `QUIT`
- String KV: `SET key value`, `SET key value EX seconds`, `GET`, `DEL`, `EXISTS`
- Integer counters: `INCR`, `DECR`, `INCRBY`, `DECRBY`
- Expiry / introspection: `EXPIRE`, `TTL`, `PERSIST`, `TYPE`, `KEYS`
- Pub/Sub: `SUBSCRIBE`, `UNSUBSCRIBE`, `PUBLISH`

Not implemented in `0.1.0`:

- AUTH / ACL / TLS
- replication / cluster
- Lua scripting, streams, modules
- RDB persistence
- `PSUBSCRIBE` / `PUNSUBSCRIBE`
- hashes, sorted sets, lists, sets
- full `MULTI` / `EXEC` transaction semantics

## Known behavioral deltas vs real Redis

- **Lagging Pub/Sub subscriber disconnect**: slow subscribers get an explicit `-ERR client lagged behind pub/sub buffer; disconnecting` before EOF. Real Redis generally resets the connection without that application-level frame. See [`m3-1-lagging-subscriber-disconnect`](docs/agent/findings/m3-1-lagging-subscriber-disconnect.md).
- **AOF replay corruption handling**: accepted M3.2 behavior is warn-and-continue / stop-at-first-corrupt-tail rather than a full `redis-check-aof` repair workflow. See [`m3-2-aof-replay-corruption-handling`](docs/agent/findings/m3-2-aof-replay-corruption-handling.md).
- Unsupported Redis features are out of scope rather than partially simulated; this is intentional F24 defense.

## Architecture

```text
┌──────────────────────────────────────────┐
│ SvelteKit UI (browser dev)               │
│ - /_studio/dashboard                     │
│ - /_studio/keys                          │
│ - /_studio/pubsub (read-only)            │
└─────────────────┬────────────────────────┘
                  │ REST + SSE
        ┌─────────▼──────────┐
        │   redis-server     │  Axum + tokio
        │   - /api/stats     │
        │   - /api/keys SSE  │
        │   - /api/pubsub    │
        │   - TCP :6380 RESP │
        └────┬──────────┬────┘
             │          │
     ┌───────▼──┐  ┌────▼───────────┐
     │ redis-   │  │ redis-storage  │
     │ protocol │  │ - hashbrown KV │
     │ (RESP)   │  │ - expiry       │
     │          │  │ - Pub/Sub      │
     │          │  │ - AOF writer   │
     └──────────┘  └────────────────┘
```

`rust-embed` single-binary packaging remains future work. The current release surface is the Rust server plus SvelteKit browser dashboard.

## Status

- ✅ M0 scaffold
- ✅ M1 backend MVP: RESP + TCP server + core commands + oracle hardening
- ✅ M2 frontend MVP: Axum HTTP/SSE control plane + SvelteKit dashboard
- ✅ M3 Pub/Sub + AOF
- ✅ M4.1 critical fixes: security + AOF + dispatch + Pub/Sub hardening
- ✅ M4.2 doc sweep + release artifacts ([ADR-0012](docs/agent/adr/0012-m4-2-doc-sweep-release-artifacts.md))
- ✅ M4.4 scope correction: withdrawn wrong-session desktop packaging requirement ([finding](docs/agent/findings/m4-4-cross-session-tauri-contamination.md))

## Docs

- Human docs, Chinese: [`docs/human/zh/`](docs/human/zh/)
- Human docs, English: [`docs/human/en/`](docs/human/en/)
- Agent ADRs: [`docs/agent/adr/`](docs/agent/adr/)
- Agent findings: [`docs/agent/findings/`](docs/agent/findings/)
- Root methodology status: [`../METHODOLOGY-STATUS.md`](../METHODOLOGY-STATUS.md)

## ADSD trigger points observed

- F1 sediment / constitution-vs-ADR drift: README, local charter, and ADR metadata drifted during fast wave merges.
- F17 KPI self-report fidelity gap: P9 missed the shared doc-coverage script in M1.1.
- F18 CTO-as-implementer drift: CTO wrote M1.3 code instead of dispatching.
- F23-A oracle value and gap: Redis oracle caught TTL rounding, but happy-path-only oracle missed malformed-input rejection until deep-source audit.
- F24 primitive-as-everything defense: unsupported Redis data structures are not faked with primitive containers.
- Cross-session requirement contamination: a desktop packaging requirement from another session entered cs01 and was withdrawn in M4.4.

## License

Apache-2.0 + MIT dual, matching the repository root.
