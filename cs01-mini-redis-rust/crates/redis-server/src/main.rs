//! mini-redis-server entry point.
//!
//! Wave M1.3 (ADR-0005) — RESP TCP listener wired end-to-end:
//! parse args → init tracing → build `Store` → run accept-loop until
//! `ctrl_c`.
//!
//! Wave M1.4 (ADR-0006) — adds `--max-frame-size` to bound the
//! per-connection buffer size as a basic DoS guard.
//!
//! `--http-port` and `--aof` are reserved for M2 (SSE) and M3 (AOF)
//! and currently logged as informational placeholders.

#![forbid(unsafe_code)]

use std::net::{IpAddr, SocketAddr};

use clap::Parser;
use redis_server::server::DEFAULT_MAX_FRAME_SIZE;
use redis_storage::Store;

#[derive(Parser, Debug)]
#[command(name = "mini-redis-server", version, about = "ADSD CS-01 demo")]
struct Args {
    /// RESP TCP port
    #[arg(long, default_value_t = 6380)]
    port: u16,

    /// RESP TCP bind address
    #[arg(long, default_value = "0.0.0.0")]
    bind: String,

    /// HTTP control-plane port (M2 placeholder — not yet wired)
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
    let addr = SocketAddr::new(ip, args.port);

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
        "mini-redis-server starting (M1.4)"
    );

    let store = Store::new();
    redis_server::server::run(addr, store, max_frame_size).await?;

    tracing::info!("mini-redis-server stopped cleanly");
    Ok(())
}
