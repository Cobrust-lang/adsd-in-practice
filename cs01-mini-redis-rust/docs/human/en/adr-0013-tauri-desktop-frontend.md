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

## Phase 2 implementation note

M4.3 adds a Tauri v2 app under `web/src-tauri/`. The desktop shell reuses the existing SvelteKit pages and starts a loopback `redis-server` sidecar by default:

- RESP: `127.0.0.1:6380`
- HTTP/SSE: `127.0.0.1:6381`
- UI failure state: the shared layout shows a Tauri sidecar banner for `starting`, `running`, `failed`, and `stopped` states.
- Development override: set `CS01_REDIS_SERVER_BIN=/absolute/path/to/redis-server` when the sidecar binary is not in the default Cargo target path.

`scripts/tauri-gate.sh` is lightweight by default: SvelteKit check/test/build plus targeted `cargo check --manifest-path web/src-tauri/Cargo.toml`. Full desktop bundle creation is opt-in with `CS01_TAURI_FULL_BUILD=1` and must record disk usage before/after.

## M4.3 gate-return patch

The CTO gate return identified two runtime hardening gaps, both patched without changing the ADR-0013 architecture:

- The sidecar no longer connects stdout/stderr to undrained pipes. `web/src-tauri/src/main.rs` uses `Stdio::null()` so long runs or abnormal logging cannot fill a pipe buffer and block `redis-server`.
- `/api/stats`, `/api/keys`, and `/api/pubsub` SSE responses now include minimal CORS headers. The Tauri production UI issues `EventSource(http://127.0.0.1:6381/api/*)` from the app/WebView origin, so the backend treats it as a cross-origin browser request and returns `Access-Control-Allow-Origin: *` without credentials while the listener remains loopback-only.
