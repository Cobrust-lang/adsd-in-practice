# ADR-0010 English abstract: M3.2 AOF persistence

> Full ADR: [docs/agent/adr/0010-m3-2-aof-persistence.md](../../agent/adr/0010-m3-2-aof-persistence.md).

## Decision (compact)

| Sub-item | Choice |
|---|---|
| AOF format | **RESP-encoded `Frame::to_bytes()`** — identical to Redis, can `redis-cli --pipe` replay |
| Which commands hit AOF | **SET/DEL/EXPIRE/PERSIST/INCR/DECR** (6 writables); read-only commands skip; SUBSCRIBE/UNSUBSCRIBE/PUBLISH skip (volatile) |
| Write path | **Hook inside `Store::execute`** + mpsc to background writer task (no blocking on the RESP path) |
| Fsync policy | `--aof-fsync` flag, **`everysec` default** + `always` / `no` available |
| TTL drift | Persist relative `EXPIRE k seconds`; replay recomputes from current clock; <1s drift accepted (matches real Redis) |
| Corrupted tail | Log warn + treat file length as truncation point (candidate finding) |
| Replay order | **Replay before binding listeners** (deterministic ready signal) |
| Oracle | New `tests/oracle_aof.py`: restart round-trip vs `real redis --appendonly yes`, 7 fixtures |

## Numeric targets

- Backend tests ≥ 260 (M3.1 baseline 243, +17+)
- Oracle matrix 35/35 (22 RESP + 6 pubsub + 7 AOF restart)

## Accepted debt

- AOF rewrite (compact same-key duplicates) deferred to M4 / v0.2
- Corrupted tail warn-and-truncate: M4 will decide whether to upgrade to refuse-to-start
- Replay-before-accept means large AOFs slow startup (< 100MB acceptable)

## Status

`accepted` — 2026-05-12.
