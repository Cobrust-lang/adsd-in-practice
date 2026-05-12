//! mini-redis-server entry point.
//!
//! Wave M1.0 — scaffold only. Real argument parsing + tokio runtime
//! + RESP TCP listener land at Wave M1.1+.

#![forbid(unsafe_code)]

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "mini-redis-server", version, about = "ADSD CS-01 demo")]
struct Args {
    /// RESP TCP port
    #[arg(long, default_value_t = 6380)]
    port: u16,

    /// HTTP control-plane port
    #[arg(long, default_value_t = 6381)]
    http_port: u16,

    /// AOF file path (if absent, persistence is disabled)
    #[arg(long)]
    aof: Option<String>,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    tracing::info!(
        port = args.port,
        http_port = args.http_port,
        aof = ?args.aof,
        "mini-redis-server starting (M1.0 scaffold; real loop lands at M1.1)"
    );

    // M1.1 — tokio runtime + bind + listen
    println!("mini-redis-server M1.0 scaffold; not yet functional.");
    Ok(())
}
