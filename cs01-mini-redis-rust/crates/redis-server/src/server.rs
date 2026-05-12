//! RESP TCP listener (ADR-0005, ADR-0006, ADR-0009).
//!
//! Accept-loop pattern (M3.1):
//! ```text
//! TcpListener::accept
//!   → spawn(handle_conn)
//!     → loop {
//!         select! {
//!             read_buf → parse Frame → dispatch → reply
//!             pubsub_recv → push `message` Frame to socket
//!         }
//!       }
//! ```
//!
//! Locked sub-decisions:
//! - Per-connection task via `tokio::spawn` (fault isolation, ADR-0005).
//! - `BytesMut::with_capacity(4096)` + manual drain — **no** `Framed`
//!   codec (that would re-wrap the pure-function parser; F24 candidate).
//! - Protocol error → send `-ERR ...` then close socket.
//! - `QUIT` → send `+OK`, then close socket (caller-side responsibility).
//! - Graceful shutdown: `ctrl_c` breaks the accept loop; in-flight tasks
//!   finish naturally (M3.2 may upgrade to drain mode).
//! - M1.4 (ADR-0006): `max_frame_size` guard on the *buffer total length*
//!   after each `read_buf`.  Default 512 MiB (matches Redis
//!   `proto-max-bulk-len`).
//! - M3.1 (ADR-0009): per-connection [`ConnState`] enum tracks
//!   `Normal` vs `Subscribed { rxs }`.  Sub mode rejects any command
//!   except (P)SUBSCRIBE / (P)UNSUBSCRIBE / PING / QUIT / RESET with
//!   the verbatim Redis 7 error string.

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use bytes::{Buf, BytesMut};
use redis_protocol::{Frame, ProtocolError};
use redis_storage::{Command, Reply};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

use crate::dispatch::from_frame;
use crate::encode::reply_to_frame;
use crate::state::{AppState, ConnGuard};

/// Default `max-frame-size` guard threshold: 512 MiB.
///
/// Matches Redis' `proto-max-bulk-len` default.  Server `run`/`run_on`
/// callers pass an explicit `usize`; this constant is the canonical
/// default used by `main.rs` and tests.
pub const DEFAULT_MAX_FRAME_SIZE: usize = 512 * 1024 * 1024;

/// Bind a `TcpListener` on `addr`, accept connections forever, and
/// dispatch each one to `handle_conn`.
///
/// `max_frame_size` is the per-connection buffer ceiling (bytes).  When
/// a single read brings the buffer beyond this limit the connection is
/// terminated with `-ERR Protocol error: frame too big` (matches Redis).
/// Use [`DEFAULT_MAX_FRAME_SIZE`] for the production default.
///
/// Shuts down on `tokio::signal::ctrl_c`: the accept loop exits and
/// already-spawned per-connection tasks finish naturally (no in-flight
/// deadline in M1.3; M3 will upgrade to drain mode).
///
/// # Errors
///
/// Returns `io::Error` if the listener cannot bind.  Per-connection
/// IO errors are logged but **never** propagated — one bad connection
/// must not kill the server (ADR-0005 §"Consequences/正面").
pub async fn run(addr: SocketAddr, state: AppState) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    tracing::info!(
        addr = %local_addr,
        max_frame_size = state.max_frame_size,
        "RESP listener bound"
    );
    run_on(listener, state).await
}

/// Run the accept loop on an already-bound `TcpListener`.
///
/// Split from [`run`] so integration tests can bind on `127.0.0.1:0`,
/// read back the OS-assigned port, and exercise the *exact* same
/// accept loop the production binary uses.
///
/// `max_frame_size` is forwarded to each spawned `handle_conn`.
///
/// Note: this variant does **not** install a `ctrl_c` handler — tests
/// stop the loop by aborting the spawned task or dropping the
/// owning `Store` (which lets the listener's task exit naturally on
/// the next iteration).  The production [`run`] wraps it with the
/// signal handler.
///
/// # Errors
///
/// Per-connection IO errors are logged, not propagated.  This
/// function only returns when its `select!` arm chooses `ctrl_c`
/// (production) or the task is aborted (tests).
pub async fn run_on(listener: TcpListener, state: AppState) -> io::Result<()> {
    loop {
        tokio::select! {
            accept = listener.accept() => {
                match accept {
                    Ok((socket, peer)) => {
                        let state = state.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_conn(socket, state).await {
                                tracing::warn!(peer = %peer, error = %e, "conn closed with error");
                            }
                        });
                    }
                    Err(e) => {
                        // Accept errors are typically transient (e.g.,
                        // EMFILE).  Log and continue rather than crash.
                        tracing::warn!(error = %e, "accept failed");
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("ctrl_c received — shutting down");
                return Ok(());
            }
        }
    }
}

/// Per-connection Pub/Sub state (ADR-0009 §Q4).
///
/// `Normal` — the connection accepts the full RESP command set
/// (default state for any newly-accepted socket).
///
/// `Subscribed` — the connection is in "sub mode": one or more
/// `SUBSCRIBE` commands have registered receivers in `rxs`, and the
/// dispatch wall rejects everything except (P)SUBSCRIBE / (P)UNSUBSCRIBE
/// / PING / QUIT / RESET.  Sub-mode → Normal transition fires exactly
/// when the last receiver is removed via UNSUBSCRIBE.
///
/// We model this as an enum (rather than a `bool + Option<HashMap>`)
/// to make the two states distinguishable in `match` arms — the
/// compiler then enforces wall-coverage rather than relying on us
/// remembering to check both `is_some` and `subscribed_flag`.
enum ConnState {
    Normal,
    Subscribed {
        /// Per-channel broadcast receivers.  Ordered insertion in a
        /// `Vec<(String, …)>` would let us preserve subscribe ordering
        /// for the sake of ack frame ordering, but the per-channel ack
        /// is emitted *at SUBSCRIBE time* — `rxs` only needs to map
        /// channel → receiver for select-time fan-in.  `HashMap` is
        /// fine (and removes the O(N) unsubscribe walk).
        rxs: HashMap<String, broadcast::Receiver<Arc<Vec<u8>>>>,
    },
}

impl ConnState {
    fn is_subscribed(&self) -> bool {
        matches!(self, ConnState::Subscribed { .. })
    }

    /// Number of channels currently subscribed.  0 in `Normal`.
    fn subscription_count(&self) -> usize {
        match self {
            ConnState::Normal => 0,
            ConnState::Subscribed { rxs } => rxs.len(),
        }
    }
}

/// Drive a single client connection.  See module-level docs.
///
/// M3.1 (ADR-0009): the per-conn loop is now `tokio::select!` between
/// socket reads and per-channel Pub/Sub receivers.  When in sub mode,
/// a `message` frame is pushed for each broadcast::recv result; on
/// `Lagged`, the connection is reset (matches Redis behaviour of
/// disconnecting over-buffered Pub/Sub clients).
async fn handle_conn(mut socket: TcpStream, state: AppState) -> io::Result<()> {
    // RAII counter (ADR-0007 §Q6): increment connections_active here,
    // decrement on Drop.  Take the guard BEFORE any `?` so even
    // abnormal task termination (panic, early-return) still
    // decrements the counter via stack unwind.
    let _conn_guard = ConnGuard::new(state.connections_active.clone());

    let max_frame_size = state.max_frame_size;
    let mut buf = BytesMut::with_capacity(4096);
    let mut conn_state = ConnState::Normal;

    loop {
        // 1. Read more bytes OR receive a pub/sub message.  In Normal
        //    mode the second arm of `recv_any_subscription` returns
        //    `Pending` forever, so we just block on the read.  In
        //    Subscribed mode either arm may fire.
        let read_n = tokio::select! {
            biased; // Prefer reads over message fan-out so the client
                    // can't be starved by a very chatty publisher.
            r = socket.read_buf(&mut buf) => Some(r?),
            pushed = recv_any_subscription(&mut conn_state) => {
                match pushed {
                    Ok((channel, payload)) => {
                        // Push a `message` frame.  Cloning the Arc'd
                        // payload bytes into a Vec<u8> is required by
                        // the Frame::BulkString shape; the heavy
                        // allocation was avoided by sharing the Arc
                        // across all subscribers.
                        let msg = Reply::Message {
                            channel,
                            payload: (*payload).clone(),
                        };
                        let frame_out = reply_to_frame(msg);
                        socket.write_all(&frame_out.to_bytes()).await?;
                        None
                    }
                    Err(SubRecvError::Lagged) => {
                        // Matches real Redis: kill the laggard.
                        let err = Frame::Error(
                            "ERR client lagged behind pub/sub buffer; disconnecting".to_owned(),
                        );
                        let _ = socket.write_all(&err.to_bytes()).await;
                        return Ok(());
                    }
                }
            }
        };

        if let Some(n) = read_n {
            if n == 0 {
                // Peer closed.  If there's stale data in the buffer
                // it's an incomplete frame — treat as a clean disconnect.
                return Ok(());
            }
        } else {
            // Pub/Sub message branch — loop back to the next select.
            continue;
        }

        // 1a. M1.4 (ADR-0006) — frame-size guard.  If the accumulated
        //     buffer exceeds the configured ceiling, the client is
        //     either malicious or buggy; reject and close.
        if buf.len() > max_frame_size {
            let err = Frame::Error("ERR Protocol error: frame too big".to_owned());
            let _ = socket.write_all(&err.to_bytes()).await;
            return Ok(());
        }

        // 2. Drain as many complete frames as we have buffered.  Set
        //    `close_after = true` if a QUIT was processed; we still
        //    flush its reply before returning so the client sees `+OK`.
        let mut close_after = false;
        loop {
            match Frame::parse(&buf[..]) {
                Ok((frame, n)) => {
                    // ADR-0007 §Q6: count every parsed frame, even
                    // unknown commands / arity errors.  This matches
                    // real Redis `total_commands_processed`.
                    state.commands_total.fetch_add(1, Ordering::Relaxed);

                    // Dispatch frame → command (or arity-error Reply).
                    let cmd_result = from_frame(frame);
                    let replies: Vec<Reply> = match cmd_result {
                        Ok(cmd) => {
                            // Mark socket-close intent BEFORE handing the
                            // command to the store (ADR-0005 watchout).
                            if matches!(cmd, Command::Quit) {
                                close_after = true;
                            }
                            handle_command(&state, &mut conn_state, cmd)
                        }
                        Err(reply) => vec![reply],
                    };

                    for r in replies {
                        let frame_out = reply_to_frame(r);
                        socket.write_all(&frame_out.to_bytes()).await?;
                    }
                    buf.advance(n);
                }
                Err(ProtocolError::Incomplete) => {
                    // Need more bytes from the socket.
                    break;
                }
                Err(ProtocolError::Invalid(msg)) => {
                    // Best-effort: report error then close.  ADR-0005
                    // locks the message format with the literal "ERR "
                    // prefix; the Frame::Error variant adds the `-`.
                    let err_frame = Frame::Error(format!("ERR Protocol error: {msg}"));
                    let _ = socket.write_all(&err_frame.to_bytes()).await;
                    return Ok(());
                }
                Err(ProtocolError::Utf8(e)) => {
                    let err_frame = Frame::Error(format!("ERR Protocol error: utf-8: {e}"));
                    let _ = socket.write_all(&err_frame.to_bytes()).await;
                    return Ok(());
                }
            }
        }

        if close_after {
            // QUIT — reply already flushed above; drop the socket.
            return Ok(());
        }
    }
}

/// Outcome of one Pub/Sub fan-in poll.
enum SubRecvError {
    /// One of our broadcast receivers reported `Lagged` — the conn
    /// has fallen behind and Redis-style behaviour is to disconnect.
    Lagged,
}

/// Race over every subscribed channel's `broadcast::Receiver`.  Returns
/// the (channel, payload) of whichever channel produced the next
/// message, or `SubRecvError::Lagged` if any receiver fell behind.
///
/// In `Normal` mode this future is `Pending` forever — the caller's
/// `tokio::select!` will always favour the socket read arm.
///
/// Borrow-checker note: we can't build a `Vec<BoxFuture<…>>` over
/// `rxs` because each future would borrow the underlying receiver, and
/// the Vec would simultaneously alias N mutable borrows.  Instead, we
/// build a single composed future via `iter_mut().map(|(name, rx)| …)`
/// plus `select_all` — `iter_mut` returns disjoint `&mut` references
/// in one chain so the borrow checker is happy.
async fn recv_any_subscription(
    state: &mut ConnState,
) -> Result<(String, Arc<Vec<u8>>), SubRecvError> {
    match state {
        ConnState::Normal => std::future::pending().await,
        ConnState::Subscribed { rxs } => {
            use futures_util::future::FutureExt as _;
            if rxs.is_empty() {
                // Transient state during unsubscribe-all → Normal
                // transition; treat as pending so the caller hits the
                // read arm.
                std::future::pending().await
            } else {
                // Pair each receiver's `recv()` future with its channel
                // name (cloned, since the future owns its name copy).
                // `iter_mut` yields disjoint `&mut` borrows, so the
                // resulting Vec doesn't alias the map.
                let futures: Vec<_> = rxs
                    .iter_mut()
                    .map(|(name, rx)| {
                        let name = name.clone();
                        async move { (name, rx.recv().await) }.boxed()
                    })
                    .collect();
                let ((channel, result), _idx, _rest) =
                    futures_util::future::select_all(futures).await;
                match result {
                    Ok(payload) => Ok((channel, payload)),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        Err(SubRecvError::Lagged)
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        // The Sender side dropped — only possible if
                        // the Store itself is being torn down, which
                        // shouldn't happen mid-conn.  Return Pending
                        // forever — the conn will close on the next
                        // socket activity / EOF.
                        std::future::pending().await
                    }
                }
            }
        }
    }
}

/// Handle one parsed [`Command`] under the current [`ConnState`].
///
/// Returns 1..N `Reply` values that the caller flushes in order.
/// SUBSCRIBE / UNSUBSCRIBE emit one ack per channel; everything else
/// produces exactly one reply.
fn handle_command(state: &AppState, conn_state: &mut ConnState, cmd: Command) -> Vec<Reply> {
    // Sub-mode command-filter wall (ADR-0009 §Q4 watch-out): in
    // Subscribed mode, accept only the allow-list.  Error string is
    // verbatim Redis 7.
    if conn_state.is_subscribed() {
        let allowed = matches!(
            cmd,
            Command::Subscribe { .. }
                | Command::Unsubscribe { .. }
                | Command::Ping { .. }
                | Command::Quit
        );
        if !allowed {
            let name = command_name_for_wall(&cmd);
            return vec![Reply::Error(format!(
                "ERR Can't execute '{name}': only (P)SUBSCRIBE / (P)UNSUBSCRIBE / PING / QUIT / RESET are allowed in this context"
            ))];
        }
    }

    match cmd {
        // SUBSCRIBE / UNSUBSCRIBE are server-state-machine commands
        // (ADR-0009 §Q4); they bypass Store::execute.
        Command::Subscribe { channels } => handle_subscribe(state, conn_state, &channels),
        Command::Unsubscribe { channels } => handle_unsubscribe(conn_state, &channels),

        // Everything else goes through the store.
        other => {
            let r = match state.store.execute(other) {
                Ok(r) => r,
                Err(e) => Reply::Error(format!("ERR internal: {e}")),
            };
            vec![r]
        }
    }
}

/// Short command name used to build the sub-mode wall error string.
///
/// Redis lowercases the command name in this particular error message
/// (`"ERR Can't execute 'set'"`), so we match that style verbatim.
fn command_name_for_wall(cmd: &Command) -> &'static str {
    match cmd {
        Command::Ping { .. } => "ping",
        Command::Get { .. } => "get",
        Command::Set { .. } => "set",
        Command::Del { .. } => "del",
        Command::Exists { .. } => "exists",
        Command::Incr { .. } => "incr",
        Command::Decr { .. } => "decr",
        Command::Echo { .. } => "echo",
        Command::Select { .. } => "select",
        Command::Quit => "quit",
        Command::Expire { .. } => "expire",
        Command::Ttl { .. } => "ttl",
        Command::Persist { .. } => "persist",
        Command::Type { .. } => "type",
        Command::Keys { .. } => "keys",
        Command::Subscribe { .. } => "subscribe",
        Command::Unsubscribe { .. } => "unsubscribe",
        Command::Publish { .. } => "publish",
    }
}

/// SUBSCRIBE handler — registers a receiver per channel, returns one
/// `SubscribeAck` per requested channel with running totals.
fn handle_subscribe(
    state: &AppState,
    conn_state: &mut ConnState,
    channels: &[String],
) -> Vec<Reply> {
    // Transition Normal → Subscribed lazily.
    if !conn_state.is_subscribed() {
        *conn_state = ConnState::Subscribed {
            rxs: HashMap::new(),
        };
    }
    let ConnState::Subscribed { rxs } = conn_state else {
        // Just-set above; this branch is provably unreachable.
        return Vec::new();
    };

    let mut out: Vec<Reply> = Vec::with_capacity(channels.len());
    for channel in channels {
        // Idempotent re-subscribe: Redis keeps a single Receiver per
        // channel per client; if already subscribed, the running
        // count does NOT change and a fresh ack is still emitted with
        // the current count.
        if !rxs.contains_key(channel) {
            let rx = state.store.subscribe(channel);
            rxs.insert(channel.clone(), rx);
        }
        let count = i64::try_from(rxs.len()).unwrap_or(i64::MAX);
        out.push(Reply::SubscribeAck {
            channel: channel.clone(),
            count,
        });
    }
    out
}

/// UNSUBSCRIBE handler — removes receivers; transitions back to Normal
/// when zero channels remain.  Empty `channels` = unsubscribe from all.
fn handle_unsubscribe(conn_state: &mut ConnState, channels: &[String]) -> Vec<Reply> {
    // Handle the special case where the connection has no current
    // subscriptions.  Redis still emits a single `unsubscribe / nil / 0`
    // ack (ADR-0009 watch-out).  Verified by the M3.1 oracle.
    if !conn_state.is_subscribed() {
        return vec![Reply::UnsubscribeAck {
            channel: None,
            count: 0,
        }];
    }

    // SAFETY: just guarded above with is_subscribed().
    let ConnState::Subscribed { rxs } = conn_state else {
        return Vec::new();
    };

    // Determine the target list.
    let targets: Vec<String> = if channels.is_empty() {
        // UNSUBSCRIBE (no args) — every currently-subscribed channel,
        // in some deterministic-ish order.  Use insertion-style sort
        // so test snapshots are stable.
        let mut all: Vec<String> = rxs.keys().cloned().collect();
        all.sort();
        all
    } else {
        channels.to_vec()
    };

    let mut out: Vec<Reply> = Vec::with_capacity(targets.len().max(1));
    for channel in &targets {
        // Drop the receiver if present.  If the channel was not
        // subscribed, Redis still emits an ack with the current count.
        rxs.remove(channel);
        let count = i64::try_from(rxs.len()).unwrap_or(i64::MAX);
        out.push(Reply::UnsubscribeAck {
            channel: Some(channel.clone()),
            count,
        });
    }

    // Empty target list (i.e. `UNSUBSCRIBE` with no args AND no
    // current subscriptions) was redirected to the
    // "!is_subscribed()" branch above; we shouldn't get here with
    // an empty `out`, but be defensive: emit the nil/0 ack.
    if out.is_empty() {
        out.push(Reply::UnsubscribeAck {
            channel: None,
            count: 0,
        });
    }

    if conn_state.subscription_count() == 0 {
        // Fan-in back to Normal mode.
        *conn_state = ConnState::Normal;
    }

    out
}
