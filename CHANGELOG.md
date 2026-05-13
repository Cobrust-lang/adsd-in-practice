# Changelog

All notable changes to ADSD in Practice are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). This repository records the finalized cs01 `v0.1.0` release documentation; the annotated tag is created after this finalization commit.

## [0.1.0] - 2026-05-13

### Added

- CS-01 `mini-redis-rust` progressed from scaffold to a Redis-compatible subset with RESP TCP serving, string KV storage, TTL, command dispatch, Axum HTTP/SSE control plane, SvelteKit browser dashboard, Pub/Sub, AOF append/replay, and M4.1 hardening.
- Root release artifacts: dual-license text, ADSD-aware contribution guide, security disclosure guidance, changelog, and methodology status report.
- Bilingual ADR and finding mirrors for cs01 human docs, including release-readiness audit findings.

### Changed

- Documentation coverage gate now treats finding abstracts like ADR abstracts: every agent finding must have matching `docs/human/zh/finding-*.md` and `docs/human/en/finding-*.md` files.
- CS-01 M4.4 withdraws an accidental Tauri desktop packaging scope that came from a wrong-session requirement; the valid cs01 release surface is the Rust server plus SvelteKit browser dashboard.

### Fixed

- M4.1 closed pre-release critical issues around default bind address, max frame depth, max clients, stricter command parsing, AOF file mode / bounded channel semantics, AOF flush naming, and Pub/Sub lag policy documentation.
- M4.2 removes stale README, bootstrap, ADR metadata, and methodology placeholders surfaced by the 8-agent audit.
- M4.4 removes Tauri-specific code, dependencies, gate scripts, and release-readiness claims that entered cs01 from cross-session contamination.

### Release-readiness status

- CTO final audit on the cs01 line passed doc-coverage, cargo fmt, cargo clippy, cargo test, prior integration oracle coverage (23/23 RESP, 6/6 Pub/Sub, 7/7 AOF), and frontend browser gates.
- Desktop packaging, installers, signing, and notarization are not part of cs01 `0.1.0` readiness.
- AUTH, TLS, replication, cluster, Lua, streams, modules, RDB, PSUBSCRIBE/PUNSUBSCRIBE, and full transaction semantics remain out of scope for cs01 `0.1.0`.
