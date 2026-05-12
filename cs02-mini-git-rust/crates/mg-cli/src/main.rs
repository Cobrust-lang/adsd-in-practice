//! mg — minimal git plumbing CLI.
//!
//! M1 implements blob `hash-object`, `hash-object -w`, `cat-file -p`, and a
//! deliberately narrow `init` that creates only `.mg/objects`.

#![forbid(unsafe_code)]

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use mg_core::object::{self, Kind};

#[derive(Parser, Debug)]
#[command(name = "mg", version, about = "ADSD CS-02 minimal git plumbing")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Initialize a minimal `.mg/objects` object database in cwd.
    Init,
    /// Hash a blob (`mg hash-object [-w] <file>`).
    HashObject {
        #[arg(short = 'w')]
        write: bool,
        file: PathBuf,
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
        Cmd::Init => init(Path::new(".mg"))?,
        Cmd::HashObject { write, file } => hash_object(write, &file)?,
        Cmd::CatFile { pretty, sha } => cat_file(pretty, &sha)?,
    }
    Ok(())
}

fn init(mg_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(mg_dir.join("objects"))?;
    println!("Initialized empty mg object database in .mg/objects");
    Ok(())
}

fn hash_object(write: bool, file: &Path) -> anyhow::Result<()> {
    let payload = fs::read(file)?;
    let sha = if write {
        object::write_loose(Kind::Blob, &payload, Path::new(".mg"))?
    } else {
        object::hash(Kind::Blob, &payload)
    };
    println!("{sha}");
    Ok(())
}

fn cat_file(pretty: bool, sha: &str) -> anyhow::Result<()> {
    if !pretty {
        anyhow::bail!("M1 supports only `mg cat-file -p <sha>`");
    }
    let decoded = object::read_loose(Path::new(".mg"), sha)?;
    if decoded.kind != Kind::Blob {
        anyhow::bail!("M1 cat-file -p supports blob objects only");
    }
    io::stdout().write_all(&decoded.payload)?;
    Ok(())
}
