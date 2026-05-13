---
adr: 0005
title: M4 release filesystem hardening and documentation honesty
status: accepted
date: 2026-05-13
case: cs02-mini-git-rust
supersedes: none
last_verified_commit: pending
---

# ADR-0005: M4 release filesystem hardening and documentation honesty

## Context

CS-02 M1-M3 now has the v0.1.0 functional slice: Git-compatible blob/tree/commit loose objects, Git index v2, recursive regular-file staging, repository discovery, current-branch commits, and first-parent log. The M4 pre-release audit found no BLOCK issue, but it did find HIGH/MED release-readiness gaps in the local-filesystem domain.

This case is intentionally about filesystem-backed state. A `0.1.0` claim is not credible if object, index, and ref writes can be interrupted into partial files, if obvious repository-internal paths can be staged, or if crafted local files can force unbounded allocations before validation.

The same audit also found documentation overreach: README and human docs imply `.mg/` and `.git/` are fully interchangeable and list unsupported `ls-files`. The implemented promise is narrower: the v0.1 supported loose-object/index/tree/commit/log subset is readable by real Git under the oracle path.

## Options Considered

### Option A: Harden the v0.1 filesystem boundary before release

- Add repository-local safe write helpers for loose objects, refs, and index files.
- Use temp-then-rename writes and avoid following existing final-path symlinks where practical.
- Add a minimal `.mg/index.lock` lockfile around index updates.
- Reject `.mg/**` and `.git/**` staging paths.
- Bound index entry counts by file length before allocation.
- Bound decoded loose-object inflation.
- Reject unsupported index flag bits and non-lowercase SHA-1 input in v0.1 public parsing paths.
- Correct public docs to describe subset compatibility and remove unsupported commands.

Pros:
- Closes the highest-risk M4 audit items in the case's core domain.
- Preserves the current v0.1 command surface instead of adding new Git features during release hardening.
- Makes public docs match the actual oracle-backed contract.

Cons:
- Adds implementation work after the functional milestone is already green.
- Still does not make cs02 a complete Git implementation.
- Locking remains minimal and local; it is not a cross-platform replacement for every Git lockfile nuance.

### Option B: Ship M3 as v0.1.0 and document hardening as future work

Pros:
- Fastest path to a tag.
- Avoids touching already-green object/index/repo code.

Cons:
- Ships known filesystem-state hazards in a case whose central domain is filesystem state.
- Turns a HIGH pre-release finding into accepted release debt without a strong reason.
- Encourages ADSD sediment: "release-ready" docs would not match the audit evidence.

### Option C: Expand M4 into a broader Git compatibility milestone

- Add branch creation, status, ls-files, deletion tracking, packfiles, or more Git ref semantics while hardening.

Pros:
- Could reduce the gap between `.mg` and `.git` in user perception.

Cons:
- Violates release-hardening scope control.
- Increases the chance of new compatibility bugs while trying to close pre-release findings.
- Confuses v0.1.0 readiness with a feature expansion.

## Decision

Choose **Option A**.

M4 is a release-hardening sprint, not a new feature milestone. The implementation must keep the v0.1 command surface and harden the filesystem/parsing/documentation boundary around that surface:

1. Add internal filesystem write helpers for repository-owned files where practical:
   - write to a same-directory temp file;
   - sync and rename into place;
   - refuse final paths that are symlinks before replacement;
   - keep loose-object, index, and ref writes inside `.mg/` paths already owned by `Repository` / object helpers.
2. Add a minimal `.mg/index.lock` lockfile during index update paths so concurrent `mg add` / `mg commit` cannot interleave writes silently.
3. Reject repository-internal staging paths whose first path component is `.mg` or `.git`.
4. Before allocating index entries, prove the file is large enough to contain the declared entry count's minimum fixed-size records and reject impossible counts.
5. Bound loose-object decoded size during zlib inflation before hashing and decoding.
6. Reject unsupported Git index flags for the v0.1 parser instead of silently accepting entries with semantics cs02 does not implement.
7. Require lowercase 40-character SHA-1 hex in v0.1 validation paths. Uppercase input is not accepted or normalized in this release.
8. Correct README and human docs to say "Git-compatible v0.1 subset" rather than full `.mg` / `.git` interchangeability, and remove unsupported `ls-files` claims.
9. Keep out of scope: branch creation, checkout, merge, rebase, packfiles, remotes, ignore files, deletion tracking, symlink entries, submodules, and `git status`.

## Consequences

### Positive

- The v0.1 release claim is tied to the actual pre-release audit evidence.
- Filesystem failure modes become explicit tests instead of hidden release debt.
- Public documentation becomes narrower but more trustworthy.

### Negative / accepted debt

- Atomic rename and a minimal lockfile are not a complete clone of Git's lockfile and fsync discipline.
- A decoded-object size cap means cs02 intentionally rejects very large loose objects in v0.1.
- Strict lowercase SHA input is less permissive than some user expectations but keeps object identity canonical for this release.
- `.mg` remains the primary repository directory name; real Git compatibility is validated by exposing that supported subset as `.git` in the oracle.

## Done Criteria

- [x] Finding `m4-pre-release-filesystem-hardening` is indexed in agent and human docs.
- [x] ADR-0004 is stamped with the M3 merge SHA and all M3 done criteria checked.
- [x] Loose-object writes use an atomic same-directory temp-then-rename path, reject symlink targets and ancestor symlink redirection, and fsync the parent directory after rename.
- [x] Ref writes use an atomic same-directory temp-then-rename path, validate the supported current-branch ref path, reject symlink ancestors, and fsync the parent directory after rename.
- [x] Index writes use `.mg/index.lock`, atomic same-directory replacement, and remove the lock on write failure.
- [x] `mg add` rejects `.mg/**` and `.git/**` paths from repository root and subdirectories.
- [x] Index parsing rejects impossible entry counts and an explicit v0.1 cap before allocating entry vectors.
- [x] Loose-object reads enforce a decoded-size cap during zlib inflation.
- [x] Index parsing rejects unsupported index flags for v0.1.
- [x] SHA-1 validation rejects uppercase/non-lowercase object IDs in public parsing paths.
- [x] README and human docs describe v0.1 subset compatibility and remove `ls-files` claims.
- [x] The oracle covers the new M4 negative cases and still passes at least 1000 deterministic randomized path/content cases.
- [x] M4 gates pass: doc coverage, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and the Git oracle script.

## Cross-references

- Finding: `docs/agent/findings/m4-pre-release-filesystem-hardening.md`.
- ADR-0002: object identity and loose object storage.
- ADR-0003: Git index v2 and canonical tree compatibility.
- ADR-0004: repository state, commits, and first-parent log.
- Local constitution §2: real Git oracle is mandatory.
