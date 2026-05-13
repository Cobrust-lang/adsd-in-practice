# Changelog — cs02-mini-git-rust

All notable changes to CS-02 are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). CS-02 records the finalized local `0.1.0` release documentation for the Git-compatible v0.1 subset.

## [0.1.0] - 2026-05-13

### Added

- Git-compatible blob identity and loose-object storage via `mg hash-object` / `mg cat-file`.
- Git index v2 compatibility plus `mg add` / `mg write-tree` for the supported regular-file subset.
- Minimal repository state with `.mg/HEAD`, refs, recursive tree writing, `mg commit-tree`, `mg commit -m`, and first-parent `mg log`.
- Real Git oracle with 1000 deterministic randomized path/content cases across M1-M3 functional coverage.
- M4 filesystem-hardening artifacts: ADR-0005, pre-release hardening finding, bilingual human abstracts, and methodology-status conclusions.

### Changed

- Public docs now explicitly claim a Git-compatible v0.1 subset rather than full `.mg` / `.git` interchangeability.
- Oracle coverage now includes M4 negative cases for internal-path rejection, lowercase SHA validation, symlink ancestry protection, lock cleanup, decoded-size caps, and index allocation bounds.

### Fixed

- Repository-owned writes for loose objects, refs, and index now use same-directory atomic temp-then-rename paths with ancestor-symlink rejection and parent-directory fsync.
- Index writes now use a minimal `.mg/index.lock` and clean it up on write failure.
- `mg add` now rejects repository-internal `.mg/**` / `.git/**` paths and symlink file inputs.
- Index parsing now rejects impossible entry counts, unsupported flags, and explicit over-cap allocations before `Vec` growth.
- Loose-object reads now cap decoded zlib size, and SHA-1 public validation rejects uppercase/non-lowercase object IDs.

### Release-readiness status

- CTO final audit on the cs02 line passed doc-coverage, cargo fmt, cargo clippy, cargo test, and the real Git oracle with M4 hardening negatives.
- The release claim is limited to the supported v0.1 subset: loose objects, index, trees, commits, repository discovery, and first-parent log.

### Known behavioral deltas vs full Git

- Unsupported in `0.1.0`: branch creation, checkout, merge, rebase, packfiles, remotes, ignore files, deletion tracking, symlink entries, submodules, and `git status`.
- `.mg` remains the primary repository directory name; real Git compatibility is validated through the supported subset and oracle exposure as `.git`.
- The lockfile and fsync discipline are intentionally minimal for v0.1 and not a byte-for-byte clone of all Git durability semantics.
