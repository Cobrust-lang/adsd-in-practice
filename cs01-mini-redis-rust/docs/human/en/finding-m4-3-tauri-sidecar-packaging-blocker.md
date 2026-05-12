# Finding M4.3: Tauri sidecar packaging blocker

> Agent finding: [docs/agent/findings/m4-3-tauri-sidecar-packaging-blocker.md](../../agent/findings/m4-3-tauri-sidecar-packaging-blocker.md).

## Summary

The Tauri desktop shell and local-dev sidecar lifecycle are implemented, but full bundle-time sidecar packaging is not claimed as release-ready yet. A check with `bundle.resources` pointing to `../../../target/debug/redis-server` failed because the binary was not staged at that path.

## Impact

M4.3 can be validated as a local desktop preview with `CS01_REDIS_SERVER_BIN` or a local Cargo target binary. A release bundle still needs a deterministic sidecar staging/signing step and one explicit `CS01_TAURI_FULL_BUILD=1` gate run with disk usage recorded.
