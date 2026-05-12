//! Shared server state (ADR-0007 §Q6).
//!
//! `AppState` carries the [`Store`] handle plus two `Arc<AtomicU64>`
//! counters and a `started: Instant` — everything the RESP listener
//! and the HTTP listener need to share **without** routing through a
//! second data source (ADR-0007 §Q4 — no `Store` shadow counters).
//!
//! `AppState: Clone` is cheap (all fields are `Arc<…>` /
//! `tokio::sync::broadcast::Sender` clones).
//!
//! Counter discipline (ADR-0007 watch-out):
//!
//! - `connections_active` uses an RAII [`ConnGuard`] so abnormal task
//!   termination (panic, early `?`, abort) still decrements the
//!   counter — the increment happens at guard construction, the
//!   decrement on `Drop`.
//! - `commands_total` is a flat `AtomicU64::fetch_add(1)` per
//!   incoming RESP frame *after* a successful `Frame::parse` — see
//!   `server::handle_conn`.  Increments for unknown commands too,
//!   matching real Redis' `total_commands_processed` info field.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use redis_storage::Store;
use tokio::sync::broadcast;

/// Capacity of the SSE fan-out broadcast channels (ADR-0007 §Q7).
///
/// Kept deliberately small: a lagging client (slower than the 1 Hz
/// sampler) will trip `RecvError::Lagged` after ~32 missed frames and
/// be disconnected cleanly — preferable to memory pressure from
/// buffering frames a stalled client may never drain.
pub const BROADCAST_CAPACITY: usize = 32;

/// Shared state co-owned by the RESP listener, the HTTP listener,
/// and the 1 Hz sampler task.
///
/// All fields are individually `Arc`-shared / `Clone`-cheap so a full
/// `AppState::clone()` is `O(7 atomic ref-count increments)`.
#[derive(Clone)]
pub struct AppState {
    /// The in-memory store.  `Store: Clone` is itself an `Arc` bump.
    pub store: Store,
    /// `--max-frame-size` value — forwarded to per-conn RESP handler.
    pub max_frame_size: usize,
    /// Number of active RESP connections.  See [`ConnGuard`].
    pub connections_active: Arc<AtomicU64>,
    /// Cumulative number of RESP frames parsed (any outcome).
    pub commands_total: Arc<AtomicU64>,
    /// Server start time (used to compute `uptime_secs`).
    pub started: Instant,
    /// Broadcast sender for `/api/stats` SSE.  The 1 Hz sampler task
    /// owns the corresponding source loop; HTTP handlers obtain a
    /// fresh `Receiver` via `stats_tx.subscribe()`.
    pub stats_tx: broadcast::Sender<StatsSnapshot>,
    /// Broadcast sender for `/api/keys` SSE.  Same fan-out shape.
    pub keys_tx: broadcast::Sender<KeysSnapshot>,
}

impl AppState {
    /// Construct a fresh state bound to `store` with `max_frame_size`
    /// forwarded to the RESP listener.
    ///
    /// Spins up the broadcast channels (capacity [`BROADCAST_CAPACITY`])
    /// but does NOT spawn the sampler — that's `http::run`'s job so
    /// tests can construct `AppState` standalone.
    #[must_use]
    pub fn new(store: Store, max_frame_size: usize) -> Self {
        let (stats_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (keys_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            store,
            max_frame_size,
            connections_active: Arc::new(AtomicU64::new(0)),
            commands_total: Arc::new(AtomicU64::new(0)),
            started: Instant::now(),
            stats_tx,
            keys_tx,
        }
    }

    /// Snapshot of all counters at the call instant — used by the
    /// 1 Hz sampler.  Three atomic loads + one `Instant::elapsed`.
    #[must_use]
    pub fn snapshot_stats(&self) -> StatsSnapshot {
        let store_m = self.store.metrics();
        StatsSnapshot {
            connections_active: self.connections_active.load(Ordering::Relaxed),
            commands_total: self.commands_total.load(Ordering::Relaxed),
            keys_active: store_m.key_count,
            mem_value_bytes: store_m.total_value_bytes,
            uptime_secs: self.started.elapsed().as_secs(),
        }
    }
}

/// One frame of `/api/stats` SSE.
///
/// Field names are LOCKED for the M2.2 frontend contract (ADR-0007
/// §Q2 + Done Criteria — `event: stats` payload schema).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct StatsSnapshot {
    pub connections_active: u64,
    pub commands_total: u64,
    pub keys_active: u64,
    pub mem_value_bytes: u64,
    pub uptime_secs: u64,
}

/// One frame of `/api/keys` SSE — a list of `KeyInfo`.
///
/// Wrapped in a newtype so the broadcast channel signature is a
/// concrete `broadcast::Sender<KeysSnapshot>` (the SSE handler clones
/// the inner vec; `Vec` is `Clone`, so this stays efficient).
#[derive(Debug, Clone, Default)]
pub struct KeysSnapshot(pub Vec<redis_storage::KeyInfo>);

/// RAII guard that increments `connections_active` on construction
/// and decrements on `Drop`.
///
/// **Usage**: install at the very top of `handle_conn` *before* any
/// `?` operator — abnormal exits (panic / early return) still trigger
/// the decrement via stack unwind.
pub struct ConnGuard {
    counter: Arc<AtomicU64>,
}

impl ConnGuard {
    /// Increment `counter` and return a guard that will decrement on
    /// drop.  Uses `Ordering::Relaxed` for both ends — the counter is
    /// monotonic-with-time, not used for synchronisation.
    #[must_use]
    pub fn new(counter: Arc<AtomicU64>) -> Self {
        counter.fetch_add(1, Ordering::Relaxed);
        Self { counter }
    }
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn conn_guard_increments_and_decrements() {
        let counter = Arc::new(AtomicU64::new(0));
        {
            let _g = ConnGuard::new(Arc::clone(&counter));
            assert_eq!(counter.load(Ordering::Relaxed), 1);
            let _g2 = ConnGuard::new(Arc::clone(&counter));
            assert_eq!(counter.load(Ordering::Relaxed), 2);
        }
        // Both guards dropped — counter should be back to zero.
        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn conn_guard_decrements_on_unwind() {
        // Even if a panic unwinds across a guard, Drop still runs.
        let counter = Arc::new(AtomicU64::new(0));
        let counter_clone = Arc::clone(&counter);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = ConnGuard::new(counter_clone);
            panic!("boom");
        }));
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn app_state_snapshot_initial() {
        let store = Store::new();
        let state = AppState::new(store, 4096);
        let snap = state.snapshot_stats();
        assert_eq!(snap.connections_active, 0);
        assert_eq!(snap.commands_total, 0);
        assert_eq!(snap.keys_active, 0);
        assert_eq!(snap.mem_value_bytes, 0);
        // uptime_secs may be 0 or 1 depending on wall-clock tick.
        assert!(snap.uptime_secs <= 2);
    }
}
