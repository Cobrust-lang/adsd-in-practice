---
finding: m4-pre-release-filesystem-hardening
date: 2026-05-13
case: cs02-mini-git-rust
severity: high
status: accepted
---

# Finding: M4 pre-release audit found filesystem-hardening and documentation gaps

- **Milestone**: M4
- **Date**: 2026-05-13
- **Severity**: High
- **Status**: accepted for release-hardening sprint
- **Related ADR**: ADR-0005

## Summary

CS-02 M1-M3 reached functional Git compatibility for the supported subset, but the pre-release audit found release-readiness gaps around filesystem writes, bounded parsing, repository-internal path handling, and public documentation claims.

No BLOCK issue was found, but the HIGH/MED findings should be closed before claiming `0.1.0` readiness because this case's core domain is local filesystem state.

## Evidence

### HIGH: unguarded repository writes

The audit flagged direct writes for loose objects, refs, and the index:

- `crates/mg-core/src/object.rs`: loose object writes use direct filesystem writes.
- `crates/mg-core/src/repo.rs`: current branch refs are written directly.
- `crates/mg-core/src/index.rs`: `.mg/index` is written directly.

Risks:

- symlink replacement under `.mg/` can redirect writes outside the repository;
- interrupted writes can leave partially written index/ref/object files;
- concurrent `mg add` / `mg commit` can corrupt `.mg/index` or refs.

### MED: malicious index entry count can allocate too much memory

The index reader trusts the file header entry count enough to allocate a vector before proving the file can contain that many entries. A crafted `.mg/index` can set a huge count in a short file and cause memory pressure before the parser rejects it.

### MED: loose-object decompression is unbounded

`cat-file` / `log` inflate loose objects before enforcing a decoded-size cap. A malicious zlib object can consume excessive memory before hash validation finishes.

### MED: repository-internal paths can be staged

Repository discovery canonicalizes paths under the worktree, but it does not reject `.mg/**` or `.git/**` paths. A user can accidentally stage repository internals such as `.mg/HEAD` or `.mg/index`.

### LOW: public docs over-claim compatibility

The human guides currently say `.mg/` is interchangeable with `.git/`, and list `ls-files` as supported. The CLI does not implement `ls-files`, and compatibility is only for the v0.1 supported loose-object/index/tree/commit/log subset.

## Decision

Create ADR-0005 for the M4 release-hardening sprint:

- add atomic/no-follow or temp-then-rename repository write helpers where practical;
- add a minimal `.mg/index.lock` lockfile for index updates;
- reject `.mg/**` and `.git/**` staging paths;
- cap index entry count by file length before allocation;
- cap decoded loose-object size;
- strengthen refname validation for the v0.1 supported current branch path;
- correct README/human docs to describe subset compatibility and remove `ls-files` claims.

## Cross-references

- ADR-0002: object identity and loose object storage.
- ADR-0003: Git index v2 and tree compatibility.
- ADR-0004: repository state, commits, and first-parent log.
