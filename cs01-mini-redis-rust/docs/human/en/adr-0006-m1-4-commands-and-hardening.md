# ADR-0006 English abstract: M1.4 command expansion + hardening

> Full ADR: [docs/agent/adr/0006-m1-4-commands-and-hardening.md](../../agent/adr/0006-m1-4-commands-and-hardening.md).

## Decision

M1.4 ships all 8 remaining cs01 Wave M1 items in one sprint:

1. **EXPIRE / TTL / PERSIST**: DelayQueue uses Option A (let stale entries naturally `skip` in the expiry task), TTL semantics strictly aligned with Redis (-2 / -1 / remaining seconds)
2. **TYPE**: new `Reply::SimpleString(String)` variant; v0.1 emits only `"string"` / `"none"`
3. **KEYS pattern**: self-implemented ~50 LOC glob matcher with `*` / `?` / `[a-z]` / `\` escape (F24 defence: no globset)
4. **PING optional message**: extend `Command::Ping { message: Option<Vec<u8>> }`
5. **max-frame-size guard**: 512 MiB default + `--max-frame-size` CLI flag (F5 hardening)
6. **docker oracle**: `tests/oracle.sh` does real `redis:7-alpine` round-trip (F23-A); opt-in via `CS01_RUN_ORACLE=1`
7. **Target test count ≥ 100** (M1.3 is at 82)
8. **All 5 gates green** including `bash ../_shared/doc-coverage.sh`

## Rejected alternatives

- DelayQueue Option B with true cancel (premature optimisation)
- KEYS via `globset` crate (over-dependency + semantic mismatch)
- KEYS degraded to `*` / `?` only (F23-A oracle would fail immediately)
- `Reply::Bulk` faking SimpleString (bytes differ, F24)

## Status

`accepted` — 2026-05-12.
