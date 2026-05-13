//! mg-core — git plumbing layer (objects + index + repo discovery).
//!
//! Pure library. No clap dependency, no stdin/stdout.
//! The CLI binary lives in `mg-cli`.

#![forbid(unsafe_code)]

pub mod hash;
pub mod index;
pub mod object;
pub mod repo;

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid object: {0}")]
    InvalidObject(String),
    #[error("invalid index: {0}")]
    InvalidIndex(String),
    #[error("invalid repository: {0}")]
    InvalidRepo(String),
    #[error("unsupported object kind: {0}")]
    UnsupportedKind(String),
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
}

pub type Result<T> = std::result::Result<T, Error>;

static TMP_NAME_SALT: AtomicU64 = AtomicU64::new(0);

pub(crate) fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    reject_symlink_target(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| Error::Io(std::io::Error::other("path has no parent directory")))?;
    reject_symlink_ancestors(parent)?;
    fs::create_dir_all(parent)?;

    let tmp_path = temp_path_next_to(path);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)?;
    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);

    fs::rename(&tmp_path, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp_path);
    })?;
    sync_directory(parent)?;

    Ok(())
}

pub(crate) fn acquire_lock(lock_path: &Path) -> Result<File> {
    reject_symlink_target(lock_path)?;
    let parent = lock_path
        .parent()
        .ok_or_else(|| Error::Io(std::io::Error::other("lock path has no parent directory")))?;
    reject_symlink_ancestors(parent)?;
    fs::create_dir_all(parent)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)
        .map_err(Into::into)
}

fn reject_symlink_target(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(Error::Io(std::io::Error::other(
            format!("refusing to overwrite symlink target: {}", path.display()),
        ))),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(Error::Io(err)),
    }
}

pub(crate) fn reject_symlink_ancestors(path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::CurDir) {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(Error::Io(std::io::Error::other(format!(
                    "refusing to traverse symlink ancestor: {}",
                    current.display()
                ))));
            }
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => break,
            Err(err) => return Err(Error::Io(err)),
        }
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn temp_path_next_to(path: &Path) -> PathBuf {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let salt = TMP_NAME_SALT.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .expect("atomic write path should always have a filename")
        .to_string_lossy();
    path.with_file_name(format!(".{file_name}.tmp-{pid}-{nanos}-{salt}"))
}
