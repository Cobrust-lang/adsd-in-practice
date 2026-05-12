# ADR-0004 English abstract: Repository state, commits, and log

> Full ADR: [docs/agent/adr/0004-repository-state-commits-log.md](../../agent/adr/0004-repository-state-commits-log.md).

## Decision

M3 turns cs02 from object/index/tree slices into a minimal usable repository state machine:

- `mg-core::repo` owns upward `.mg` discovery, initialization layout, and HEAD/ref reads and writes.
- `mg init` creates Git-compatible `.mg/objects`, `.mg/refs/heads`, `.mg/HEAD`, and `.mg/config`.
- `mg add <path>` uses repo discovery to compute worktree-relative paths and supports slash-separated regular-file paths.
- `mg write-tree` writes recursive tree objects from the index.
- `mg commit-tree` / `mg commit -m` write Git-compatible commit objects, and porcelain commit advances `refs/heads/main`.
- `mg log` traverses first-parent history from HEAD.

## Why

M2 already wrote minimal HEAD/config so real Git could read the index. M3 must turn that into an explicit repository-state model instead of leaving it as CLI helper behavior. Flat-only staging is also too visible for v0.1.0 `mg add` and repository discovery, so M3 closes it before release hardening.

## Status

`accepted` — 2026-05-13.
