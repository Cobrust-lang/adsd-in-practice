# ADR-0005 English abstract: RESP TCP listener

> Full ADR: [docs/agent/adr/0005-tcp-listener.md](../../agent/adr/0005-tcp-listener.md).

## Decision

Classic `TcpListener::accept` + per-connection `tokio::spawn(handle_conn)` + `BytesMut` accumulation + manual drain of `Frame::parse` until `Incomplete`:

- Each connection in its own task — natural fault isolation
- No `tokio_util::codec::Framed` (that wraps ADR-0002's pure-function parser in another layer; F24 candidate)
- `Reply → Frame` mapping lives in `redis-server::encode` as a free function (storage must not know RESP — same layering rationale as ADR-0004)
- Protocol error: emit `-ERR ...` then close socket
- Graceful shutdown: M1.3 simple version — `tokio::signal::ctrl_c()` stops accept loop, in-flight tasks finish naturally; M3 upgrades to drain mode
- E2E tests use in-process `tokio::net::TcpStream::connect`, no docker; real oracle deferred to M1.4
- Also lands `ECHO / SELECT 0 / QUIT` (cs01 CLAUDE.md §3 lists them in M1)

## Rejected alternatives

- **`tokio_util::codec::Framed`**: extra abstraction layer, violates cs01 §1 "no framework masking of primitives"
- **`monoio` / `tokio-uring` zero-copy**: conflicts with ADR-0001 (tokio lock-in), deferred to v0.2

## Accepted debt

- Single accept loop: fine below ~50k long connections, re-evaluate at M3
- Unbounded `BytesMut`: malicious `$<u64::MAX>` could trigger a large alloc — **M3 adds a max-frame-size guard** (finding candidate)
- Graceful shutdown lacks in-flight deadline, upgraded at M3

## Status

`accepted` — 2026-05-12.
