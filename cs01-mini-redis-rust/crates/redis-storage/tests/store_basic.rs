//! Integration tests for `redis-storage::Store`.
//!
//! Covers ADR-0003 §"Done Criteria" — all 7 items.
//! Time control: `#[tokio::test(start_paused = true)]` + `tokio::time::advance()`
//! so TTL tests never sleep.

use redis_storage::{Command, Reply, Store};
use tokio::time::Duration;

// ── ADR-0003 Criterion 1 ────────────────────────────────────────────────────
// SET writes and returns Reply::Ok.

#[tokio::test(start_paused = true)]
async fn set_returns_ok() {
    let store = Store::new();
    let reply = store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .expect("set infallible");
    assert_eq!(reply, Reply::Ok);
}

// ── ADR-0003 Criterion 2 ────────────────────────────────────────────────────
// GET returns Bulk(Some(value)) for existing key, Bulk(None) for missing.

#[tokio::test(start_paused = true)]
async fn get_existing_key() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "name".to_owned(),
            value: b"alice".to_vec(),
            ttl_secs: None,
        })
        .expect("set");
    let reply = store
        .execute(Command::Get {
            key: "name".to_owned(),
        })
        .expect("get");
    assert_eq!(reply, Reply::Bulk(Some(b"alice".to_vec())));
}

#[tokio::test(start_paused = true)]
async fn get_missing_key_returns_nil() {
    let store = Store::new();
    let reply = store
        .execute(Command::Get {
            key: "no-such-key".to_owned(),
        })
        .expect("get");
    assert_eq!(reply, Reply::Bulk(None));
}

// ── ADR-0003 Criterion 3 ────────────────────────────────────────────────────
// DEL returns Integer(count) = number of keys actually deleted.

#[tokio::test(start_paused = true)]
async fn del_returns_correct_count() {
    let store = Store::new();
    for k in ["a", "b"] {
        store
            .execute(Command::Set {
                key: k.to_owned(),
                value: b"x".to_vec(),
                ttl_secs: None,
            })
            .expect("set");
    }
    // Delete 2 existing + 1 non-existing → count = 2.
    let reply = store
        .execute(Command::Del {
            keys: vec!["a".to_owned(), "b".to_owned(), "nonexistent".to_owned()],
        })
        .expect("del");
    assert_eq!(reply, Reply::Integer(2));

    // Confirm both gone.
    for k in ["a", "b"] {
        let r = store
            .execute(Command::Get { key: k.to_owned() })
            .expect("get");
        assert_eq!(r, Reply::Bulk(None));
    }
}

// ── ADR-0003 Criterion 4 ────────────────────────────────────────────────────
// EXISTS returns Integer(count) matching live keys.

#[tokio::test(start_paused = true)]
async fn exists_counts_live_keys() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "live".to_owned(),
            value: b"1".to_vec(),
            ttl_secs: None,
        })
        .expect("set");

    let r = store
        .execute(Command::Exists {
            keys: vec!["live".to_owned(), "dead".to_owned()],
        })
        .expect("exists");
    assert_eq!(r, Reply::Integer(1));
}

// ── ADR-0003 Criterion 5 ────────────────────────────────────────────────────
// INCR on non-integer value returns Reply::Error (not StoreError).

#[tokio::test(start_paused = true)]
async fn incr_non_integer_value_returns_error() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "x".to_owned(),
            value: b"notanint".to_vec(),
            ttl_secs: None,
        })
        .expect("set");
    let reply = store
        .execute(Command::Incr {
            key: "x".to_owned(),
        })
        .expect("incr infallible at StoreError level");
    assert_eq!(
        reply,
        Reply::Error("ERR value is not an integer or out of range".to_owned())
    );
}

#[tokio::test(start_paused = true)]
async fn incr_missing_key_starts_at_one() {
    let store = Store::new();
    let reply = store
        .execute(Command::Incr {
            key: "counter".to_owned(),
        })
        .expect("incr");
    assert_eq!(reply, Reply::Integer(1));

    let reply2 = store
        .execute(Command::Incr {
            key: "counter".to_owned(),
        })
        .expect("incr");
    assert_eq!(reply2, Reply::Integer(2));
}

#[tokio::test(start_paused = true)]
async fn decr_existing_key() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "n".to_owned(),
            value: b"10".to_vec(),
            ttl_secs: None,
        })
        .expect("set");
    let reply = store
        .execute(Command::Decr {
            key: "n".to_owned(),
        })
        .expect("decr");
    assert_eq!(reply, Reply::Integer(9));
}

// ── ADR-0003 Criterion 6 ────────────────────────────────────────────────────
// TTL active expiration: SET k v EX 1; advance 1.1s; GET k → Nil.
// Uses tokio::time::pause + advance — no real sleep.

#[tokio::test(start_paused = true)]
async fn ttl_active_expiration() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "ttlkey".to_owned(),
            value: b"value".to_vec(),
            ttl_secs: Some(1),
        })
        .expect("set with ttl");

    // Key should be alive before expiry.
    let before = store
        .execute(Command::Get {
            key: "ttlkey".to_owned(),
        })
        .expect("get before expiry");
    assert_eq!(before, Reply::Bulk(Some(b"value".to_vec())));

    // Advance time past the TTL.
    tokio::time::advance(Duration::from_millis(1100)).await;

    // Give the background task a chance to run.
    tokio::task::yield_now().await;

    let after = store
        .execute(Command::Get {
            key: "ttlkey".to_owned(),
        })
        .expect("get after expiry");
    assert_eq!(after, Reply::Bulk(None), "key should be gone after TTL");
}

// ── ADR-0005 (M1.3) — ECHO / SELECT / QUIT ──────────────────────────────────

#[tokio::test(start_paused = true)]
async fn echo_returns_message_verbatim() {
    let store = Store::new();
    let reply = store
        .execute(Command::Echo {
            message: b"hello world".to_vec(),
        })
        .expect("echo infallible");
    assert_eq!(reply, Reply::Bulk(Some(b"hello world".to_vec())));
}

#[tokio::test(start_paused = true)]
async fn select_db_zero_is_ok() {
    let store = Store::new();
    let reply = store
        .execute(Command::Select { db: 0 })
        .expect("select infallible");
    assert_eq!(reply, Reply::Ok);
}

#[tokio::test(start_paused = true)]
async fn select_non_zero_db_is_error() {
    let store = Store::new();
    let reply = store
        .execute(Command::Select { db: 9 })
        .expect("select infallible at StoreError level");
    assert_eq!(
        reply,
        Reply::Error("ERR DB index is out of range".to_owned())
    );
}

#[tokio::test(start_paused = true)]
async fn quit_returns_ok() {
    let store = Store::new();
    let reply = store.execute(Command::Quit).expect("quit infallible");
    assert_eq!(reply, Reply::Ok);
}

// ── ADR-0003 Criterion 7 ────────────────────────────────────────────────────
// SET without TTL overwrites an expiring key; key survives past old TTL.

#[tokio::test(start_paused = true)]
async fn set_no_ttl_clears_expiry() {
    let store = Store::new();
    // First insert with short TTL.
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"old".to_vec(),
            ttl_secs: Some(1),
        })
        .expect("set with ttl");

    // Overwrite without TTL before expiry fires.
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"new".to_vec(),
            ttl_secs: None,
        })
        .expect("set without ttl");

    // Advance well past the original TTL.
    tokio::time::advance(Duration::from_secs(2)).await;
    tokio::task::yield_now().await;

    // The key should survive because the second SET had no TTL.
    let r = store
        .execute(Command::Get {
            key: "k".to_owned(),
        })
        .expect("get");
    // The background task may remove the key (old delay fires) but the entry's
    // expires_at is None after the second SET, so the guard in the task will not
    // remove it.  Verify the value is still present.
    assert_eq!(r, Reply::Bulk(Some(b"new".to_vec())));
}
