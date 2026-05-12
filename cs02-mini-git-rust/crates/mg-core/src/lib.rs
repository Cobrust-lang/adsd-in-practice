//! mg-core — git plumbing layer (objects + index + repo discovery).
//!
//! Pure library. No clap dependency, no stdin/stdout.
//! The CLI binary lives in `mg-cli`.

#![forbid(unsafe_code)]

pub mod hash;
pub mod object;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid object: {0}")]
    InvalidObject(&'static str),
    #[error("hash mismatch")]
    HashMismatch,
}

pub type Result<T> = std::result::Result<T, Error>;
