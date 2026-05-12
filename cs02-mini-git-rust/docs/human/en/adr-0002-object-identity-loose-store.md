# ADR-0002 English abstract: Object identity and loose object store

> Full ADR: [docs/agent/adr/0002-object-identity-loose-store.md](../../agent/adr/0002-object-identity-loose-store.md).

## Decision

M1 implements Git-compatible blob identity and loose object storage directly:

- Object bytes are exactly `"<kind> <size>\0<payload>"`.
- v0.1.0 uses SHA-1 object IDs; SHA-256 remains a reserved abstraction path.
- `mg hash-object -w` writes zlib-compressed `.mg/objects/aa/bb...` loose objects.
- `mg cat-file -p` initially reads blob payloads.
- M1 may implement a minimal `mg init` that creates `.mg/objects`; full HEAD/ref/index/commit semantics stay in M3.

## Why

If M1 only implemented library helpers or a temporary mg-only format, the oracle would become self-authored. Moving minimal object-database initialization into M1 lets real `git cat-file -p` validate mg-written objects from the first implementation wave.

## Status

`accepted` — 2026-05-13.
