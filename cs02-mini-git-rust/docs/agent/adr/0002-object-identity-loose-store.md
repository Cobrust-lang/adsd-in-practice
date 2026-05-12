---
adr: 0002
title: Object identity and loose object store compatibility
status: accepted
date: 2026-05-13
case: cs02-mini-git-rust
supersedes: none
last_verified_commit: 7de4224b092ab5f7738f779ae2ebb7d4e0c69dc8
---

# ADR-0002: Object identity and loose object store compatibility

## Context

CS-02 M1 must make `mg hash-object`, `mg hash-object -w`, and `mg cat-file -p` byte-compatible with real Git for blob objects before index/tree/commit work starts. This is the first point where F23-A can bite: if we author our own tests without using the real `git` binary as oracle, `mg` may look correct while writing objects Git cannot read.

The original scaffold placed full `mg init` in M3 with commit/log work. M1 object commands expose a narrower dependency: `hash-object -w` and `cat-file` need an object database path, but they do not need HEAD, refs, index, commit creation, or repository discovery yet.

## Options Considered

### Option A: Git-compatible loose objects from M1 with a minimal object-database init

- Encode objects exactly as `"<kind> <size>\0<payload>"`.
- Hash that exact byte stream with SHA-1 for v0.1.0.
- Compress the same byte stream with zlib and write `.mg/objects/<first-two-hex>/<remaining-hex>`.
- Promote a minimal M1 `mg init` that creates `.mg/objects` only; full HEAD/ref/index semantics remain M3.

Pros:
- `git cat-file -p` can validate objects immediately by renaming `.mg` to `.git` or by pointing Git at the object directory.
- Keeps F24 honest: no JSON, sqlite, uncompressed object files, or mg-only sidecar metadata.
- Makes `cat-file` and `hash-object -w` useful in M1 without prematurely implementing index/commit.

Cons:
- Slightly changes the scaffold milestone order by pulling minimal init earlier.
- Requires clear documentation so M1 `init` is not mistaken for full repo semantics.

### Option B: Pure library object writing in M1, CLI write/read delayed until M3

Pros:
- Preserves the original milestone order exactly.
- Keeps M1 surface smaller.

Cons:
- Defers the real oracle path, because users cannot run `mg hash-object -w` / `mg cat-file -p` end-to-end.
- Increases risk that library tests become self-authored fixtures rather than Git-compatible behavior.

### Option C: Temporary mg-only object database and conversion later

Pros:
- Fastest implementation path.
- Can avoid zlib and path layout until later.

Cons:
- Violates CS-02 F24 constraints.
- Creates migration debt in the core identity layer.
- Would make M1 evidence meaningless for the stated goal: `.mg/` compatibility with `.git/`.

## Decision

Choose **Option A**.

M1 owns the blob object identity boundary:

1. `mg-core` exposes object header construction, SHA-1 object ID computation, zlib loose-object write, and zlib loose-object read for blob objects.
2. `mg hash-object <path>` prints the same SHA-1 as `git hash-object <path>`.
3. `mg hash-object -w <path>` writes a zlib-compressed loose object under `.mg/objects`.
4. `mg cat-file -p <sha>` reads the loose object and prints the payload.
5. M1 may implement a **minimal** `mg init` that creates `.mg/objects`; M3 remains responsible for HEAD, refs, index, commit, log, and repository discovery.

SHA-1 remains the v0.1.0 object ID algorithm. SHA-256 stays behind the hash abstraction reserved by ADR-0001, not as a second implementation path in M1.

## Consequences

### Positive

- The first behavior slice is externally checkable with the real `git` binary.
- Object identity becomes a stable primitive before tree/index/commit layers build on it.
- The minimal init scope keeps M1 small while avoiding unusable CLI write/read commands.

### Negative / accepted debt

- M1 `mg init` is intentionally incomplete. It must not claim full repository initialization until M3.
- The `cat-file` implementation can initially support pretty-printing blob payloads only; tree/commit decoding lands in later waves.
- Packfile support remains out of scope for v0.1.0.

## Done Criteria

- [x] `Cargo.lock` is committed so `--locked` gates can run for cs02.
- [x] ADR-0001's raw-SHA vs Git-object-SHA wording is corrected.
- [x] `mg-core` has tests for blob object ID compatibility with Git fixtures, including empty payload and `hello`.
- [x] `mg hash-object <file>` matches `git hash-object <file>` on fixed fixtures and at least 1000 randomized file contents.
- [x] `mg hash-object -w <file>` writes a zlib loose object that real Git can read as the same blob.
- [x] `mg cat-file -p <sha>` can read mg-written and Git-written loose blob objects.
- [x] M1 gates pass: doc coverage, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and the Git oracle script for M1.

## Cross-references

- ADR-0001: stack choice for RustCrypto SHA-1, SHA-2 reserve path, flate2, clap, anyhow/thiserror.
- Local constitution §1 and §2: Git-compatible loose object layout and real `git` oracle are non-negotiable.
- Future ADR-0004: index format must build on this object identity boundary.
