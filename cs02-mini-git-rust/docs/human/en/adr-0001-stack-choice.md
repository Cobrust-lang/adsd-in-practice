# ADR-0001 English abstract: Stack choice

> Full ADR: [docs/agent/adr/0001-stack-choice.md](../../agent/adr/0001-stack-choice.md).

## Decision

- **Hash**: RustCrypto (`sha1` / `sha2` crates)
- **zlib**: `flate2` (default miniz_oxide backend, pure Rust)
- **CLI**: `clap` derive
- **Errors**: `anyhow` + `thiserror`

## Why

- **Cross-compile is a hard constraint** (wasm/arm/windows), so **pure-Rust path only** — excludes C bindings (`ring`, `openssl-sys`, `libz-sys`).
- Aligned with cs01 / Cobrust Studio stack.
- RustCrypto sha1 is ~30% slower than OpenSSL but adequate for git workloads.

## Status

`accepted` — 2026-05-12.
