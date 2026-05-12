//! Integration tests for `redis-storage::aof` + Store AOF hook (M3.2,
//! ADR-0010).  Covers the "Write path", "Replay", and "TTL across
//! restart" Done Criteria from the ADR.
//!
//! Each test uses a unique temp file under `std::env::temp_dir()` so
//! parallel `cargo test` runs don't collide.  The file is cleaned up
//! after the test via the `TempAof` RAII guard.
//!
//! No `tempfile` crate dep — keeping the workspace dep-list flat
//! (ADR-0010 §"No new workspace deps").
//!
//! Time control: AOF replay tests use deterministic file contents
//! (hand-built bytes or earlier-written bytes), so they DON'T need
//! `start_paused = true`.  The single TTL-roundtrip test uses a real
//! 1.1 s sleep to verify active expiration after replay — same
//! pattern the server e2e tests use.

#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::time::Duration;

use redis_storage::{Command, FsyncPolicy, Reply, Store};

/// RAII guard around a temp AOF file path.  Drops the file on Drop
/// so parallel test runs don't pile up garbage under `/tmp`.
struct TempAof {
    path: PathBuf,
}

impl TempAof {
    fn new(stem: &str) -> Self {
        let pid = std::process::id();
        // ns-precision counter for uniqueness inside a single process
        // (multiple TempAofs spawned in the same `cargo test` job).
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let mut path = std::env::temp_dir();
        path.push(format!("cs01-aof-test-{stem}-{pid}-{nonce}.aof"));
        Self { path }
    }
}

impl Drop for TempAof {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// ── Write path ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn set_appends_resp_array_to_aof_file() {
    let temp = TempAof::new("set-writes");
    let store = Store::with_aof(temp.path.clone(), FsyncPolicy::Always)
        .await
        .expect("with_aof open");

    let reply = store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .await
        .expect("set");
    assert_eq!(reply, Reply::Ok);

    // Flush + drop the store so the AOF file content is durably
    // visible before we read it.  `aof_flush` blocks on the writer
    // task's checkpoint, so no `tokio::time::sleep` guesswork.
    store.aof_flush().await;
    drop(store);

    let bytes = std::fs::read(&temp.path).expect("read aof");
    assert_eq!(bytes, b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n");
}

#[tokio::test]
async fn set_with_ex_appends_five_part_array() {
    let temp = TempAof::new("set-ex");
    let store = Store::with_aof(temp.path.clone(), FsyncPolicy::Always)
        .await
        .expect("with_aof");
    let _ = store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: Some(60),
        })
        .await
        .expect("set ex");
    store.aof_flush().await;
    drop(store);

    let bytes = std::fs::read(&temp.path).expect("read aof");
    assert_eq!(
        bytes,
        b"*5\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n$2\r\nEX\r\n$2\r\n60\r\n"
    );
}

#[tokio::test]
async fn read_only_commands_do_not_extend_aof() {
    let temp = TempAof::new("readonly");
    let store = Store::with_aof(temp.path.clone(), FsyncPolicy::Always)
        .await
        .expect("with_aof");
    let _ = store
        .execute(Command::Get {
            key: "missing".to_owned(),
        })
        .await
        .expect("get");
    let _ = store
        .execute(Command::Exists {
            keys: vec!["x".to_owned()],
        })
        .await
        .expect("exists");
    let _ = store
        .execute(Command::Ping { message: None })
        .await
        .expect("ping");
    let _ = store
        .execute(Command::Echo {
            message: b"hi".to_vec(),
        })
        .await
        .expect("echo");
    store.aof_flush().await;
    drop(store);

    let bytes = std::fs::read(&temp.path).expect("read aof");
    assert!(
        bytes.is_empty(),
        "read-only commands must not extend AOF, got {} bytes: {:?}",
        bytes.len(),
        bytes
    );
}

#[tokio::test]
async fn pubsub_commands_do_not_extend_aof() {
    let temp = TempAof::new("pubsub-noaof");
    let store = Store::with_aof(temp.path.clone(), FsyncPolicy::Always)
        .await
        .expect("with_aof");
    let _ = store
        .execute(Command::Publish {
            channel: "c".to_owned(),
            message: b"m".to_vec(),
        })
        .await
        .expect("publish");
    // SUBSCRIBE / UNSUBSCRIBE reach execute via dispatch-bug path
    // but the AOF encoder still returns None for them; exercise.
    let _ = store
        .execute(Command::Subscribe {
            channels: vec!["c".to_owned()],
        })
        .await
        .expect("subscribe-routes-to-error-but-still-tests-encode");
    let _ = store
        .execute(Command::Unsubscribe {
            channels: vec!["c".to_owned()],
        })
        .await
        .expect("unsubscribe-routes-to-error");
    store.aof_flush().await;
    drop(store);

    let bytes = std::fs::read(&temp.path).expect("read aof");
    assert!(
        bytes.is_empty(),
        "pubsub commands must not extend AOF, got {} bytes",
        bytes.len()
    );
}

#[tokio::test]
async fn incr_decr_del_expire_persist_all_appended() {
    let temp = TempAof::new("six-writables");
    let store = Store::with_aof(temp.path.clone(), FsyncPolicy::Always)
        .await
        .expect("with_aof");
    let _ = store
        .execute(Command::Incr {
            key: "c".to_owned(),
        })
        .await;
    let _ = store
        .execute(Command::Decr {
            key: "c".to_owned(),
        })
        .await;
    let _ = store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .await;
    let _ = store
        .execute(Command::Expire {
            key: "k".to_owned(),
            seconds: 30,
        })
        .await;
    let _ = store
        .execute(Command::Persist {
            key: "k".to_owned(),
        })
        .await;
    let _ = store
        .execute(Command::Del {
            keys: vec!["k".to_owned(), "missing".to_owned()],
        })
        .await;
    store.aof_flush().await;
    drop(store);

    let want: Vec<u8> = [
        &b"*2\r\n$4\r\nINCR\r\n$1\r\nc\r\n"[..],
        &b"*2\r\n$4\r\nDECR\r\n$1\r\nc\r\n"[..],
        &b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n"[..],
        &b"*3\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$2\r\n30\r\n"[..],
        &b"*2\r\n$7\r\nPERSIST\r\n$1\r\nk\r\n"[..],
        &b"*3\r\n$3\r\nDEL\r\n$1\r\nk\r\n$7\r\nmissing\r\n"[..],
    ]
    .concat();
    let got = std::fs::read(&temp.path).expect("read aof");
    assert_eq!(got, want);
}

#[tokio::test]
async fn incr_error_does_not_append_to_aof() {
    // Setting a non-integer and then INCR'ing should leave only the
    // SET in the AOF — INCR returns Reply::Error, so we don't append
    // a command that would just re-produce an error during replay.
    let temp = TempAof::new("incr-err");
    let store = Store::with_aof(temp.path.clone(), FsyncPolicy::Always)
        .await
        .expect("with_aof");
    let _ = store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"not-a-number".to_vec(),
            ttl_secs: None,
        })
        .await;
    let reply = store
        .execute(Command::Incr {
            key: "k".to_owned(),
        })
        .await
        .expect("incr");
    assert!(matches!(reply, Reply::Error(_)), "expected Reply::Error");
    store.aof_flush().await;
    drop(store);

    let bytes = std::fs::read(&temp.path).expect("read aof");
    // Only the SET should be in the file.
    assert_eq!(
        bytes,
        b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$12\r\nnot-a-number\r\n"
    );
}

// ── Replay path ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn replay_nonexistent_path_returns_zero() {
    let temp = TempAof::new("missing-path");
    // Don't touch the file.
    let store = Store::new();
    let count = store.replay_from_path(&temp.path).await.expect("replay");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn replay_empty_file_returns_zero() {
    let temp = TempAof::new("empty");
    std::fs::write(&temp.path, b"").expect("create empty");
    let store = Store::new();
    let count = store.replay_from_path(&temp.path).await.expect("replay");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn replay_round_trip_rebuilds_state() {
    let temp = TempAof::new("round-trip");

    // Write phase.
    {
        let store = Store::with_aof(temp.path.clone(), FsyncPolicy::Always)
            .await
            .expect("with_aof");
        let _ = store
            .execute(Command::Set {
                key: "foo".to_owned(),
                value: b"bar".to_vec(),
                ttl_secs: None,
            })
            .await;
        let _ = store
            .execute(Command::Set {
                key: "n".to_owned(),
                value: b"10".to_vec(),
                ttl_secs: None,
            })
            .await;
        let _ = store
            .execute(Command::Incr {
                key: "n".to_owned(),
            })
            .await;
        let _ = store
            .execute(Command::Set {
                key: "doomed".to_owned(),
                value: b"x".to_vec(),
                ttl_secs: None,
            })
            .await;
        let _ = store
            .execute(Command::Del {
                keys: vec!["doomed".to_owned()],
            })
            .await;
        store.aof_flush().await;
        drop(store);
    }

    // Replay into a fresh in-memory store and assert state matches.
    let store2 = Store::new();
    let count = store2.replay_from_path(&temp.path).await.expect("replay");
    assert_eq!(count, 5, "5 writable commands recorded");

    let foo = store2
        .execute(Command::Get {
            key: "foo".to_owned(),
        })
        .await
        .expect("get foo");
    assert_eq!(foo, Reply::Bulk(Some(b"bar".to_vec())));
    let n = store2
        .execute(Command::Get {
            key: "n".to_owned(),
        })
        .await
        .expect("get n");
    assert_eq!(n, Reply::Bulk(Some(b"11".to_vec())));
    let doomed = store2
        .execute(Command::Get {
            key: "doomed".to_owned(),
        })
        .await
        .expect("get doomed");
    assert_eq!(doomed, Reply::Bulk(None));
}

#[tokio::test]
async fn replay_does_not_re_extend_file() {
    // After replay, the file must be the same length as before
    // (because replay uses execute_no_aof, which skips the AOF
    // append).
    let temp = TempAof::new("no-re-extend");
    {
        let store = Store::with_aof(temp.path.clone(), FsyncPolicy::Always)
            .await
            .expect("with_aof");
        let _ = store
            .execute(Command::Set {
                key: "k".to_owned(),
                value: b"v".to_vec(),
                ttl_secs: None,
            })
            .await;
        store.aof_flush().await;
        drop(store);
    }
    let len_before = std::fs::metadata(&temp.path).expect("meta").len();

    let store2 = Store::new();
    let count = store2.replay_from_path(&temp.path).await.expect("replay");
    assert_eq!(count, 1);

    let len_after = std::fs::metadata(&temp.path).expect("meta").len();
    assert_eq!(
        len_before, len_after,
        "replay must not re-extend file (was {len_before}, now {len_after})"
    );
}

#[tokio::test]
async fn replay_truncated_tail_logs_and_returns_valid_count() {
    // Valid SET frame + 4 garbage bytes that look like the start of
    // another frame but are incomplete.
    let temp = TempAof::new("truncated");
    let mut bytes: Vec<u8> = b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n".to_vec();
    bytes.extend_from_slice(b"*3\r\n"); // truncated next frame
    std::fs::write(&temp.path, &bytes).expect("write");

    let store = Store::new();
    let count = store.replay_from_path(&temp.path).await.expect("replay");
    assert_eq!(count, 1, "valid prefix replayed");

    // State should reflect the one valid command.
    let r = store
        .execute(Command::Get {
            key: "k".to_owned(),
        })
        .await
        .expect("get");
    assert_eq!(r, Reply::Bulk(Some(b"v".to_vec())));
}

#[tokio::test]
async fn replay_invalid_byte_mid_stream_warns_and_stops() {
    // Valid frame followed by an unknown RESP type byte
    // (Frame::parse → Invalid).
    let temp = TempAof::new("invalid");
    let mut bytes: Vec<u8> = b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n".to_vec();
    bytes.extend_from_slice(b"garbage\r\n");
    std::fs::write(&temp.path, &bytes).expect("write");

    let store = Store::new();
    let count = store.replay_from_path(&temp.path).await.expect("replay");
    assert_eq!(count, 1, "one valid command before garbage");

    let r = store
        .execute(Command::Get {
            key: "k".to_owned(),
        })
        .await
        .expect("get");
    assert_eq!(r, Reply::Bulk(Some(b"v".to_vec())));
}

#[tokio::test]
async fn replay_skips_non_writable_frames_without_failing() {
    // Hand-build an AOF with a SET, a GET (which replay skips), and
    // another SET.  Replay count should be 2 (the two SETs).  GET
    // mid-replay must not crash.
    let temp = TempAof::new("mixed");
    let bytes: Vec<u8> = [
        &b"*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n"[..],
        &b"*2\r\n$3\r\nGET\r\n$1\r\na\r\n"[..],
        &b"*3\r\n$3\r\nSET\r\n$1\r\nb\r\n$1\r\n2\r\n"[..],
    ]
    .concat();
    std::fs::write(&temp.path, &bytes).expect("write");

    let store = Store::new();
    let count = store.replay_from_path(&temp.path).await.expect("replay");
    assert_eq!(count, 2, "only SETs count");
    let a = store
        .execute(Command::Get {
            key: "a".to_owned(),
        })
        .await
        .expect("get a");
    assert_eq!(a, Reply::Bulk(Some(b"1".to_vec())));
    let b = store
        .execute(Command::Get {
            key: "b".to_owned(),
        })
        .await
        .expect("get b");
    assert_eq!(b, Reply::Bulk(Some(b"2".to_vec())));
}

// ── TTL across restart ──────────────────────────────────────────────────────

#[tokio::test]
async fn ttl_short_lived_key_expires_after_replay() {
    // SET k v EX 1; kill (drop); wait > 1s; replay → key should be
    // gone OR fire shortly after replay.
    let temp = TempAof::new("ttl-short");

    {
        let store = Store::with_aof(temp.path.clone(), FsyncPolicy::Always)
            .await
            .expect("with_aof");
        let _ = store
            .execute(Command::Set {
                key: "k".to_owned(),
                value: b"v".to_vec(),
                ttl_secs: Some(1),
            })
            .await;
        store.aof_flush().await;
        drop(store);
    }

    // Wait past 1 sec so any newly-replayed EXPIRE expires quickly.
    tokio::time::sleep(Duration::from_millis(1200)).await;

    let store2 = Store::new();
    let count = store2.replay_from_path(&temp.path).await.expect("replay");
    assert_eq!(count, 1);

    // Allow the DelayQueue to fire (TTL relative, replay treats it
    // as "1 sec from now").  Sleep ~1.2 s.
    tokio::time::sleep(Duration::from_millis(1200)).await;

    let r = store2
        .execute(Command::Get {
            key: "k".to_owned(),
        })
        .await
        .expect("get");
    assert_eq!(
        r,
        Reply::Bulk(None),
        "short-lived key must be gone after replay + 1 sec"
    );
}

#[tokio::test]
async fn long_ttl_survives_restart_with_drift_under_one_sec() {
    let temp = TempAof::new("ttl-long");

    {
        let store = Store::with_aof(temp.path.clone(), FsyncPolicy::Always)
            .await
            .expect("with_aof");
        let _ = store
            .execute(Command::Set {
                key: "k".to_owned(),
                value: b"v".to_vec(),
                ttl_secs: Some(100),
            })
            .await;
        store.aof_flush().await;
        drop(store);
    }

    let store2 = Store::new();
    let _ = store2.replay_from_path(&temp.path).await.expect("replay");

    // Value present
    let v = store2
        .execute(Command::Get {
            key: "k".to_owned(),
        })
        .await
        .expect("get");
    assert_eq!(v, Reply::Bulk(Some(b"v".to_vec())));

    // TTL ≈ 100, with up to 1 sec drift downward.
    let ttl_reply = store2
        .execute(Command::Ttl {
            key: "k".to_owned(),
        })
        .await
        .expect("ttl");
    let Reply::Integer(n) = ttl_reply else {
        panic!("expected Reply::Integer, got {ttl_reply:?}");
    };
    assert!(
        (99..=100).contains(&n),
        "expected TTL ~100 (drift < 1s), got {n}"
    );
}

#[tokio::test]
async fn restart_round_trip_set_set_ex_del() {
    // ADR-0010 §"Restart round-trip":
    // Server-A: SET k1 v1; SET k2 v2 EX 100; DEL k1 → kill
    // Server-B: GET k1 = nil, GET k2 = v2, TTL k2 ≈ 100.
    let temp = TempAof::new("round-trip-3");

    {
        let store = Store::with_aof(temp.path.clone(), FsyncPolicy::Always)
            .await
            .expect("with_aof");
        let _ = store
            .execute(Command::Set {
                key: "k1".to_owned(),
                value: b"v1".to_vec(),
                ttl_secs: None,
            })
            .await;
        let _ = store
            .execute(Command::Set {
                key: "k2".to_owned(),
                value: b"v2".to_vec(),
                ttl_secs: Some(100),
            })
            .await;
        let _ = store
            .execute(Command::Del {
                keys: vec!["k1".to_owned()],
            })
            .await;
        store.aof_flush().await;
        drop(store);
    }

    let store2 = Store::new();
    let count = store2.replay_from_path(&temp.path).await.expect("replay");
    assert_eq!(count, 3);

    assert_eq!(
        store2
            .execute(Command::Get {
                key: "k1".to_owned()
            })
            .await
            .expect("get k1"),
        Reply::Bulk(None),
        "k1 must be gone after DEL"
    );
    assert_eq!(
        store2
            .execute(Command::Get {
                key: "k2".to_owned()
            })
            .await
            .expect("get k2"),
        Reply::Bulk(Some(b"v2".to_vec())),
        "k2 must survive"
    );
    let ttl = store2
        .execute(Command::Ttl {
            key: "k2".to_owned(),
        })
        .await
        .expect("ttl k2");
    let Reply::Integer(n) = ttl else {
        panic!("ttl reply: {ttl:?}");
    };
    assert!((99..=100).contains(&n), "k2 ttl ~100, got {n}");
}

// ── M4.1 (ADR-0011 §#4) — AOF file mode 0o600 (Unix only) ───────────────────

#[cfg(unix)]
#[tokio::test]
async fn aof_file_is_mode_600_on_unix() {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = TempAof::new("mode-0600");
    let _store = Store::with_aof(temp.path.clone(), FsyncPolicy::Everysec)
        .await
        .expect("with_aof open");

    let perms = std::fs::metadata(&temp.path)
        .expect("metadata")
        .permissions();
    let mode = perms.mode() & 0o777;
    assert_eq!(
        mode, 0o600,
        "AOF file must be mode 0o600 on Unix, got {mode:o}"
    );
}

// ── M4.1 (ADR-0011 §#7) — `AlwaysBlocking` is durable-before-return ─────────

#[tokio::test]
async fn always_blocking_writes_visible_on_disk_before_return() {
    // After `execute(...).await` returns under `AlwaysBlocking`, the
    // bytes MUST already be `sync_data`-ed.  We assert this by
    // reading the file IMMEDIATELY after execute returns — without
    // any explicit `aof_flush().await`.  Under `Always` (async)
    // there would be a race; under `AlwaysBlocking` there cannot.
    let temp = TempAof::new("alwaysblocking-durable");
    let store = Store::with_aof(temp.path.clone(), FsyncPolicy::AlwaysBlocking)
        .await
        .expect("with_aof alwaysblocking");

    let reply = store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .await
        .expect("set");
    assert_eq!(reply, Reply::Ok);

    // No flush — read the file straight away.  Bytes must be present.
    let bytes = std::fs::read(&temp.path).expect("read aof");
    assert_eq!(
        bytes, b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n",
        "AlwaysBlocking must persist the SET before execute returns"
    );
}

#[tokio::test]
async fn always_blocking_round_trip_via_replay() {
    // Stronger end-to-end check: write N commands under
    // AlwaysBlocking, drop the store (which closes the file), and
    // verify every command is in the replayed state.  This is the
    // "kill -9 + restart" simulation from ADR-0011 §Done Criteria.
    let temp = TempAof::new("alwaysblocking-replay");

    {
        let store = Store::with_aof(temp.path.clone(), FsyncPolicy::AlwaysBlocking)
            .await
            .expect("with_aof");
        for i in 0..5 {
            let key = format!("k{i}");
            let value = format!("v{i}");
            let _ = store
                .execute(Command::Set {
                    key,
                    value: value.into_bytes(),
                    ttl_secs: None,
                })
                .await
                .expect("set");
        }
        // Intentionally NO aof_flush — AlwaysBlocking gives us the
        // durability guarantee already.
        drop(store);
    }

    let store2 = Store::new();
    let count = store2.replay_from_path(&temp.path).await.expect("replay");
    assert_eq!(count, 5, "all 5 AlwaysBlocking commands must replay");

    for i in 0..5 {
        let key = format!("k{i}");
        let want = format!("v{i}");
        let r = store2
            .execute(Command::Get { key: key.clone() })
            .await
            .expect("get");
        assert_eq!(r, Reply::Bulk(Some(want.into_bytes())));
    }
}

// ── M4.1 (ADR-0011 §#9) — streaming replay handles many small frames ──────

#[tokio::test]
async fn streaming_replay_handles_thousand_small_frames() {
    // Write 1024 SET commands into a single AOF and replay them all.
    // This exercises the streaming reader's chunk-refill loop with
    // mixed frame boundaries inside `READ_CHUNK` (64 KiB).
    let temp = TempAof::new("streaming-1024");

    {
        let store = Store::with_aof(temp.path.clone(), FsyncPolicy::Always)
            .await
            .expect("with_aof");
        for i in 0..1024 {
            let _ = store
                .execute(Command::Set {
                    key: format!("k{i}"),
                    value: format!("v{i}").into_bytes(),
                    ttl_secs: None,
                })
                .await
                .expect("set");
        }
        store.aof_flush().await;
        drop(store);
    }

    let store2 = Store::new();
    let count = store2.replay_from_path(&temp.path).await.expect("replay");
    assert_eq!(count, 1024);
    // Spot check.
    let r = store2
        .execute(Command::Get {
            key: "k512".to_owned(),
        })
        .await
        .expect("get");
    assert_eq!(r, Reply::Bulk(Some(b"v512".to_vec())));
}
