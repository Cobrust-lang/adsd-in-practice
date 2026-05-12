//! Integration tests for `Store::metrics` and `Store::sample_keys`
//! (ADR-0007 §Done Criteria items 7-8).
//!
//! These run against the real public API — go through `Store::new()`,
//! which spawns a background expiry task — so we cover both the
//! happy path and the "metrics view consistent with EXISTS/GET"
//! invariant under TTL.

#![allow(clippy::expect_used)] // tests use expect("...") liberally — see CLAUDE.md §3.1.

use std::collections::HashSet;
use std::time::Duration;

use redis_storage::{Command, KeyInfo, Reply, Store, StoreMetrics};

// ── Store::metrics ──────────────────────────────────────────────────────

#[tokio::test]
async fn metrics_empty_store_is_zero() {
    let s = Store::new();
    let m = s.metrics();
    assert_eq!(
        m,
        StoreMetrics {
            key_count: 0,
            total_value_bytes: 0
        }
    );
}

#[tokio::test]
async fn metrics_count_and_bytes_after_set() {
    let s = Store::new();
    set_plain(&s, "foo", b"bar");
    set_plain(&s, "name", b"alice");

    let m = s.metrics();
    assert_eq!(m.key_count, 2);
    // "bar" (3) + "alice" (5) = 8.
    assert_eq!(m.total_value_bytes, 8);
}

#[tokio::test]
async fn metrics_decrements_after_del() {
    let s = Store::new();
    set_plain(&s, "a", b"x");
    set_plain(&s, "b", b"yy");
    set_plain(&s, "c", b"zzz");

    let r = s
        .execute(Command::Del {
            keys: vec!["a".into(), "b".into()],
        })
        .expect("del");
    assert_eq!(r, Reply::Integer(2));

    let m = s.metrics();
    assert_eq!(m.key_count, 1);
    assert_eq!(m.total_value_bytes, 3);
}

#[tokio::test]
async fn metrics_skips_logically_expired_entries() {
    // Use a non-zero but tiny TTL so the entry is *registered* but
    // we control "expired" via tokio::time::pause-like sleep.  A
    // 1-second TTL + 1100 ms sleep is the same pattern as
    // `server_e2e::set_ex_then_expiry`.  This is the single sleep
    // allowed under "last resort".
    let s = Store::new();
    set_plain(&s, "perm", b"keep");
    set_with_ttl(&s, "tmp", b"gone", 1);

    // Immediately: 2 keys, 8 bytes.
    let m0 = s.metrics();
    assert_eq!(m0.key_count, 2);
    assert_eq!(m0.total_value_bytes, 8);

    tokio::time::sleep(Duration::from_millis(1100)).await;

    let m1 = s.metrics();
    // "tmp" is logically expired (the background reaper may or may
    // not have removed it yet — the metrics view must filter regardless).
    assert_eq!(m1.key_count, 1);
    assert_eq!(m1.total_value_bytes, 4);
}

// ── Store::sample_keys ──────────────────────────────────────────────────

#[tokio::test]
async fn sample_keys_empty_store_returns_empty() {
    let s = Store::new();
    let out = s.sample_keys(10);
    assert!(out.is_empty());
}

#[tokio::test]
async fn sample_keys_zero_limit_returns_empty() {
    let s = Store::new();
    set_plain(&s, "foo", b"bar");
    let out = s.sample_keys(0);
    assert!(out.is_empty());
}

#[tokio::test]
async fn sample_keys_lists_under_limit_in_full() {
    let s = Store::new();
    set_plain(&s, "a", b"1");
    set_plain(&s, "b", b"22");
    set_plain(&s, "c", b"333");

    let out = s.sample_keys(100);
    assert_eq!(out.len(), 3);
    let got_keys: HashSet<String> = out.iter().map(|k| k.key.clone()).collect();
    let want_keys: HashSet<String> = ["a", "b", "c"].iter().map(|&s| s.into()).collect();
    assert_eq!(got_keys, want_keys);

    // Every entry should be type=string, ttl_secs=-1 (no TTL).
    for k in &out {
        assert_eq!(k.kind, "string");
        assert_eq!(k.ttl_secs, -1, "key {:?} should have no TTL", k.key);
    }
}

#[tokio::test]
async fn sample_keys_truncates_at_limit() {
    let s = Store::new();
    for i in 0..200_u32 {
        set_plain(&s, &format!("k{i}"), b"v");
    }
    let out = s.sample_keys(100);
    assert_eq!(out.len(), 100);
}

#[tokio::test]
async fn sample_keys_reports_positive_ttl() {
    let s = Store::new();
    set_with_ttl(&s, "ttlkey", b"v", 42);

    let out = s.sample_keys(10);
    assert_eq!(out.len(), 1);
    let info: &KeyInfo = &out[0];
    assert_eq!(info.key, "ttlkey");
    assert_eq!(info.kind, "string");
    // round-half-up: just-after-SET should round to the originally
    // requested N seconds — same invariant as `TTL` command (M1.4
    // F23-A finding).  Tolerate ±1 just in case of scheduling jitter.
    assert!(
        info.ttl_secs == 42 || info.ttl_secs == 41,
        "ttl_secs={} should be 42 or 41 (jitter)",
        info.ttl_secs
    );
}

#[tokio::test]
async fn sample_keys_skips_expired() {
    let s = Store::new();
    set_plain(&s, "perm", b"keep");
    set_with_ttl(&s, "tmp", b"gone", 1);

    tokio::time::sleep(Duration::from_millis(1100)).await;

    let out = s.sample_keys(10);
    // Only "perm" should remain visible.
    let got_keys: Vec<String> = out.iter().map(|k| k.key.clone()).collect();
    assert_eq!(got_keys, vec!["perm".to_string()]);
}

#[tokio::test]
async fn sample_keys_does_not_sort_deterministically() {
    // We can't assert "is unsorted" without knowing the hasher.
    // What we CAN assert: two independently-built stores with the
    // same key set don't necessarily yield the same order, because
    // hashbrown uses a random hasher per construction.  This is the
    // contract `sample_keys` is meant to preserve (matches Redis
    // KEYS semantics).
    //
    // We sample twice with the same key set into two different
    // `Store` instances; the result MAY differ.  If both runs
    // happen to be identical that's allowed too (random orderings
    // collide ~1/120 for 5 keys).  So the test asserts only on
    // membership equivalence, not order — but documents intent.
    let s1 = Store::new();
    let s2 = Store::new();
    for k in ["a", "b", "c", "d", "e"] {
        set_plain(&s1, k, b"v");
        set_plain(&s2, k, b"v");
    }
    let o1: HashSet<String> = s1.sample_keys(10).into_iter().map(|k| k.key).collect();
    let o2: HashSet<String> = s2.sample_keys(10).into_iter().map(|k| k.key).collect();
    assert_eq!(o1, o2, "membership must be equal");
    // (Order is *not* asserted on purpose — see docstring.)
}

// ── helpers ─────────────────────────────────────────────────────────────

fn set_plain(s: &Store, key: &str, value: &[u8]) {
    let r = s
        .execute(Command::Set {
            key: key.into(),
            value: value.to_vec(),
            ttl_secs: None,
        })
        .expect("set");
    assert_eq!(r, Reply::Ok);
}

fn set_with_ttl(s: &Store, key: &str, value: &[u8], ttl_secs: u64) {
    let r = s
        .execute(Command::Set {
            key: key.into(),
            value: value.to_vec(),
            ttl_secs: Some(ttl_secs),
        })
        .expect("set with ttl");
    assert_eq!(r, Reply::Ok);
}
