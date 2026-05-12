---
adr: 0013
title: M4.3 — Tauri desktop frontend becomes the primary release surface
status: accepted
date: 2026-05-13
case: cs01-mini-redis-rust
supersedes: ADR-0008 rust-embed release target; ADR-0012 M4.3 rust-embed-only framing
last_verified_commit: pending
---

# ADR-0013: Tauri desktop frontend becomes the primary release surface

## Context

ADR-0008 chose SvelteKit + adapter-static because the M2.2 goal was a fast frontend iteration loop and a future M4 path to `rust-embed`. ADR-0012 then framed the M4.3 release as doc sweep + release artifacts followed by a rust-embed/single-binary release readiness pass.

The product direction changed on 2026-05-13: the user explicitly wants the frontend implemented with **Tauri**. That is a strategic surface change, not a cosmetic packaging detail. If we keep pushing the old rust-embed release plan, we optimize for a browser-served admin page while the desired product shape is a desktop app.

The existing SvelteKit UI is still valuable. Tauri can consume the same Vite/Svelte frontend; the decision is about the shell, backend lifecycle, release artifacts, and gates. This ADR therefore supersedes the rust-embed-as-primary-release-target parts of ADR-0008 / ADR-0012, but does **not** throw away the M2.2 SvelteKit work.

A local constraint also changed: disk space is tight (`df -h .` showed the workspace volume at 93% used, 14 GiB available). Tauri, Rust workspaces, pnpm, and multi-worktree builds can easily create multi-GB `target/`, bundle, `node_modules`, and Vite cache output. The Tauri migration must explicitly control build artifacts.

## Options Considered

### Option A: Keep rust-embed as the primary M4.3 release target

- **Pros**:
  - Minimal change to existing ADR-0008 / ADR-0012 plan.
  - Keeps a single `redis-server` binary with HTTP + RESP + static UI.
  - Lowest new dependency surface.
- **Cons**:
  - Contradicts the new product direction.
  - Browser-served admin UI is less differentiated as an ADSD case surface than desktop packaging + local process management.
  - Still leaves macOS/desktop first-run UX untested.

### Option B: Tauri desktop app with `redis-server` as a managed sidecar

- **Pros**:
  - Reuses existing SvelteKit/Vite UI with minimal frontend rewrite.
  - Reuses existing `redis-server` CLI and `/api/*` HTTP/SSE control plane without forcing a library-lifecycle refactor in the release sprint.
  - Makes process lifecycle, local-only binding, logs, and first-run UX explicit desktop concerns.
  - Keeps `redis-cli`/oracle behavior intact because RESP remains served by the same binary.
- **Cons**:
  - Release artifact is no longer a single Rust binary; it is a desktop bundle plus sidecar server binary.
  - Sidecar lifecycle needs careful shutdown, port selection, and error reporting.
  - Tauri bundle builds are heavier and must be guarded by storage discipline.

### Option C: Tauri desktop app embeds the Redis backend in-process as a Rust library

- **Pros**:
  - Cleanest long-term desktop architecture: one process, no sidecar binary lifecycle.
  - Enables Tauri commands/events instead of localhost HTTP for management features.
  - Avoids port conflicts for the control plane.
- **Cons**:
  - Requires extracting a reusable server lifecycle API from `redis-server/src/main.rs` into the library surface.
  - Increases M4.3 risk because async runtime ownership, shutdown, and testability become implementation work.
  - Risks turning the release sprint into a backend refactor instead of a packaging/surface migration.

### Option D: Tauri shell only; require users to start `redis-server` manually

- **Pros**:
  - Fastest possible Tauri scaffold.
  - Minimal backend changes and minimal sidecar configuration.
- **Cons**:
  - Poor product UX; the desktop app would fail first-run unless a separate terminal command is already running.
  - Does not validate the desktop lifecycle problem that makes Tauri strategically interesting.
  - Looks like a wrapper demo rather than a coherent release artifact.

## Decision

**Chosen: Option B — Tauri desktop app with `redis-server` as a managed sidecar for v0.1.0.**

Rationale:

1. It satisfies the strategic direction while preserving the existing SvelteKit UI and Redis-compatible backend.
2. It avoids premature backend lifecycle refactoring in M4.3; Option C is better as a v0.2 architecture cleanup after the Tauri sidecar UX is proven.
3. It keeps the F23-A oracle path stable: `redis-cli` still talks to the same `redis-server` binary, so protocol compatibility evidence remains comparable to M1-M4.
4. It turns M4.3 into a bounded release-surface sprint: Tauri shell, sidecar lifecycle, docs/gates, and release artifacts — not a rewrite of storage/protocol/server.
5. It respects the disk constraint by deferring full Tauri bundle builds to explicit release verification, not every local check.

### Scope boundaries

- `web/` remains the SvelteKit source of truth for UI pages.
- Add a Tauri app under the existing frontend tree unless P9 finds a clearly simpler Tauri-v2 convention; the default expected path is `cs01-mini-redis-rust/web/src-tauri/`.
- The Tauri app manages a local `redis-server` sidecar bound to loopback, with explicit logs and failure states.
- Browser dev mode remains supported for fast frontend iteration (`pnpm dev` + backend on `--http-port 6381`).
- `rust-embed` is no longer a v0.1.0 blocker. It may remain as a dependency until a later cleanup ADR removes or repurposes it.
- Do **not** introduce AUTH/TLS in this sprint; M4.1 already documents the local-bind security posture, and desktop local-only sidecar keeps the same boundary.

### Storage / artifact discipline

- P9 must report `df -h .` before and after heavy validation.
- Do not run full `pnpm tauri build` during inner-loop work.
- Prefer `pnpm check`, `pnpm test`, `pnpm build`, and targeted Rust checks before final release verification.
- If a Tauri/Rust build creates `web/src-tauri/target/` or another large target directory, clean it before reporting unless it is needed for an immediately following gate.
- Avoid creating additional worktrees that each run full Rust + Tauri builds unless the sprint is explicitly split.

## Consequences

### 正面

- Aligns cs01 with the user's intended product shape without discarding M2.2 frontend work.
- Adds a new ADSD stress-test dimension: desktop packaging + local process lifecycle, not just web/server code.
- Keeps protocol/oracle evidence stable because the sidecar is the same server binary.
- Makes release-readiness artifacts more realistic for non-terminal users.

### 负面 / 接受的债

- v0.1.0 will not be a pure single-binary Rust deployment; it becomes desktop bundle + sidecar server.
- Tauri introduces platform-specific packaging gates, especially on macOS.
- Sidecar lifecycle can fail in ways browser dev mode cannot: missing sidecar binary, port collision, child-process shutdown, log capture.
- Option C's cleaner in-process backend remains deferred and must not be half-implemented in this sprint.

### 不可逆性

- This decision is mostly reversible before v0.1.0 tag: we can return to rust-embed or manual browser mode by removing the Tauri app and release docs.
- After a public v0.1.0 desktop release, changing the primary UI surface becomes a user-facing compatibility/expectation break and should require a new ADR.

## Done Criteria(falsifiable)

### Phase 1 anchor

- [ ] ADR-0013 exists in `docs/agent/adr/` and is listed in the ADR roster.
- [ ] zh/en human ADR abstracts exist and reference `0013-tauri-desktop-frontend.md`.
- [ ] `cs01-mini-redis-rust/CLAUDE.md` no longer describes rust-embed as the M4 primary release target.
- [ ] `cs01-mini-redis-rust/README.md` marks Tauri desktop as the M4.3 target and does not claim rust-embed is already shipped.

### Phase 2 implementation

- [ ] A Tauri v2 app is added under `web/` or another ADR-justified path.
- [ ] The app renders the existing SvelteKit dashboard/keys/pubsub pages inside the desktop shell.
- [ ] The app starts/stops a loopback `redis-server` sidecar or documents a clearly safer equivalent if P9 proves sidecar packaging is infeasible.
- [ ] Sidecar startup failures are visible in the UI, not hidden in logs only.
- [ ] The RESP port and HTTP control-plane port are local-only by default.
- [ ] Browser dev mode still works.
- [ ] `scripts/frontend-gate.sh` is updated or a new `scripts/tauri-gate.sh` is added with lightweight default checks and an explicit opt-in heavy bundle step.
- [ ] `.gitignore` covers Tauri build artifacts (`src-tauri/target/`, bundle output, Vite/Tauri caches) without hiding source files.
- [ ] README documents both dev mode and Tauri desktop mode.
- [ ] M4.2 release docs are reconciled: rust-embed wording becomes deferred/optional, not the v0.1.0 blocker.

### Gates

- [ ] Rust fmt/clippy/build/test/doc-coverage pass before merge.
- [ ] Frontend gate pass before merge.
- [ ] Tauri lightweight gate pass before merge.
- [ ] Full Tauri bundle build is run only once for release readiness, with disk usage before/after recorded.
- [ ] Oracle compatibility remains non-regressed because the sidecar binary is the same `redis-server`.

## Cross-references

- ADR-0008: SvelteKit UI remains the frontend source, but rust-embed is no longer the primary release target.
- ADR-0011: default loopback bind and `--insecure-no-auth` posture remain required for sidecar safety.
- ADR-0012: doc sweep remains valid, but its M4.3 rust-embed-only release framing is superseded by this ADR.
- Finding `m4-pre-release-audit-team-aggregation.md`: Sarah/Mei public-readiness pressure remains the source for release-artifact cleanup.
- Local constitution: `cs01-mini-redis-rust/CLAUDE.md` §3 / §4 must follow this ADR to avoid constitution-vs-ADR drift.

## Phase 2 implementation note (M4.3 P9)

Implementation adds `web/src-tauri/` as a Tauri v2 app without refactoring `redis-server` into an in-process library. The desktop shell reuses the SvelteKit/Vite frontend source and manages a loopback sidecar:

- RESP sidecar endpoint:`127.0.0.1:6380`.
- HTTP/SSE control plane:`127.0.0.1:6381`.
- Sidecar discovery order: `CS01_REDIS_SERVER_BIN`, packaged resource `bin/redis-server`, then local dev Cargo target paths.
- Startup failures are visible through a Tauri command consumed by the Svelte layout banner; missing binary, port reuse, and readiness timeout are not silent.
- Browser dev mode remains supported because non-Tauri runtime still uses relative `/api/*` paths and Vite proxy.
- `scripts/tauri-gate.sh` is lightweight by default and only runs the full Tauri bundle when `CS01_TAURI_FULL_BUILD=1` is set.

## Notes

- P9 implementation prompt must explicitly forbid heavy Tauri bundle loops under low disk conditions.
- Full signing/notarization/platform bundle readiness remains unclaimed until a release-readiness run records disk usage and executes `CS01_TAURI_FULL_BUILD=1 bash scripts/tauri-gate.sh`.
- If Tauri sidecar packaging turns out to require additional platform-specific signing/notarization work, record that as a finding and keep v0.1.0 as an unsigned local-dev desktop preview rather than inventing fake release readiness.
