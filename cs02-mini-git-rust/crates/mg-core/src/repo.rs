//! Repository discovery and `.mg` state management.
//!
//! M3 keeps the repository state model in the library: upward discovery,
//! worktree/git-dir paths, init layout, symbolic HEAD, and current branch refs.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

const HEAD_MAIN: &str = "ref: refs/heads/main\n";
const CONFIG: &[u8] = b"[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n\tlogallrefupdates = true\n";

/// A discovered non-bare `.mg` repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    worktree_root: PathBuf,
    git_dir: PathBuf,
}

impl Repository {
    /// Discover a repository by walking upward from `start` until `.mg` is found.
    pub fn discover(start: &Path) -> Result<Self> {
        let start = if start.as_os_str().is_empty() {
            Path::new(".")
        } else {
            start
        };
        let mut current = fs::canonicalize(start)?;
        if current.is_file() {
            current = current
                .parent()
                .ok_or_else(|| Error::InvalidRepo("start file has no parent".to_owned()))?
                .to_path_buf();
        }

        loop {
            let git_dir = current.join(".mg");
            if git_dir.is_dir() {
                return Ok(Self {
                    worktree_root: current,
                    git_dir,
                });
            }
            if !current.pop() {
                return Err(Error::InvalidRepo(
                    "not an mg repository (or any parent): .mg not found".to_owned(),
                ));
            }
        }
    }

    /// Initialize a non-bare `.mg` repository rooted at `worktree_root`.
    pub fn init(worktree_root: &Path) -> Result<Self> {
        let root = if worktree_root.as_os_str().is_empty() {
            Path::new(".")
        } else {
            worktree_root
        };
        fs::create_dir_all(root)?;
        let worktree_root = fs::canonicalize(root)?;
        let git_dir = worktree_root.join(".mg");
        fs::create_dir_all(git_dir.join("objects"))?;
        fs::create_dir_all(git_dir.join("refs").join("heads"))?;
        crate::atomic_write(&git_dir.join("HEAD"), HEAD_MAIN.as_bytes())?;
        crate::atomic_write(&git_dir.join("config"), CONFIG)?;
        Ok(Self {
            worktree_root,
            git_dir,
        })
    }

    /// Worktree root path.
    #[must_use]
    pub fn worktree_root(&self) -> &Path {
        &self.worktree_root
    }

    /// Git-compatible `.mg` directory path.
    #[must_use]
    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    /// `.mg/index` path.
    #[must_use]
    pub fn index_path(&self) -> PathBuf {
        self.git_dir.join("index")
    }

    /// Resolve a user path from any cwd into `(absolute_path, worktree_relative_path)`.
    pub fn resolve_worktree_path(&self, cwd: &Path, input: &Path) -> Result<(PathBuf, PathBuf)> {
        let base = if input.is_absolute() {
            input.to_path_buf()
        } else {
            cwd.join(input)
        };
        let metadata = fs::symlink_metadata(&base)?;
        if metadata.file_type().is_symlink() {
            return Err(Error::InvalidRepo(format!(
                "mg add does not support symlink inputs: {}",
                input.display()
            )));
        }
        let absolute = fs::canonicalize(&base)?;
        let relative = absolute
            .strip_prefix(&self.worktree_root)
            .map_err(|_| {
                Error::InvalidRepo(format!(
                    "path is outside repository worktree: {}",
                    input.display()
                ))
            })?
            .to_path_buf();
        validate_index_path(&relative)?;
        Ok((absolute, relative))
    }

    /// Read `.mg/HEAD` as a symbolic ref path.
    pub fn head_ref(&self) -> Result<PathBuf> {
        let raw = fs::read_to_string(self.git_dir.join("HEAD"))?;
        let line = raw.trim_end();
        let Some(ref_name) = line.strip_prefix("ref: ") else {
            return Err(Error::InvalidRepo(
                "detached HEAD is out of scope for v0.1.0".to_owned(),
            ));
        };
        let path = PathBuf::from(ref_name);
        validate_ref_path(&path)?;
        Ok(path)
    }

    /// Reset HEAD to the v0.1.0 default symbolic branch.
    pub fn write_head_main(&self) -> Result<()> {
        crate::atomic_write(&self.git_dir.join("HEAD"), HEAD_MAIN.as_bytes())?;
        Ok(())
    }

    /// Read the current branch commit, if the branch ref exists and is non-empty.
    pub fn read_current_branch(&self) -> Result<Option<String>> {
        let ref_path = self.head_ref()?;
        self.read_ref(&ref_path)
    }

    /// Write the current branch ref to `sha`.
    pub fn write_current_branch(&self, sha: &str) -> Result<()> {
        crate::object::validate_sha1_hex(sha)?;
        let ref_path = self.head_ref()?;
        self.write_ref(&ref_path, sha)
    }

    /// Read a ref path relative to `.mg`, returning `None` for an unborn branch.
    pub fn read_ref(&self, ref_path: &Path) -> Result<Option<String>> {
        validate_ref_path(ref_path)?;
        let path = self.git_dir.join(ref_path);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(path)?;
        let value = raw.trim();
        if value.is_empty() {
            return Ok(None);
        }
        crate::object::validate_sha1_hex(value)?;
        Ok(Some(value.to_owned()))
    }

    /// Write a ref path relative to `.mg`.
    pub fn write_ref(&self, ref_path: &Path, sha: &str) -> Result<()> {
        validate_ref_path(ref_path)?;
        crate::object::validate_sha1_hex(sha)?;
        let path = self.git_dir.join(ref_path);
        crate::atomic_write(&path, format!("{sha}\n").as_bytes())?;
        Ok(())
    }
}

fn validate_ref_path(path: &Path) -> Result<()> {
    if path.is_absolute()
        || path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(Error::InvalidRepo(format!(
            "invalid repository ref path: {}",
            path.display()
        )));
    }
    let text = path
        .to_str()
        .ok_or_else(|| Error::InvalidRepo("ref paths must be UTF-8".to_owned()))?;
    if !text.starts_with("refs/heads/") || text.as_bytes().contains(&0) {
        return Err(Error::InvalidRepo(format!(
            "unsupported ref path for v0.1.0: {text}"
        )));
    }
    Ok(())
}

fn validate_index_path(path: &Path) -> Result<()> {
    reject_internal_repo_path(path)?;
    crate::index::validate_relative_path(path).map_err(|err| match err {
        Error::InvalidIndex(message) => Error::InvalidRepo(message),
        other => other,
    })
}

fn reject_internal_repo_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Err(Error::InvalidRepo("repository path is empty".to_owned()));
    }
    for component in path.components() {
        let std::path::Component::Normal(part) = component else {
            return Err(Error::InvalidRepo(format!(
                "path must stay within repository worktree: {}",
                path.display()
            )));
        };
        let part = part.to_string_lossy();
        if matches!(part.as_ref(), ".mg" | ".git") {
            return Err(Error::InvalidRepo(format!(
                "refusing to stage repository-internal path: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_writes_symbolic_head_and_ref_round_trip() {
        let tmp = std::env::temp_dir().join(format!("mg-repo-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let repo = Repository::init(&tmp).expect("repo init should succeed");
        assert_eq!(
            repo.head_ref().expect("HEAD should parse"),
            PathBuf::from("refs/heads/main")
        );
        assert_eq!(
            repo.read_current_branch().expect("unborn ref should read"),
            None
        );
        let sha = "0123456789abcdef0123456789abcdef01234567";
        repo.write_current_branch(sha)
            .expect("ref write should succeed");
        assert_eq!(
            repo.read_current_branch().expect("ref should read"),
            Some(sha.to_owned())
        );
        fs::remove_dir_all(&tmp).expect("test temp repo should be removed");
    }
}
