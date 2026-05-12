//! RESP (REdis Serialization Protocol) parser + serializer.
//!
//! Scope: RESP v2 (`+` simple string / `-` error / `:` integer /
//! `$` bulk string / `*` array). RESP v3 (`,` doubles, `_` null,
//! `#` boolean, `(` big number, `=` verbatim, `~` set, `%` map,
//! `>` push, `|` attribute) is **out of scope for v0.1.0** (ADR-0002).
//!
//! Pure functions only — no IO. The server crate owns transport.

#![forbid(unsafe_code)]

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("incomplete frame")]
    Incomplete,
    #[error("protocol error: {0}")]
    Invalid(&'static str),
    #[error("utf-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// A single RESP frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<Frame>>),
}

impl Frame {
    /// Parse one frame from a byte buffer.
    ///
    /// Returns the parsed frame + bytes consumed, or `Incomplete` if the
    /// buffer needs more data.
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` on malformed input.
    pub fn parse(_input: &[u8]) -> Result<(Self, usize), ProtocolError> {
        // M1.1 stub — fill in actual parsing per RESP v2 spec.
        Err(ProtocolError::Incomplete)
    }

    /// Serialize a frame to a `Vec<u8>` (heap-allocating).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        // M1.1 stub — fill in serialization per RESP v2 spec.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_simple_string_round_trip_stub() {
        // M1.1 — RESP encoder/decoder not yet implemented.
        // Real test lives at oracle level: tests/oracle.sh round-trips
        // against redis:7-alpine docker container.
        let f = Frame::SimpleString("OK".into());
        let _bytes = f.to_bytes();
        // assert_eq!(bytes, b"+OK\r\n");
    }
}
