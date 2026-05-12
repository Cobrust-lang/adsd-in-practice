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

// ── ADR-0006 (M1.4) — PING optional message ────────────────────────────────

#[tokio::test(start_paused = true)]
async fn ping_no_message_returns_pong() {
    let s = Store::new();
    let r = s
        .execute(Command::Ping { message: None })
        .expect("ping infallible");
    assert_eq!(r, Reply::Pong);
}

#[tokio::test(start_paused = true)]
async fn ping_with_message_returns_bulk() {
    let s = Store::new();
    let r = s
        .execute(Command::Ping {
            message: Some(b"hello".to_vec()),
        })
        .expect("ping infallible");
    assert_eq!(r, Reply::Bulk(Some(b"hello".to_vec())));
}

// ── ADR-0006 (M1.4) — EXPIRE / TTL / PERSIST ───────────────────────────────

#[tokio::test(start_paused = true)]
async fn expire_existing_key_returns_one() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .expect("set");
    let r = store
        .execute(Command::Expire {
            key: "k".to_owned(),
            seconds: 60,
        })
        .expect("expire");
    assert_eq!(r, Reply::Integer(1));
}

#[tokio::test(start_paused = true)]
async fn expire_missing_key_returns_zero() {
    let store = Store::new();
    let r = store
        .execute(Command::Expire {
            key: "nope".to_owned(),
            seconds: 60,
        })
        .expect("expire");
    assert_eq!(r, Reply::Integer(0));
}

#[tokio::test(start_paused = true)]
async fn ttl_missing_key_is_minus_two() {
    let store = Store::new();
    let r = store
        .execute(Command::Ttl {
            key: "nope".to_owned(),
        })
        .expect("ttl");
    assert_eq!(r, Reply::Integer(-2));
}

#[tokio::test(start_paused = true)]
async fn ttl_no_ttl_key_is_minus_one() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .expect("set");
    let r = store
        .execute(Command::Ttl {
            key: "k".to_owned(),
        })
        .expect("ttl");
    assert_eq!(r, Reply::Integer(-1));
}

#[tokio::test(start_paused = true)]
async fn ttl_with_ttl_key_is_remaining_seconds() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: Some(100),
        })
        .expect("set");
    let r = store
        .execute(Command::Ttl {
            key: "k".to_owned(),
        })
        .expect("ttl");
    // Allow ±1s drift per ADR-0006 done criteria (paused-time so we expect 100).
    match r {
        Reply::Integer(n) => assert!(
            (99..=100).contains(&n),
            "expected remaining secs near 100, got {n}"
        ),
        other => panic!("expected Integer, got {other:?}"),
    }
}

#[tokio::test(start_paused = true)]
async fn persist_existing_ttl_returns_one_and_clears_ttl() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: Some(100),
        })
        .expect("set");
    let r = store
        .execute(Command::Persist {
            key: "k".to_owned(),
        })
        .expect("persist");
    assert_eq!(r, Reply::Integer(1));
    // TTL after PERSIST is -1.
    let after = store
        .execute(Command::Ttl {
            key: "k".to_owned(),
        })
        .expect("ttl");
    assert_eq!(after, Reply::Integer(-1));
}

#[tokio::test(start_paused = true)]
async fn persist_no_ttl_returns_zero() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .expect("set");
    let r = store
        .execute(Command::Persist {
            key: "k".to_owned(),
        })
        .expect("persist");
    assert_eq!(r, Reply::Integer(0));
}

#[tokio::test(start_paused = true)]
async fn persist_missing_key_returns_zero() {
    let store = Store::new();
    let r = store
        .execute(Command::Persist {
            key: "nope".to_owned(),
        })
        .expect("persist");
    assert_eq!(r, Reply::Integer(0));
}

#[tokio::test(start_paused = true)]
async fn expire_then_advance_makes_key_disappear() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .expect("set");
    store
        .execute(Command::Expire {
            key: "k".to_owned(),
            seconds: 1,
        })
        .expect("expire");
    tokio::time::advance(Duration::from_millis(1100)).await;
    tokio::task::yield_now().await;
    let r = store
        .execute(Command::Get {
            key: "k".to_owned(),
        })
        .expect("get");
    assert_eq!(r, Reply::Bulk(None));
}

#[tokio::test(start_paused = true)]
async fn expire_repeated_last_wins() {
    // EXPIRE multiple times — last setting wins; old DelayQueue entries
    // must not delete the key prematurely (ADR-0006 stale-fire skip).
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .expect("set");
    store
        .execute(Command::Expire {
            key: "k".to_owned(),
            seconds: 1,
        })
        .expect("expire 1");
    // Re-EXPIRE longer.
    store
        .execute(Command::Expire {
            key: "k".to_owned(),
            seconds: 10,
        })
        .expect("expire 10");
    // Advance past the original 1s TTL — stale DelayQueue entry would
    // fire here.  Key must SURVIVE (expires_at now points to the new
    // longer deadline; guard in `Store::new` should skip the stale fire).
    tokio::time::advance(Duration::from_millis(1500)).await;
    tokio::task::yield_now().await;
    let r = store
        .execute(Command::Get {
            key: "k".to_owned(),
        })
        .expect("get");
    assert_eq!(
        r,
        Reply::Bulk(Some(b"v".to_vec())),
        "key must survive stale expiry fire"
    );
}

#[tokio::test(start_paused = true)]
async fn persist_then_stale_fire_does_not_delete_key() {
    // PERSIST regression: SET k v EX 1; PERSIST k; advance > 1s; key
    // must still be present (old DelayQueue entry sees expires_at=None
    // and skips removal).
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: Some(1),
        })
        .expect("set ttl");
    store
        .execute(Command::Persist {
            key: "k".to_owned(),
        })
        .expect("persist");
    tokio::time::advance(Duration::from_millis(1500)).await;
    tokio::task::yield_now().await;
    let r = store
        .execute(Command::Get {
            key: "k".to_owned(),
        })
        .expect("get");
    assert_eq!(
        r,
        Reply::Bulk(Some(b"v".to_vec())),
        "PERSIST must defuse the stale DelayQueue fire"
    );
}

#[tokio::test(start_paused = true)]
async fn expire_zero_or_negative_deletes_key() {
    // Real Redis: EXPIRE k 0 (or negative) deletes the key.  Our impl
    // mirrors that.
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .expect("set");
    let r = store
        .execute(Command::Expire {
            key: "k".to_owned(),
            seconds: 0,
        })
        .expect("expire 0");
    assert_eq!(r, Reply::Integer(1));
    let g = store
        .execute(Command::Get {
            key: "k".to_owned(),
        })
        .expect("get");
    assert_eq!(g, Reply::Bulk(None));
}

// ── ADR-0006 (M1.4) — TYPE ─────────────────────────────────────────────────

#[tokio::test(start_paused = true)]
async fn type_existing_string_returns_string() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .expect("set");
    let r = store
        .execute(Command::Type {
            key: "k".to_owned(),
        })
        .expect("type");
    assert_eq!(r, Reply::SimpleString("string".to_owned()));
}

#[tokio::test(start_paused = true)]
async fn type_missing_returns_none() {
    let store = Store::new();
    let r = store
        .execute(Command::Type {
            key: "nope".to_owned(),
        })
        .expect("type");
    assert_eq!(r, Reply::SimpleString("none".to_owned()));
}

#[tokio::test(start_paused = true)]
async fn type_expired_key_returns_none() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "k".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: Some(1),
        })
        .expect("set");
    tokio::time::advance(Duration::from_millis(1100)).await;
    // Do NOT yield — we want to check the "logically expired but not yet
    // swept" path (entry still in map, expires_at in past).
    let r = store
        .execute(Command::Type {
            key: "k".to_owned(),
        })
        .expect("type");
    assert_eq!(r, Reply::SimpleString("none".to_owned()));
}

// ── ADR-0006 (M1.4) — KEYS ─────────────────────────────────────────────────

#[tokio::test(start_paused = true)]
async fn keys_star_returns_all_live_keys() {
    let store = Store::new();
    for k in ["a", "b", "c"] {
        store
            .execute(Command::Set {
                key: k.to_owned(),
                value: b"v".to_vec(),
                ttl_secs: None,
            })
            .expect("set");
    }
    let r = store
        .execute(Command::Keys {
            pattern: "*".to_owned(),
        })
        .expect("keys");
    let Reply::Array(Some(mut keys)) = r else {
        panic!("expected Array(Some(_)), got {r:?}");
    };
    keys.sort();
    assert_eq!(keys, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
}

#[tokio::test(start_paused = true)]
async fn keys_prefix_pattern() {
    let store = Store::new();
    for k in ["user:1", "user:2", "post:1"] {
        store
            .execute(Command::Set {
                key: k.to_owned(),
                value: b"v".to_vec(),
                ttl_secs: None,
            })
            .expect("set");
    }
    let r = store
        .execute(Command::Keys {
            pattern: "user:*".to_owned(),
        })
        .expect("keys");
    let Reply::Array(Some(mut keys)) = r else {
        panic!("expected Array(Some(_)), got {r:?}");
    };
    keys.sort();
    assert_eq!(keys, vec![b"user:1".to_vec(), b"user:2".to_vec()]);
}

#[tokio::test(start_paused = true)]
async fn keys_question_pattern_single_char() {
    let store = Store::new();
    for k in ["user:1", "user:2", "user:42"] {
        store
            .execute(Command::Set {
                key: k.to_owned(),
                value: b"v".to_vec(),
                ttl_secs: None,
            })
            .expect("set");
    }
    let r = store
        .execute(Command::Keys {
            pattern: "user:?".to_owned(),
        })
        .expect("keys");
    let Reply::Array(Some(mut keys)) = r else {
        panic!("expected Array(Some(_)), got {r:?}");
    };
    keys.sort();
    assert_eq!(keys, vec![b"user:1".to_vec(), b"user:2".to_vec()]);
}

#[tokio::test(start_paused = true)]
async fn keys_class_pattern() {
    let store = Store::new();
    for k in ["apple", "banana", "cherry", "date"] {
        store
            .execute(Command::Set {
                key: k.to_owned(),
                value: b"v".to_vec(),
                ttl_secs: None,
            })
            .expect("set");
    }
    let r = store
        .execute(Command::Keys {
            pattern: "[abc]*".to_owned(),
        })
        .expect("keys");
    let Reply::Array(Some(mut keys)) = r else {
        panic!("expected Array(Some(_)), got {r:?}");
    };
    keys.sort();
    assert_eq!(
        keys,
        vec![b"apple".to_vec(), b"banana".to_vec(), b"cherry".to_vec()]
    );
}

#[tokio::test(start_paused = true)]
async fn keys_escaped_star_is_literal() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "*".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .expect("set literal *");
    store
        .execute(Command::Set {
            key: "x".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .expect("set x");
    let r = store
        .execute(Command::Keys {
            pattern: "\\*".to_owned(),
        })
        .expect("keys");
    assert_eq!(r, Reply::Array(Some(vec![b"*".to_vec()])));
}

#[tokio::test(start_paused = true)]
async fn keys_empty_db_returns_empty_array() {
    let store = Store::new();
    let r = store
        .execute(Command::Keys {
            pattern: "*".to_owned(),
        })
        .expect("keys");
    assert_eq!(r, Reply::Array(Some(vec![])));
}

#[tokio::test(start_paused = true)]
async fn keys_skips_expired() {
    let store = Store::new();
    store
        .execute(Command::Set {
            key: "live".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: None,
        })
        .expect("set live");
    store
        .execute(Command::Set {
            key: "dead".to_owned(),
            value: b"v".to_vec(),
            ttl_secs: Some(1),
        })
        .expect("set dead");
    tokio::time::advance(Duration::from_millis(1100)).await;
    // Do NOT yield to background — we want the "logically expired but
    // not yet swept" path.
    let r = store
        .execute(Command::Keys {
            pattern: "*".to_owned(),
        })
        .expect("keys");
    assert_eq!(r, Reply::Array(Some(vec![b"live".to_vec()])));
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
