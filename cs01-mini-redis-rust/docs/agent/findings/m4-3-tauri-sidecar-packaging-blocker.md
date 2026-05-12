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

## M4.4 update: package-version blocker closed

The later M4.4 full-bundle gate reached Tauri's bundler and failed on a separate release blocker: mixed Tauri minor versions across ecosystems. Cargo had resolved Rust `tauri` to `2.11.1`, while npm still locked `@tauri-apps/api` to `2.9.0` and `@tauri-apps/cli` to `2.9.5`; Tauri rejected the bundle with `Found version mismatched Tauri packages`.

This blocker is closed by explicit release pins and regenerated lockfiles:

- npm: `@tauri-apps/api = 2.11.0`, `@tauri-apps/cli = 2.11.0`
- Cargo: `tauri = 2.11.1`, `tauri-build = 2.6.1`

The engineering choice is to move npm forward to the Rust-resolved 2.11 line, not downgrade Rust crates, because the existing Cargo lock already validated `tauri 2.11.1` through lightweight checks and `@tauri-apps/api 2.11.1` is not published while `@tauri-apps/api 2.11.0` is the current npm 2.11 API release. Full sidecar signing/notarization remains a separate release-readiness risk.

## Bilingual sync

Human-readable abstracts exist in:

- `docs/human/zh/finding-m4-3-tauri-sidecar-packaging-blocker.md`
- `docs/human/en/finding-m4-3-tauri-sidecar-packaging-blocker.md`
