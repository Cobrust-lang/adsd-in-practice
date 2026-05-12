//! Integration tests for `redis_server::dispatch::from_frame`.
//!
//! Covers ADR-0004 §"Done Criteria" — all 7 items.
//! Frames are constructed via `redis_protocol::Frame::parse` (never hand-built).

use redis_protocol::Frame;
use redis_server::dispatch::from_frame;
use redis_storage::{Command, Reply};

/// Parse a RESP byte string and return the first frame; panics on error.
fn parse(input: &[u8]) -> Frame {
    let (frame, _) = Frame::parse(input).expect("test fixture must be valid RESP");
    frame
}

// ── ADR-0004 Criterion 1 ─────────────────────────────────────────────────────
// PING → Ok(Command::Ping)

#[test]
fn dispatch_ping() {
    let frame = parse(b"*1\r\n$4\r\nPING\r\n");
    let cmd = from_frame(frame).expect("PING must parse");
    assert!(matches!(cmd, Command::Ping));
}

// ── ADR-0004 Criterion 2 ─────────────────────────────────────────────────────
// GET foo → Ok(Command::Get { key: "foo" })

#[test]
fn dispatch_get() {
    let frame = parse(b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n");
    let cmd = from_frame(frame).expect("GET must parse");
    assert!(matches!(cmd, Command::Get { key } if key == "foo"));
}

// ── ADR-0004 Criterion 3 ─────────────────────────────────────────────────────
// SET foo bar (no TTL) → Ok(Command::Set { key, value, ttl_secs: None })

#[test]
fn dispatch_set_no_ttl() {
    let frame = parse(b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
    let cmd = from_frame(frame).expect("SET must parse");
    match cmd {
        Command::Set {
            key,
            value,
            ttl_secs,
        } => {
            assert_eq!(key, "foo");
            assert_eq!(value, b"bar");
            assert_eq!(ttl_secs, None);
        }
        other => panic!("expected Set, got {other:?}"),
    }
}

// ── ADR-0004 Criterion 4 ─────────────────────────────────────────────────────
// SET k v EX 60 → Ok(Command::Set { ..., ttl_secs: Some(60) })

#[test]
fn dispatch_set_with_ex() {
    let frame = parse(b"*5\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n$2\r\nEX\r\n$2\r\n60\r\n");
    let cmd = from_frame(frame).expect("SET EX must parse");
    match cmd {
        Command::Set {
            key,
            value,
            ttl_secs,
        } => {
            assert_eq!(key, "k");
            assert_eq!(value, b"v");
            assert_eq!(ttl_secs, Some(60));
        }
        other => panic!("expected Set, got {other:?}"),
    }
}

// ── ADR-0004 Criterion 5 ─────────────────────────────────────────────────────
// Command names are case-insensitive: `set`, `Set`, `SET` all work.

#[test]
fn dispatch_case_insensitive_set() {
    for raw in [
        b"*3\r\n$3\r\nset\r\n$1\r\na\r\n$1\r\nb\r\n".as_slice(),
        b"*3\r\n$3\r\nSet\r\n$1\r\na\r\n$1\r\nb\r\n".as_slice(),
        b"*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\nb\r\n".as_slice(),
    ] {
        let cmd = from_frame(parse(raw)).expect("case variant must parse");
        assert!(
            matches!(cmd, Command::Set { .. }),
            "expected Set for input starting with {}",
            std::str::from_utf8(&raw[5..8]).unwrap_or("?")
        );
    }
}

// ── ADR-0004 Criterion 6 ─────────────────────────────────────────────────────
// Unknown command → Err(Reply::Error("ERR unknown command 'XYZ'"))

#[test]
fn dispatch_unknown_command() {
    let frame = parse(b"*1\r\n$3\r\nXYZ\r\n");
    let err = from_frame(frame).expect_err("unknown command must error");
    assert_eq!(err, Reply::Error("ERR unknown command 'XYZ'".to_owned()));
}

// ── ADR-0004 Criterion 7 ─────────────────────────────────────────────────────
// Wrong number of arguments → Err(Reply::Error("ERR wrong number of arguments ..."))

#[test]
fn dispatch_set_missing_value_is_error() {
    // SET with only key, no value → 2 parts (cmd + key) is too few.
    let frame = parse(b"*2\r\n$3\r\nSET\r\n$3\r\nfoo\r\n");
    let err = from_frame(frame).expect_err("SET with only key must error");
    assert!(
        matches!(&err, Reply::Error(msg) if msg.contains("wrong number of arguments")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn dispatch_get_too_many_args_is_error() {
    // GET with 2 keys is not valid.
    let frame = parse(b"*3\r\n$3\r\nGET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
    let err = from_frame(frame).expect_err("GET with 2 keys must error");
    assert!(
        matches!(&err, Reply::Error(msg) if msg.contains("wrong number of arguments")),
        "unexpected error: {err:?}"
    );
}

// ── Extra coverage ────────────────────────────────────────────────────────────

#[test]
fn dispatch_del_single_key() {
    let frame = parse(b"*2\r\n$3\r\nDEL\r\n$3\r\nfoo\r\n");
    let cmd = from_frame(frame).expect("DEL must parse");
    assert!(matches!(cmd, Command::Del { keys } if keys == ["foo"]));
}

#[test]
fn dispatch_del_multiple_keys() {
    let frame = parse(b"*3\r\n$3\r\nDEL\r\n$1\r\na\r\n$1\r\nb\r\n");
    let cmd = from_frame(frame).expect("DEL multi must parse");
    assert!(matches!(cmd, Command::Del { keys } if keys == ["a", "b"]));
}

#[test]
fn dispatch_exists() {
    let frame = parse(b"*2\r\n$6\r\nEXISTS\r\n$3\r\nfoo\r\n");
    let cmd = from_frame(frame).expect("EXISTS must parse");
    assert!(matches!(cmd, Command::Exists { keys } if keys == ["foo"]));
}

#[test]
fn dispatch_incr() {
    let frame = parse(b"*2\r\n$4\r\nINCR\r\n$7\r\ncounter\r\n");
    let cmd = from_frame(frame).expect("INCR must parse");
    assert!(matches!(cmd, Command::Incr { key } if key == "counter"));
}

#[test]
fn dispatch_decr() {
    let frame = parse(b"*2\r\n$4\r\nDECR\r\n$7\r\ncounter\r\n");
    let cmd = from_frame(frame).expect("DECR must parse");
    assert!(matches!(cmd, Command::Decr { key } if key == "counter"));
}

#[test]
fn dispatch_nil_array_is_error() {
    // *-1\r\n is a nil array frame.
    let frame = parse(b"*-1\r\n");
    let err = from_frame(frame).expect_err("nil array must error");
    assert!(matches!(&err, Reply::Error(_)));
}
