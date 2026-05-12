//! mg — minimal git plumbing CLI.
//!
//! M2 supports blob `hash-object`, `cat-file -p`, flat-file `add`, and
//! `write-tree` on a Git-compatible index/tree subset.

#![forbid(unsafe_code)]

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use mg_core::index::{self, Entry};
use mg_core::object::{self, Kind, TreeEntry};

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
    /// Stage one flat regular file in `.mg/index` and write its blob object.
    Add { path: PathBuf },
    /// Write a tree object from `.mg/index` and print its SHA-1.
    WriteTree,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Cli::parse();
    match args.cmd {
        Cmd::Init => init(Path::new(".mg"))?,
        Cmd::HashObject { write, file } => hash_object(write, &file)?,
        Cmd::CatFile { pretty, sha } => cat_file(pretty, &sha)?,
        Cmd::Add { path } => add(&path)?,
        Cmd::WriteTree => write_tree()?,
    }
    Ok(())
}

fn init(mg_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(mg_dir.join("objects"))?;
    fs::create_dir_all(mg_dir.join("refs").join("heads"))?;
    fs::write(mg_dir.join("HEAD"), b"ref: refs/heads/main\n")?;
    fs::write(
        mg_dir.join("config"),
        b"[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n\tlogallrefupdates = true\n",
    )?;
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
        anyhow::bail!("M2 supports only `mg cat-file -p <sha>`");
    }
    let decoded = object::read_loose(Path::new(".mg"), sha)?;
    match decoded.kind {
        Kind::Blob | Kind::Tree => io::stdout().write_all(&decoded.payload)?,
        Kind::Commit | Kind::Tag => anyhow::bail!("M2 cat-file -p supports blob/tree objects only"),
    }
    Ok(())
}

fn add(path: &Path) -> anyhow::Result<()> {
    ensure_flat_path(path)?;
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        anyhow::bail!(
            "mg add supports regular files only in M2: {}",
            path.display()
        );
    }

    let payload = fs::read(path)?;
    let sha = object::write_loose(Kind::Blob, &payload, Path::new(".mg"))?;
    let entry = Entry::from_worktree_file(path, &sha)?;
    let index_path = Path::new(".mg").join("index");
    let entries = index::read(&index_path)?;
    let entries = index::upsert_entry(entries, entry);
    index::write(&index_path, &entries)?;
    Ok(())
}

fn write_tree() -> anyhow::Result<()> {
    let index_path = Path::new(".mg").join("index");
    let entries = index::read(&index_path)?;
    let tree_entries = entries
        .iter()
        .map(TreeEntry::from_index_entry)
        .collect::<mg_core::Result<Vec<_>>>()?;
    let payload = object::tree_payload(&tree_entries)?;
    let sha = object::write_loose(Kind::Tree, &payload, Path::new(".mg"))?;
    println!("{sha}");
    Ok(())
}

fn ensure_flat_path(path: &Path) -> anyhow::Result<()> {
    if path.as_os_str().is_empty() || !path.is_relative() || path.components().count() != 1 {
        anyhow::bail!(
            "M2 supports only flat repository-root file paths, got {}",
            path.display()
        );
    }
    Ok(())
}
