//! Read-only metrics surface for the store (ADR-0007 §Q4).
//!
//! Both functions take a single `inner.read()` lock and walk the map
//! once.  They skip logically-expired entries (`expires_at <= now`)
//! so the numbers always reflect what `GET` / `EXISTS` / `KEYS` would
//! see — i.e. the metrics view is consistent with the wire view.
//!
//! Layer rule (cs01 CLAUDE.md §4 + ADR-0007 watch-out):
//! the storage crate is **protocol-agnostic**.  We expose plain
//! structs (`StoreMetrics`, `KeyInfo`) with `i64` / `u64` / `String`
//! fields — JSON serialization happens in `redis-server::http`.
//!
//! `sample_keys` ordering: `hashbrown::HashMap` iteration order is
//! non-deterministic across processes (random hasher per construction).
//! This deliberately matches Redis `KEYS` semantics ("returns no
//! specified order").  We do NOT sort — that would be a hidden
//! contract callers might come to rely on.

use tokio::time::Instant;

use crate::{Inner, Store};

/// Aggregate counters over the live keyspace.
///
/// "Live" = `expires_at` is `None` OR strictly in the future.
/// Already-expired-but-not-yet-reaped entries are excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreMetrics {
    /// Number of live keys.
    pub key_count: u64,
    /// Sum of `entry.value.len()` over live keys (bytes).
    ///
    /// This is a coarse "value footprint" — it ignores key-string
    /// overhead and `HashMap` slot overhead.  Adequate for the M2
    /// dashboard; release-readiness can refine later.
    pub total_value_bytes: u64,
}

/// Per-key descriptor returned by [`Store::sample_keys`].
///
/// `kind` is currently always `"string"` (v0.1 single-type build).
/// `ttl_secs` follows the Redis `TTL` wire semantics:
/// * `-1` — key exists, no TTL set
/// * positive — remaining seconds (round-half-up to match real Redis)
///
/// `-2` (absent) cannot appear because sampling already filters out
/// missing / expired keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInfo {
    pub key: String,
    pub kind: &'static str,
    pub ttl_secs: i64,
}

impl Store {
    /// Snapshot live-keyspace counters under a single read lock.
    ///
    /// Walks the inner map once.  O(n) in key count, no allocation
    /// beyond the returned `StoreMetrics` value.  Expired-but-not-yet-
    /// reaped entries are skipped so the count is consistent with
    /// what `EXISTS` / `KEYS` would report at the same instant.
    #[must_use]
    pub fn metrics(&self) -> StoreMetrics {
        let guard = self.inner.read();
        compute_metrics(&guard, Instant::now())
    }

    /// Sample up to `limit` live keys with type + TTL info.
    ///
    /// Iteration order is the underlying `hashbrown` hasher's order —
    /// non-deterministic across processes; this matches the Redis
    /// `KEYS` contract ("returns no specified order").
    ///
    /// Skips logically-expired entries.  If `limit == 0`, returns an
    /// empty vec.  No allocation per iteration beyond the per-entry
    /// `KeyInfo`.
    ///
    /// TTL rounding matches the `TTL` command path
    /// (`(pttl_ms + 500) / 1000`) — see `Store::do_ttl` and the M1.4
    /// F23-A finding for why floor is wrong.
    #[must_use]
    pub fn sample_keys(&self, limit: usize) -> Vec<KeyInfo> {
        if limit == 0 {
            return Vec::new();
        }
        let guard = self.inner.read();
        let now = Instant::now();
        sample_keys_locked(&guard, now, limit)
    }
}

// ── helpers — kept module-private so they can be unit-tested without
//    exposing the locked `Inner` type ────────────────────────────────────

/// Walk the locked `Inner` once and return [`StoreMetrics`].
///
/// Split from the public API so the unit tests can call this with a
/// hand-built `Inner` snapshot without going through `Store::new()`
/// (which spawns a background task — overkill for pure-logic tests).
fn compute_metrics(inner: &Inner, now: Instant) -> StoreMetrics {
    let mut key_count: u64 = 0;
    let mut total_value_bytes: u64 = 0;
    for entry in inner.map.values() {
        if entry.expires_at.is_some_and(|t| t <= now) {
            continue;
        }
        key_count = key_count.saturating_add(1);
        total_value_bytes = total_value_bytes.saturating_add(entry.value.len() as u64);
    }
    StoreMetrics {
        key_count,
        total_value_bytes,
    }
}

/// Walk the locked `Inner` and collect up to `limit` live keys.
fn sample_keys_locked(inner: &Inner, now: Instant, limit: usize) -> Vec<KeyInfo> {
    let mut out: Vec<KeyInfo> = Vec::with_capacity(limit.min(inner.map.len()));
    for (k, entry) in &inner.map {
        if out.len() >= limit {
            break;
        }
        if entry.expires_at.is_some_and(|t| t <= now) {
            continue;
        }
        let ttl_secs: i64 = match entry.expires_at {
            None => -1,
            Some(t) => {
                let remaining = t.saturating_duration_since(now);
                let ms: u128 = remaining.as_millis();
                let secs_rounded: u128 = (ms + 500) / 1000;
                i64::try_from(secs_rounded).unwrap_or(i64::MAX)
            }
        };
        out.push(KeyInfo {
            key: k.clone(),
            kind: "string",
            ttl_secs,
        });
    }
    out
}
