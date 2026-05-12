<div align="center">

# CS-01 · mini-redis-rust

**Redis-compatible in-memory KV + monitoring dashboard · Rust implementation**

*ADSD case study #1 — network service + protocol parsing + persistence + live monitoring*

</div>

---

## Why this exists

This is a Redis-compatible subset built from zero with ADSD multi-agent methodology, deliberately far from Cobrust's compiler domain, to test whether the methodology still works for network services, persistence, and a frontend release surface.

The product goal is a small, inspectable Redis-like server that works with `redis-cli` for the supported subset. The research goal is the evidence trail: ADRs, findings, bilingual docs, and pre-release multi-agent audits that show where ADSD works or breaks.

## What this is

CS-01 implements a real RESP TCP server, command router, in-memory storage, TTL handling, AOF persistence, Pub/Sub, Axum HTTP/SSE control plane, and SvelteKit monitoring UI.

It is not production Redis. It is a tag-ready case study for `0.1.0`; DMG creation, signing, and notarization remain separate future release-engineering work.

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

M2.2 shipped the SvelteKit frontend under `web/` with Dashboard, Keys, and Pub/Sub pages. ADR-0013 changes the primary M4.3 release target to a Tauri desktop app that manages a `redis-server` sidecar. Browser dev mode remains supported for development.

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

## Tauri desktop mode (M4.3)

ADR-0013 的 v0.1.0 release surface 是 `web/src-tauri/` 下的 Tauri v2 desktop shell。它复用同一份 SvelteKit/Vite UI,并尝试管理本地 `redis-server` sidecar:

- RESP 端口固定为 `127.0.0.1:6380`。
- HTTP/SSE control plane 固定为 `127.0.0.1:6381`。
- 桌面 UI 会显示 sidecar 状态 banner:starting / running / failed / stopped;缺少 sidecar binary、端口冲突或启动超时不会静默失败。
- Browser dev mode 保持不变:`pnpm dev` 仍通过 Vite proxy 访问 `/api → http://localhost:6381`。

轻量桌面开发流程:

```bash
# 先构建 sidecar binary；Tauri dev 会自动查找 target/debug/redis-server
cargo build -p redis-server

cd web
pnpm install
pnpm tauri:dev
```

如果 `redis-server` 不在默认位置,可显式指定:

```bash
CS01_REDIS_SERVER_BIN=/absolute/path/to/redis-server pnpm tauri:dev
```

轻量 gate 默认不跑完整 bundle,避免在低磁盘环境反复创建 `src-tauri/target/` / bundle artifacts:

```bash
bash scripts/tauri-gate.sh
# release-readiness 才跑完整 bundle:
CS01_TAURI_FULL_BUILD=1 bash scripts/tauri-gate.sh
```

当前 tag-prep 状态:`31b52a1` 已通过 opt-in full Tauri `.app` bundle gate:`CS01_TAURI_FULL_BUILD=1 bash scripts/tauri-gate.sh`,产物为 `web/src-tauri/target/release/bundle/macos/CS01 mini-redis.app`。DMG、signing、notarization 不在标准 gate 内,仍是 separate future release-engineering work,本 README 不声明它们已完成。

## Architecture

```text
┌──────────────────────────────────────────┐
│ SvelteKit UI (browser dev)               │
│ M4.3 target: Tauri desktop shell         │
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

`rust-embed` single-binary packaging was the original ADR-0001/0008 direction, but ADR-0013 supersedes the release target for `0.1.0`: Tauri desktop + managed sidecar first; rust-embed may return in a later release.

## Status

- ✅ M0 scaffold
- ✅ M1 backend MVP: RESP + TCP server + core commands + oracle hardening
- ✅ M2 frontend MVP: Axum HTTP/SSE control plane + SvelteKit dashboard
- ✅ M3 Pub/Sub + AOF
- ✅ M4.1 critical fixes: security + AOF + dispatch + Pub/Sub hardening
- ✅ M4.2 doc sweep + release artifacts ([ADR-0012](docs/agent/adr/0012-m4-2-doc-sweep-release-artifacts.md))
- ✅ M4.3/M4.4 Tauri desktop frontend + managed sidecar + macOS `.app` bundle gate ([ADR-0013](docs/agent/adr/0013-tauri-desktop-frontend.md)); DMG/signing/notarization remain separate future release-engineering work; rust-embed single-binary deferred

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

## License

Apache-2.0 + MIT dual, matching the repository root.
