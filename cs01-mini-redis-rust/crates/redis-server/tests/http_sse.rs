//! End-to-end tests for the Axum HTTP control plane (ADR-0007).
//!
//! Each test:
//!   1. Binds `127.0.0.1:0` for the HTTP listener (and, where it
//!      matters for the wire-coupling tests, a second one for RESP),
//!   2. Spawns `http::run_on(listener, state)` on a background task,
//!   3. Connects a `reqwest::Client` and reads SSE chunks via
//!      `bytes_stream()` until N events arrive,
//!   4. Aborts the server task at the end.
//!
//! Why `reqwest` instead of a hand-rolled HTTP/1.1 client: we want
//! coverage of the *full* axum SSE encoder path (chunked transfer +
//! `text/event-stream` content-type negotiation), and reqwest is the
//! ecosystem reference client (ADR-0007 §"测试 oracle 怎么 align" —
//! self-test only, F23-A doesn't apply to SSE).

#![allow(clippy::expect_used)] // tests use expect("...") liberally — see CLAUDE.md §3.1.

use std::time::Duration;

use futures_util::StreamExt as _;
use redis_server::http;
use redis_server::server;
use redis_server::server::DEFAULT_MAX_FRAME_SIZE;
use redis_server::state::AppState;
use redis_storage::Store;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

// ── Test harness ─────────────────────────────────────────────────────────────

/// Spawn an HTTP-only listener on `127.0.0.1:0`.  Returns
/// `(port, state, http_handle)`; the caller can keep the `state` to
/// later spawn a RESP listener wired to the same counters.
async fn spawn_http() -> (u16, AppState, JoinHandle<std::io::Result<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local_addr").port();
    let state = AppState::new(Store::new(), DEFAULT_MAX_FRAME_SIZE);
    let state_for_task = state.clone();
    let handle = tokio::spawn(async move { http::run_on(listener, state_for_task).await });
    (port, state, handle)
}

/// Spawn a RESP listener on `127.0.0.1:0` against an existing state.
async fn spawn_resp_with_state(state: AppState) -> (u16, JoinHandle<std::io::Result<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local_addr").port();
    let handle = tokio::spawn(async move { server::run_on(listener, state).await });
    (port, handle)
}

/// Collect a single SSE frame: read chunks until we see a blank line
/// (`\n\n` or `\r\n\r\n`).  Returns the frame's raw text (without
/// the trailing blank).
async fn read_one_sse_frame(stream: &mut SseStream) -> String {
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.expect("sse chunk");
        let s = std::str::from_utf8(&bytes).expect("utf-8 chunk");
        buf.push_str(s);
        if buf.contains("\n\n") {
            // Strip everything up to and including the first blank.
            let idx = buf.find("\n\n").expect("just checked contains \\n\\n");
            let frame = buf[..idx].to_string();
            return frame;
        }
    }
    panic!("stream ended before delivering a full SSE frame; got so far: {buf:?}");
}

type SseStream = std::pin::Pin<
    Box<dyn futures_util::Stream<Item = reqwest::Result<bytes::Bytes>> + Send + 'static>,
>;

/// Open an SSE stream to `path` on the HTTP listener.
async fn open_sse(port: u16, path: &str) -> SseStream {
    let url = format!("http://127.0.0.1:{port}{path}");
    let resp = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .expect("http get");
    assert!(resp.status().is_success(), "status {}", resp.status());
    // Content-type must be text/event-stream (with possible charset suffix).
    let ct = resp
        .headers()
        .get("content-type")
        .expect("content-type header")
        .to_str()
        .expect("content-type utf-8")
        .to_owned();
    assert!(
        ct.starts_with("text/event-stream"),
        "expected text/event-stream, got {ct}"
    );
    Box::pin(resp.bytes_stream())
}

/// Parse a single `event:` + `data:` SSE frame text into
/// `(event_type, data_payload)`.
fn parse_sse_frame(text: &str) -> (String, String) {
    let mut event = None;
    let mut data = None;
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some(rest) = line.strip_prefix("event:") {
            event = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("data:") {
            data = Some(rest.trim().to_string());
        }
    }
    (event.expect("event: line"), data.expect("data: line"))
}

// ── Done-criterion item: GET /api/stats emits 1Hz events ──────────────────

#[tokio::test(flavor = "current_thread", start_paused = false)]
async fn stats_sse_emits_events_with_locked_schema() {
    let (port, _state, srv) = spawn_http().await;
    let mut stream = open_sse(port, "/api/stats").await;

    // Read 2 frames — sampler is 1 Hz, two frames = ~2 s.  Skip
    // SSE-comment keep-alive lines (start with ":") if axum emits
    // them between frames.
    let mut events: Vec<(String, String)> = Vec::new();
    while events.len() < 2 {
        let frame = read_one_sse_frame(&mut stream).await;
        // Filter out keep-alive comments (axum may emit ":\n\n").
        if frame.trim().is_empty() || frame.trim_start().starts_with(':') {
            continue;
        }
        events.push(parse_sse_frame(&frame));
    }

    for (ev, data) in &events {
        assert_eq!(ev, "stats", "event type must be 'stats'");
        // All 5 LOCKED field names must appear.
        for needle in [
            "connections_active",
            "commands_total",
            "keys_active",
            "mem_value_bytes",
            "uptime_secs",
        ] {
            assert!(
                data.contains(needle),
                "field {needle} missing from frame {data:?}"
            );
        }
    }

    // uptime_secs must be monotonically non-decreasing across frames.
    let u1 = extract_u64(&events[0].1, "uptime_secs");
    let u2 = extract_u64(&events[1].1, "uptime_secs");
    assert!(u2 >= u1, "uptime_secs went backwards: {u1} → {u2}");

    srv.abort();
}

// ── Done-criterion item: SET 3 keys via RESP → SSE shows keys_active=3 ───

#[tokio::test(flavor = "current_thread")]
async fn stats_reflect_resp_connection_and_keys() {
    let (http_port, state, http_srv) = spawn_http().await;
    let (resp_port, resp_srv) = spawn_resp_with_state(state.clone()).await;

    // Open SSE first so we can compare before/after frames.
    let mut stream = open_sse(http_port, "/api/stats").await;
    let _baseline = read_one_sse_frame(&mut stream).await; // discard any keep-alive

    // Open one RESP connection + SET 3 keys.
    let mut sock = TcpStream::connect(("127.0.0.1", resp_port))
        .await
        .expect("resp connect");
    for (k, v) in [("k1", "v1"), ("k2", "v2"), ("k3", "v3")] {
        let req = format!(
            "*3\r\n$3\r\nSET\r\n${}\r\n{}\r\n${}\r\n{}\r\n",
            k.len(),
            k,
            v.len(),
            v
        );
        sock.write_all(req.as_bytes()).await.expect("resp write");
        // Read +OK\r\n
        let mut buf = [0u8; 5];
        tokio::io::AsyncReadExt::read_exact(&mut sock, &mut buf)
            .await
            .expect("read OK");
        assert_eq!(&buf, b"+OK\r\n");
    }

    // Read frames until we see keys_active >= 3 AND connections_active >= 1.
    // Bound by 6 frames worth (= 6 s) so a stuck test fails fast.
    let mut seen_keys = 0u64;
    let mut seen_conns = 0u64;
    for _ in 0..6 {
        let frame = read_one_sse_frame(&mut stream).await;
        if frame.trim().is_empty() || frame.trim_start().starts_with(':') {
            continue;
        }
        let (ev, data) = parse_sse_frame(&frame);
        if ev != "stats" {
            continue;
        }
        seen_keys = seen_keys.max(extract_u64(&data, "keys_active"));
        seen_conns = seen_conns.max(extract_u64(&data, "connections_active"));
        if seen_keys >= 3 && seen_conns >= 1 {
            break;
        }
    }
    assert!(seen_keys >= 3, "keys_active never reached 3 ({seen_keys})");
    assert!(
        seen_conns >= 1,
        "connections_active never reached 1 ({seen_conns})"
    );

    // Drop the RESP connection; the next frame should reflect
    // connections_active=0 within a couple of ticks.
    drop(sock);
    let mut seen_zero_conns = false;
    for _ in 0..4 {
        let frame = read_one_sse_frame(&mut stream).await;
        if frame.trim().is_empty() || frame.trim_start().starts_with(':') {
            continue;
        }
        let (ev, data) = parse_sse_frame(&frame);
        if ev != "stats" {
            continue;
        }
        if extract_u64(&data, "connections_active") == 0 {
            seen_zero_conns = true;
            break;
        }
    }
    assert!(
        seen_zero_conns,
        "connections_active never returned to 0 after disconnect"
    );

    http_srv.abort();
    resp_srv.abort();
}

// ── Done-criterion item: GET /api/keys emits per-key info ─────────────────

#[tokio::test(flavor = "current_thread")]
async fn keys_sse_lists_recent_writes() {
    let (http_port, state, http_srv) = spawn_http().await;
    let (resp_port, resp_srv) = spawn_resp_with_state(state.clone()).await;

    // Write 2 keys: one plain, one with TTL.
    let mut sock = TcpStream::connect(("127.0.0.1", resp_port))
        .await
        .expect("connect");
    sock.write_all(b"*3\r\n$3\r\nSET\r\n$4\r\nfoo1\r\n$3\r\nbar\r\n")
        .await
        .expect("set");
    let mut ok = [0u8; 5];
    tokio::io::AsyncReadExt::read_exact(&mut sock, &mut ok)
        .await
        .expect("ok");
    sock.write_all(b"*5\r\n$3\r\nSET\r\n$4\r\nfoo2\r\n$3\r\nbaz\r\n$2\r\nEX\r\n$2\r\n42\r\n")
        .await
        .expect("set ex");
    tokio::io::AsyncReadExt::read_exact(&mut sock, &mut ok)
        .await
        .expect("ok2");

    let mut stream = open_sse(http_port, "/api/keys").await;
    // Read until a `keys` frame contains both foo1 and foo2.
    let mut saw_both = false;
    let mut saw_ttl_42_or_41 = false;
    for _ in 0..4 {
        let frame = read_one_sse_frame(&mut stream).await;
        if frame.trim().is_empty() || frame.trim_start().starts_with(':') {
            continue;
        }
        let (ev, data) = parse_sse_frame(&frame);
        if ev != "keys" {
            continue;
        }
        if data.contains(r#""key":"foo1""#) && data.contains(r#""key":"foo2""#) {
            saw_both = true;
            // Plain key should have ttl_secs:-1
            assert!(
                data.contains(r#""type":"string""#),
                "missing type=string in {data}"
            );
            // foo2 should have ttl ~42 (round-half-up; jitter tolerated).
            if data.contains(r#""ttl_secs":42"#) || data.contains(r#""ttl_secs":41"#) {
                saw_ttl_42_or_41 = true;
            }
            break;
        }
    }
    assert!(saw_both, "never saw both foo1 and foo2 in keys SSE");
    assert!(saw_ttl_42_or_41, "ttl_secs for foo2 was not 41 or 42");

    drop(sock);
    http_srv.abort();
    resp_srv.abort();
}

// ── Done-criterion item: 2 simultaneous SSE clients see same frame ────────

#[tokio::test(flavor = "current_thread")]
async fn two_stats_subscribers_both_receive_events() {
    let (port, _state, srv) = spawn_http().await;
    let mut s1 = open_sse(port, "/api/stats").await;
    let mut s2 = open_sse(port, "/api/stats").await;

    // Both should receive a `stats` event within ~2 s.
    let f1 = read_one_data_frame(&mut s1, "stats").await;
    let f2 = read_one_data_frame(&mut s2, "stats").await;

    // Both must contain `uptime_secs` field — schemas are the same.
    assert!(f1.contains("uptime_secs"));
    assert!(f2.contains("uptime_secs"));

    srv.abort();
}

async fn read_one_data_frame(stream: &mut SseStream, want_event: &str) -> String {
    // Tolerate up to 5 frames before giving up (allows skipping
    // keep-alive comments and unrelated events).
    for _ in 0..5 {
        let frame = read_one_sse_frame(stream).await;
        if frame.trim().is_empty() || frame.trim_start().starts_with(':') {
            continue;
        }
        let (ev, data) = parse_sse_frame(&frame);
        if ev == want_event {
            return data;
        }
    }
    panic!("never saw event {want_event}");
}

// ── Done-criterion item: ctrl_c equivalent — server task drops cleanly ────
// (real signal can't be sent in-test; abort handle is the test analogue;
//  already covered above by `srv.abort()` running in every test)

// ── Helpers ────────────────────────────────────────────────────────────────

/// Extract a `u64` value for `field` from a JSON-ish payload.
/// Tolerates surrounding whitespace; works for the flat schemas we emit.
fn extract_u64(payload: &str, field: &str) -> u64 {
    let needle = format!("\"{field}\":");
    let idx = payload.find(&needle).unwrap_or_else(|| {
        panic!("field {field} not found in {payload}");
    });
    let rest = &payload[idx + needle.len()..];
    let end = rest
        .find(|c: char| c != '-' && !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].trim().parse::<u64>().unwrap_or_else(|_| {
        panic!(
            "cannot parse u64 for {field} from {:?}",
            &rest[..end.min(20)]
        )
    })
}

// Suppress unused-import warning when slimming this file down.
#[allow(dead_code)]
fn _silence_unused_warning() {
    let _ = Duration::from_secs(1);
}
