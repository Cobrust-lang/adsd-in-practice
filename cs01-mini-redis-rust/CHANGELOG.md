# Changelog — cs01-mini-redis-rust

All notable changes to CS-01 are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). CS-01 records the finalized `v0.1.0` release documentation; the annotated tag is created after this finalization commit.

## [0.1.0] - 2026-05-13

### Added

- RESP parser / serializer and TCP server compatible with `redis-cli` for the supported command subset.
- In-memory string KV store with TTL and command dispatch for `PING`, `ECHO`, `SELECT`, `QUIT`, `GET`, `SET`, `DEL`, `EXISTS`, integer counters, expiry, type, keys, Pub/Sub, and AOF-backed persistence.
- Axum HTTP/SSE control plane for stats, keys, and Pub/Sub subscriber snapshots.
- SvelteKit browser dashboard with Dashboard, Keys, and read-only Pub/Sub pages.
- AOF append/replay mode via `--aof <path>`.
- Bilingual human ADR/finding abstracts plus agent-facing ADR/finding source documents.

### Changed

- Pub/Sub page wording now describes the implemented read-only dashboard instead of the M2 placeholder.
- Quick start runs without AOF so a fresh checkout does not fail on a missing `data/` directory.
- M4.4 withdraws the accidental Tauri desktop packaging scope introduced from a wrong-session requirement; the valid cs01 release surface is the Rust server plus SvelteKit browser dashboard.

### Fixed

- M4.1 hardened default bind address, parser depth, max-client handling, AOF write queue/file mode, AOF flush naming, SET trailing-token parsing, and confusing comments.
- M4.2 closes documentation sediment around README status, bootstrap hints, ADR metadata, release artifacts, and methodology status.
- M4.4 removes Tauri-specific code, dependencies, gate scripts, and release-readiness claims that entered cs01 from cross-session contamination.

### Release-readiness status

- CTO final audit on the cs01 line passed doc-coverage, cargo fmt, cargo clippy, cargo test, prior integration oracle coverage (23/23 RESP, 6/6 Pub/Sub, 7/7 AOF), and frontend browser gates.
- Desktop packaging, installers, signing, and notarization are not part of cs01 `0.1.0` readiness.

### Known behavioral deltas vs Redis 7

- Slow Pub/Sub subscribers receive an explicit error before disconnect rather than a silent reset.
- AOF corruption handling is warn-and-continue for accepted M3.2 debt; production-grade repair tooling is out of scope.
- Unsupported Redis features return errors or are absent: AUTH, ACL, TLS, replication, cluster, Lua, streams, modules, RDB, PSUBSCRIBE/PUNSUBSCRIBE, hashes, sorted sets, lists, sets, and full transaction semantics.
