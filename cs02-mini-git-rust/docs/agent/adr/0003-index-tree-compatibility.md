---
adr: 0003
title: Index v2 and canonical tree compatibility
status: accepted
date: 2026-05-13
case: cs02-mini-git-rust
supersedes: none
last_verified_commit: pending
---

# ADR-0003: Index v2 and canonical tree compatibility

## Context

CS-02 M2 builds on ADR-0002's blob object identity boundary. The next slice is `mg add <path>` and `mg write-tree`: read worktree files, stage them in `.mg/index`, write missing blob objects, and encode a Git-compatible tree object that real Git can inspect.

This is the first point where CS-02 can accidentally pass unit tests while failing the real Git oracle. Git's index format is binary and packed; tree objects also have a binary payload where entries are sorted and object IDs are raw 20-byte SHA-1 values, not hex strings. Any JSON index, ad-hoc text index, sqlite store, or hex-in-tree encoding would be F24 primitive-as-everything.

## Options Considered

### Option A: Implement Git index v2 subset plus canonical tree objects from M2

- `.mg/index` uses the Git index v2 binary format: `DIRC`, version `2`, entry count, fixed metadata fields, path bytes, NUL padding to 8-byte alignment, and trailing SHA-1 checksum over the index bytes.
- `mg add <path>` writes blob loose objects through ADR-0002 code and writes/updates index entries.
- `mg write-tree` encodes canonical tree entries as `<mode> <name>\0<raw 20-byte object id>` and writes a tree loose object.
- M2 supports regular files at repository root first; recursive directories can land later if the flat path oracle is green.

Pros:
- Real `git ls-files --stage` can validate our index immediately.
- Real `git cat-file -p <tree>` can validate our tree object immediately.
- Avoids creating a throwaway index format that later needs migration.

Cons:
- Binary parsing/writing is more careful than a text index.
- Requires explicit path/mode/sort rules in code and tests.

### Option B: Use a temporary mg-only text index, then convert to Git index in M3

Pros:
- Faster first implementation.
- Easier to inspect manually.

Cons:
- Violates the case's core compatibility goal.
- Delays the oracle until after multiple layers depend on the wrong abstraction.
- Creates high migration risk for `add`, `write-tree`, and commit creation.

### Option C: Skip index in M2 and have `write-tree` scan the worktree directly

Pros:
- Avoids index binary format initially.
- Can produce a tree for simple demos.

Cons:
- Not Git semantics: `write-tree` writes the staged index, not the live worktree.
- Makes `mg add` meaningless and hides staging-state bugs until M3.
- Fails the planned `git ls-files --stage` oracle.

## Decision

Choose **Option A**.

M2 owns the staging and tree boundary:

1. `mg-core` adds an index module for Git index v2 read/write for regular files.
2. `mg add <path>` updates `.mg/index` and writes blob objects under `.mg/objects`.
3. `mg write-tree` reads `.mg/index`, writes a tree object, and prints its SHA-1.
4. The implementation validates against real Git via `git ls-files --stage`, `git write-tree`, and `git cat-file -p` on matching simple fixtures.
5. M2 may remain flat-file only if this is stated in tests and docs; recursive directory trees can be a later M2.x/M3 extension before release.

## Consequences

### Positive

- M2 remains externally checkable with real Git rather than self-authored fixtures.
- M3 commit creation can build on a real tree object instead of replacing a temporary staging model.
- The binary index reader/writer gives CS-02 a stronger ADSD stress test than cs01: file-format exactness plus checksum correctness.

### Negative / accepted debt

- M2 has to handle platform metadata carefully. The initial subset should only preserve the fields Git needs for regular-file staging; unsupported modes must fail clearly.
- Flat-file-only M2 is acceptable as an implementation slice, but the oracle must explicitly cover that boundary and not imply full recursive Git staging.
- Index locking/concurrent `mg add` is not solved in M2 unless a P9 implementation chooses to add atomic lockfile semantics; file-system race handling remains a likely finding candidate.

## Done Criteria

- [ ] `mg-core::index` can read and write Git index v2 files with checksum verification.
- [ ] `mg add <path>` stages regular files, writes blob loose objects, and preserves Git-compatible stage-0 index entries.
- [ ] `mg write-tree` writes a tree object whose payload real Git can pretty-print.
- [ ] `git ls-files --stage` can read an mg-written `.mg/index` after `.mg` is renamed or exposed as `.git`.
- [ ] For flat-file fixtures, `mg write-tree` and `git write-tree` produce the same tree SHA when staging the same files.
- [ ] The M2 oracle script includes fixed fixtures and at least 1000 deterministic randomized filenames/contents for add/write-tree compatibility.
- [ ] M2 gates pass: doc coverage, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and the Git oracle script.

## Cross-references

- ADR-0001: stack choice.
- ADR-0002: object identity and loose object storage.
- Local constitution §2 and §5: real Git oracle and at least 1000 randomized cases.
- Future ADR: full repository state machine, HEAD/refs, commits, log, and repository discovery.
