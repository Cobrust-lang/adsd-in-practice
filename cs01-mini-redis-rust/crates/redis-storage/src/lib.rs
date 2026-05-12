//! In-memory KV store + TTL + AOF persistence.
//!
//! No network IO; this crate is pure storage. The server crate calls
//! `Store::execute(cmd) -> Reply` per request.
//!
//! Internal layout (ADR-0003):
//! - `Arc<parking_lot::RwLock<Inner>>` for cheap clone + fast concurrent reads.
//! - `hashbrown::HashMap<String, Entry>` as the underlying KV map.
//! - `tokio::time::DelayQueue` for **active** TTL expiration (never lazy/on-read).
//!
//! M3.2 (ADR-0010) adds optional AOF persistence:
//! - [`Store::with_aof`] constructs a store with a background `AofWriter`.
//! - [`Store::execute`] writes a RESP-encoded copy of each *writable*
//!   command to the AOF after the in-memory mutation succeeds.
//! - [`Store::replay_from_path`] replays an existing AOF file into the
//!   store *before* the AOF writer is wired in, so re-execution does
//!   not duplicate the file.

#![forbid(unsafe_code)]

pub mod aof;
pub mod glob;
pub mod metrics;

pub use aof::{AofWriter, FsyncPolicy, aof_encode};
pub use metrics::{KeyInfo, StoreMetrics};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bytes::{Buf, BytesMut};
use futures_util::StreamExt as _;
use hashbrown::HashMap;
use parking_lot::RwLock;
use redis_protocol::{Frame, ProtocolError};
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
///            + ECHO / SELECT / QUIT (M1.3, ADR-0005)
///            + EXPIRE / TTL / PERSIST / TYPE / KEYS (M1.4, ADR-0006).
/// Wave M3.1 (ADR-0009) — adds SUBSCRIBE / UNSUBSCRIBE / PUBLISH.
#[derive(Debug, Clone)]
pub enum Command {
    /// `PING` or `PING <message>`.  With `Some(bytes)` returns the bytes
    /// as a bulk string (matches real Redis).  With `None` returns `+PONG`.
    Ping {
        message: Option<Vec<u8>>,
    },
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

    // ── M1.4 (ADR-0006) ─────────────────────────────────────────────────
    /// `EXPIRE key seconds` — set / reset TTL on an existing key.
    /// Returns `Integer(1)` on success, `Integer(0)` when key absent.
    Expire {
        key: String,
        seconds: i64,
    },
    /// `TTL key` — Redis wire semantics:
    /// `-2` = key absent, `-1` = key without TTL, otherwise remaining secs.
    Ttl {
        key: String,
    },
    /// `PERSIST key` — clear TTL on existing key.
    /// Returns `Integer(1)` when an existing TTL was cleared,
    /// `Integer(0)` otherwise (no TTL or key absent).
    Persist {
        key: String,
    },
    /// `TYPE key` — single-string build: `"string"` or `"none"`.
    /// Returns `SimpleString` (matches Redis wire format).
    Type {
        key: String,
    },
    /// `KEYS pattern` — glob-match all live keys.  Returns `Array(Some(_))`
    /// of `BulkString(Some(_))` entries.  See [`glob::matches`].
    Keys {
        pattern: String,
    },

    // ── M3.1 (ADR-0009) Pub/Sub ─────────────────────────────────────────
    /// `SUBSCRIBE channel [channel ...]` — register the *current connection*
    /// (caller-tracked) on each channel.  The wire protocol response is one
    /// `Reply::SubscribeAck` per requested channel; counts are running
    /// totals managed by the server (per-conn) layer, NOT by the store.
    /// `Store::execute` does NOT process this variant: the server crate
    /// calls [`Store::subscribe`] directly because the store can only
    /// return a `Reply` value, but SUBSCRIBE produces *N* replies plus a
    /// `broadcast::Receiver`.  Keeping the enum complete lets the
    /// dispatch layer parse uniformly; the variant simply must not reach
    /// `Store::execute`.  See ADR-0009 §Q4.
    Subscribe {
        channels: Vec<String>,
    },
    /// `UNSUBSCRIBE [channel ...]` — drop registrations.  Same routing
    /// note as `Subscribe`: the *server* handles state transitions; the
    /// variant exists for parse-side uniformity.  Empty `channels` means
    /// "unsubscribe from all" — handled by the server crate.
    Unsubscribe {
        channels: Vec<String>,
    },
    /// `PUBLISH channel message` — broadcast `message` to all current
    /// subscribers of `channel`.  Returns
    /// `Reply::Integer(receiver_count_at_send_instant)`.  This is the
    /// one Pub/Sub command that fits cleanly into `Store::execute`
    /// because the side-effect (broadcast send) is synchronous and the
    /// reply is a single integer.
    Publish {
        channel: String,
        message: Vec<u8>,
    },
}

/// Reply enum (encoded to RESP frame at the server boundary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Pong,
    Bulk(Option<Vec<u8>>),
    Integer(i64),
    Ok,
    Error(String),
    /// M1.4 (ADR-0006): `+<string>\r\n` simple string reply.
    /// Used by `TYPE` (`+string\r\n` / `+none\r\n`).  Distinct from
    /// `Bulk` because the RESP wire bytes differ — we do NOT abuse
    /// `Bulk(Some)` to fake `SimpleString` (F24 defence).
    SimpleString(String),
    /// M1.4 (ADR-0006): RESP array of bulk strings.  Used by `KEYS`.
    /// `None` represents the nil array (RESP `*-1\r\n`); v0.1 only emits
    /// `Some(_)` (possibly empty), but the variant carries the option
    /// for symmetry with the underlying `Frame::Array`.
    Array(Option<Vec<Vec<u8>>>),

    // ── M3.1 (ADR-0009) Pub/Sub ──────────────────────────────────────────
    /// Pub/Sub `subscribe` acknowledgement frame.
    /// Wire shape (Redis 7):
    /// `*3\r\n$9\r\nsubscribe\r\n$<n>\r\n<channel>\r\n:<count>\r\n`
    /// where `count` is the *running total* of channels the issuing
    /// connection is subscribed to *after* this acknowledgement.
    SubscribeAck {
        channel: String,
        count: i64,
    },
    /// Pub/Sub `unsubscribe` acknowledgement frame.
    /// Wire shape (Redis 7):
    /// `*3\r\n$11\r\nunsubscribe\r\n$<n>\r\n<channel>\r\n:<count>\r\n`
    /// where `count` is the *remaining* subscriptions after this
    /// removal.  `channel = None` is the special "unsubscribe-from-all
    /// when there was nothing subscribed" case Redis really does emit;
    /// the wire serialises the channel slot as `$-1\r\n` (nil bulk).
    UnsubscribeAck {
        channel: Option<String>,
        count: i64,
    },
    /// Pub/Sub server-pushed message frame.
    /// Wire shape: `*3\r\n$7\r\nmessage\r\n$<n>\r\n<channel>\r\n$<n>\r\n<payload>\r\n`.
    /// Distinct from `Bulk` / `Array` because it carries a *channel +
    /// payload* tuple that has no natural representation in the
    /// pre-M3.1 enum (F24: don't fake a tuple inside an Array).
    Message {
        channel: String,
        payload: Vec<u8>,
    },
}

/// A single stored entry.
pub(crate) struct Entry {
    pub(crate) value: Vec<u8>,
    pub(crate) expires_at: Option<Instant>,
}

/// Shared inner state — held behind an `RwLock`.
pub(crate) struct Inner {
    pub(crate) map: HashMap<String, Entry>,
    /// M3.1 (ADR-0009): per-channel `tokio::sync::broadcast::Sender`
    /// for Pub/Sub fan-out.  The wrapped payload is `Arc<Vec<u8>>` so
    /// N subscribers share the bytes — no per-subscriber clone of the
    /// payload in the hot path.
    ///
    /// Eviction (ADR-0009 §Decision Q1+Q2): we do NOT remove a
    /// channel's `Sender` once its `receiver_count()` reaches zero —
    /// M3.1 accepts this small memory debt; M4 release-readiness adds
    /// an eviction sweep.
    pub(crate) subscribers: HashMap<String, tokio::sync::broadcast::Sender<Arc<Vec<u8>>>>,
}

impl Inner {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            subscribers: HashMap::new(),
        }
    }
}

/// Per-channel broadcast capacity for Pub/Sub fan-out (ADR-0009).
///
/// 128 frames is a deliberate small ceiling: a lagging subscriber
/// trips `RecvError::Lagged` and is disconnected (matches real Redis'
/// behaviour of resetting an over-buffered Pub/Sub client).
pub const PUBSUB_BROADCAST_CAPACITY: usize = 128;

/// The store. Cheap to clone; internally `Arc<RwLock<Inner>>` (ADR-0003).
///
/// `Store::new()` spawns a background tokio task that drives a
/// `DelayQueue` for **active** TTL expiration (not lazy/on-read).
///
/// `Store::with_aof()` (M3.2, ADR-0010) additionally wires an
/// [`AofWriter`]; once installed, every successful *writable* command
/// (SET / DEL / EXPIRE / PERSIST / INCR / DECR) is RESP-encoded and
/// appended to the AOF file via a non-blocking mpsc.
#[derive(Clone)]
pub struct Store {
    pub(crate) inner: Arc<RwLock<Inner>>,
    /// Sender side of the delay queue channel; used by SET with EX.
    ttl_tx: tokio::sync::mpsc::UnboundedSender<(String, std::time::Duration)>,
    /// Handle kept so the runtime doesn't cancel the task prematurely.
    _expiry_task: Arc<JoinHandle<()>>,
    /// Optional AOF writer.  `None` for the in-memory-only build
    /// produced by [`Store::new`].  [`Store::with_aof`] sets it to
    /// `Some(_)`.  Wrapped in `Arc` so cloning `Store` shares the
    /// same writer task (no duplicate fsync intervals).
    aof: Option<Arc<AofWriter>>,
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
            aof: None,
        }
    }

    /// Construct a `Store` *and* spawn an [`AofWriter`] bound to
    /// `path` with the given fsync cadence.
    ///
    /// The AOF writer is installed **after** any replay step (see
    /// [`Store::replay_from_path`] — the caller runs replay against a
    /// store built via [`Store::new`] first, then upgrades to AOF via
    /// this constructor before binding listeners).
    ///
    /// Convenience wrapper around [`Store::new`] + [`Store::attach_aof`]
    /// for tests / callers that don't need to replay first.
    ///
    /// # Errors
    /// Surfaces the underlying `io::Error` if the AOF file cannot be
    /// opened for append (e.g. parent dir missing, permission denied).
    pub async fn with_aof(path: PathBuf, fsync: FsyncPolicy) -> Result<Self, StoreError> {
        Self::new().attach_aof(path, fsync).await
    }

    /// Request the AOF writer to flush its queue and `sync_data` the
    /// file.  Returns immediately when the store has no AOF writer.
    ///
    /// Used by tests and by the (future, M4) graceful-shutdown path
    /// to guarantee a durability anchor — the file on disk reflects
    /// every command that has reached this point.
    pub async fn aof_flush(&self) {
        if let Some(writer) = self.aof.as_ref() {
            writer.flush().await;
        }
    }

    /// Graft an [`AofWriter`] onto an already-constructed `Store`,
    /// preserving the existing in-memory map.
    ///
    /// Designed for the `main.rs` replay-then-bind flow:
    /// `Store::new()` → `replay_from_path` → `attach_aof` →
    /// `server::run`.  Doing replay against the no-AOF store first
    /// guarantees the file isn't re-extended by the replayed
    /// commands.
    ///
    /// # Errors
    /// Same as [`Store::with_aof`].
    pub async fn attach_aof(
        mut self,
        path: PathBuf,
        fsync: FsyncPolicy,
    ) -> Result<Self, StoreError> {
        let writer = AofWriter::new(path, fsync).await?;
        self.aof = Some(Arc::new(writer));
        Ok(self)
    }

    /// Replay a RESP-encoded AOF file into the store.
    ///
    /// Iterates the file's bytes with [`Frame::parse`], converts each
    /// `Array` frame into a [`Command`], and re-executes it via the
    /// AOF-skipping path ([`Store::execute_no_aof`]) so the same file
    /// is not re-extended during replay.
    ///
    /// Behaviour for malformed inputs:
    /// - `Frame::parse` returning `Incomplete` at end-of-file is
    ///   treated as a *truncated tail* — log a warning, return the
    ///   number of complete commands replayed so far.  The next live
    ///   write extends the file from its true length (`OpenOptions
    ///   .append(true)` semantics; no rewrite-on-corruption in v0.1).
    /// - `Frame::parse` returning `Invalid` mid-stream is also
    ///   treated as a truncation: stop the replay at that offset,
    ///   log a warn, return the count.  This matches the ADR-0010
    ///   §"AOF 损坏 — warn-and-truncate" decision (the M3.2 finding
    ///   `m3-2-aof-replay-corruption-handling.md` documents the
    ///   trade-off vs refuse-to-start).
    /// - A frame that *parses* but is not a writable command
    ///   (because someone hand-edited the file to contain GET, say)
    ///   is silently skipped — read-only commands have no effect on
    ///   state anyway.
    /// - Path that does not exist → return Ok(0) — same effect as
    ///   replaying an empty AOF.  Callers (`main.rs`) check
    ///   `Path::exists()` themselves but we tolerate the case here.
    ///
    /// Returns the number of commands successfully replayed.
    ///
    /// # Errors
    /// Returns [`StoreError::Io`] only for fatal IO errors when
    /// reading the file (permission denied, mid-read EOF on a real
    /// disk failure).  Format errors do NOT propagate — they are
    /// logged and treated as truncations per the policy above.
    pub fn replay_from_path(&self, path: &Path) -> Result<usize, StoreError> {
        if !path.exists() {
            return Ok(0);
        }
        let bytes = std::fs::read(path)?;
        let mut buf: BytesMut = BytesMut::with_capacity(bytes.len());
        buf.extend_from_slice(&bytes);

        let mut count: usize = 0;
        loop {
            match Frame::parse(&buf[..]) {
                Ok((frame, n)) => {
                    // Convert the frame to a command using the same
                    // case-insensitive lookup the dispatch layer uses
                    // — but the dispatch crate lives upstream of us.
                    // Inline a tiny parser specialised for the 6
                    // writable verbs (ADR-0010 §"writable command
                    // set"); anything else is silently skipped so
                    // hand-edited AOFs with GET / TYPE / etc. don't
                    // crash replay.
                    if let Some(cmd) = parse_writable_frame(&frame) {
                        // execute_no_aof is infallible for the
                        // writable subset (no IO, just map ops).
                        // Errors here would mean a logic bug.
                        let _ = self.execute_no_aof(cmd);
                        count += 1;
                    } else {
                        tracing::debug!("AOF replay: skipping non-writable / unparseable frame");
                    }
                    buf.advance(n);
                }
                Err(ProtocolError::Incomplete) => {
                    if !buf.is_empty() {
                        tracing::warn!(
                            tail_bytes = buf.len(),
                            "AOF replay: truncated tail; ignoring remaining bytes"
                        );
                    }
                    break;
                }
                Err(ProtocolError::Invalid(msg)) => {
                    tracing::warn!(
                        error = msg,
                        tail_bytes = buf.len(),
                        "AOF replay: malformed frame; stopping replay (warn-and-truncate)"
                    );
                    break;
                }
                Err(ProtocolError::Utf8(e)) => {
                    tracing::warn!(
                        error = %e,
                        tail_bytes = buf.len(),
                        "AOF replay: utf-8 error in frame; stopping replay (warn-and-truncate)"
                    );
                    break;
                }
            }
        }
        Ok(count)
    }

    /// Execute a command and return a reply.
    ///
    /// Identical to [`Store::execute_no_aof`] plus an AOF append on
    /// success for the *writable* subset (SET / DEL / EXPIRE /
    /// PERSIST / INCR / DECR).  Read-only and Pub/Sub commands skip
    /// the append.  When no AOF writer is configured (i.e.
    /// [`Store::new`] build), this is a thin wrapper.
    ///
    /// # Errors
    ///
    /// Returns `StoreError` on internal IO or invariant violation.
    /// "Key not found" maps to `Reply::Bulk(None)`, not an error.
    pub fn execute(&self, cmd: Command) -> Result<Reply, StoreError> {
        // Encode FIRST so we don't consume `cmd` before we can both
        // run it and emit it.  `aof_encode` is allocation-light
        // (one Vec<Frame> + a single to_bytes pass) and returns
        // `None` immediately for non-writable arms — cost is
        // negligible for the read-only hot path.
        let encoded = self.aof.as_ref().and_then(|_| aof_encode(&cmd));

        let reply = self.execute_no_aof(cmd)?;

        // Only append if the in-memory mutation succeeded AND the
        // reply is not an `Error` (e.g. INCR on a non-integer string
        // returns Reply::Error and must NOT enter the AOF — replay
        // would re-emit the same error and waste cycles).
        if let (Some(writer), Some(bytes), false) =
            (self.aof.as_ref(), encoded, matches!(reply, Reply::Error(_)))
        {
            writer.append(bytes);
        }

        Ok(reply)
    }

    /// Execute a command without AOF side-effects.
    ///
    /// Public-but-internal entry point used by [`Store::replay_from_path`]
    /// so replay does not re-extend the file it just read.  Identical
    /// semantics to [`Store::execute`] except the AOF append is skipped.
    ///
    /// # Errors
    ///
    /// Same shape as [`Store::execute`].
    pub fn execute_no_aof(&self, cmd: Command) -> Result<Reply, StoreError> {
        match cmd {
            Command::Ping { message } => match message {
                // `PING` with no message → `+PONG\r\n`.
                None => Ok(Reply::Pong),
                // `PING hello` → `$5\r\nhello\r\n` bulk string (matches Redis).
                Some(bytes) => Ok(Reply::Bulk(Some(bytes))),
            },

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

            // ── M1.4 (ADR-0006) ──────────────────────────────────────────
            // Each M1.4 arm is delegated to a per-command method to keep
            // the top-level dispatch match readable (ADR-0004 §Notes).
            Command::Expire { key, seconds } => Ok(self.do_expire(key, seconds)),
            Command::Ttl { key } => Ok(Self::do_ttl(&self.inner, &key)),
            Command::Persist { key } => Ok(Self::do_persist(&self.inner, &key)),
            Command::Type { key } => Ok(Self::do_type(&self.inner, &key)),
            Command::Keys { pattern } => Ok(Self::do_keys(&self.inner, &pattern)),

            // ── M3.1 (ADR-0009) Pub/Sub ──────────────────────────────────
            // SUBSCRIBE / UNSUBSCRIBE produce *N replies + per-conn
            // state*, which doesn't fit `execute -> single Reply`.
            // The server crate calls `Store::subscribe` / `unsubscribe`
            // directly; if either variant reaches `execute` it's a
            // dispatch bug — return a generic error rather than
            // panicking (CLAUDE.md §3.1 — non-test code no .unwrap()).
            Command::Subscribe { .. } | Command::Unsubscribe { .. } => Ok(Reply::Error(
                "ERR internal: subscribe/unsubscribe must be handled by the server layer"
                    .to_owned(),
            )),
            Command::Publish { channel, message } => Ok(self.do_publish(&channel, message)),
        }
    }

    // ── M3.1 (ADR-0009) Pub/Sub helpers ─────────────────────────────────────

    /// Register interest in `channel` and return a `broadcast::Receiver`
    /// the caller (server crate, per-connection task) must drain.
    ///
    /// Creates a fresh broadcast channel of capacity
    /// [`PUBSUB_BROADCAST_CAPACITY`] on first subscribe; subsequent
    /// subscribers share the same `Sender`.
    ///
    /// Takes the inner write lock briefly (one hashmap lookup +
    /// possible insert).  Caller is expected to hold the returned
    /// `Receiver` for as long as the connection is subscribed.
    #[must_use]
    pub fn subscribe(&self, channel: &str) -> tokio::sync::broadcast::Receiver<Arc<Vec<u8>>> {
        let mut guard = self.inner.write();
        let tx = guard
            .subscribers
            .entry(channel.to_owned())
            .or_insert_with(|| tokio::sync::broadcast::channel(PUBSUB_BROADCAST_CAPACITY).0);
        tx.subscribe()
    }

    /// Read-only snapshot of `(channel, receiver_count)` pairs for the
    /// `/api/pubsub` dashboard.  Sorted by channel name for stable SSE
    /// output (ADR-0009 §Q7).
    ///
    /// Walks the subscribers map under a single read lock; O(N) in
    /// channel count with one allocation for the output `Vec`.
    #[must_use]
    pub fn pubsub_snapshot(&self) -> Vec<(String, usize)> {
        let guard = self.inner.read();
        let mut out: Vec<(String, usize)> = guard
            .subscribers
            .iter()
            .map(|(k, tx)| (k.clone(), tx.receiver_count()))
            .collect();
        // hashbrown iteration order is non-deterministic — sort by name
        // so the SSE consumer sees a stable shape (matches the keys
        // dashboard convention of pure data, not iteration order).
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// `PUBLISH channel message` — fan-out the payload via the channel's
    /// broadcast sender.  Returns `Reply::Integer(N)` where `N` is the
    /// number of receivers reached at send instant.
    ///
    /// - `channel` not in the subscribers map → `N = 0` (no Sender to
    ///   send through).
    /// - `Sender::send` returns `Err(SendError(_))` when *all* receivers
    ///   have dropped between `entry()` and `send()` — also `N = 0`.
    ///
    /// `Arc::new(message)` so M subscribers share one allocation
    /// (ADR-0009 §"不允许在热路径里 allocate" — N subscribers don't pay
    /// N × Vec<u8> clones, only N × atomic ref-count increments).
    fn do_publish(&self, channel: &str, message: Vec<u8>) -> Reply {
        let guard = self.inner.read();
        let Some(tx) = guard.subscribers.get(channel) else {
            return Reply::Integer(0);
        };
        let payload = Arc::new(message);
        match tx.send(payload) {
            Ok(n) => Reply::Integer(i64::try_from(n).unwrap_or(i64::MAX)),
            // `SendError` here means receiver_count() was 0 at the send
            // instant.  Match Redis behaviour: returns 0.
            Err(_) => Reply::Integer(0),
        }
    }

    // ── M1.4 (ADR-0006) per-command helpers ─────────────────────────────────

    /// `EXPIRE key seconds` — DelayQueue Option A:
    /// rewrite `entry.expires_at` AND send a fresh `(key, dur)` to
    /// `ttl_tx`.  The old DelayQueue entry, when it fires, will check
    /// `entry.expires_at <= now` and skip if mismatched — that guard
    /// already exists in `Store::new()`'s background task.
    fn do_expire(&self, key: String, seconds: i64) -> Reply {
        let mut guard = self.inner.write();
        if !guard.map.contains_key(&key) {
            return Reply::Integer(0);
        }
        // Negative / zero TTL on real Redis deletes the key immediately.
        if seconds <= 0 {
            guard.map.remove(&key);
            return Reply::Integer(1);
        }
        let secs_u64: u64 = u64::try_from(seconds).unwrap_or(0);
        let new_expires_at = Instant::now() + std::time::Duration::from_secs(secs_u64);
        if let Some(entry) = guard.map.get_mut(&key) {
            entry.expires_at = Some(new_expires_at);
        }
        drop(guard);
        let _ = self
            .ttl_tx
            .send((key, std::time::Duration::from_secs(secs_u64)));
        Reply::Integer(1)
    }

    /// `TTL key` — Redis wire semantics:
    /// `-2 = absent`, `-1 = no TTL`, otherwise remaining seconds.
    ///
    /// Rounding: matches real Redis `(pttl_ms + 500) / 1000` — i.e.
    /// round-to-nearest, half-up.  ADR-0006 originally specified `floor`
    /// but the F23-A docker oracle (`redis:7-alpine`) revealed that real
    /// Redis returns the originally-requested N immediately after
    /// `SET k v EX N`; floor would return N − 1 once any sub-millisecond
    /// of dispatch latency has elapsed.  This is logged as an ADR-0006
    /// addendum in the M1.4 completion report.
    fn do_ttl(inner: &Arc<RwLock<Inner>>, key: &str) -> Reply {
        let guard = inner.read();
        let now = Instant::now();
        match guard.map.get(key) {
            None => Reply::Integer(-2),
            Some(entry) => match entry.expires_at {
                None => Reply::Integer(-1),
                Some(t) if t <= now => Reply::Integer(-2),
                Some(t) => {
                    let remaining = t.saturating_duration_since(now);
                    // Round-to-nearest (half up).  `as_millis()` returns
                    // u128; we go via i64 with saturation.
                    let ms: u128 = remaining.as_millis();
                    let secs_rounded: u128 = (ms + 500) / 1000;
                    let secs: i64 = i64::try_from(secs_rounded).unwrap_or(i64::MAX);
                    Reply::Integer(secs)
                }
            },
        }
    }

    /// `PERSIST key` — clear TTL on existing key.  The stale DelayQueue
    /// entry, when fired, will check `expires_at == None` → skip.
    fn do_persist(inner: &Arc<RwLock<Inner>>, key: &str) -> Reply {
        let mut guard = inner.write();
        match guard.map.get_mut(key) {
            None => Reply::Integer(0),
            Some(entry) => {
                if entry.expires_at.is_some() {
                    entry.expires_at = None;
                    Reply::Integer(1)
                } else {
                    Reply::Integer(0)
                }
            }
        }
    }

    /// `TYPE key` — v0.1: `"string"` or `"none"` SimpleString.
    fn do_type(inner: &Arc<RwLock<Inner>>, key: &str) -> Reply {
        let guard = inner.read();
        let now = Instant::now();
        let kind = match guard.map.get(key) {
            Some(entry) if entry.expires_at.is_none_or(|t| t > now) => "string",
            _ => "none",
        };
        Reply::SimpleString(kind.to_owned())
    }

    /// `KEYS pattern` — full keyspace scan with glob match.  Skips
    /// logically-expired entries (mirrors EXISTS).  CLAUDE.md §3.3:
    /// the matcher takes byte slices and does not allocate per call.
    fn do_keys(inner: &Arc<RwLock<Inner>>, pattern: &str) -> Reply {
        let guard = inner.read();
        let now = Instant::now();
        let pattern_bytes = pattern.as_bytes();
        let mut out: Vec<Vec<u8>> = Vec::with_capacity(guard.map.len());
        for (k, entry) in &guard.map {
            if entry.expires_at.is_some_and(|t| t <= now) {
                continue;
            }
            if glob::matches(pattern_bytes, k.as_bytes()) {
                out.push(k.as_bytes().to_vec());
            }
        }
        Reply::Array(Some(out))
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

/// Parse a RESP `Frame::Array` into one of the 6 writable Commands.
///
/// Returns `None` for anything that is not an Array of BulkStrings
/// starting with a recognised writable verb.  This is deliberately
/// independent of `redis-server::dispatch` — the storage crate must
/// stay self-contained (cs01 CLAUDE.md §4 layer rule).  Wire shapes
/// match `aof_encode`'s output, so replay is a true round-trip.
///
/// Hand-edited AOFs with unknown commands (or RESP shapes other than
/// Array-of-Bulk) are skipped silently — see [`Store::replay_from_path`].
fn parse_writable_frame(f: &Frame) -> Option<Command> {
    let Frame::Array(Some(parts)) = f else {
        return None;
    };
    let name = bulk_str(parts.first())?.to_ascii_uppercase();

    match name.as_str() {
        "SET" => {
            // SET k v        → 3 parts
            // SET k v EX n   → 5 parts
            let key = bulk_str(parts.get(1))?;
            let value = bulk_bytes(parts.get(2))?;
            let ttl_secs = match parts.len() {
                3 => None,
                5 => {
                    let opt = bulk_str(parts.get(3))?;
                    if !opt.eq_ignore_ascii_case("EX") {
                        return None;
                    }
                    let secs_str = bulk_str(parts.get(4))?;
                    Some(secs_str.parse::<u64>().ok()?)
                }
                _ => return None,
            };
            Some(Command::Set {
                key,
                value,
                ttl_secs,
            })
        }
        "DEL" => {
            if parts.len() < 2 {
                return None;
            }
            let mut keys: Vec<String> = Vec::with_capacity(parts.len() - 1);
            for p in &parts[1..] {
                keys.push(bulk_str(Some(p))?);
            }
            Some(Command::Del { keys })
        }
        "EXPIRE" => {
            if parts.len() != 3 {
                return None;
            }
            let key = bulk_str(parts.get(1))?;
            let seconds: i64 = bulk_str(parts.get(2))?.parse().ok()?;
            Some(Command::Expire { key, seconds })
        }
        "PERSIST" => {
            if parts.len() != 2 {
                return None;
            }
            Some(Command::Persist {
                key: bulk_str(parts.get(1))?,
            })
        }
        "INCR" => {
            if parts.len() != 2 {
                return None;
            }
            Some(Command::Incr {
                key: bulk_str(parts.get(1))?,
            })
        }
        "DECR" => {
            if parts.len() != 2 {
                return None;
            }
            Some(Command::Decr {
                key: bulk_str(parts.get(1))?,
            })
        }
        _ => None,
    }
}

/// Helper: pull a UTF-8 String out of a `Frame::BulkString(Some(...))`.
fn bulk_str(f: Option<&Frame>) -> Option<String> {
    match f {
        Some(Frame::BulkString(Some(b))) => String::from_utf8(b.clone()).ok(),
        _ => None,
    }
}

/// Helper: pull raw bytes out of a `Frame::BulkString(Some(...))`.
fn bulk_bytes(f: Option<&Frame>) -> Option<Vec<u8>> {
    match f {
        Some(Frame::BulkString(Some(b))) => Some(b.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn store_ping_no_message() {
        let s = Store::new();
        let r = s
            .execute(Command::Ping { message: None })
            .expect("ping infallible");
        assert_eq!(r, Reply::Pong);
    }

    #[tokio::test]
    async fn store_ping_with_message_returns_bulk() {
        let s = Store::new();
        let r = s
            .execute(Command::Ping {
                message: Some(b"hello".to_vec()),
            })
            .expect("ping infallible");
        assert_eq!(r, Reply::Bulk(Some(b"hello".to_vec())));
    }
}
