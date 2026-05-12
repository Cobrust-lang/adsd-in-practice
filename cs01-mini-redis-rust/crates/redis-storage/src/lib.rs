//! In-memory KV store + TTL + AOF persistence.
//!
//! No network IO; this crate is pure storage. The server crate calls
//! `Store::execute(cmd) -> Reply` per request.
//!
//! Internal layout (ADR-0003):
//! - `Arc<parking_lot::RwLock<Inner>>` for cheap clone + fast concurrent reads.
//! - `hashbrown::HashMap<String, Entry>` as the underlying KV map.
//! - `tokio::time::DelayQueue` for **active** TTL expiration (never lazy/on-read).

#![forbid(unsafe_code)]

use std::sync::Arc;

use futures_util::StreamExt as _;
use hashbrown::HashMap;
use parking_lot::RwLock;
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_util::time::DelayQueue;

/// Errors that represent internal/invariant failures.
/// "key not found" is NOT an error — it maps to `Reply::Bulk(None)`.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Top-level command enum (decoded from RESP `Array(Some(bulk*))`).
///
/// Wave M1 — covers PING / GET / SET / DEL / EXISTS / INCR / DECR
///            + ECHO / SELECT / QUIT (M1.3, ADR-0005).
/// Wave M3 — adds SUBSCRIBE / PUBLISH / UNSUBSCRIBE.
#[derive(Debug, Clone)]
pub enum Command {
    Ping,
    Get {
        key: String,
    },
    Set {
        key: String,
        value: Vec<u8>,
        ttl_secs: Option<u64>,
    },
    Del {
        keys: Vec<String>,
    },
    Exists {
        keys: Vec<String>,
    },
    Incr {
        key: String,
    },
    Decr {
        key: String,
    },
    /// `ECHO message` — returns `Bulk(Some(message))`.
    Echo {
        message: Vec<u8>,
    },
    /// `SELECT db` — accepts only `db == 0` in this single-DB build.
    /// Non-zero index returns `Reply::Error("ERR DB index is out of range")`.
    Select {
        db: i64,
    },
    /// `QUIT` — server closes the socket after flushing the reply.
    /// The store side returns `Reply::Ok`; the socket close is the
    /// caller's (server crate) responsibility — see ADR-0005.
    Quit,
}

/// Reply enum (encoded to RESP frame at the server boundary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Pong,
    Bulk(Option<Vec<u8>>),
    Integer(i64),
    Ok,
    Error(String),
}

/// A single stored entry.
struct Entry {
    value: Vec<u8>,
    expires_at: Option<Instant>,
}

/// Shared inner state — held behind an `RwLock`.
struct Inner {
    map: HashMap<String, Entry>,
}

impl Inner {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

/// The store. Cheap to clone; internally `Arc<RwLock<Inner>>` (ADR-0003).
///
/// `Store::new()` spawns a background tokio task that drives a
/// `DelayQueue` for **active** TTL expiration (not lazy/on-read).
#[derive(Clone)]
pub struct Store {
    inner: Arc<RwLock<Inner>>,
    /// Sender side of the delay queue channel; used by SET with EX.
    ttl_tx: tokio::sync::mpsc::UnboundedSender<(String, std::time::Duration)>,
    /// Handle kept so the runtime doesn't cancel the task prematurely.
    _expiry_task: Arc<JoinHandle<()>>,
}

impl Store {
    /// Create a new store and spawn the expiration background task.
    #[must_use]
    pub fn new() -> Self {
        let inner = Arc::new(RwLock::new(Inner::new()));
        let inner_clone = Arc::clone(&inner);

        // Unbounded channel: SET sends (key, duration) → expiry task schedules.
        let (ttl_tx, mut ttl_rx) =
            tokio::sync::mpsc::unbounded_channel::<(String, std::time::Duration)>();

        let handle = tokio::spawn(async move {
            let mut dq: DelayQueue<String> = DelayQueue::new();

            loop {
                tokio::select! {
                    // New TTL registration from Store::execute(Set { ttl_secs }).
                    msg = ttl_rx.recv() => {
                        match msg {
                            Some((key, dur)) => { dq.insert(key, dur); }
                            // Channel closed → all Store clones dropped → exit.
                            None => break,
                        }
                    }
                    // A key has expired.
                    Some(expired) = dq.next() => {
                        let key = expired.into_inner();
                        let mut guard = inner_clone.write();
                        // Only remove if the entry is actually expired
                        // (a later SET with no TTL would have cleared expires_at).
                        if let Some(entry) = guard.map.get(&key)
                            && entry.expires_at.is_some_and(|t| t <= Instant::now())
                        {
                            guard.map.remove(&key);
                        }
                    }
                }
            }
        });

        Self {
            inner,
            ttl_tx,
            _expiry_task: Arc::new(handle),
        }
    }

    /// Execute a command and return a reply.
    ///
    /// # Errors
    ///
    /// Returns `StoreError` on internal IO or invariant violation.
    /// "Key not found" maps to `Reply::Bulk(None)`, not an error.
    pub fn execute(&self, cmd: Command) -> Result<Reply, StoreError> {
        match cmd {
            Command::Ping => Ok(Reply::Pong),

            Command::Get { key } => {
                let guard = self.inner.read();
                match guard.map.get(&key) {
                    Some(entry) => {
                        // Check active expiry: entry may have been inserted before
                        // the background task fires.
                        if entry.expires_at.is_some_and(|t| t <= Instant::now()) {
                            // Logically expired; treat as absent.
                            Ok(Reply::Bulk(None))
                        } else {
                            Ok(Reply::Bulk(Some(entry.value.clone())))
                        }
                    }
                    None => Ok(Reply::Bulk(None)),
                }
            }

            Command::Set {
                key,
                value,
                ttl_secs,
            } => {
                let expires_at =
                    ttl_secs.map(|secs| Instant::now() + std::time::Duration::from_secs(secs));

                {
                    let mut guard = self.inner.write();
                    guard.map.insert(key.clone(), Entry { value, expires_at });
                }

                if let Some(secs) = ttl_secs {
                    // Enqueue TTL in background task; ignore send error
                    // (task may have exited if store is being dropped).
                    let _ = self
                        .ttl_tx
                        .send((key, std::time::Duration::from_secs(secs)));
                }

                Ok(Reply::Ok)
            }

            Command::Del { keys } => {
                let mut guard = self.inner.write();
                let mut count: i64 = 0;
                for key in &keys {
                    if guard.map.remove(key).is_some() {
                        count += 1;
                    }
                }
                Ok(Reply::Integer(count))
            }

            Command::Exists { keys } => {
                let guard = self.inner.read();
                let now = Instant::now();
                let mut count: i64 = 0;
                for key in &keys {
                    if let Some(entry) = guard.map.get(key) {
                        // Not expired?
                        if entry.expires_at.is_none_or(|t| t > now) {
                            count += 1;
                        }
                    }
                }
                Ok(Reply::Integer(count))
            }

            Command::Incr { key } => Ok(Self::incr_by(&self.inner, key, 1)),
            Command::Decr { key } => Ok(Self::incr_by(&self.inner, key, -1)),

            // ── M1.3 (ADR-0005) ──────────────────────────────────────────
            // ECHO returns the message verbatim as a bulk string.
            Command::Echo { message } => Ok(Reply::Bulk(Some(message))),

            // SELECT db — single-DB build: only db 0 succeeds.  Real Redis
            // wire string for the failure case is exactly
            // "ERR DB index is out of range".
            Command::Select { db } => {
                if db == 0 {
                    Ok(Reply::Ok)
                } else {
                    Ok(Reply::Error("ERR DB index is out of range".to_owned()))
                }
            }

            // QUIT — the store side just returns Ok.  ADR-0005 makes the
            // server crate responsible for closing the socket *after*
            // flushing this reply.
            Command::Quit => Ok(Reply::Ok),
        }
    }

    /// Shared INCR/DECR implementation.
    ///
    /// Returns `Reply::Error` (not `StoreError`) for non-integer value —
    /// matching Redis wire protocol behaviour exactly.
    fn incr_by(inner: &Arc<RwLock<Inner>>, key: String, delta: i64) -> Reply {
        let mut guard = inner.write();
        let now = Instant::now();

        let current: i64 = match guard.map.get(&key) {
            None => 0,
            Some(entry) => {
                // Treat an expired entry as absent.
                if entry.expires_at.is_some_and(|t| t <= now) {
                    guard.map.remove(&key);
                    0
                } else {
                    // Parse the stored bytes as a decimal integer.
                    let Some(v) = std::str::from_utf8(&entry.value)
                        .ok()
                        .and_then(|s| s.parse::<i64>().ok())
                    else {
                        return Reply::Error(
                            "ERR value is not an integer or out of range".to_owned(),
                        );
                    };
                    v
                }
            }
        };

        let Some(next) = current.checked_add(delta) else {
            return Reply::Error("ERR value is not an integer or out of range".to_owned());
        };

        let value = next.to_string().into_bytes();
        guard.map.insert(
            key,
            Entry {
                value,
                expires_at: None,
            },
        );
        Reply::Integer(next)
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn store_ping() {
        let s = Store::new();
        let r = s.execute(Command::Ping).expect("ping infallible");
        assert_eq!(r, Reply::Pong);
    }
}
