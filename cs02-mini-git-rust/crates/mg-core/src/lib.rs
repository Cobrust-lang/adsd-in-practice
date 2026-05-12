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
    InvalidObject(String),
    #[error("unsupported object kind: {0}")]
    UnsupportedKind(String),
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
}

pub type Result<T> = std::result::Result<T, Error>;
