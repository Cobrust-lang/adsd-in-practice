# ADR-0005 English abstract: M4 release filesystem hardening and documentation honesty

> Full ADR: [docs/agent/adr/0005-release-filesystem-hardening.md](../../agent/adr/0005-release-filesystem-hardening.md).

## Decision

M4 is a release-hardening sprint, not a new Git feature milestone. It hardens the local-filesystem and parser boundary around the v0.1 supported subset:

- move loose object, index, and ref writes to same-directory temp-then-rename paths, refusing final-path symlink overwrite where practical;
- add a minimal `.mg/index.lock` around index updates so concurrent `mg add` / `mg commit` cannot silently interleave writes;
- make `mg add` reject repository-internal `.mg/**` and `.git/**` paths;
- bound index entry counts by file length before allocating entry vectors;
- cap decoded loose-object zlib inflation;
- reject index flags unsupported by v0.1;
- require lowercase 40-character SHA-1 hex in public validation paths;
- update README and human docs to claim Git-compatible v0.1 subset behavior, not full `.mg` / `.git` interchangeability, and remove unsupported `ls-files` claims.

## Why

cs02's core domain is filesystem-backed Git state. M1-M3 passed the functional oracle, but the pre-release audit found write-atomicity, symlink, concurrency, bounded-parsing, repository-internal-path, and documentation-overclaim gaps. Shipping `0.1.0` before closing these would turn HIGH/MED audit evidence into release debt.

## Status

`accepted` — 2026-05-13.
