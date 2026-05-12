# Changelog

All notable changes to ADSD in Practice are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). This repository is ready for the cs01 `v0.1.0` tag; the tag itself has not been created in this changelog entry.

## [0.1.0-ready] - 2026-05-13

### Added

- CS-01 `mini-redis-rust` progressed from scaffold to a Redis-compatible subset with RESP TCP serving, string KV storage, TTL, command dispatch, Axum HTTP/SSE control plane, SvelteKit dashboard, Pub/Sub, AOF append/replay, and M4.1 hardening.
- Root release artifacts: dual-license text, ADSD-aware contribution guide, security disclosure guidance, changelog, and methodology status report.
- Bilingual ADR and finding mirrors for cs01 human docs, including release-readiness audit findings.

### Changed

- CS-01 release frontend direction now targets a Tauri desktop application with managed `redis-server` sidecar per ADR-0013; rust-embed single-binary packaging is deferred and no longer described as a v0.1.0 blocker.
- Documentation coverage gate now treats finding abstracts like ADR abstracts: every agent finding must have matching `docs/human/zh/finding-*.md` and `docs/human/en/finding-*.md` files.

### Fixed

- M4.1 closed pre-release critical issues around default bind address, max frame depth, max clients, stricter command parsing, AOF file mode / bounded channel semantics, AOF flush naming, and Pub/Sub lag policy documentation.
- M4.2 removes stale README, bootstrap, ADR metadata, and methodology placeholders surfaced by the 8-agent audit.

### Release-readiness status

- CTO final audit on main `31b52a1` passed doc-coverage, cargo fmt, cargo clippy, cargo test, prior integration oracle coverage (23/23 RESP, 6/6 Pub/Sub, 7/7 AOF), and the opt-in Tauri `.app` bundle gate: `CS01_TAURI_FULL_BUILD=1 bash scripts/tauri-gate.sh` produced `cs01-mini-redis-rust/web/src-tauri/target/release/bundle/macos/CS01 mini-redis.app`.
- DMG creation, signing, and notarization remain out-of-gate future release-engineering risks and are not claimed as complete for this tag-prep state.
- AUTH, TLS, replication, cluster, Lua, streams, modules, RDB, PSUBSCRIBE/PUNSUBSCRIBE, and full transaction semantics remain out of scope for cs01 `0.1.0`.
