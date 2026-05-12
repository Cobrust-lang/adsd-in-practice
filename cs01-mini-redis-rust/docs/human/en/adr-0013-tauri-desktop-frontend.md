# ADR-0013 English abstract: M4.3 Tauri desktop frontend

> Full ADR: [docs/agent/adr/0013-tauri-desktop-frontend.md](../../agent/adr/0013-tauri-desktop-frontend.md).

## Decision

cs01's frontend release surface moves from a rust-embed browser admin page to a **Tauri desktop app**. The existing SvelteKit UI is not discarded; it remains the page source. Tauri owns the desktop shell, local sidecar lifecycle, visible logs/errors, and release surface.

## Chosen approach

v0.1.0 uses **Tauri app + managed `redis-server` sidecar**:

- Reuse the existing `redis-server` CLI, RESP listener, and `/api/*` HTTP/SSE control plane.
- Keep `redis-cli` oracle evidence comparable because the sidecar is the same server binary.
- Avoid doing a backend lifecycle library refactor inside M4.3; an in-process backend can be considered for v0.2 via a later ADR.
- rust-embed is no longer a v0.1.0 blocker; it can later be removed, retained, or made an optional web deployment target.

## Low-disk constraint

Tauri / Rust / pnpm builds can create large `target/`, bundle, and cache directories. P9 implementation must:

- Report `df -h .` before and after heavy builds.
- Avoid repeated `pnpm tauri build` during the inner loop.
- Keep default gates lightweight; run the full Tauri bundle build once for release readiness.
- Clean unneeded `web/src-tauri/target/`, bundle output, and Vite caches.

## Next

ADR-0013 is the Phase 1 strategic anchor. Phase 2 must be implemented by P9: Tauri scaffold, sidecar lifecycle, gate updates, and docs reconciliation. The CTO must not write implementation code directly.
