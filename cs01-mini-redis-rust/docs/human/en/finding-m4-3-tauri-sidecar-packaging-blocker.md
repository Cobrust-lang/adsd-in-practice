# Finding M4.3: Tauri sidecar packaging blocker

> Agent finding: [docs/agent/findings/m4-3-tauri-sidecar-packaging-blocker.md](../../agent/findings/m4-3-tauri-sidecar-packaging-blocker.md).

## Summary

The Tauri desktop shell and local-dev sidecar lifecycle are implemented, but full bundle-time sidecar packaging is not claimed as release-ready yet. A check with `bundle.resources` pointing to `../../../target/debug/redis-server` failed because the binary was not staged at that path.

## Impact

M4.3 can be validated as a local desktop preview with `CS01_REDIS_SERVER_BIN` or a local Cargo target binary. A release bundle still needs a deterministic sidecar staging/signing step and one explicit `CS01_TAURI_FULL_BUILD=1` gate run with disk usage recorded.

## M4.4 update

The full bundle gate then exposed a cross-ecosystem Tauri minor mismatch: `@tauri-apps/api` 2.9.0 / `@tauri-apps/cli` 2.9.5 paired with Rust `tauri` 2.11.1 is rejected by the bundler. The fix moves npm forward to the 2.11 line and pins explicitly: `@tauri-apps/api` 2.11.0, `@tauri-apps/cli` 2.11.0, `tauri` 2.11.1, and `tauri-build` 2.6.1.
