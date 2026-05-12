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

/// Find the position of the first `\r\n` in `buf`, starting at `offset`.
/// Returns `Some(pos)` where `pos` is the index of `\r`, or `None` if not found.
fn find_crlf(buf: &[u8], offset: usize) -> Option<usize> {
    let buf = &buf[offset..];
    buf.windows(2)
        .position(|w| w == b"\r\n")
        .map(|p| p + offset)
}

/// Parse a decimal integer (possibly negative) from bytes, returning `(value, end_offset)`.
/// `end_offset` points to the byte *after* the last digit (i.e., where `\r\n` starts).
///
/// Uses `i128` as an intermediate accumulator to correctly handle `i64::MIN`
/// (`-9223372036854775808`), whose absolute value exceeds `i64::MAX`.
fn parse_integer_bytes(buf: &[u8], start: usize) -> Result<(i64, usize), ProtocolError> {
    let crlf = find_crlf(buf, start).ok_or(ProtocolError::Incomplete)?;
    let slice = &buf[start..crlf];
    if slice.is_empty() {
        return Err(ProtocolError::Invalid("empty integer line"));
    }
    // Fast ASCII decimal parse — avoid String allocation.
    let (negative, digits) = if slice[0] == b'-' {
        if slice.len() < 2 {
            return Err(ProtocolError::Invalid("lone minus sign"));
        }
        (true, &slice[1..])
    } else {
        (false, slice)
    };
    // Accumulate in i128 so that i64::MIN (abs > i64::MAX) doesn't overflow mid-parse.
    let mut value: i128 = 0;
    for &b in digits {
        if !b.is_ascii_digit() {
            return Err(ProtocolError::Invalid("non-digit in integer"));
        }
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(i128::from(b - b'0')))
            .ok_or(ProtocolError::Invalid("integer overflow"))?;
    }
    let signed: i64 = if negative {
        (-value)
            .try_into()
            .map_err(|_| ProtocolError::Invalid("integer overflow"))?
    } else {
        value
            .try_into()
            .map_err(|_| ProtocolError::Invalid("integer overflow"))?
    };
    Ok((signed, crlf))
}

impl Frame {
    /// Parse one frame from a byte buffer.
    ///
    /// Returns `(frame, bytes_consumed)` or `Err(Incomplete)` if the buffer
    /// needs more data, or `Err(Invalid(...))` for malformed input.
    ///
    /// Caller loop pattern:
    /// ```text
    /// while let Ok((frame, n)) = Frame::parse(&buf) {
    ///     buf.advance(n);
    ///     dispatch(frame);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError::Incomplete` when more bytes are needed.
    /// Returns `ProtocolError::Invalid` on malformed protocol bytes.
    /// Returns `ProtocolError::Utf8` when a simple-string / error line is not UTF-8.
    pub fn parse(input: &[u8]) -> Result<(Self, usize), ProtocolError> {
        if input.is_empty() {
            return Err(ProtocolError::Incomplete);
        }
        let type_byte = input[0];
        match type_byte {
            b'+' => {
                // Simple string: "+<str>\r\n"
                let crlf = find_crlf(input, 1).ok_or(ProtocolError::Incomplete)?;
                let s = String::from_utf8(input[1..crlf].to_vec())?;
                let consumed = crlf + 2; // include the \r\n
                Ok((Frame::SimpleString(s), consumed))
            }
            b'-' => {
                // Error: "-<str>\r\n"
                let crlf = find_crlf(input, 1).ok_or(ProtocolError::Incomplete)?;
                let s = String::from_utf8(input[1..crlf].to_vec())?;
                let consumed = crlf + 2;
                Ok((Frame::Error(s), consumed))
            }
            b':' => {
                // Integer: ":<number>\r\n"
                let (value, crlf) = parse_integer_bytes(input, 1)?;
                let consumed = crlf + 2;
                Ok((Frame::Integer(value), consumed))
            }
            b'$' => {
                // Bulk string: "$<len>\r\n<data>\r\n"  or  "$-1\r\n" (nil)
                let (len, crlf) = parse_integer_bytes(input, 1)?;
                if len == -1 {
                    // Nil bulk string
                    let consumed = crlf + 2;
                    return Ok((Frame::BulkString(None), consumed));
                }
                if len < 0 {
                    return Err(ProtocolError::Invalid("negative bulk length"));
                }
                let data_start = crlf + 2;
                let data_len = usize::try_from(len)
                    .map_err(|_| ProtocolError::Invalid("bulk length too large for usize"))?;
                let data_end = data_start + data_len;
                // Need data_len bytes + trailing \r\n
                if input.len() < data_end + 2 {
                    return Err(ProtocolError::Incomplete);
                }
                if input[data_end] != b'\r' || input[data_end + 1] != b'\n' {
                    return Err(ProtocolError::Invalid("bulk string missing trailing CRLF"));
                }
                let data = input[data_start..data_end].to_vec();
                let consumed = data_end + 2;
                Ok((Frame::BulkString(Some(data)), consumed))
            }
            b'*' => {
                // Array: "*<count>\r\n<element>*"  or  "*-1\r\n" (nil)
                let (count, crlf) = parse_integer_bytes(input, 1)?;
                if count == -1 {
                    let consumed = crlf + 2;
                    return Ok((Frame::Array(None), consumed));
                }
                if count < 0 {
                    return Err(ProtocolError::Invalid("negative array count"));
                }
                let n = usize::try_from(count)
                    .map_err(|_| ProtocolError::Invalid("array count too large for usize"))?;
                let mut elements = Vec::with_capacity(n);
                let mut cursor = crlf + 2;
                for _ in 0..n {
                    let remaining = &input[cursor..];
                    let (elem, elem_consumed) = Frame::parse(remaining)?;
                    cursor += elem_consumed;
                    elements.push(elem);
                }
                Ok((Frame::Array(Some(elements)), cursor))
            }
            _ => Err(ProtocolError::Invalid("unknown RESP type byte")),
        }
    }

    /// Serialize a frame to a `Vec<u8>` (RESP v2 wire format).
    ///
    /// Inverse of [`Frame::parse`]; `parse(to_bytes(f)).0 == *f` for all valid frames.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode_into(&mut out);
        out
    }

    /// Internal recursive encoder — writes into a pre-allocated buffer.
    fn encode_into(&self, out: &mut Vec<u8>) {
        match self {
            Frame::SimpleString(s) => {
                out.push(b'+');
                out.extend_from_slice(s.as_bytes());
                out.extend_from_slice(b"\r\n");
            }
            Frame::Error(s) => {
                out.push(b'-');
                out.extend_from_slice(s.as_bytes());
                out.extend_from_slice(b"\r\n");
            }
            Frame::Integer(n) => {
                out.push(b':');
                // Format i64 without String alloc via itoa-style manual encoding.
                let s = n.to_string();
                out.extend_from_slice(s.as_bytes());
                out.extend_from_slice(b"\r\n");
            }
            Frame::BulkString(None) => {
                out.extend_from_slice(b"$-1\r\n");
            }
            Frame::BulkString(Some(data)) => {
                out.push(b'$');
                out.extend_from_slice(data.len().to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
                out.extend_from_slice(data);
                out.extend_from_slice(b"\r\n");
            }
            Frame::Array(None) => {
                out.extend_from_slice(b"*-1\r\n");
            }
            Frame::Array(Some(elements)) => {
                out.push(b'*');
                out.extend_from_slice(elements.len().to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
                for elem in elements {
                    elem.encode_into(out);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_simple_string_round_trip() {
        let f = Frame::SimpleString("OK".into());
        let bytes = f.to_bytes();
        assert_eq!(bytes, b"+OK\r\n");
        let (parsed, consumed) = Frame::parse(&bytes).expect("parse ok");
        assert_eq!(parsed, f);
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn frame_integer_round_trip() {
        let f = Frame::Integer(42);
        let bytes = f.to_bytes();
        assert_eq!(bytes, b":42\r\n");
        let (parsed, consumed) = Frame::parse(&bytes).expect("parse ok");
        assert_eq!(parsed, f);
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn frame_bulk_string_round_trip() {
        let f = Frame::BulkString(Some(b"hello".to_vec()));
        let bytes = f.to_bytes();
        assert_eq!(bytes, b"$5\r\nhello\r\n");
        let (parsed, consumed) = Frame::parse(&bytes).expect("parse ok");
        assert_eq!(parsed, f);
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn frame_nil_bulk_string() {
        let bytes = b"$-1\r\n";
        let (f, consumed) = Frame::parse(bytes).expect("parse ok");
        assert_eq!(f, Frame::BulkString(None));
        assert_eq!(consumed, 5);
        assert_eq!(f.to_bytes(), b"$-1\r\n");
    }

    #[test]
    fn frame_incomplete_returns_error() {
        let result = Frame::parse(b"+OK\r");
        assert!(matches!(result, Err(ProtocolError::Incomplete)));
    }
}
