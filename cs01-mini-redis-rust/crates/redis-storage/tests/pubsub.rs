//! Integration tests for the Pub/Sub side of `redis-storage::Store`
//! (ADR-0009 M3.1).
//!
//! Covers the `Store::subscribe` / `Store::pubsub_snapshot` /
//! `Command::Publish` paths *below* the server crate — the per-conn
//! state machine itself is tested in `crates/redis-server/tests/`.
//!
//! No real sleeps: every test pairs subscribe/publish in the same
//! tokio task with a `tokio::time::timeout` bound for the recv side.

#![allow(clippy::expect_used)]

use std::time::Duration;

use redis_storage::{Command, Reply, Store};

// ── subscribe / pubsub_snapshot ──────────────────────────────────────────────

#[tokio::test]
async fn pubsub_snapshot_empty_on_fresh_store() {
    let store = Store::new();
    assert_eq!(store.pubsub_snapshot(), Vec::<(String, usize)>::new());
}

#[tokio::test]
async fn subscribe_creates_channel_and_increments_count() {
    let store = Store::new();
    let _rx = store.subscribe("news");
    let snap = store.pubsub_snapshot();
    assert_eq!(snap, vec![("news".to_owned(), 1)]);
}

#[tokio::test]
async fn two_subscribers_same_channel_share_one_entry() {
    let store = Store::new();
    let _rx1 = store.subscribe("news");
    let _rx2 = store.subscribe("news");
    let snap = store.pubsub_snapshot();
    assert_eq!(snap, vec![("news".to_owned(), 2)]);
}

#[tokio::test]
async fn pubsub_snapshot_is_sorted_by_channel_name() {
    let store = Store::new();
    let _rx_z = store.subscribe("zeta");
    let _rx_a = store.subscribe("alpha");
    let _rx_m = store.subscribe("mu");
    let snap = store.pubsub_snapshot();
    let names: Vec<&str> = snap.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["alpha", "mu", "zeta"]);
}

#[tokio::test]
async fn dropping_receiver_decrements_count_but_keeps_entry() {
    // ADR-0009 §Decision Q1+Q2: M3.1 intentionally does NOT GC empty
    // channels; we only check that the count drops to 0 and the
    // channel still exists.
    let store = Store::new();
    let rx = store.subscribe("ephemeral");
    drop(rx);
    let snap = store.pubsub_snapshot();
    assert_eq!(snap, vec![("ephemeral".to_owned(), 0)]);
}

// ── publish ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn publish_with_no_subscribers_returns_zero() {
    let store = Store::new();
    let reply = store
        .execute(Command::Publish {
            channel: "nobody-home".to_owned(),
            message: b"hi".to_vec(),
        })
        .expect("publish infallible");
    assert_eq!(reply, Reply::Integer(0));
}

#[tokio::test]
async fn publish_to_a_subscribed_channel_returns_one_and_delivers_payload() {
    let store = Store::new();
    let mut rx = store.subscribe("news");

    let reply = store
        .execute(Command::Publish {
            channel: "news".to_owned(),
            message: b"hello".to_vec(),
        })
        .expect("publish");
    assert_eq!(reply, Reply::Integer(1));

    let received = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .expect("recv timed out")
        .expect("recv ok");
    assert_eq!(&*received, b"hello");
}

#[tokio::test]
async fn publish_to_three_subscribers_returns_three_and_delivers_to_all() {
    let store = Store::new();
    let mut rx_a = store.subscribe("chat");
    let mut rx_b = store.subscribe("chat");
    let mut rx_c = store.subscribe("chat");

    let reply = store
        .execute(Command::Publish {
            channel: "chat".to_owned(),
            message: b"go".to_vec(),
        })
        .expect("publish");
    assert_eq!(reply, Reply::Integer(3));

    for rx in [&mut rx_a, &mut rx_b, &mut rx_c] {
        let got = tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("timeout")
            .expect("ok");
        assert_eq!(&*got, b"go");
    }
}

#[tokio::test]
async fn publish_binary_payload_preserved() {
    let store = Store::new();
    let mut rx = store.subscribe("bin");
    let payload = vec![0u8, 1, 2, 3, 0xff, 0xfe, b'\r', b'\n'];

    let reply = store
        .execute(Command::Publish {
            channel: "bin".to_owned(),
            message: payload.clone(),
        })
        .expect("publish");
    assert_eq!(reply, Reply::Integer(1));

    let got = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .expect("timeout")
        .expect("ok");
    assert_eq!(&*got, &payload[..]);
}

#[tokio::test]
async fn publish_payload_is_arc_shared_across_subscribers() {
    // Two subscribers should receive the same `Arc<Vec<u8>>` pointer
    // (zero-copy fan-out, ADR-0009 §"不允许在热路径里 allocate").
    let store = Store::new();
    let mut rx_a = store.subscribe("zerocopy");
    let mut rx_b = store.subscribe("zerocopy");

    let _ = store
        .execute(Command::Publish {
            channel: "zerocopy".to_owned(),
            message: b"X".to_vec(),
        })
        .expect("publish");

    let a = tokio::time::timeout(Duration::from_millis(200), rx_a.recv())
        .await
        .expect("timeout")
        .expect("ok");
    let b = tokio::time::timeout(Duration::from_millis(200), rx_b.recv())
        .await
        .expect("timeout")
        .expect("ok");
    assert!(std::sync::Arc::ptr_eq(&a, &b), "Arc payload must be shared");
}

// ── command routing ─────────────────────────────────────────────────────────

#[tokio::test]
async fn subscribe_unsubscribe_via_execute_return_error_reply() {
    // ADR-0009 §Q4: SUBSCRIBE / UNSUBSCRIBE must NOT be routed through
    // Store::execute (they need per-conn state + N replies).  The arms
    // exist for parse uniformity but reaching execute is a bug; the
    // store returns a generic error rather than panicking.
    let store = Store::new();
    let r1 = store
        .execute(Command::Subscribe {
            channels: vec!["c".to_owned()],
        })
        .expect("execute returns Ok wrapping a Reply::Error");
    assert!(matches!(r1, Reply::Error(_)));

    let r2 = store
        .execute(Command::Unsubscribe {
            channels: vec!["c".to_owned()],
        })
        .expect("execute returns Ok wrapping a Reply::Error");
    assert!(matches!(r2, Reply::Error(_)));
}
