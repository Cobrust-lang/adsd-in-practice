//! RESP TCP listener (ADR-0005, ADR-0006).
//!
//! Accept-loop pattern:
//! ```text
//! TcpListener::accept
//!   → spawn(handle_conn)
//!     → loop {
//!         read_buf into BytesMut
//!         if buf.len() > max_frame_size { reject + close }   // M1.4
//!         while let Ok((frame, n)) = Frame::parse(&buf) {
//!             buf.advance(n);
//!             let cmd = dispatch::from_frame(frame);
//!             let reply = store.execute(cmd?)?;
//!             socket.write_all(&reply_to_frame(reply).to_bytes()).await;
//!         }
//!       }
//! ```
//!
//! Locked sub-decisions (ADR-0005 + ADR-0006):
//! - Per-connection task via `tokio::spawn` (fault isolation).
//! - `BytesMut::with_capacity(4096)` + manual drain — **no** `Framed`
//!   codec (that would re-wrap the pure-function parser; F24 candidate).
//! - Protocol error → send `-ERR ...` then close socket.
//! - `QUIT` → send `+OK`, then close socket (caller-side responsibility).
//! - Graceful shutdown M1.3 simplification: `ctrl_c` breaks the accept
//!   loop; in-flight tasks finish naturally.  M3 upgrades to drain mode.
//! - M1.4 (ADR-0006): `max_frame_size` guard on the *buffer total length*
//!   after each `read_buf`.  Default 512 MiB (matches Redis
//!   `proto-max-bulk-len`).  Tighter per-frame measurement deferred to v0.2.

use std::io;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;

use bytes::{Buf, BytesMut};
use redis_protocol::{Frame, ProtocolError};
use redis_storage::Command;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

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

/// Drive a single client connection: read into `BytesMut`, drain all
/// complete `Frame`s, dispatch each, write the reply, repeat.
///
/// On peer EOF (`read_buf == 0`) the function returns `Ok(())`.
///
/// On `ProtocolError::Invalid` or `ProtocolError::Utf8`, an `-ERR ...`
/// reply is best-effort-sent and the connection is closed (returns Ok).
///
/// On `Command::Quit`, the `+OK` reply is flushed and the function
/// returns (which drops the socket).
///
/// M1.4 (ADR-0006): after each `read_buf`, if `buf.len() > max_frame_size`,
/// we send `-ERR Protocol error: frame too big` and close the socket.
/// This is the *buffer total* not per-frame; v0.2 may tighten this.
async fn handle_conn(mut socket: TcpStream, state: AppState) -> io::Result<()> {
    // RAII counter (ADR-0007 §Q6): increment connections_active here,
    // decrement on Drop.  Take the guard BEFORE any `?` so even
    // abnormal task termination (panic, early-return) still
    // decrements the counter via stack unwind.
    let _conn_guard = ConnGuard::new(state.connections_active.clone());

    let max_frame_size = state.max_frame_size;
    let mut buf = BytesMut::with_capacity(4096);

    loop {
        // 1. Read more bytes.  read_buf grows the BytesMut on demand.
        if socket.read_buf(&mut buf).await? == 0 {
            // Peer closed.  If there's stale data in the buffer it's
            // an incomplete frame — treat as a clean disconnect.
            return Ok(());
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
                    let reply = match from_frame(frame) {
                        Ok(cmd) => {
                            // Mark socket-close intent BEFORE handing the
                            // command to the store (ADR-0005 watchout).
                            if matches!(cmd, Command::Quit) {
                                close_after = true;
                            }
                            // Store::execute is infallible at the wire
                            // level (StoreError is internal-only); map
                            // any future StoreError to a generic ERR
                            // reply rather than panicking.
                            match state.store.execute(cmd) {
                                Ok(r) => r,
                                Err(e) => redis_storage::Reply::Error(format!("ERR internal: {e}")),
                            }
                        }
                        Err(reply) => reply,
                    };

                    let frame_out = reply_to_frame(reply);
                    socket.write_all(&frame_out.to_bytes()).await?;
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
