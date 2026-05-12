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
    assert!(matches!(cmd, Command::Ping { message: None }));
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

// ── M1.3 (ADR-0005) — ECHO / SELECT / QUIT ──────────────────────────────────

#[test]
fn dispatch_echo() {
    let frame = parse(b"*2\r\n$4\r\nECHO\r\n$5\r\nhello\r\n");
    let cmd = from_frame(frame).expect("ECHO must parse");
    assert!(matches!(cmd, Command::Echo { message } if message == b"hello"));
}

#[test]
fn dispatch_echo_case_insensitive() {
    let frame = parse(b"*2\r\n$4\r\necho\r\n$2\r\nhi\r\n");
    let cmd = from_frame(frame).expect("echo (lowercase) must parse");
    assert!(matches!(cmd, Command::Echo { message } if message == b"hi"));
}

#[test]
fn dispatch_echo_wrong_arity_is_error() {
    let frame = parse(b"*1\r\n$4\r\nECHO\r\n");
    let err = from_frame(frame).expect_err("ECHO without message must error");
    assert!(
        matches!(&err, Reply::Error(msg) if msg.contains("wrong number of arguments for 'echo'")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn dispatch_select_zero() {
    let frame = parse(b"*2\r\n$6\r\nSELECT\r\n$1\r\n0\r\n");
    let cmd = from_frame(frame).expect("SELECT 0 must parse");
    assert!(matches!(cmd, Command::Select { db: 0 }));
}

#[test]
fn dispatch_select_non_zero() {
    let frame = parse(b"*2\r\n$6\r\nSELECT\r\n$1\r\n9\r\n");
    let cmd = from_frame(frame).expect("SELECT 9 must parse (store reports range error)");
    assert!(matches!(cmd, Command::Select { db: 9 }));
}

#[test]
fn dispatch_select_non_integer_is_error() {
    let frame = parse(b"*2\r\n$6\r\nSELECT\r\n$3\r\nabc\r\n");
    let err = from_frame(frame).expect_err("SELECT abc must error at dispatch level");
    assert!(
        matches!(&err, Reply::Error(msg) if msg.contains("not an integer")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn dispatch_quit() {
    let frame = parse(b"*1\r\n$4\r\nQUIT\r\n");
    let cmd = from_frame(frame).expect("QUIT must parse");
    assert!(matches!(cmd, Command::Quit));
}

#[test]
fn dispatch_quit_case_insensitive() {
    let frame = parse(b"*1\r\n$4\r\nquit\r\n");
    let cmd = from_frame(frame).expect("quit must parse");
    assert!(matches!(cmd, Command::Quit));
}

#[test]
fn dispatch_quit_with_args_is_error() {
    let frame = parse(b"*2\r\n$4\r\nQUIT\r\n$3\r\nfoo\r\n");
    let err = from_frame(frame).expect_err("QUIT with arg must error");
    assert!(
        matches!(&err, Reply::Error(msg) if msg.contains("wrong number of arguments for 'quit'")),
        "unexpected error: {err:?}"
    );
}

// ── M1.4 (ADR-0006) — PING optional message ─────────────────────────────────

#[test]
fn dispatch_ping_with_message() {
    let frame = parse(b"*2\r\n$4\r\nPING\r\n$5\r\nhello\r\n");
    let cmd = from_frame(frame).expect("PING hello must parse");
    assert!(matches!(cmd, Command::Ping { message: Some(b) } if b == b"hello"));
}

#[test]
fn dispatch_ping_too_many_args_is_error() {
    let frame = parse(b"*3\r\n$4\r\nPING\r\n$1\r\na\r\n$1\r\nb\r\n");
    let err = from_frame(frame).expect_err("PING with 2 messages must error");
    assert!(
        matches!(&err, Reply::Error(msg) if msg.contains("wrong number of arguments for 'ping'")),
        "unexpected error: {err:?}"
    );
}

// ── M1.4 (ADR-0006) — EXPIRE / TTL / PERSIST / TYPE / KEYS ─────────────────

#[test]
fn dispatch_expire() {
    let frame = parse(b"*3\r\n$6\r\nEXPIRE\r\n$3\r\nfoo\r\n$2\r\n60\r\n");
    let cmd = from_frame(frame).expect("EXPIRE must parse");
    assert!(matches!(cmd, Command::Expire { key, seconds: 60 } if key == "foo"));
}

#[test]
fn dispatch_expire_case_insensitive() {
    let frame = parse(b"*3\r\n$6\r\nexpire\r\n$1\r\nk\r\n$1\r\n5\r\n");
    let cmd = from_frame(frame).expect("expire lower must parse");
    assert!(matches!(cmd, Command::Expire { key, seconds: 5 } if key == "k"));
}

#[test]
fn dispatch_expire_wrong_arity_is_error() {
    let frame = parse(b"*2\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n");
    let err = from_frame(frame).expect_err("EXPIRE without seconds must error");
    assert!(
        matches!(&err, Reply::Error(msg) if msg.contains("wrong number of arguments for 'expire'")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn dispatch_expire_non_integer_seconds_is_error() {
    let frame = parse(b"*3\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$3\r\nabc\r\n");
    let err = from_frame(frame).expect_err("EXPIRE with non-integer seconds must error");
    assert!(
        matches!(&err, Reply::Error(msg) if msg.contains("not an integer")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn dispatch_ttl() {
    let frame = parse(b"*2\r\n$3\r\nTTL\r\n$3\r\nfoo\r\n");
    let cmd = from_frame(frame).expect("TTL must parse");
    assert!(matches!(cmd, Command::Ttl { key } if key == "foo"));
}

#[test]
fn dispatch_ttl_wrong_arity_is_error() {
    let frame = parse(b"*1\r\n$3\r\nTTL\r\n");
    let err = from_frame(frame).expect_err("TTL with no key must error");
    assert!(matches!(&err, Reply::Error(_)));
}

#[test]
fn dispatch_persist() {
    let frame = parse(b"*2\r\n$7\r\nPERSIST\r\n$3\r\nfoo\r\n");
    let cmd = from_frame(frame).expect("PERSIST must parse");
    assert!(matches!(cmd, Command::Persist { key } if key == "foo"));
}

#[test]
fn dispatch_type() {
    let frame = parse(b"*2\r\n$4\r\nTYPE\r\n$3\r\nfoo\r\n");
    let cmd = from_frame(frame).expect("TYPE must parse");
    assert!(matches!(cmd, Command::Type { key } if key == "foo"));
}

#[test]
fn dispatch_keys_star() {
    let frame = parse(b"*2\r\n$4\r\nKEYS\r\n$1\r\n*\r\n");
    let cmd = from_frame(frame).expect("KEYS * must parse");
    assert!(matches!(cmd, Command::Keys { pattern } if pattern == "*"));
}

#[test]
fn dispatch_keys_complex_pattern() {
    // "user:?*" = 7 bytes.
    let frame = parse(b"*2\r\n$4\r\nKEYS\r\n$7\r\nuser:?*\r\n");
    let cmd = from_frame(frame).expect("KEYS user:?* must parse");
    assert!(matches!(cmd, Command::Keys { pattern } if pattern == "user:?*"));
}

#[test]
fn dispatch_keys_wrong_arity_is_error() {
    let frame = parse(b"*1\r\n$4\r\nKEYS\r\n");
    let err = from_frame(frame).expect_err("KEYS with no pattern must error");
    assert!(matches!(&err, Reply::Error(_)));
}

// ── M3.1 (ADR-0009) Pub/Sub parsers ───────────────────────────────────────────

#[test]
fn dispatch_subscribe_single_channel() {
    let frame = parse(b"*2\r\n$9\r\nSUBSCRIBE\r\n$4\r\nnews\r\n");
    let cmd = from_frame(frame).expect("SUBSCRIBE news must parse");
    match cmd {
        Command::Subscribe { channels } => assert_eq!(channels, vec!["news".to_owned()]),
        other => panic!("expected Subscribe, got {other:?}"),
    }
}

#[test]
fn dispatch_subscribe_multiple_channels() {
    let frame = parse(b"*4\r\n$9\r\nSUBSCRIBE\r\n$1\r\na\r\n$1\r\nb\r\n$1\r\nc\r\n");
    let cmd = from_frame(frame).expect("SUBSCRIBE a b c must parse");
    match cmd {
        Command::Subscribe { channels } => {
            assert_eq!(
                channels,
                vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
            );
        }
        other => panic!("expected Subscribe, got {other:?}"),
    }
}

#[test]
fn dispatch_subscribe_case_insensitive() {
    let frame = parse(b"*2\r\n$9\r\nsubscribe\r\n$4\r\nnews\r\n");
    let cmd = from_frame(frame).expect("lowercase subscribe must parse");
    assert!(matches!(cmd, Command::Subscribe { .. }));
}

#[test]
fn dispatch_subscribe_no_args_is_error() {
    let frame = parse(b"*1\r\n$9\r\nSUBSCRIBE\r\n");
    let err = from_frame(frame).expect_err("SUBSCRIBE with no channels must error");
    let Reply::Error(msg) = err else {
        panic!("expected Reply::Error");
    };
    assert!(msg.contains("'subscribe'"), "got msg: {msg}");
}

#[test]
fn dispatch_unsubscribe_no_args_means_all() {
    let frame = parse(b"*1\r\n$11\r\nUNSUBSCRIBE\r\n");
    let cmd = from_frame(frame).expect("UNSUBSCRIBE with no args must parse");
    match cmd {
        Command::Unsubscribe { channels } => {
            assert!(
                channels.is_empty(),
                "expected empty (unsub-all), got {channels:?}"
            );
        }
        other => panic!("expected Unsubscribe, got {other:?}"),
    }
}

#[test]
fn dispatch_unsubscribe_specific_channels() {
    let frame = parse(b"*3\r\n$11\r\nUNSUBSCRIBE\r\n$1\r\na\r\n$1\r\nb\r\n");
    let cmd = from_frame(frame).expect("UNSUBSCRIBE a b must parse");
    match cmd {
        Command::Unsubscribe { channels } => {
            assert_eq!(channels, vec!["a".to_owned(), "b".to_owned()]);
        }
        other => panic!("expected Unsubscribe, got {other:?}"),
    }
}

#[test]
fn dispatch_publish_round_trip() {
    let frame = parse(b"*3\r\n$7\r\nPUBLISH\r\n$4\r\nnews\r\n$5\r\nhello\r\n");
    let cmd = from_frame(frame).expect("PUBLISH news hello must parse");
    match cmd {
        Command::Publish { channel, message } => {
            assert_eq!(channel, "news");
            assert_eq!(message, b"hello".to_vec());
        }
        other => panic!("expected Publish, got {other:?}"),
    }
}

#[test]
fn dispatch_publish_case_insensitive() {
    let frame = parse(b"*3\r\n$7\r\npublish\r\n$4\r\nnews\r\n$5\r\nhello\r\n");
    let cmd = from_frame(frame).expect("lowercase publish must parse");
    assert!(matches!(cmd, Command::Publish { .. }));
}

#[test]
fn dispatch_publish_wrong_arity_is_error() {
    let frame = parse(b"*2\r\n$7\r\nPUBLISH\r\n$4\r\nnews\r\n");
    let err = from_frame(frame).expect_err("PUBLISH without message must error");
    let Reply::Error(msg) = err else {
        panic!("expected Reply::Error");
    };
    assert!(msg.contains("'publish'"), "got msg: {msg}");
}

#[test]
fn dispatch_publish_binary_payload_round_trips() {
    // Payload with NUL byte + non-utf8 (0xff).  PUBLISH must preserve raw bytes.
    let frame = parse(b"*3\r\n$7\r\nPUBLISH\r\n$1\r\nx\r\n$3\r\n\x00\xff\x7f\r\n");
    let cmd = from_frame(frame).expect("PUBLISH with binary payload must parse");
    match cmd {
        Command::Publish { channel, message } => {
            assert_eq!(channel, "x");
            assert_eq!(message, vec![0x00, 0xff, 0x7f]);
        }
        other => panic!("expected Publish, got {other:?}"),
    }
}

// ── M4.1 (ADR-0011 §#10) — `parse_set` strict arity ─────────────────────────

#[test]
fn parse_set_rejects_trailing_token() {
    // `SET k v EX 60 GARBAGE` — 6 parts, must reject with verbatim
    // `ERR syntax error` to match real Redis.  M3.2 accepted this
    // (parts.len() >= 5 with trailing tokens silently ignored).
    let frame = parse(
        b"*6\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n$2\r\nEX\r\n$2\r\n60\r\n$7\r\nGARBAGE\r\n",
    );
    let err = from_frame(frame).expect_err("SET with trailing token must error");
    assert_eq!(err, Reply::Error("ERR syntax error".to_owned()));
}

#[test]
fn parse_set_rejects_more_than_one_trailing_token() {
    // 7 parts: `SET k v EX 60 X Y` — same error.
    let frame = parse(
        b"*7\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n$2\r\nEX\r\n$2\r\n60\r\n$1\r\nX\r\n$1\r\nY\r\n",
    );
    let err = from_frame(frame).expect_err("SET with two trailing tokens must error");
    assert_eq!(err, Reply::Error("ERR syntax error".to_owned()));
}

#[test]
fn parse_set_accepts_canonical_ex_form() {
    // `SET k v EX 60` — 5 parts, ttl_secs = Some(60).
    let frame = parse(b"*5\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n$2\r\nEX\r\n$2\r\n60\r\n");
    let cmd = from_frame(frame).expect("SET k v EX 60 must parse");
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
