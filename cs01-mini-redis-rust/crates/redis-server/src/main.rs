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
//! only the RESP listener will start.  `--aof` is still an M3
//! placeholder, logged informationally.

#![forbid(unsafe_code)]

use std::net::{IpAddr, SocketAddr};

use clap::Parser;
use redis_server::server::DEFAULT_MAX_FRAME_SIZE;
use redis_server::state::AppState;
use redis_storage::Store;

#[derive(Parser, Debug)]
#[command(name = "mini-redis-server", version, about = "ADSD CS-01 demo")]
struct Args {
    /// RESP TCP port
    #[arg(long, default_value_t = 6380)]
    port: u16,

    /// RESP TCP bind address (shared by RESP + HTTP listener)
    #[arg(long, default_value = "0.0.0.0")]
    bind: String,

    /// HTTP control-plane port.  Set to 0 to disable the HTTP
    /// listener (only RESP will start).
    #[arg(long, default_value_t = 6381)]
    http_port: u16,

    /// AOF file path (M3 placeholder — persistence disabled if absent)
    #[arg(long)]
    aof: Option<String>,

    /// Per-connection buffer size ceiling (bytes).  When exceeded the
    /// connection is terminated with `-ERR Protocol error: frame too big`.
    /// Matches Redis' `proto-max-bulk-len` (default 512 MiB).
    #[arg(long, default_value_t = DEFAULT_MAX_FRAME_SIZE as u64)]
    max_frame_size: u64,
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

    tracing::info!(
        port = args.port,
        bind = %args.bind,
        http_port = args.http_port,
        aof = ?args.aof,
        max_frame_size,
        "mini-redis-server starting (M2.1)"
    );

    let store = Store::new();
    let state = AppState::new(store, max_frame_size);

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

    tracing::info!("mini-redis-server stopped cleanly");
    Ok(())
}
