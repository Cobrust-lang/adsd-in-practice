# Changelog — cs01-mini-redis-rust

All notable changes to CS-01 are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). `0.1.0-rc` is a release-candidate documentation state; final `v0.1.0` still depends on the M4.3 desktop release surface.

## [0.1.0-rc] - 2026-05-12

### Added

- RESP parser / serializer and TCP server compatible with `redis-cli` for the supported command subset.
- In-memory string KV store with TTL and command dispatch for `PING`, `ECHO`, `SELECT`, `QUIT`, `GET`, `SET`, `DEL`, `EXISTS`, integer counters, expiry, type, keys, Pub/Sub, and AOF-backed persistence.
- Axum HTTP/SSE control plane for stats, keys, and Pub/Sub subscriber snapshots.
- SvelteKit browser dashboard with Dashboard, Keys, and read-only Pub/Sub pages.
- AOF append/replay mode via `--aof <path>`.
- Bilingual human ADR/finding abstracts plus agent-facing ADR/finding source documents.

### Changed

- M4.3 release target is now Tauri desktop app + managed `redis-server` sidecar per ADR-0013; rust-embed single-binary packaging is deferred.
- Pub/Sub page wording now describes the implemented read-only dashboard instead of the M2 placeholder.
- Quick start runs without AOF so a fresh checkout does not fail on a missing `data/` directory.

### Fixed

- M4.1 hardened default bind address, parser depth, max-client handling, AOF write queue/file mode, AOF flush naming, SET trailing-token parsing, and confusing comments.
- M4.2 closes documentation sediment around README status, bootstrap hints, ADR metadata, release artifacts, and methodology status.

### Known behavioral deltas vs Redis 7

- Slow Pub/Sub subscribers receive an explicit error before disconnect rather than a silent reset.
- AOF corruption handling is warn-and-continue for accepted M3.2 debt; production-grade repair tooling is out of scope.
- Unsupported Redis features return errors or are absent: AUTH, ACL, TLS, replication, cluster, Lua, streams, modules, RDB, PSUBSCRIBE/PUNSUBSCRIBE, hashes, sorted sets, lists, sets, and full transaction semantics.
