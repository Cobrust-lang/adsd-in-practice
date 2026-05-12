//! mg — minimal git plumbing CLI.
//!
//! Wave M0 scaffold. Real subcommands land at M1.0+ following
//! `docs/agent/adr/0001-stack-choice.md` order.

#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "mg", version, about = "ADSD CS-02 minimal git plumbing")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Initialize an empty `.mg/` directory in cwd.
    Init,
    /// Hash a blob (`mg hash-object [-w] <file>`).
    HashObject {
        #[arg(short = 'w')]
        write: bool,
        file: String,
    },
    /// Pretty-print object contents (`mg cat-file -p <sha>`).
    CatFile {
        #[arg(short = 'p')]
        pretty: bool,
        sha: String,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Cli::parse();
    match args.cmd {
        Cmd::Init => {
            println!("mg init — M1.0 scaffold; real implementation lands at M3.");
        }
        Cmd::HashObject { write, file } => {
            println!("mg hash-object {} -w={} — M1.0 scaffold", file, write);
        }
        Cmd::CatFile { pretty, sha } => {
            println!("mg cat-file -p={} {} — M1.0 scaffold", pretty, sha);
        }
    }
    Ok(())
}
