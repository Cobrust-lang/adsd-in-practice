//! mini-redis-server entry point.
//!
//! Wave M1.3 (ADR-0005) — RESP TCP listener wired end-to-end:
//! parse args → init tracing → build `Store` → run accept-loop until
//! `ctrl_c`.
//!
//! Wave M1.4 (ADR-0006) — adds `--max-frame-size` to bound the
//! per-connection buffer size as a basic DoS guard.
//!
//! Wave M2.1 (ADR-0007) — adds the Axum HTTP control plane on
//! `--http-port` (default 6381).  Pass `--http-port 0` to disable;
//! only the RESP listener will start.
//!
//! Wave M3.2 (ADR-0010) — wires AOF persistence.  When `--aof <path>`
//! is supplied:
//!
//!   1. construct `Store::new()` (in-memory, no writer yet)
//!   2. if the file exists, run `Store::replay_from_path(path)` —
//!      this re-executes every recorded writable command into the
//!      fresh map, **without** appending to the file
//!   3. upgrade to `Store::with_aof(path, fsync)` so subsequent live
//!      writes are appended to the same file
//!   4. **then** bind RESP + HTTP listeners (replay-before-bind
//!      guarantees clients never see partial state — ADR-0010
//!      §"Replay 期间 accept" decision)
//!
//! When `--aof` is omitted the server is in-memory only (M3.1 behaviour).
//!
//! Wave M4.1 (ADR-0011) — security + hardening:
//! - Default bind is now `127.0.0.1`; bind to all interfaces with
//!   `--bind 0.0.0.0` only when fronted by AUTH / TLS / a trusted
//!   reverse proxy.  `--insecure-no-auth` is required to acknowledge
//!   the AUTH gap when binding to non-loopback addresses.
//! - `--max-clients <N>` caps concurrent RESP connections (default
//!   matches the cs01 SLO 1000).
//! - `--aof-fsync alwaysblocking` provides synchronous-durability
//!   semantics for callers that need "reply ⇒ on disk".

#![forbid(unsafe_code)]

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use clap::Parser;
use redis_server::server::DEFAULT_MAX_FRAME_SIZE;
use redis_server::state::{AppState, DEFAULT_MAX_CLIENTS};
use redis_storage::{FsyncPolicy, Store};

#[derive(Parser, Debug)]
#[command(name = "mini-redis-server", version, about = "ADSD CS-01 demo")]
struct Args {
    /// RESP TCP port
    #[arg(long, default_value_t = 6380)]
    port: u16,

    /// RESP TCP bind address (shared by RESP + HTTP listener).
    /// Defaults to `127.0.0.1` for safety — pass `--bind 0.0.0.0`
    /// explicitly to expose to the LAN.  Combine with
    /// `--insecure-no-auth` to acknowledge the AUTH gap.
    #[arg(long, default_value = "127.0.0.1")]
    bind: String,

    /// HTTP control-plane port.  Set to 0 to disable the HTTP
    /// listener (only RESP will start).
    #[arg(long, default_value_t = 6381)]
    http_port: u16,

    /// AOF file path.  When set, the server replays the file on
    /// start (if present) and appends every subsequent writable
    /// command (SET / DEL / EXPIRE / PERSIST / INCR / DECR) to it
    /// in RESP wire format.  Omit to disable persistence.
    #[arg(long)]
    aof: Option<PathBuf>,

    /// fsync cadence for the AOF writer.  One of `always` /
    /// `alwaysblocking` / `everysec` / `no`.  `always` and
    /// `alwaysblocking` both fsync on every record; `alwaysblocking`
    /// additionally waits for the fsync before returning the reply
    /// (use when "reply received ⇒ command durable" is required).
    /// Default `everysec` (1 Hz background flush).  Ignored when
    /// `--aof` is not provided.
    #[arg(long, default_value = "everysec")]
    aof_fsync: String,

    /// Per-connection buffer size ceiling (bytes).  When exceeded the
    /// connection is terminated with `-ERR Protocol error: frame too big`.
    /// Matches Redis' `proto-max-bulk-len` (default 512 MiB).
    #[arg(long, default_value_t = DEFAULT_MAX_FRAME_SIZE as u64)]
    max_frame_size: u64,

    /// Maximum number of concurrent RESP connections (M4.1, ADR-0011 #2).
    /// New clients beyond this ceiling receive
    /// `-ERR max number of clients reached\r\n` and are dropped before
    /// a per-conn task is spawned.  Default matches the cs01 SLO
    /// (`docs/agent/cs01 CLAUDE.md §5`).
    #[arg(long, default_value_t = DEFAULT_MAX_CLIENTS)]
    max_clients: usize,

    /// Acknowledge that this build does not implement AUTH and that
    /// the operator is binding to a public address knowingly
    /// (M4.1, ADR-0011 #1).  Pure documentation flag — does not
    /// disable any check; it only emits a startup warning to make
    /// the AUTH gap visible in operator logs.  AUTH lands in v0.2.
    #[arg(long, default_value_t = false)]
    insecure_no_auth: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    let ip: IpAddr = args
        .bind
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid --bind address {bind:?}: {e}", bind = args.bind))?;
    let resp_addr = SocketAddr::new(ip, args.port);

    let max_frame_size: usize = usize::try_from(args.max_frame_size).map_err(|_| {
        anyhow::anyhow!(
            "invalid --max-frame-size {n}: does not fit in platform usize",
            n = args.max_frame_size
        )
    })?;

    let fsync_policy: FsyncPolicy = FsyncPolicy::parse(&args.aof_fsync)
        .map_err(|e| anyhow::anyhow!("invalid --aof-fsync: {e}"))?;

    tracing::info!(
        port = args.port,
        bind = %args.bind,
        http_port = args.http_port,
        aof = ?args.aof,
        aof_fsync = ?fsync_policy,
        max_frame_size,
        max_clients = args.max_clients,
        "mini-redis-server starting (M4.1)"
    );

    // ADR-0011 §#1: explicit warning when AUTH is not in place.  We
    // do NOT block the run; this is "loud-but-permissive" so the
    // operator sees the gap in their logs.  The flag itself does
    // nothing else — AUTH lands at v0.2.
    if args.insecure_no_auth {
        tracing::warn!(
            "Starting WITHOUT authentication. Do not expose to public networks. \
             AUTH command lands at v0.2."
        );
    }

    // ── AOF replay-before-bind (ADR-0010 §"Replay 期间 accept") ──────────
    let store = if let Some(aof_path) = args.aof.as_ref() {
        // Run replay against a fresh AOF-less store first, so the
        // re-executed commands don't extend the file we are reading.
        let store = Store::new();
        if aof_path.exists() {
            match store.replay_from_path(aof_path).await {
                Ok(count) => tracing::info!(
                    replayed = count,
                    path = %aof_path.display(),
                    "AOF replay complete"
                ),
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        path = %aof_path.display(),
                        "AOF replay failed; aborting startup"
                    );
                    return Err(anyhow::anyhow!(e));
                }
            }
        } else {
            tracing::info!(
                path = %aof_path.display(),
                "AOF path does not yet exist — starting empty (will be created on first write)"
            );
        }
        // Graft the AofWriter onto the just-replayed store; the writer task only owns the new file handle and shares the existing inner map via Arc.
        store.attach_aof(aof_path.clone(), fsync_policy).await?
    } else {
        Store::new()
    };
    let state = AppState::new_with_limits(store, max_frame_size, args.max_clients);

    // RESP listener — always on.
    let resp_state = state.clone();
    let resp_handle = tokio::spawn(async move {
        redis_server::server::run(resp_addr, resp_state)
            .await
            .map_err(anyhow::Error::from)
    });

    // HTTP listener — opt-out via `--http-port 0`.
    let http_handle = if args.http_port == 0 {
        tracing::info!("HTTP listener disabled (--http-port 0)");
        None
    } else {
        let http_addr = SocketAddr::new(ip, args.http_port);
        let http_state = state.clone();
        Some(tokio::spawn(async move {
            redis_server::http::run(http_addr, http_state)
                .await
                .map_err(anyhow::Error::from)
        }))
    };

    // Wait for both listeners.  Linux delivers SIGINT to the whole
    // process so both ctrl_c arms fire simultaneously; if one
    // listener crashes we surface its error.
    if let Some(http_handle) = http_handle {
        let (resp_res, http_res) = tokio::try_join!(resp_handle, http_handle)?;
        resp_res?;
        http_res?;
    } else {
        resp_handle.await??;
    }

    // M3.2 graceful shutdown: flush the AOF writer so every record
    // accepted on the RESP path before ctrl_c is on disk by the time
    // we return.  `aof_flush` is a no-op when AOF is disabled.
    // This is the single durability anchor between the RESP listener
    // exiting and the tokio runtime aborting our spawned tasks.
    state.store.aof_flush().await;

    tracing::info!("mini-redis-server stopped cleanly");
    Ok(())
}
