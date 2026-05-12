//! `Reply` → RESP `Frame` mapping (ADR-0005).
//!
//! Lives in the server crate (not in `redis-storage`) because the storage
//! layer is RESP-agnostic — it owns the `Reply` enum but knows nothing
//! about wire format.  Layer rule: ADR-0004 §"Decision" + ADR-0005 §
//! "关键子决策" both lock this boundary.

use redis_protocol::Frame;
use redis_storage::Reply;

/// Convert a storage `Reply` into a RESP `Frame` ready for `to_bytes()`.
///
/// Mapping (all 7 variants covered):
///
/// | Reply                 | Frame                             |
/// |-----------------------|-----------------------------------|
/// | `Pong`                | `SimpleString("PONG")`            |
/// | `Ok`                  | `SimpleString("OK")`              |
/// | `Bulk(opt)`           | `BulkString(opt)`                 |
/// | `Integer(n)`          | `Integer(n)`                      |
/// | `Error(msg)`          | `Error(msg)` (no leading `-`!)    |
/// | `SimpleString(s)`     | `SimpleString(s)` (M1.4)          |
/// | `Array(Some(items))`  | `Array(Some(bulk*))`              |
/// | `Array(None)`         | `Array(None)` (`*-1\r\n`)         |
///
/// **Note**: `Reply::Error(msg)` is mapped to `Frame::Error(msg)` *without*
/// a leading `-` prefix — `Frame::to_bytes()` adds the byte itself.
#[must_use]
pub fn reply_to_frame(reply: Reply) -> Frame {
    match reply {
        Reply::Pong => Frame::SimpleString("PONG".to_owned()),
        Reply::Ok => Frame::SimpleString("OK".to_owned()),
        Reply::Bulk(opt) => Frame::BulkString(opt),
        Reply::Integer(n) => Frame::Integer(n),
        Reply::Error(msg) => Frame::Error(msg),
        Reply::SimpleString(s) => Frame::SimpleString(s),
        Reply::Array(None) => Frame::Array(None),
        Reply::Array(Some(items)) => Frame::Array(Some(
            items
                .into_iter()
                .map(|b| Frame::BulkString(Some(b)))
                .collect(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pong_maps_to_simple_string() {
        let frame = reply_to_frame(Reply::Pong);
        assert_eq!(frame.to_bytes(), b"+PONG\r\n");
    }

    #[test]
    fn ok_maps_to_simple_string() {
        let frame = reply_to_frame(Reply::Ok);
        assert_eq!(frame.to_bytes(), b"+OK\r\n");
    }

    #[test]
    fn bulk_some_maps_to_bulk_string() {
        let frame = reply_to_frame(Reply::Bulk(Some(b"hi".to_vec())));
        assert_eq!(frame.to_bytes(), b"$2\r\nhi\r\n");
    }

    #[test]
    fn bulk_none_maps_to_nil_bulk() {
        let frame = reply_to_frame(Reply::Bulk(None));
        assert_eq!(frame.to_bytes(), b"$-1\r\n");
    }

    #[test]
    fn integer_maps_to_integer() {
        let frame = reply_to_frame(Reply::Integer(-7));
        assert_eq!(frame.to_bytes(), b":-7\r\n");
    }

    #[test]
    fn error_maps_without_dash_prefix() {
        // The Reply::Error message must NOT already have a `-`; the byte
        // is added by Frame::to_bytes itself.
        let frame = reply_to_frame(Reply::Error("ERR boom".to_owned()));
        assert_eq!(frame.to_bytes(), b"-ERR boom\r\n");
    }

    // ── M1.4 (ADR-0006) ──────────────────────────────────────────────────

    #[test]
    fn simple_string_maps_with_plus_prefix() {
        let frame = reply_to_frame(Reply::SimpleString("string".to_owned()));
        assert_eq!(frame.to_bytes(), b"+string\r\n");
    }

    #[test]
    fn simple_string_none_for_type() {
        let frame = reply_to_frame(Reply::SimpleString("none".to_owned()));
        assert_eq!(frame.to_bytes(), b"+none\r\n");
    }

    #[test]
    fn array_some_maps_to_bulk_array() {
        let frame = reply_to_frame(Reply::Array(Some(vec![b"a".to_vec(), b"bb".to_vec()])));
        assert_eq!(frame.to_bytes(), b"*2\r\n$1\r\na\r\n$2\r\nbb\r\n");
    }

    #[test]
    fn array_empty_maps_to_zero_length_array() {
        let frame = reply_to_frame(Reply::Array(Some(vec![])));
        assert_eq!(frame.to_bytes(), b"*0\r\n");
    }

    #[test]
    fn array_none_maps_to_nil_array() {
        let frame = reply_to_frame(Reply::Array(None));
        assert_eq!(frame.to_bytes(), b"*-1\r\n");
    }
}
