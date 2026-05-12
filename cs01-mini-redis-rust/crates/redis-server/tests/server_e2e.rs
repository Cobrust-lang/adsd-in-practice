//! End-to-end tests for the RESP TCP listener (ADR-0005).
//!
//! Each test:
//!   1. Binds `127.0.0.1:0` so the OS picks a free port,
//!   2. Spawns `server::run_on(listener, store)` on a background task,
//!   3. Connects an in-process `TcpStream` and exercises a behaviour,
//!   4. Aborts the server task at the end.
//!
//! No `sleep`, no docker — covers 12 of ADR-0005's behavioural Done
//! Criteria.  The 13th criterion (`ctrl_c → exit 0`) cannot be exercised
//! in unit-test form (no real signal); it is documented in the P9
//! completion report.
//!
//! Note: TTL tests cannot use `tokio::time::pause` because the server
//! runs on a separate tokio task and the listener relies on real
//! socket IO.  We use **one** small real-time `tokio::time::sleep`
//! (≤ 1.1s) for the SET EX → expiry criterion, isolated to that single
//! test.  See the comment on `set_ex_then_expiry` for rationale.

#![allow(clippy::expect_used)] // tests use expect("...") liberally — see CLAUDE.md §3.1 caveat.

use std::time::Duration;

use redis_server::server;
use redis_server::server::DEFAULT_MAX_FRAME_SIZE;
use redis_server::state::AppState;
use redis_storage::Store;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

// ── Test harness ─────────────────────────────────────────────────────────────

/// Spawn a server on `127.0.0.1:0` with the default max-frame-size.
async fn spawn_server() -> (u16, JoinHandle<std::io::Result<()>>) {
    spawn_server_with_limit(DEFAULT_MAX_FRAME_SIZE).await
}

/// Spawn a server with a caller-specified `max_frame_size` (M1.4 tests).
async fn spawn_server_with_limit(max_frame_size: usize) -> (u16, JoinHandle<std::io::Result<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local_addr").port();
    let state = AppState::new(Store::new(), max_frame_size);
    let handle = tokio::spawn(async move { server::run_on(listener, state).await });
    (port, handle)
}

/// Open a fresh client socket to `port`.
async fn connect(port: u16) -> TcpStream {
    TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("client connect")
}

/// Read exactly `n` bytes from `sock`.
async fn read_exact_n(sock: &mut TcpStream, n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    sock.read_exact(&mut buf).await.expect("read_exact");
    buf
}

/// Read until `sock` returns EOF.
async fn read_to_end(sock: &mut TcpStream) -> Vec<u8> {
    let mut buf = Vec::new();
    sock.read_to_end(&mut buf).await.expect("read_to_end");
    buf
}

// ── Criterion 1: PING → PONG ─────────────────────────────────────────────────

#[tokio::test]
async fn ping_returns_pong() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*1\r\n$4\r\nPING\r\n")
        .await
        .expect("write PING");
    let reply = read_exact_n(&mut sock, b"+PONG\r\n".len()).await;
    assert_eq!(reply, b"+PONG\r\n");
    srv.abort();
}

// ── Criterion 2: SET foo bar → OK; GET foo → "bar" ───────────────────────────

#[tokio::test]
async fn set_then_get_round_trip() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n")
        .await
        .expect("write SET");
    let ok = read_exact_n(&mut sock, b"+OK\r\n".len()).await;
    assert_eq!(ok, b"+OK\r\n");

    sock.write_all(b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n")
        .await
        .expect("write GET");
    let bulk = read_exact_n(&mut sock, b"$3\r\nbar\r\n".len()).await;
    assert_eq!(bulk, b"$3\r\nbar\r\n");
    srv.abort();
}

// ── Criterion 3: SET k v EX 1 → wait → GET k → nil ───────────────────────────
//
// Rationale for the real-time sleep: ADR-0005 §"Done Criteria" demands
// that `SET ... EX 1` then a delay then `GET` returns nil, which
// exercises the storage layer's *real* `DelayQueue` task running on
// the server's tokio runtime.  `tokio::time::pause` would only affect
// the *test*'s runtime, not the server task, so the DelayQueue would
// not fire.  This is the single sleep allowed under the prompt's
// "last resort" exception; isolated to one test, capped at 1100 ms.

#[tokio::test]
async fn set_ex_then_expiry() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    // SET ttlkey value EX 1
    sock.write_all(b"*5\r\n$3\r\nSET\r\n$6\r\nttlkey\r\n$5\r\nvalue\r\n$2\r\nEX\r\n$1\r\n1\r\n")
        .await
        .expect("write SET EX 1");
    let ok = read_exact_n(&mut sock, b"+OK\r\n".len()).await;
    assert_eq!(ok, b"+OK\r\n");

    // Wait just past the TTL.  See file-level comment for why a real
    // sleep is unavoidable here.
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // GET ttlkey → nil bulk
    sock.write_all(b"*2\r\n$3\r\nGET\r\n$6\r\nttlkey\r\n")
        .await
        .expect("write GET");
    let nil = read_exact_n(&mut sock, b"$-1\r\n".len()).await;
    assert_eq!(nil, b"$-1\r\n", "key should be gone after TTL");
    srv.abort();
}

// ── Criterion 4: DEL a b c — count = number actually removed ────────────────

#[tokio::test]
async fn del_multi_returns_actual_count() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;

    // SET a 1; SET b 1
    sock.write_all(b"*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n")
        .await
        .expect("set a");
    let _ = read_exact_n(&mut sock, b"+OK\r\n".len()).await;
    sock.write_all(b"*3\r\n$3\r\nSET\r\n$1\r\nb\r\n$1\r\n1\r\n")
        .await
        .expect("set b");
    let _ = read_exact_n(&mut sock, b"+OK\r\n".len()).await;

    // DEL a b nonexistent → 2
    sock.write_all(b"*4\r\n$3\r\nDEL\r\n$1\r\na\r\n$1\r\nb\r\n$1\r\nc\r\n")
        .await
        .expect("del");
    let reply = read_exact_n(&mut sock, b":2\r\n".len()).await;
    assert_eq!(reply, b":2\r\n");
    srv.abort();
}

// ── Criterion 5: INCR counter → 1, 2, 3, ... ─────────────────────────────────

#[tokio::test]
async fn incr_increments() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;

    for expected in 1..=3_i64 {
        sock.write_all(b"*2\r\n$4\r\nINCR\r\n$7\r\ncounter\r\n")
            .await
            .expect("write INCR");
        let want = format!(":{expected}\r\n");
        let got = read_exact_n(&mut sock, want.len()).await;
        assert_eq!(got, want.as_bytes());
    }
    srv.abort();
}

// ── Criterion 6: ECHO hi → "hi" ──────────────────────────────────────────────

#[tokio::test]
async fn echo_returns_message() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*2\r\n$4\r\nECHO\r\n$2\r\nhi\r\n")
        .await
        .expect("write ECHO");
    let reply = read_exact_n(&mut sock, b"$2\r\nhi\r\n".len()).await;
    assert_eq!(reply, b"$2\r\nhi\r\n");
    srv.abort();
}

// ── Criterion 7: SELECT 0 → OK; SELECT 9 → ERR DB index is out of range ─────

#[tokio::test]
async fn select_zero_ok_select_nine_error() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;

    sock.write_all(b"*2\r\n$6\r\nSELECT\r\n$1\r\n0\r\n")
        .await
        .expect("write SELECT 0");
    let ok = read_exact_n(&mut sock, b"+OK\r\n".len()).await;
    assert_eq!(ok, b"+OK\r\n");

    sock.write_all(b"*2\r\n$6\r\nSELECT\r\n$1\r\n9\r\n")
        .await
        .expect("write SELECT 9");
    // Exact Redis wire string per ADR-0005 watch-out.
    let want = b"-ERR DB index is out of range\r\n";
    let got = read_exact_n(&mut sock, want.len()).await;
    assert_eq!(got, want);
    srv.abort();
}

// ── Criterion 8: QUIT → +OK then connection closes ───────────────────────────

#[tokio::test]
async fn quit_replies_ok_and_closes() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*1\r\n$4\r\nQUIT\r\n")
        .await
        .expect("write QUIT");
    // read_to_end will block until EOF; the server must close the
    // socket after flushing `+OK\r\n`.
    let buf = read_to_end(&mut sock).await;
    assert_eq!(buf, b"+OK\r\n", "QUIT must reply +OK then close");
    srv.abort();
}

// ── Criterion 9: Unknown command → -ERR unknown command 'XYZ'; conn stays ───

#[tokio::test]
async fn unknown_command_does_not_close_conn() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*1\r\n$3\r\nXYZ\r\n")
        .await
        .expect("write XYZ");
    let want = b"-ERR unknown command 'XYZ'\r\n";
    let got = read_exact_n(&mut sock, want.len()).await;
    assert_eq!(got, want);

    // Connection must stay open — PING should still work.
    sock.write_all(b"*1\r\n$4\r\nPING\r\n")
        .await
        .expect("write PING after unknown");
    let pong = read_exact_n(&mut sock, b"+PONG\r\n".len()).await;
    assert_eq!(pong, b"+PONG\r\n");
    srv.abort();
}

// ── Criterion 10: Pipelining — multiple commands in one write ───────────────

#[tokio::test]
async fn pipelining_three_commands_one_write() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;

    // Three commands in a single write_all call.
    let pipeline: Vec<u8> = [
        &b"*1\r\n$4\r\nPING\r\n"[..],
        &b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n"[..],
        &b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n"[..],
    ]
    .concat();
    sock.write_all(&pipeline)
        .await
        .expect("write pipelined commands");

    // Read all three replies in order.
    let want: Vec<u8> = [&b"+PONG\r\n"[..], &b"+OK\r\n"[..], &b"$1\r\nv\r\n"[..]].concat();
    let got = read_exact_n(&mut sock, want.len()).await;
    assert_eq!(got, want, "pipelined replies must arrive in send order");
    srv.abort();
}

// ── Criterion 11: Half-frame — split a single frame across two writes ───────

#[tokio::test]
async fn half_frame_assembles_correctly() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;

    // Split PING into two pieces.
    sock.write_all(b"*1\r\n$4\r\nP")
        .await
        .expect("write half 1");
    // Yield to give the server task a chance to run a read+parse
    // cycle on the partial frame — it should report Incomplete and
    // wait for more bytes (i.e., NOT respond yet).
    tokio::task::yield_now().await;
    sock.write_all(b"ING\r\n").await.expect("write half 2");

    let reply = read_exact_n(&mut sock, b"+PONG\r\n".len()).await;
    assert_eq!(reply, b"+PONG\r\n");
    srv.abort();
}

// ── Criterion 12: Protocol error — garbage → -ERR then close ────────────────

#[tokio::test]
async fn protocol_error_responds_and_closes() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;

    // `garbage\r\n` does not start with one of `+-:$*` — Frame::parse
    // returns ProtocolError::Invalid("unknown RESP type byte").
    sock.write_all(b"garbage\r\n").await.expect("write garbage");
    let buf = read_to_end(&mut sock).await;
    let s = String::from_utf8_lossy(&buf);
    assert!(
        s.starts_with("-ERR Protocol error"),
        "expected -ERR Protocol error prefix, got {s:?}"
    );
    assert!(s.ends_with("\r\n"), "reply must end with CRLF, got {s:?}");
    srv.abort();
}

// ── Extra defensive coverage (beyond the 12 done criteria) ──────────────────

/// SELECT with a non-integer arg is caught at dispatch (parse_select)
/// and returns ERR but the connection survives.
#[tokio::test]
async fn select_non_integer_is_dispatch_error() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*2\r\n$6\r\nSELECT\r\n$3\r\nabc\r\n")
        .await
        .expect("write SELECT abc");
    let want = b"-ERR value is not an integer or out of range\r\n";
    let got = read_exact_n(&mut sock, want.len()).await;
    assert_eq!(got, want);
    // Connection stays open.
    sock.write_all(b"*1\r\n$4\r\nPING\r\n")
        .await
        .expect("PING after");
    let pong = read_exact_n(&mut sock, b"+PONG\r\n".len()).await;
    assert_eq!(pong, b"+PONG\r\n");
    srv.abort();
}

/// Multiple sequential clients on the same server — accept loop must
/// keep running between connections.
#[tokio::test]
async fn multiple_sequential_clients() {
    let (port, srv) = spawn_server().await;
    for _ in 0..3 {
        let mut sock = connect(port).await;
        sock.write_all(b"*1\r\n$4\r\nPING\r\n")
            .await
            .expect("write PING");
        let pong = read_exact_n(&mut sock, b"+PONG\r\n".len()).await;
        assert_eq!(pong, b"+PONG\r\n");
    }
    srv.abort();
}

// ── M1.4 (ADR-0006) ──────────────────────────────────────────────────────────

/// PING with a message must echo the message as a bulk string.
#[tokio::test]
async fn ping_with_message_returns_bulk_string() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*2\r\n$4\r\nPING\r\n$5\r\nhello\r\n")
        .await
        .expect("write PING hello");
    let want = b"$5\r\nhello\r\n";
    let got = read_exact_n(&mut sock, want.len()).await;
    assert_eq!(got, want);
    srv.abort();
}

/// EXPIRE on an existing key → :1, then TTL returns ~100.
#[tokio::test]
async fn expire_and_ttl_round_trip() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n")
        .await
        .expect("set");
    let _ = read_exact_n(&mut sock, b"+OK\r\n".len()).await;

    sock.write_all(b"*3\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$3\r\n100\r\n")
        .await
        .expect("expire");
    let reply = read_exact_n(&mut sock, b":1\r\n".len()).await;
    assert_eq!(reply, b":1\r\n");

    sock.write_all(b"*2\r\n$3\r\nTTL\r\n$1\r\nk\r\n")
        .await
        .expect("ttl");
    // TTL response is ":<n>\r\n"; we only check the prefix and CRLF.
    let mut buf = [0u8; 8];
    let n = sock.read(&mut buf).await.expect("read ttl");
    let s = String::from_utf8_lossy(&buf[..n]);
    assert!(s.starts_with(':'), "TTL must start with ':', got {s:?}");
    assert!(s.ends_with("\r\n"), "TTL must end with CRLF, got {s:?}");
    // Strip ':' and CRLF, parse.
    let n: i64 = s
        .trim_start_matches(':')
        .trim_end_matches("\r\n")
        .parse()
        .expect("integer body");
    assert!((99..=100).contains(&n), "expected TTL near 100, got {n}");
    srv.abort();
}

/// EXPIRE on missing key returns :0; PERSIST/TTL/TYPE on missing return
/// their respective sentinel replies.
#[tokio::test]
async fn expire_persist_on_missing_returns_zero() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*3\r\n$6\r\nEXPIRE\r\n$4\r\nnope\r\n$2\r\n60\r\n")
        .await
        .expect("expire missing");
    let r = read_exact_n(&mut sock, b":0\r\n".len()).await;
    assert_eq!(r, b":0\r\n");

    sock.write_all(b"*2\r\n$7\r\nPERSIST\r\n$4\r\nnope\r\n")
        .await
        .expect("persist missing");
    let r2 = read_exact_n(&mut sock, b":0\r\n".len()).await;
    assert_eq!(r2, b":0\r\n");

    sock.write_all(b"*2\r\n$3\r\nTTL\r\n$4\r\nnope\r\n")
        .await
        .expect("ttl missing");
    let r3 = read_exact_n(&mut sock, b":-2\r\n".len()).await;
    assert_eq!(r3, b":-2\r\n");
    srv.abort();
}

/// TYPE existing string → +string, missing → +none.
#[tokio::test]
async fn type_round_trip() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n")
        .await
        .expect("set");
    let _ = read_exact_n(&mut sock, b"+OK\r\n".len()).await;

    sock.write_all(b"*2\r\n$4\r\nTYPE\r\n$1\r\nk\r\n")
        .await
        .expect("type k");
    let got = read_exact_n(&mut sock, b"+string\r\n".len()).await;
    assert_eq!(got, b"+string\r\n");

    sock.write_all(b"*2\r\n$4\r\nTYPE\r\n$4\r\nnope\r\n")
        .await
        .expect("type nope");
    let got2 = read_exact_n(&mut sock, b"+none\r\n".len()).await;
    assert_eq!(got2, b"+none\r\n");
    srv.abort();
}

/// KEYS * returns an array containing all live keys.
#[tokio::test]
async fn keys_star_returns_all_live() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n")
        .await
        .expect("set a");
    let _ = read_exact_n(&mut sock, b"+OK\r\n".len()).await;
    sock.write_all(b"*3\r\n$3\r\nSET\r\n$1\r\nb\r\n$1\r\n2\r\n")
        .await
        .expect("set b");
    let _ = read_exact_n(&mut sock, b"+OK\r\n".len()).await;

    sock.write_all(b"*2\r\n$4\r\nKEYS\r\n$1\r\n*\r\n")
        .await
        .expect("keys *");
    // Read enough bytes for two single-char bulk strings in an *2 array.
    // The order is not deterministic (HashMap iteration), so we check
    // shape + bytes-set.
    let want_len = b"*2\r\n$1\r\na\r\n$1\r\nb\r\n".len();
    let got = read_exact_n(&mut sock, want_len).await;
    let s = String::from_utf8_lossy(&got);
    assert!(s.starts_with("*2\r\n"), "expected *2 array, got {s:?}");
    assert!(s.contains("$1\r\na\r\n"), "missing key 'a' in {s:?}");
    assert!(s.contains("$1\r\nb\r\n"), "missing key 'b' in {s:?}");
    srv.abort();
}

/// KEYS on an empty DB returns a zero-length array.
#[tokio::test]
async fn keys_empty_db_returns_empty_array() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*2\r\n$4\r\nKEYS\r\n$1\r\n*\r\n")
        .await
        .expect("keys *");
    let got = read_exact_n(&mut sock, b"*0\r\n".len()).await;
    assert_eq!(got, b"*0\r\n");
    srv.abort();
}

/// Sending a buffer that exceeds the configured `--max-frame-size` must
/// yield `-ERR Protocol error: frame too big` then close.
#[tokio::test]
async fn frame_too_big_protocol_error() {
    // Tight ceiling: 64 bytes (the BytesMut initial capacity is 4 KiB so
    // it grows on demand, but our guard fires on `buf.len()` regardless).
    let (port, srv) = spawn_server_with_limit(64).await;
    let mut sock = connect(port).await;

    // Craft a frame whose body is larger than the limit but whose header
    // is valid RESP — `$200\r\n` followed by 200 bytes of payload.
    // Even before the body completes, accumulated bytes exceed 64.
    let header = b"$200\r\n";
    let payload = vec![b'A'; 200];
    sock.write_all(header).await.expect("write header");
    sock.write_all(&payload).await.expect("write payload");

    let buf = read_to_end(&mut sock).await;
    let s = String::from_utf8_lossy(&buf);
    assert!(
        s.starts_with("-ERR Protocol error: frame too big"),
        "expected frame-too-big ERR, got {s:?}"
    );
    assert!(s.ends_with("\r\n"));
    srv.abort();
}

/// Below-limit traffic on a tight ceiling must still work normally.
#[tokio::test]
async fn small_frame_under_limit_passes() {
    let (port, srv) = spawn_server_with_limit(64).await;
    let mut sock = connect(port).await;
    sock.write_all(b"*1\r\n$4\r\nPING\r\n")
        .await
        .expect("write PING");
    let pong = read_exact_n(&mut sock, b"+PONG\r\n".len()).await;
    assert_eq!(pong, b"+PONG\r\n");
    srv.abort();
}

// ── M3.1 (ADR-0009) Pub/Sub ──────────────────────────────────────────────────

/// SUBSCRIBE on a single channel returns the verbatim Redis 7 ack
/// frame: `*3\r\n$9\r\nsubscribe\r\n$4\r\nnews\r\n:1\r\n`.
#[tokio::test]
async fn subscribe_single_channel_returns_redis7_ack() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;

    sock.write_all(b"*2\r\n$9\r\nSUBSCRIBE\r\n$4\r\nnews\r\n")
        .await
        .expect("write SUBSCRIBE");
    let want = b"*3\r\n$9\r\nsubscribe\r\n$4\r\nnews\r\n:1\r\n";
    let got = read_exact_n(&mut sock, want.len()).await;
    assert_eq!(got, want);
    srv.abort();
}

/// SUBSCRIBE a b c → three acks with running counts 1, 2, 3.
#[tokio::test]
async fn subscribe_multi_channel_returns_running_counts() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;

    sock.write_all(b"*4\r\n$9\r\nSUBSCRIBE\r\n$1\r\na\r\n$1\r\nb\r\n$1\r\nc\r\n")
        .await
        .expect("write SUBSCRIBE a b c");

    let want = [
        &b"*3\r\n$9\r\nsubscribe\r\n$1\r\na\r\n:1\r\n"[..],
        &b"*3\r\n$9\r\nsubscribe\r\n$1\r\nb\r\n:2\r\n"[..],
        &b"*3\r\n$9\r\nsubscribe\r\n$1\r\nc\r\n:3\r\n"[..],
    ]
    .concat();
    let got = read_exact_n(&mut sock, want.len()).await;
    assert_eq!(got, want);
    srv.abort();
}

/// In sub mode, sending GET must return the verbatim Redis 7 error and
/// keep the connection alive.
#[tokio::test]
async fn sub_mode_rejects_get_with_verbatim_error() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;

    sock.write_all(b"*2\r\n$9\r\nSUBSCRIBE\r\n$1\r\nx\r\n")
        .await
        .expect("write SUBSCRIBE");
    let _ack = read_exact_n(
        &mut sock,
        b"*3\r\n$9\r\nsubscribe\r\n$1\r\nx\r\n:1\r\n".len(),
    )
    .await;

    // GET foo in sub mode → -ERR Can't execute 'get': ...
    sock.write_all(b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n")
        .await
        .expect("write GET");
    let want = b"-ERR Can't execute 'get': only (P)SUBSCRIBE / (P)UNSUBSCRIBE / PING / QUIT / RESET are allowed in this context\r\n";
    let got = read_exact_n(&mut sock, want.len()).await;
    assert_eq!(got, want);

    // Connection must still be open — PING (allowed in sub mode) should
    // still produce +PONG.
    sock.write_all(b"*1\r\n$4\r\nPING\r\n")
        .await
        .expect("write PING");
    let pong = read_exact_n(&mut sock, b"+PONG\r\n".len()).await;
    assert_eq!(pong, b"+PONG\r\n");
    srv.abort();
}

/// Sub-mode SET wall: lowercase `'set'` in the error string per Redis 7.
#[tokio::test]
async fn sub_mode_rejects_set_with_lowercase_cmd_in_error() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*2\r\n$9\r\nSUBSCRIBE\r\n$1\r\nx\r\n")
        .await
        .expect("write SUBSCRIBE");
    let _ack = read_exact_n(
        &mut sock,
        b"*3\r\n$9\r\nsubscribe\r\n$1\r\nx\r\n:1\r\n".len(),
    )
    .await;

    sock.write_all(b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n")
        .await
        .expect("write SET");
    let want = b"-ERR Can't execute 'set': only (P)SUBSCRIBE / (P)UNSUBSCRIBE / PING / QUIT / RESET are allowed in this context\r\n";
    let got = read_exact_n(&mut sock, want.len()).await;
    assert_eq!(got, want);
    srv.abort();
}

/// UNSUBSCRIBE of the only channel → ack count = 0, conn returns to
/// Normal mode (SET now works again).
#[tokio::test]
async fn unsubscribe_returns_to_normal_mode() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;

    // Enter sub mode.
    sock.write_all(b"*2\r\n$9\r\nSUBSCRIBE\r\n$3\r\nfoo\r\n")
        .await
        .expect("write SUBSCRIBE");
    let _ack = read_exact_n(
        &mut sock,
        b"*3\r\n$9\r\nsubscribe\r\n$3\r\nfoo\r\n:1\r\n".len(),
    )
    .await;

    // Unsubscribe foo → ack with count = 0.
    sock.write_all(b"*2\r\n$11\r\nUNSUBSCRIBE\r\n$3\r\nfoo\r\n")
        .await
        .expect("write UNSUBSCRIBE");
    let want = b"*3\r\n$11\r\nunsubscribe\r\n$3\r\nfoo\r\n:0\r\n";
    let got = read_exact_n(&mut sock, want.len()).await;
    assert_eq!(got, want);

    // Conn should be back in Normal mode → SET works.
    sock.write_all(b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n")
        .await
        .expect("write SET");
    let ok = read_exact_n(&mut sock, b"+OK\r\n".len()).await;
    assert_eq!(ok, b"+OK\r\n");
    srv.abort();
}

/// UNSUBSCRIBE with no args from a non-subscribed conn → single
/// `unsubscribe / nil / 0` ack frame (Redis 7 quirk; ADR-0009 watch-out).
#[tokio::test]
async fn unsubscribe_no_args_on_normal_conn_emits_nil_ack() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;

    sock.write_all(b"*1\r\n$11\r\nUNSUBSCRIBE\r\n")
        .await
        .expect("write UNSUBSCRIBE");
    let want = b"*3\r\n$11\r\nunsubscribe\r\n$-1\r\n:0\r\n";
    let got = read_exact_n(&mut sock, want.len()).await;
    assert_eq!(got, want);
    srv.abort();
}

/// PUBLISH against no subscribers → :0.
#[tokio::test]
async fn publish_zero_subscribers_returns_zero() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;

    sock.write_all(b"*3\r\n$7\r\nPUBLISH\r\n$5\r\nnowhe\r\n$3\r\nmsg\r\n")
        .await
        .expect("write PUBLISH");
    let want = b":0\r\n";
    let got = read_exact_n(&mut sock, want.len()).await;
    assert_eq!(got, want);
    srv.abort();
}

/// SUBSCRIBE on one conn, PUBLISH on another → `message` push frame.
#[tokio::test]
async fn publish_propagates_to_subscriber() {
    let (port, srv) = spawn_server().await;

    // Subscriber socket.
    let mut sub = connect(port).await;
    sub.write_all(b"*2\r\n$9\r\nSUBSCRIBE\r\n$4\r\nnews\r\n")
        .await
        .expect("write SUBSCRIBE");
    let _ack = read_exact_n(
        &mut sub,
        b"*3\r\n$9\r\nsubscribe\r\n$4\r\nnews\r\n:1\r\n".len(),
    )
    .await;

    // Publisher socket.
    let mut pubsock = connect(port).await;
    pubsock
        .write_all(b"*3\r\n$7\r\nPUBLISH\r\n$4\r\nnews\r\n$5\r\nhello\r\n")
        .await
        .expect("write PUBLISH");
    let want_pub = b":1\r\n";
    let got_pub = read_exact_n(&mut pubsock, want_pub.len()).await;
    assert_eq!(got_pub, want_pub);

    // Subscriber receives `message / news / hello`.
    let want_msg = b"*3\r\n$7\r\nmessage\r\n$4\r\nnews\r\n$5\r\nhello\r\n";
    let got_msg = read_exact_n(&mut sub, want_msg.len()).await;
    assert_eq!(got_msg, want_msg);

    srv.abort();
}

/// PUBLISH against 3 subscribers → :3 and all three receive identical
/// `message` frames.
#[tokio::test]
async fn publish_fans_out_to_three_subscribers() {
    let (port, srv) = spawn_server().await;

    let mut s1 = connect(port).await;
    let mut s2 = connect(port).await;
    let mut s3 = connect(port).await;
    for s in [&mut s1, &mut s2, &mut s3] {
        s.write_all(b"*2\r\n$9\r\nSUBSCRIBE\r\n$1\r\nc\r\n")
            .await
            .expect("sub");
        let _ack = read_exact_n(s, b"*3\r\n$9\r\nsubscribe\r\n$1\r\nc\r\n:1\r\n".len()).await;
    }

    let mut pubsock = connect(port).await;
    pubsock
        .write_all(b"*3\r\n$7\r\nPUBLISH\r\n$1\r\nc\r\n$2\r\nGO\r\n")
        .await
        .expect("publish");
    let got = read_exact_n(&mut pubsock, b":3\r\n".len()).await;
    assert_eq!(got, b":3\r\n");

    let want_msg = b"*3\r\n$7\r\nmessage\r\n$1\r\nc\r\n$2\r\nGO\r\n";
    for s in [&mut s1, &mut s2, &mut s3] {
        let got = read_exact_n(s, want_msg.len()).await;
        assert_eq!(got, want_msg);
    }

    srv.abort();
}

/// PING in sub mode → +PONG.  Redis 7 behaviour per ADR-0009 §Notes
/// (NOT the Redis 6 `*2\r\n...pong...` Array shape); verified by the
/// M3.1 oracle.
#[tokio::test]
async fn ping_in_sub_mode_returns_plain_pong() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*2\r\n$9\r\nSUBSCRIBE\r\n$1\r\nx\r\n")
        .await
        .expect("sub");
    let _ack = read_exact_n(
        &mut sock,
        b"*3\r\n$9\r\nsubscribe\r\n$1\r\nx\r\n:1\r\n".len(),
    )
    .await;

    sock.write_all(b"*1\r\n$4\r\nPING\r\n").await.expect("ping");
    let pong = read_exact_n(&mut sock, b"+PONG\r\n".len()).await;
    assert_eq!(pong, b"+PONG\r\n");
    srv.abort();
}

/// QUIT in sub mode flushes +OK and closes the socket (sub-mode wall
/// must allow QUIT through).
#[tokio::test]
async fn quit_in_sub_mode_closes_socket_after_ok() {
    let (port, srv) = spawn_server().await;
    let mut sock = connect(port).await;
    sock.write_all(b"*2\r\n$9\r\nSUBSCRIBE\r\n$1\r\nq\r\n")
        .await
        .expect("sub");
    let _ack = read_exact_n(
        &mut sock,
        b"*3\r\n$9\r\nsubscribe\r\n$1\r\nq\r\n:1\r\n".len(),
    )
    .await;
    sock.write_all(b"*1\r\n$4\r\nQUIT\r\n").await.expect("quit");
    let buf = read_to_end(&mut sock).await;
    assert_eq!(buf, b"+OK\r\n");
    srv.abort();
}

// ── M3.2 (ADR-0010) AOF restart-roundtrip ──────────────────────────────────

use std::path::PathBuf;

use redis_storage::FsyncPolicy;

/// RAII temp-AOF guard: removes the file on Drop so parallel test
/// runs don't leak.  No `tempfile` workspace dep (ADR-0010 §"No new
/// workspace deps").
struct E2eTempAof {
    path: PathBuf,
}

impl E2eTempAof {
    fn new(stem: &str) -> Self {
        let pid = std::process::id();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let mut path = std::env::temp_dir();
        path.push(format!("cs01-aof-e2e-{stem}-{pid}-{nonce}.aof"));
        Self { path }
    }
}

impl Drop for E2eTempAof {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Bind a fresh listener with a `Store::with_aof`-equipped state.
/// Caller owns the JoinHandle and aborts it to simulate server kill.
async fn spawn_server_with_aof(
    path: PathBuf,
    fsync: FsyncPolicy,
) -> (u16, JoinHandle<std::io::Result<()>>, Store) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local_addr").port();
    // Run replay against an in-memory store (file may not yet exist
    // on a first run), then attach AOF.
    let store = Store::new();
    let count = store
        .replay_from_path(&path)
        .expect("replay_from_path during e2e setup");
    if count > 0 {
        // Tracing isn't initialised in tests; print so the harness
        // log captures it.
        eprintln!(
            "server e2e: replayed {count} commands from {}",
            path.display()
        );
    }
    let store = store.attach_aof(path, fsync).await.expect("attach_aof");
    let state = AppState::new(store.clone(), DEFAULT_MAX_FRAME_SIZE);
    let handle = tokio::spawn(async move { server::run_on(listener, state).await });
    (port, handle, store)
}

/// Server-A → write SET k1 v1 / SET k2 v2 EX 100 / DEL k1 → kill.
/// Server-B → restart same AOF → GET k1 = nil, GET k2 = v2, TTL k2 ≈ 100.
///
/// `FsyncPolicy::Always` so the file is durable by the time `abort`
/// runs; the explicit `store.aof_flush().await` is the test-side
/// belt-and-braces (matches the storage-crate aof.rs test pattern).
#[tokio::test]
async fn restart_round_trip_via_two_listeners() {
    let temp = E2eTempAof::new("rt-two-listeners");

    // ── Server-A ────────────────────────────────────────────────────────
    {
        let (port, srv, store) =
            spawn_server_with_aof(temp.path.clone(), FsyncPolicy::Always).await;
        let mut sock = connect(port).await;

        // SET k1 v1
        sock.write_all(b"*3\r\n$3\r\nSET\r\n$2\r\nk1\r\n$2\r\nv1\r\n")
            .await
            .expect("set k1");
        let _ = read_exact_n(&mut sock, b"+OK\r\n".len()).await;

        // SET k2 v2 EX 100
        sock.write_all(b"*5\r\n$3\r\nSET\r\n$2\r\nk2\r\n$2\r\nv2\r\n$2\r\nEX\r\n$3\r\n100\r\n")
            .await
            .expect("set k2 ex");
        let _ = read_exact_n(&mut sock, b"+OK\r\n".len()).await;

        // DEL k1
        sock.write_all(b"*2\r\n$3\r\nDEL\r\n$2\r\nk1\r\n")
            .await
            .expect("del k1");
        let _ = read_exact_n(&mut sock, b":1\r\n".len()).await;

        // Flush AOF then "kill" the server.
        store.aof_flush().await;
        srv.abort();
    }

    // ── Server-B ────────────────────────────────────────────────────────
    let (port, srv, _store) = spawn_server_with_aof(temp.path.clone(), FsyncPolicy::Always).await;
    let mut sock = connect(port).await;

    // GET k1 → nil
    sock.write_all(b"*2\r\n$3\r\nGET\r\n$2\r\nk1\r\n")
        .await
        .expect("get k1");
    let nil = read_exact_n(&mut sock, b"$-1\r\n".len()).await;
    assert_eq!(nil, b"$-1\r\n", "k1 must be gone after DEL");

    // GET k2 → v2
    sock.write_all(b"*2\r\n$3\r\nGET\r\n$2\r\nk2\r\n")
        .await
        .expect("get k2");
    let v = read_exact_n(&mut sock, b"$2\r\nv2\r\n".len()).await;
    assert_eq!(v, b"$2\r\nv2\r\n");

    // TTL k2 — must be ~100 (allow 99..=100 for drift).
    sock.write_all(b"*2\r\n$3\r\nTTL\r\n$2\r\nk2\r\n")
        .await
        .expect("ttl k2");
    let mut buf = [0u8; 16];
    let n = sock.read(&mut buf).await.expect("read ttl");
    let s = String::from_utf8_lossy(&buf[..n]);
    assert!(s.starts_with(':'), "ttl prefix");
    let val: i64 = s
        .trim_start_matches(':')
        .trim_end_matches("\r\n")
        .parse()
        .expect("integer body");
    assert!((99..=100).contains(&val), "expected TTL ~100, got {val}");

    srv.abort();
}

/// Counter (commands_total) must NOT advance during replay — replay
/// re-enters via `Store::execute_no_aof` which bypasses the per-conn
/// counter (the counter is bumped in `handle_conn` per parsed RESP
/// frame).  Verified by counting RESP-side traffic only.
#[tokio::test]
async fn replay_does_not_inflate_commands_total() {
    let temp = E2eTempAof::new("counter-no-replay");

    // Phase 1 — write 3 commands.
    {
        let (port, srv, store) =
            spawn_server_with_aof(temp.path.clone(), FsyncPolicy::Always).await;
        let mut sock = connect(port).await;
        for cmd in [
            &b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n"[..],
            &b"*2\r\n$4\r\nINCR\r\n$1\r\nc\r\n"[..],
            &b"*2\r\n$4\r\nINCR\r\n$1\r\nc\r\n"[..],
        ] {
            sock.write_all(cmd).await.expect("write cmd");
        }
        // Drain replies (each is short).
        let mut buf = vec![0u8; 256];
        let _ = sock.read(&mut buf).await;
        store.aof_flush().await;
        srv.abort();
    }

    // Phase 2 — restart.  After replay we expect the AppState's
    // commands_total to be 0 (no RESP frames have arrived yet).
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let _port = listener.local_addr().expect("local_addr").port();
    let s = Store::new();
    let count = s.replay_from_path(&temp.path).expect("replay");
    assert_eq!(count, 3, "phase 2 replayed all 3 commands");
    let s = s
        .attach_aof(temp.path.clone(), FsyncPolicy::Always)
        .await
        .expect("attach");
    let state = AppState::new(s, DEFAULT_MAX_FRAME_SIZE);

    // No RESP frames have arrived yet — commands_total must be 0.
    let snap = state.snapshot_stats();
    assert_eq!(
        snap.commands_total, 0,
        "replay must NOT advance commands_total"
    );
    // And the keys are still in the store.
    assert_eq!(snap.keys_active, 2, "k + c");
}
