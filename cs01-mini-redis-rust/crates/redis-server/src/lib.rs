//! `redis-server` library root.
//!
//! Exposes:
//! - [`dispatch`] — RESP `Frame` → storage `Command` (ADR-0004).
//! - [`encode`]   — storage `Reply` → RESP `Frame` (ADR-0005).
//! - [`server`]   — TCP accept-loop + per-connection drain (ADR-0005).
//! - [`state`]    — `AppState` shared across RESP + HTTP (ADR-0007).
//! - [`http`]     — Axum control plane + SSE handlers (ADR-0007).
//!
//! All modules are `pub mod` so the integration tests in `tests/` can
//! exercise them through the library surface (matching ADSD §3.1
//! "公共 API 该用 newtype 就用 newtype" — we use modules, not random
//! re-exports).

#![forbid(unsafe_code)]

pub mod dispatch;
pub mod encode;
pub mod http;
pub mod server;
pub mod state;
