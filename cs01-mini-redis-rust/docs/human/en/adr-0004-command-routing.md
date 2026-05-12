# ADR-0004 English abstract: Command routing

> Full ADR: [docs/agent/adr/0004-command-routing.md](../../agent/adr/0004-command-routing.md).

## Decision

`redis-server::dispatch::from_frame(Frame) -> Result<Command, Reply>`:

- Frame → Command parsing lives in the **server** crate (not storage)
- Command name `to_ascii_uppercase()` then `match` — case-insensitive
- Unknown command / arg count mismatch → `Err(Reply::Error("ERR ..."))`, matching real Redis error strings
- Single match; adding a new command = one new arm

## Rejected alternatives

- **New `redis-commands` crate**: over-layered, not needed at M1.2
- **Storage parses RESP itself**: violates layered architecture (storage shouldn't know protocol)

## Status

`accepted` — 2026-05-12.
