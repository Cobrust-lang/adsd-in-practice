# ADR-0003 English abstract: Index v2 and canonical tree compatibility

> Full ADR: [docs/agent/adr/0003-index-tree-compatibility.md](../../agent/adr/0003-index-tree-compatibility.md).

## Decision

M2 implements the Git-compatible staging and tree boundary directly:

- `.mg/index` uses Git index v2 binary format with the `DIRC` header, entry metadata, path bytes, 8-byte padding, and trailing SHA-1 checksum.
- `mg add <path>` writes blob loose objects and stages regular files in the index.
- `mg write-tree` encodes canonical tree payloads as `<mode> <name>\0<raw 20-byte object id>`.
- M2 may start with flat regular files as the minimal slice; recursive directories can follow later, but the oracle must state the boundary honestly.

## Why

Git index and tree objects are binary compatibility boundaries. A JSON/text/sqlite temporary format would make M2 self-tested and force rework before commit/log. Implementing the index v2 subset now lets real `git ls-files --stage`, `git write-tree`, and `git cat-file -p` validate M2 from the start.

## Status

`accepted` — 2026-05-13.
