---
finding: m4-3-tauri-sidecar-packaging-blocker
date: 2026-05-13
case: cs01-mini-redis-rust
severity: medium
status: accepted
---

# Finding: M4.3 Tauri sidecar packaging is not release-ready yet

- **Milestone**: M4.3
- **Date**: 2026-05-13
- **Severity**: Medium
- **Status**: accepted debt for this sprint
- **Related ADR**: ADR-0013

## Summary

The M4.3 implementation can run a Tauri desktop shell in local development and can manage a loopback `redis-server` sidecar when the binary is available. However, Tauri bundle-time sidecar packaging is not claimed as release-ready in this sprint.

## Evidence

A first `cargo check --manifest-path web/src-tauri/Cargo.toml` attempt used `bundle.resources` pointing at `../../../target/debug/redis-server`. Tauri validates that resource path during the build script, and the check failed when the sidecar binary was not present:

```text
resource path `../../../target/debug/redis-server` doesn't exist
```

This is a real release-readiness boundary, not a code-style issue. Binding the bundle config to a local debug binary would make lightweight checks depend on a prior backend build and would risk hiding platform-specific packaging/signing requirements.

## Decision

For M4.3 Phase 2, keep the implementation honest:

- `web/src-tauri/tauri.conf.json` does **not** claim packaged sidecar resources by default.
- Local development sidecar discovery remains implemented in Rust:
  1. `CS01_REDIS_SERVER_BIN`
  2. packaged resource `bin/redis-server` if a future release config adds it
  3. local Cargo target paths
- `scripts/tauri-gate.sh` remains lightweight and does not run a full bundle unless `CS01_TAURI_FULL_BUILD=1` is set.
- Full release packaging must add a deterministic sidecar binary staging step and run the full Tauri bundle once with disk usage recorded.

## Bilingual sync

Human-readable abstracts exist in:

- `docs/human/zh/finding-m4-3-tauri-sidecar-packaging-blocker.md`
- `docs/human/en/finding-m4-3-tauri-sidecar-packaging-blocker.md`
