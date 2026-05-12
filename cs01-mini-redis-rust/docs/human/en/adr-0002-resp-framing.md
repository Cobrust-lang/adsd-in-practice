# ADR-0002 English abstract: RESP framing strategy

> Full ADR: [docs/agent/adr/0002-resp-framing.md](../../agent/adr/0002-resp-framing.md).

## Decision

`redis-protocol` provides a **one-shot + Incomplete sentinel** parser:

```rust
pub fn parse(input: &[u8]) -> Result<(Frame, usize), ProtocolError>;
```

- Success → `(Frame, bytes_consumed)`
- Buffer too short → `Err(Incomplete)`
- Protocol error → `Err(Invalid("reason"))`

The caller (`redis-server`) loops `parse + advance` until Incomplete, then waits for more bytes.

## Rejected alternatives

- **nom**: RESP is too simple; nom is overkill and slows compile time
- **Streaming state machine**: premature optimization, not needed before M3

## v0.2 optimization direction

Change `BulkString(Option<Vec<u8>>)` to `BulkString(Option<Bytes>)` for zero-copy on large values.

## Status

`accepted` — 2026-05-12.
