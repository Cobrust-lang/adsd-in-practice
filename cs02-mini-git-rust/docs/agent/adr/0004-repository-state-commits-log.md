---
adr: 0004
title: Repository state, commits, and first-parent log
status: accepted
date: 2026-05-13
case: cs02-mini-git-rust
supersedes: none
last_verified_commit: pending
---

# ADR-0004: Repository state, commits, and first-parent log

## Context

CS-02 M3 turns the object/index/tree slices from M1/M2 into a usable minimal repository: `mg init`, `mg commit-tree`, `mg commit -m`, `mg log`, and repository discovery from subdirectories.

M2 deliberately wrote a small `.mg/HEAD` and `.mg/config` so real Git could inspect `.mg/index` during the oracle. That was a compatibility shim for the index gate, not a complete repository state model. M3 must make this explicit and coherent: refs, HEAD, commit objects, parent selection, and log traversal must be first-class library behavior rather than ad-hoc CLI file writes.

M2 also accepted flat-file-only staging. Repository discovery and a useful `commit` command make that boundary too visible: running `mg add src/lib.rs` from a subdirectory should either work like Git or fail as a documented release limitation. For v0.1.0, the better ADSD move is to close the flat-only debt before release rather than carry a surprising core workflow gap into M4.

## Options Considered

### Option A: Implement a minimal Git-compatible repository state machine in M3

- `mg-core::repo` owns discovery, `.mg` layout, HEAD/ref reads and writes, and worktree-relative path normalization.
- `mg init` creates a Git-compatible non-bare repository skeleton: `.mg/objects`, `.mg/refs/heads`, `.mg/HEAD`, and `.mg/config`.
- `mg add <path>` uses repository discovery and stores slash-separated worktree-relative paths in index v2.
- `mg write-tree` supports recursive tree objects for staged regular files.
- `mg commit-tree <tree> [-p parent] -m <msg>` writes Git-compatible commit objects using Git identity/date environment variables when present.
- `mg commit -m <msg>` writes the current index tree, selects current HEAD as parent if present, writes the commit, and advances the current branch ref.
- `mg log` traverses first-parent commits from HEAD and prints a stable subset comparable with `git log --oneline` / `git cat-file -p` evidence.

Pros:
- Closes the M2 flat-only debt before release.
- Keeps the repository state model in `mg-core`, not scattered in CLI helpers.
- Enables strong oracle checks: `git rev-parse HEAD`, `git cat-file -p`, `git log`, and `git ls-tree` can read mg-written repositories after `.mg` is exposed as `.git`.

Cons:
- Larger M3 than a pure commit-object slice.
- Requires careful env/date handling so commit SHA oracle can be deterministic.
- Still does not implement branches beyond the current `refs/heads/main` branch.

### Option B: Keep M3 flat-only and implement commits/log on top of the M2 subset

Pros:
- Faster and lower risk for the next implementation sprint.
- Avoids changing index/tree code again immediately.

Cons:
- Leaves a surprising workflow gap for `mg add dir/file` and subdirectory use.
- Makes repository discovery less useful.
- Likely becomes a M4 audit finding because README claims `.mg`/`.git` compatibility without highlighting flat-only staging.

### Option C: Implement commit objects only, defer porcelain commit/log/repo discovery

Pros:
- Keeps M3 small and plumbing-focused.
- `commit-tree` can be oracle-checked against Git with fixed env.

Cons:
- Fails the stated CS-02 v0.1.0 command list (`mg init`, `mg add`, `mg commit -m`, `mg log`).
- Pushes core repository state semantics into release-hardening time.
- Delays the ADSD stress point this case was selected for: filesystem state + object graph coherence.

## Decision

Choose **Option A**.

M3 owns the repository state boundary:

1. Move repository discovery and `.mg` layout into `mg-core::repo`.
2. Treat `.mg/HEAD` as a symbolic ref to `refs/heads/main` for v0.1.0; detached HEAD and multiple branches remain out of scope.
3. Extend index/tree handling from flat regular files to slash-separated regular-file paths and recursive tree objects.
4. Implement Git-compatible commit object encoding.
5. Use Git author/committer environment variables for deterministic oracle compatibility:
   - `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `GIT_AUTHOR_DATE`
   - `GIT_COMMITTER_NAME`, `GIT_COMMITTER_EMAIL`, `GIT_COMMITTER_DATE`
   - fallback values are acceptable for manual local use, but oracle must pin env/date.
6. Keep `mg log` to first-parent traversal from current HEAD; merge commits and branch selection are out of scope for v0.1.0.

## Consequences

### Positive

- `mg init && mg add && mg commit -m && mg log` becomes a real end-to-end repository workflow.
- Real Git can validate mg-written commits, trees, blobs, refs, and logs.
- M4 release work can focus on audit/doc/release status instead of a known core workflow gap.

### Negative / accepted debt

- The implementation still does not support branch creation, checkout, merge, rebase, packfiles, remotes, ignore files, symlinks, submodules, or file deletion tracking.
- Commit timestamps make SHA comparisons fragile unless the oracle pins identity/date env exactly.
- Index locking/concurrent `mg add` remains a known filesystem-race candidate unless M3 implementation adds lockfile semantics.

## Done Criteria

- [ ] ADR-0003 is stamped with the M2 merge SHA and all M2 done criteria checked.
- [ ] `mg-core::repo` discovers `.mg` upward from cwd and exposes worktree root / git dir paths.
- [ ] `mg init` creates `.mg/objects`, `.mg/refs/heads`, `.mg/HEAD`, and `.mg/config` through library code.
- [ ] `mg add <path>` works from repository root and subdirectories for regular files, storing slash-separated paths in index v2.
- [ ] `mg write-tree` writes recursive tree objects for slash-separated staged paths.
- [ ] `mg commit-tree <tree> [-p parent] -m <msg>` writes a commit object matching real Git under pinned identity/date env.
- [ ] `mg commit -m <msg>` writes the index tree, uses current HEAD as parent when present, writes the commit, and advances `refs/heads/main`.
- [ ] `mg log` traverses first-parent commits from HEAD and shows enough stable data to verify commit history.
- [ ] The oracle verifies mg-written repositories with real Git: `git rev-parse HEAD`, `git cat-file -p`, `git ls-tree -r`, and `git log`.
- [ ] The oracle keeps at least 1000 deterministic randomized regular-file path/content cases.
- [ ] M3 gates pass: doc coverage, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and the Git oracle script.

## Cross-references

- ADR-0002: object identity and loose object store compatibility.
- ADR-0003: index v2 and canonical tree compatibility.
- Local constitution §2: real Git oracle is mandatory.
- CS-02 README command list for v0.1.0.
