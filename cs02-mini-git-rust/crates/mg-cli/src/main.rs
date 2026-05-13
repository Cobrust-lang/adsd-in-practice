//! mg — minimal git plumbing CLI.
//!
//! M3 supports Git-compatible blob/object IO, index/tree writes, repository
//! discovery, commit creation, porcelain `commit -m`, and first-parent `log`.

#![forbid(unsafe_code)]

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use mg_core::index::{self, Entry};
use mg_core::object::{self, Kind, Signature};
use mg_core::repo::Repository;

#[derive(Parser, Debug)]
#[command(name = "mg", version, about = "ADSD CS-02 minimal git plumbing")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Initialize a Git-compatible `.mg` repository in cwd.
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
    /// Stage one regular file in `.mg/index` and write its blob object.
    Add { path: PathBuf },
    /// Write a tree object from `.mg/index` and print its SHA-1.
    WriteTree,
    /// Write a commit object from a tree.
    CommitTree {
        tree: String,
        #[arg(short = 'p')]
        parent: Vec<String>,
        #[arg(short = 'm')]
        message: String,
    },
    /// Commit the current index and advance refs/heads/main.
    Commit {
        #[arg(short = 'm')]
        message: String,
    },
    /// Print first-parent history from HEAD.
    Log,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Cli::parse();
    match args.cmd {
        Cmd::Init => init()?,
        Cmd::HashObject { write, file } => hash_object(write, &file)?,
        Cmd::CatFile { pretty, sha } => cat_file(pretty, &sha)?,
        Cmd::Add { path } => add(&path)?,
        Cmd::WriteTree => write_tree_cmd()?,
        Cmd::CommitTree {
            tree,
            parent,
            message,
        } => commit_tree_cmd(&tree, &parent, &message)?,
        Cmd::Commit { message } => commit_cmd(&message)?,
        Cmd::Log => log_cmd()?,
    }
    Ok(())
}

fn init() -> anyhow::Result<()> {
    let repo = Repository::init(Path::new("."))?;
    println!(
        "Initialized empty mg repository in {}",
        repo.git_dir().display()
    );
    Ok(())
}

fn hash_object(write: bool, file: &Path) -> anyhow::Result<()> {
    let payload = fs::read(file)?;
    let sha = if write {
        let repo = Repository::discover(Path::new("."))?;
        object::write_loose(Kind::Blob, &payload, repo.git_dir())?
    } else {
        object::hash(Kind::Blob, &payload)
    };
    println!("{sha}");
    Ok(())
}

fn cat_file(pretty: bool, sha: &str) -> anyhow::Result<()> {
    if !pretty {
        anyhow::bail!("M3 supports only `mg cat-file -p <sha>`");
    }
    let repo = Repository::discover(Path::new("."))?;
    let decoded = object::read_loose(repo.git_dir(), sha)?;
    match decoded.kind {
        Kind::Blob | Kind::Tree | Kind::Commit | Kind::Tag => {
            io::stdout().write_all(&decoded.payload)?;
        }
    }
    Ok(())
}

fn add(path: &Path) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let repo = Repository::discover(&cwd)?;
    let (absolute, relative) = repo.resolve_worktree_path(&cwd, path)?;
    let metadata = fs::metadata(&absolute)?;
    if !metadata.is_file() {
        anyhow::bail!("mg add supports regular files only: {}", path.display());
    }

    let payload = fs::read(&absolute)?;
    let sha = object::write_loose(Kind::Blob, &payload, repo.git_dir())?;
    let entry = Entry::from_worktree_file(&absolute, &relative, &sha)?;
    let entries = index::read(&repo.index_path())?;
    let entries = index::upsert_entry(entries, entry);
    index::write(&repo.index_path(), &entries)?;
    Ok(())
}

fn write_tree_cmd() -> anyhow::Result<()> {
    let repo = Repository::discover(Path::new("."))?;
    let tree = write_tree(&repo)?;
    println!("{tree}");
    Ok(())
}

fn write_tree(repo: &Repository) -> anyhow::Result<String> {
    let entries = index::read(&repo.index_path())?;
    Ok(object::write_tree_from_index(&entries, repo.git_dir())?)
}

fn commit_tree_cmd(tree: &str, parents: &[String], message: &str) -> anyhow::Result<()> {
    let repo = Repository::discover(Path::new("."))?;
    let sha = write_commit(&repo, tree, parents, message)?;
    println!("{sha}");
    Ok(())
}

fn commit_cmd(message: &str) -> anyhow::Result<()> {
    let repo = Repository::discover(Path::new("."))?;
    let tree = write_tree(&repo)?;
    let parents = repo.read_current_branch()?.into_iter().collect::<Vec<_>>();
    let commit = write_commit(&repo, &tree, &parents, message)?;
    repo.write_current_branch(&commit)?;
    println!("{commit}");
    Ok(())
}

fn write_commit(
    repo: &Repository,
    tree: &str,
    parents: &[String],
    message: &str,
) -> anyhow::Result<String> {
    let author = author_signature()?;
    let committer = committer_signature()?;
    let payload = object::commit_payload(tree, parents, &author, &committer, message)?;
    Ok(object::write_loose(Kind::Commit, &payload, repo.git_dir())?)
}

fn log_cmd() -> anyhow::Result<()> {
    let repo = Repository::discover(Path::new("."))?;
    let mut current = repo.read_current_branch()?;
    while let Some(sha) = current {
        let decoded = object::read_loose(repo.git_dir(), &sha)?;
        if decoded.kind != Kind::Commit {
            anyhow::bail!("HEAD history object is not a commit: {sha}");
        }
        let subject = object::commit_subject(&decoded.payload)?;
        println!("{sha} {subject}");
        current = object::first_parent(&decoded.payload)?;
    }
    Ok(())
}

fn author_signature() -> anyhow::Result<Signature> {
    Ok(Signature {
        name: env_or("GIT_AUTHOR_NAME", "mg user"),
        email: env_or("GIT_AUTHOR_EMAIL", "mg@example.invalid"),
        date: git_date_from_env("GIT_AUTHOR_DATE")?,
    })
}

fn committer_signature() -> anyhow::Result<Signature> {
    Ok(Signature {
        name: env_or("GIT_COMMITTER_NAME", "mg user"),
        email: env_or("GIT_COMMITTER_EMAIL", "mg@example.invalid"),
        date: git_date_from_env("GIT_COMMITTER_DATE")?,
    })
}

fn env_or(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_owned())
}

fn git_date_from_env(name: &str) -> anyhow::Result<String> {
    if let Ok(value) = std::env::var(name) {
        normalize_git_date(&value)
    } else {
        let seconds = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        Ok(format!("{seconds} +0000"))
    }
}

fn normalize_git_date(value: &str) -> anyhow::Result<String> {
    let trimmed = value.trim();
    let mut parts = trimmed.split_whitespace();
    let Some(seconds) = parts.next() else {
        anyhow::bail!("empty Git date environment value");
    };
    let Some(zone) = parts.next() else {
        anyhow::bail!("Git date must be '<seconds> <timezone>', got {trimmed}");
    };
    if parts.next().is_some()
        || seconds.parse::<i64>().is_err()
        || zone.len() != 5
        || !matches!(zone.as_bytes()[0], b'+' | b'-')
        || !zone.as_bytes()[1..].iter().all(u8::is_ascii_digit)
    {
        anyhow::bail!("Git date must be '<seconds> <timezone>', got {trimmed}");
    }
    Ok(format!("{seconds} {zone}"))
}
