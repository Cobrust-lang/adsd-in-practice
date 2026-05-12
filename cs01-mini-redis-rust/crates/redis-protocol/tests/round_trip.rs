//! Integration tests for redis-protocol RESP v2 parser + serializer.
//!
//! Covers all 8 Done Criteria from ADR-0002, plus pipelining and error cases.

use redis_protocol::{Frame, ProtocolError};

// ─── ADR-0002 Done Criterion 1 ───────────────────────────────────────────────
// `Frame::parse(b"+OK\r\n")` returns `(Frame::SimpleString("OK"), 5)`

#[test]
fn simple_string_ok() {
    let (frame, consumed) = Frame::parse(b"+OK\r\n").expect("parse ok");
    assert_eq!(frame, Frame::SimpleString("OK".into()));
    assert_eq!(consumed, 5);
}

// ─── ADR-0002 Done Criterion 2 ───────────────────────────────────────────────
// `Frame::parse(b":42\r\n")` returns `(Frame::Integer(42), 5)`

#[test]
fn integer_42() {
    let (frame, consumed) = Frame::parse(b":42\r\n").expect("parse ok");
    assert_eq!(frame, Frame::Integer(42));
    assert_eq!(consumed, 5);
}

// ─── ADR-0002 Done Criterion 3 ───────────────────────────────────────────────
// `Frame::parse(b"$5\r\nhello\r\n")` returns `(Frame::BulkString(Some(b"hello".to_vec())), 11)`

#[test]
fn bulk_string_hello() {
    let (frame, consumed) = Frame::parse(b"$5\r\nhello\r\n").expect("parse ok");
    assert_eq!(frame, Frame::BulkString(Some(b"hello".to_vec())));
    assert_eq!(consumed, 11);
}

// ─── ADR-0002 Done Criterion 4 ───────────────────────────────────────────────
// `Frame::parse(b"$-1\r\n")` returns `(Frame::BulkString(None), 5)` (RESP nil)

#[test]
fn nil_bulk_string() {
    let (frame, consumed) = Frame::parse(b"$-1\r\n").expect("parse ok");
    assert_eq!(frame, Frame::BulkString(None));
    assert_eq!(consumed, 5);
}

// ─── ADR-0002 Done Criterion 5 ───────────────────────────────────────────────
// Array of 2 bulk strings: "*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n"

#[test]
fn array_get_foo() {
    let input = b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n";
    let (frame, consumed) = Frame::parse(input).expect("parse ok");
    assert_eq!(
        frame,
        Frame::Array(Some(vec![
            Frame::BulkString(Some(b"GET".to_vec())),
            Frame::BulkString(Some(b"foo".to_vec())),
        ]))
    );
    assert_eq!(consumed, input.len());
}

// ─── ADR-0002 Done Criterion 6 ───────────────────────────────────────────────
// `Frame::parse(b"+OK\r")` returns `Err(Incomplete)` (incomplete tail)

#[test]
fn simple_string_incomplete() {
    let result = Frame::parse(b"+OK\r");
    assert!(
        matches!(result, Err(ProtocolError::Incomplete)),
        "expected Incomplete, got {result:?}"
    );
}

// ─── ADR-0002 Done Criterion 7 ───────────────────────────────────────────────
// `Frame::to_bytes(&Frame::SimpleString("OK".into()))` returns `b"+OK\r\n"`

#[test]
fn to_bytes_simple_string() {
    let frame = Frame::SimpleString("OK".into());
    assert_eq!(frame.to_bytes(), b"+OK\r\n");
}

// ─── ADR-0002 Done Criterion 8 ───────────────────────────────────────────────
// round-trip: parse(to_bytes(f)).0 == f  (covered by proptest too; this test
// checks a concrete set of representative frames)

#[test]
fn round_trip_all_variants() {
    let frames = vec![
        Frame::SimpleString("OK".into()),
        Frame::SimpleString(String::new()),
        Frame::SimpleString("hello world".into()),
        Frame::Error("ERR syntax error".into()),
        Frame::Error("WRONGTYPE".into()),
        Frame::Integer(0),
        Frame::Integer(42),
        Frame::Integer(-1),
        Frame::Integer(i64::MAX),
        Frame::Integer(i64::MIN),
        Frame::BulkString(Some(b"hello".to_vec())),
        Frame::BulkString(Some(b"".to_vec())),
        Frame::BulkString(Some(vec![0u8, 1, 2, 255])),
        Frame::BulkString(None),
        Frame::Array(None),
        Frame::Array(Some(vec![])),
        Frame::Array(Some(vec![
            Frame::BulkString(Some(b"SET".to_vec())),
            Frame::BulkString(Some(b"key".to_vec())),
            Frame::BulkString(Some(b"value".to_vec())),
        ])),
        // Nested array
        Frame::Array(Some(vec![
            Frame::Array(Some(vec![Frame::Integer(1), Frame::Integer(2)])),
            Frame::SimpleString("inner".into()),
        ])),
    ];

    for frame in &frames {
        let bytes = frame.to_bytes();
        let (parsed, consumed) =
            Frame::parse(&bytes).unwrap_or_else(|e| panic!("parse failed for {frame:?}: {e}"));
        assert_eq!(parsed, *frame, "round-trip mismatch for {frame:?}");
        assert_eq!(
            consumed,
            bytes.len(),
            "consumed != bytes.len() for {frame:?}"
        );
    }
}

// ─── Nil array ────────────────────────────────────────────────────────────────

#[test]
fn nil_array() {
    let (frame, consumed) = Frame::parse(b"*-1\r\n").expect("parse ok");
    assert_eq!(frame, Frame::Array(None));
    assert_eq!(consumed, 5);
    assert_eq!(frame.to_bytes(), b"*-1\r\n");
}

// ─── Error frame ─────────────────────────────────────────────────────────────

#[test]
fn error_frame() {
    let input = b"-ERR unknown command\r\n";
    let (frame, consumed) = Frame::parse(input).expect("parse ok");
    assert_eq!(frame, Frame::Error("ERR unknown command".into()));
    assert_eq!(consumed, input.len());
    assert_eq!(frame.to_bytes(), input.as_ref());
}

// ─── Negative integer ────────────────────────────────────────────────────────

#[test]
fn integer_negative() {
    let (frame, consumed) = Frame::parse(b":-1\r\n").expect("parse ok");
    assert_eq!(frame, Frame::Integer(-1));
    assert_eq!(consumed, 5);
}

// ─── Pipelined frames ────────────────────────────────────────────────────────
// Two frames back-to-back in one buffer; parse twice, consuming correct bytes.

#[test]
fn pipelined_two_simple_strings() {
    let input = b"+OK\r\n+PONG\r\n";

    let (first, n1) = Frame::parse(input).expect("parse first");
    assert_eq!(first, Frame::SimpleString("OK".into()));
    assert_eq!(n1, 5);

    let (second, n2) = Frame::parse(&input[n1..]).expect("parse second");
    assert_eq!(second, Frame::SimpleString("PONG".into()));
    assert_eq!(n2, 7);

    // Total consumed == total input length
    assert_eq!(n1 + n2, input.len());
}

#[test]
fn pipelined_two_ok() {
    // ADR §pipelining: "+OK\r\n+OK\r\n" → parse twice, both succeed, correct offsets.
    let input = b"+OK\r\n+OK\r\n";

    let (f1, n1) = Frame::parse(input).expect("parse first ok");
    assert_eq!(f1, Frame::SimpleString("OK".into()));
    assert_eq!(n1, 5);

    let (f2, n2) = Frame::parse(&input[n1..]).expect("parse second ok");
    assert_eq!(f2, Frame::SimpleString("OK".into()));
    assert_eq!(n2, 5);

    assert_eq!(n1 + n2, input.len());
}

// ─── Error case: bulk string missing trailing \r\n ────────────────────────────

#[test]
fn bulk_string_missing_trailing_crlf_incomplete() {
    // "$5\r\nhello" — data present but trailing \r\n absent → Incomplete
    let result = Frame::parse(b"$5\r\nhello");
    assert!(
        matches!(result, Err(ProtocolError::Incomplete)),
        "expected Incomplete, got {result:?}"
    );
}

// ─── Error case: unknown type byte ───────────────────────────────────────────

#[test]
fn unknown_type_byte_invalid() {
    // "!unknown\r\n" — '!' is not a RESP v2 type byte → Invalid
    let result = Frame::parse(b"!unknown\r\n");
    assert!(
        matches!(result, Err(ProtocolError::Invalid(_))),
        "expected Invalid, got {result:?}"
    );
}

// ─── Empty buffer → Incomplete ───────────────────────────────────────────────

#[test]
fn empty_input_incomplete() {
    let result = Frame::parse(b"");
    assert!(matches!(result, Err(ProtocolError::Incomplete)));
}

// ─── Bulk string with binary content ─────────────────────────────────────────

#[test]
fn bulk_string_binary() {
    let data: Vec<u8> = (0u8..=255).collect();
    let frame = Frame::BulkString(Some(data.clone()));
    let bytes = frame.to_bytes();
    let (parsed, consumed) = Frame::parse(&bytes).expect("parse ok");
    assert_eq!(parsed, Frame::BulkString(Some(data)));
    assert_eq!(consumed, bytes.len());
}

// ─── Array with mixed element types ──────────────────────────────────────────

#[test]
fn array_mixed_types() {
    let frame = Frame::Array(Some(vec![
        Frame::SimpleString("status".into()),
        Frame::Integer(100),
        Frame::BulkString(Some(b"data".to_vec())),
        Frame::BulkString(None),
    ]));
    let bytes = frame.to_bytes();
    let (parsed, consumed) = Frame::parse(&bytes).expect("parse ok");
    assert_eq!(parsed, frame);
    assert_eq!(consumed, bytes.len());
}
