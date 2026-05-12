//! Append-Only-File (AOF) persistence — write path + fsync policy.
//!
//! M3.2 (ADR-0010). Two concerns live here:
//!
//! 1. **[`aof_encode`]** — convert a *writable* [`Command`] into the
//!    exact RESP byte sequence a real client would send.  Read-only
//!    and volatile-state (Pub/Sub) commands return `None` and are NOT
//!    appended.  This is the F24 defence: the AOF format is the same
//!    wire format Redis itself uses; `cat appendonly.aof | redis-cli
//!    --pipe` is a valid replay path against real Redis (the oracle
//!    `tests/oracle_aof.py` exercises this property).
//!
//! 2. **[`AofWriter`]** — async background writer that owns the file
//!    handle.  The hot RESP path pushes encoded bytes via an
//!    `mpsc::UnboundedSender` and never touches `fs::write`.  fsync
//!    cadence is one of [`FsyncPolicy::Always`] / `Everysec` / `No`,
//!    matching real Redis' `appendfsync` triplet.
//!
//! Hook position: [`crate::Store::execute`] calls `aof_encode(&cmd)`
//! **after** the in-memory mutation succeeds and **only** when the
//! Store was constructed via [`crate::Store::with_aof`].  Replay re-
//! enters the same dispatch through [`crate::Store::execute_no_aof`]
//! — a private helper that skips the AOF write so the file isn't
//! duplicated.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use redis_protocol::Frame;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::Command;

/// Message sent over the writer task's mpsc.
///
/// `Append` is the fast path; `Flush` is a control message used by
/// tests (and, eventually, graceful shutdown) to checkpoint the file.
/// Encoded as an enum so the channel signature stays simple
/// (`UnboundedSender<AofMsg>`) and so we don't need a second control
/// channel that complicates the writer task's `select!`.
enum AofMsg {
    Append(Vec<u8>),
    /// Caller waits on the oneshot Receiver; the writer signals it
    /// AFTER every prior `Append` has been flushed.
    Flush(oneshot::Sender<()>),
}

/// fsync cadence selector — mirrors Redis' `appendfsync` directive.
///
/// `Always` flushes after every record (slow but durable).
/// `Everysec` flushes once per second from a background interval
/// (good throughput, ≤ 1 s data loss on power-cut).
/// `No` lets the OS decide — typically fastest, weakest durability.
///
/// Default is [`FsyncPolicy::Everysec`] — same as real Redis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FsyncPolicy {
    Always,
    #[default]
    Everysec,
    No,
}

impl FsyncPolicy {
    /// Parse the CLI string spelling.  Accepts the same three values
    /// Redis does (case-insensitive).
    ///
    /// # Errors
    /// Returns an `Err` carrying the verbatim user input when the
    /// value is not one of `always` / `everysec` / `no`.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "always" => Ok(Self::Always),
            "everysec" => Ok(Self::Everysec),
            "no" => Ok(Self::No),
            _ => Err(format!(
                "invalid --aof-fsync {s:?}: expected one of always / everysec / no"
            )),
        }
    }
}

/// Background AOF writer.
///
/// `tx` is the producer side of an unbounded mpsc; the hot RESP path
/// uses [`AofWriter::append`] which never blocks.  The owned
/// `_task` handle is kept inside an `Arc` so cloning the `Store`
/// (and therefore the `Option<Arc<AofWriter>>` it holds) does not
/// cancel the spawned task.
pub struct AofWriter {
    tx: UnboundedSender<AofMsg>,
    /// Underscore: never read at runtime; we keep the handle alive
    /// for the lifetime of the writer.  Cloning the `Arc` extends the
    /// task's lifetime across all `Store` clones.
    _task: Arc<JoinHandle<()>>,
}

impl AofWriter {
    /// Spawn the writer task and return a producer handle.
    ///
    /// `path` is opened with `create(true).append(true)` so an
    /// existing AOF file is preserved (replay runs first; new writes
    /// extend the tail).
    ///
    /// # Errors
    /// Returns the underlying [`io::Error`] if the file cannot be
    /// opened (permission denied, parent dir missing, etc.).  Done
    /// here — at construction — so `main.rs` surfaces the problem
    /// before binding any listener.
    pub async fn new(path: PathBuf, fsync: FsyncPolicy) -> io::Result<Self> {
        // Probe the file once synchronously so a fatal open error is
        // returned to the caller (rather than swallowed by the
        // background task).  Drop the handle and reopen inside the
        // task so the file lifecycle lives entirely on that future.
        let probe = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;
        drop(probe);

        let (tx, mut rx) = mpsc::unbounded_channel::<AofMsg>();

        let task_path = path.clone();
        let handle = tokio::spawn(async move {
            let mut file = match OpenOptions::new()
                .create(true)
                .append(true)
                .open(&task_path)
                .await
            {
                Ok(f) => f,
                Err(e) => {
                    // The probe succeeded, so this branch is essentially
                    // unreachable in practice; log defensively and exit
                    // the task — the next `append` send will silently
                    // fail (matches "OS decides" durability for that
                    // record under power-cut).
                    tracing::error!(error = %e, path = %task_path.display(), "AOF writer failed to reopen file");
                    return;
                }
            };

            // Background interval only matters for the Everysec arm,
            // but we always construct it so the `select!` arity is
            // consistent.  `tick()` consumes one immediate tick when
            // first polled — we discard it so the first real fsync is
            // ~1 s into the run, not at startup.
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            interval.tick().await;

            loop {
                tokio::select! {
                    msg = rx.recv() => {
                        match msg {
                            Some(AofMsg::Append(bytes)) => {
                                if let Err(e) = file.write_all(&bytes).await {
                                    tracing::warn!(error = %e, "AOF write_all failed");
                                    continue;
                                }
                                if matches!(fsync, FsyncPolicy::Always)
                                    && let Err(e) = file.sync_data().await
                                {
                                    // ADR-0010 §Consequences flags
                                    // this as a finding candidate
                                    // for M4 (escalate to P0).
                                    tracing::warn!(error = %e, "AOF sync_data failed");
                                }
                            }
                            Some(AofMsg::Flush(ack)) => {
                                // Sync regardless of policy — Flush is
                                // a user-asked checkpoint that demands
                                // a fresh durability anchor (tests rely
                                // on it; graceful shutdown will too).
                                if let Err(e) = file.sync_data().await {
                                    tracing::warn!(error = %e, "AOF flush sync_data failed");
                                }
                                // Receiver dropping while we hold the
                                // sender is harmless — the test must
                                // have lost interest mid-flush.
                                let _ = ack.send(());
                            }
                            None => {
                                // All senders dropped → flush + exit.
                                if matches!(fsync, FsyncPolicy::Always | FsyncPolicy::Everysec)
                                    && let Err(e) = file.sync_data().await
                                {
                                    tracing::warn!(error = %e, "AOF final sync_data failed");
                                }
                                break;
                            }
                        }
                    }
                    _ = interval.tick(), if matches!(fsync, FsyncPolicy::Everysec) => {
                        if let Err(e) = file.sync_data().await {
                            tracing::warn!(error = %e, "AOF interval sync_data failed");
                        }
                    }
                }
            }
        });

        Ok(Self {
            tx,
            _task: Arc::new(handle),
        })
    }

    /// Enqueue one encoded RESP frame.  Never blocks; the mpsc is
    /// unbounded.  A closed channel (writer task gone) silently drops
    /// the record — the user has the same durability guarantee that
    /// power-cut-during-write would give them.
    pub fn append(&self, encoded: Vec<u8>) {
        // `.ok()` discards `SendError` deliberately — see doc above.
        let _ = self.tx.send(AofMsg::Append(encoded));
    }

    /// Request a flush and wait for the writer task to confirm.
    ///
    /// Sends a control `Flush` message; the writer task processes it
    /// AFTER every prior `Append`, calls `sync_data`, and signals the
    /// oneshot.  Returns immediately if the channel is closed (no
    /// writer to wait for).
    ///
    /// Used by:
    /// - Storage integration tests, to make file-content assertions
    ///   deterministic without `tokio::time::sleep` guesswork.
    /// - End-to-end restart tests (the server e2e harness), to
    ///   guarantee the AOF is fully on disk before a simulated kill.
    /// - Future graceful-shutdown path (M4 release-readiness).
    pub async fn flush(&self) {
        let (tx, rx) = oneshot::channel();
        if self.tx.send(AofMsg::Flush(tx)).is_err() {
            // Channel closed → no writer task → nothing to wait for.
            return;
        }
        let _ = rx.await;
    }
}

// ── Encoding ─────────────────────────────────────────────────────────────────

/// Encode a writable command into its RESP wire bytes, or `None` for
/// read-only / volatile commands that must NOT enter the AOF.
///
/// The wire shape matches what a real client would send (so
/// `cat appendonly.aof | redis-cli --pipe` is a valid replay path
/// against real Redis).  Implemented via [`Frame::to_bytes`] on
/// hand-built `Frame::Array(Some(...))` payloads — single source of
/// RESP serialisation logic (ADR-0010 §"Decision/AOF format" F24
/// defence).
///
/// Writable set (locked in ADR-0010 §"writable command set"):
/// `SET` (with optional `EX`), `DEL`, `EXPIRE`, `PERSIST`, `INCR`,
/// `DECR`.  Everything else returns `None`.
#[must_use]
pub fn aof_encode(cmd: &Command) -> Option<Vec<u8>> {
    let frame = match cmd {
        Command::Set {
            key,
            value,
            ttl_secs,
        } => {
            let mut parts: Vec<Frame> = Vec::with_capacity(5);
            parts.push(Frame::BulkString(Some(b"SET".to_vec())));
            parts.push(Frame::BulkString(Some(key.as_bytes().to_vec())));
            parts.push(Frame::BulkString(Some(value.clone())));
            if let Some(secs) = ttl_secs {
                parts.push(Frame::BulkString(Some(b"EX".to_vec())));
                parts.push(Frame::BulkString(Some(secs.to_string().into_bytes())));
            }
            Frame::Array(Some(parts))
        }
        Command::Del { keys } => {
            let mut parts: Vec<Frame> = Vec::with_capacity(1 + keys.len());
            parts.push(Frame::BulkString(Some(b"DEL".to_vec())));
            for k in keys {
                parts.push(Frame::BulkString(Some(k.as_bytes().to_vec())));
            }
            Frame::Array(Some(parts))
        }
        Command::Expire { key, seconds } => Frame::Array(Some(vec![
            Frame::BulkString(Some(b"EXPIRE".to_vec())),
            Frame::BulkString(Some(key.as_bytes().to_vec())),
            Frame::BulkString(Some(seconds.to_string().into_bytes())),
        ])),
        Command::Persist { key } => Frame::Array(Some(vec![
            Frame::BulkString(Some(b"PERSIST".to_vec())),
            Frame::BulkString(Some(key.as_bytes().to_vec())),
        ])),
        Command::Incr { key } => Frame::Array(Some(vec![
            Frame::BulkString(Some(b"INCR".to_vec())),
            Frame::BulkString(Some(key.as_bytes().to_vec())),
        ])),
        Command::Decr { key } => Frame::Array(Some(vec![
            Frame::BulkString(Some(b"DECR".to_vec())),
            Frame::BulkString(Some(key.as_bytes().to_vec())),
        ])),

        // ── Explicit "no-AOF" arms — listed verbatim so adding a new
        //    Command variant breaks compilation here and forces a
        //    decision rather than silently entering / skipping the
        //    AOF (F24 defence: no `_ => None` catch-all).
        Command::Ping { .. }
        | Command::Get { .. }
        | Command::Exists { .. }
        | Command::Echo { .. }
        | Command::Select { .. }
        | Command::Quit
        | Command::Ttl { .. }
        | Command::Type { .. }
        | Command::Keys { .. }
        | Command::Subscribe { .. }
        | Command::Unsubscribe { .. }
        | Command::Publish { .. } => return None,
    };

    Some(frame.to_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_set_no_ttl_is_3_part_array() {
        let cmd = Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        };
        let got = aof_encode(&cmd).expect("writable");
        assert_eq!(got, b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n");
    }

    #[test]
    fn encode_set_with_ttl_is_5_part_array() {
        let cmd = Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: Some(60),
        };
        let got = aof_encode(&cmd).expect("writable");
        assert_eq!(
            got,
            b"*5\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n$2\r\nEX\r\n$2\r\n60\r\n"
        );
    }

    #[test]
    fn encode_del_multi_key() {
        let cmd = Command::Del {
            keys: vec!["a".to_owned(), "b".to_owned()],
        };
        let got = aof_encode(&cmd).expect("writable");
        assert_eq!(got, b"*3\r\n$3\r\nDEL\r\n$1\r\na\r\n$1\r\nb\r\n");
    }

    #[test]
    fn encode_expire_persist_incr_decr() {
        let expire = aof_encode(&Command::Expire {
            key: "k".to_owned(),
            seconds: 50,
        })
        .expect("writable");
        assert_eq!(expire, b"*3\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$2\r\n50\r\n");

        let persist = aof_encode(&Command::Persist {
            key: "k".to_owned(),
        })
        .expect("writable");
        assert_eq!(persist, b"*2\r\n$7\r\nPERSIST\r\n$1\r\nk\r\n");

        let incr = aof_encode(&Command::Incr {
            key: "c".to_owned(),
        })
        .expect("writable");
        assert_eq!(incr, b"*2\r\n$4\r\nINCR\r\n$1\r\nc\r\n");

        let decr = aof_encode(&Command::Decr {
            key: "c".to_owned(),
        })
        .expect("writable");
        assert_eq!(decr, b"*2\r\n$4\r\nDECR\r\n$1\r\nc\r\n");
    }

    #[test]
    fn readonly_and_volatile_commands_return_none() {
        assert!(aof_encode(&Command::Ping { message: None }).is_none());
        assert!(
            aof_encode(&Command::Get {
                key: "x".to_owned()
            })
            .is_none()
        );
        assert!(
            aof_encode(&Command::Exists {
                keys: vec!["x".to_owned()]
            })
            .is_none()
        );
        assert!(
            aof_encode(&Command::Echo {
                message: b"hi".to_vec()
            })
            .is_none()
        );
        assert!(aof_encode(&Command::Select { db: 0 }).is_none());
        assert!(aof_encode(&Command::Quit).is_none());
        assert!(
            aof_encode(&Command::Ttl {
                key: "x".to_owned()
            })
            .is_none()
        );
        assert!(
            aof_encode(&Command::Type {
                key: "x".to_owned()
            })
            .is_none()
        );
        assert!(
            aof_encode(&Command::Keys {
                pattern: "*".to_owned()
            })
            .is_none()
        );
        assert!(
            aof_encode(&Command::Subscribe {
                channels: vec!["c".to_owned()]
            })
            .is_none()
        );
        assert!(
            aof_encode(&Command::Unsubscribe {
                channels: vec!["c".to_owned()]
            })
            .is_none()
        );
        assert!(
            aof_encode(&Command::Publish {
                channel: "c".to_owned(),
                message: b"m".to_vec(),
            })
            .is_none()
        );
    }

    #[test]
    fn fsync_policy_parse_accepts_all_three() {
        assert_eq!(
            FsyncPolicy::parse("always").expect("ok"),
            FsyncPolicy::Always
        );
        assert_eq!(
            FsyncPolicy::parse("Everysec").expect("case-insensitive"),
            FsyncPolicy::Everysec
        );
        assert_eq!(FsyncPolicy::parse("NO").expect("ok"), FsyncPolicy::No);
    }

    #[test]
    fn fsync_policy_parse_rejects_garbage() {
        let err = FsyncPolicy::parse("hourly").expect_err("not a real cadence");
        assert!(err.contains("expected one of"), "msg={err}");
    }

    #[test]
    fn fsync_policy_default_is_everysec() {
        assert_eq!(FsyncPolicy::default(), FsyncPolicy::Everysec);
    }
}
