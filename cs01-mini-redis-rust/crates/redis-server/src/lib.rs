//! `redis-server` library root.
//!
//! Exposes:
//! - [`dispatch`] — RESP `Frame` → storage `Command` (ADR-0004).
//! - [`encode`]   — storage `Reply` → RESP `Frame` (ADR-0005).
//! - [`server`]   — TCP accept-loop + per-connection drain (ADR-0005).
//!
//! All three are `pub mod` so the integration tests in `tests/` can
//! exercise them through the library surface (matching ADSD §3.1
//! "公共 API 该用 newtype 就用 newtype" — we use modules, not random
//! re-exports).

#![forbid(unsafe_code)]

pub mod dispatch;
pub mod encode;
pub mod server;
