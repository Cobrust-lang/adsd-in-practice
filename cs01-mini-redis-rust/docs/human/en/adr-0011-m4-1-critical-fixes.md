# ADR-0011 English abstract: M4.1 critical fixes

> Full ADR: [docs/agent/adr/0011-m4-1-critical-fixes.md](../../agent/adr/0011-m4-1-critical-fixes.md).

## Decision

M4 splits into three waves: M4.1 critical fixes (this ADR), M4.2 doc-sweep + release artifacts, M4.3 v0.1.0 tag.

This wave: 13 critical items derived from the 8-agent audit team:

**Security / DoS**:
1. Default bind `127.0.0.1` (was `0.0.0.0`) + `--insecure-no-auth` stub doc
2. `--max-clients <N>` flag (default 1000) + early reject
3. `Frame::parse` array recursion depth limit `MAX_FRAME_DEPTH = 32`
4. AOF file `0o600` mode (cfg unix)

**Constitution drift fix** (F1 sub-pattern — first positive repair):
5. cs01 CLAUDE.md §1 accepts broadcast Pub/Sub (over-prohibit → ADR-0009 trade-off was sound)
6. cs01 CLAUDE.md §4 accepts `storage → protocol` single-direction edge (for AOF wire compat)

**AOF hardening**:
7. `FsyncPolicy::AlwaysBlocking` variant (`Reply::Ok` returns after `sync_data`) + `Always` semantics documented
8. AOF mpsc `channel(8192)` + `try_send` → `blocking_send` backpressure
9. `Store::replay_from_path` switches to `tokio::fs` streaming (no whole-file in RAM)

**Dispatch strictness**:
10. `parse_set` requires `parts.len() == 5` for EX form (reject trailing token + oracle fixture)

**Pub/Sub perf**:
11. `recv_any_subscription` uses `tokio_stream::StreamMap` (no box-per-poll)
12. `Store::subscribe` reads first, write-locks only on miss

**Comment cleanup**:
13. `main.rs:140-146` confused comment rewrite + `Frame::Integer` encoder comment fix

## Scope creep explicitly rejected

- AUTH command: **not** implemented (M5+ scope)
- TLS: ditto
- AOF rewrite: M5+
- MAXMEMORY policy: M5+

## Numeric targets

- Backend tests ≥ 280 (M3.2 baseline 269, +11)
- Oracle 36/36 (35 + 1 trailing-token)

## Status

`accepted` — 2026-05-12.
