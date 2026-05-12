//! In-memory KV store + TTL + AOF persistence.
//!
//! No network IO; this crate is pure storage. The server crate calls
//! `Store::execute(cmd) -> Reply` per request.

#![forbid(unsafe_code)]

use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("key not found")]
    NotFound,
    #[error("wrong type")]
    WrongType,
}

/// Top-level command enum (decoded from RESP `Array(Some(bulk*))`).
///
/// Wave M1 — covers PING / GET / SET / DEL / EXISTS / INCR / DECR.
/// Wave M3 — adds SUBSCRIBE / PUBLISH / UNSUBSCRIBE.
#[derive(Debug, Clone)]
pub enum Command {
    Ping,
    Get { key: String },
    Set { key: String, value: Vec<u8>, ttl_secs: Option<u64> },
    Del { keys: Vec<String> },
    Exists { keys: Vec<String> },
    Incr { key: String },
    Decr { key: String },
}

/// Reply enum (encoded to RESP frame at the server boundary).
#[derive(Debug, Clone)]
pub enum Reply {
    Pong,
    Bulk(Option<Vec<u8>>),
    Integer(i64),
    Ok,
    Error(String),
}

/// The store. Cheap to clone; internally `Arc<inner>` (M1.2).
#[derive(Clone, Default)]
pub struct Store {
    _inner: Arc<()>, // M1.2 stub — replace with hashbrown::HashMap + DelayQueue
}

impl Store {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Execute a command and return a reply.
    ///
    /// # Errors
    ///
    /// Returns `StoreError` on internal IO or invariant violation;
    /// "key not found" maps to `Reply::Bulk(None)`, not an error.
    pub fn execute(&self, _cmd: Command) -> Result<Reply, StoreError> {
        // M1.2 stub
        Ok(Reply::Pong)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_ping_stub() {
        let s = Store::new();
        let r = s.execute(Command::Ping).expect("ping infallible");
        assert!(matches!(r, Reply::Pong));
    }
}
