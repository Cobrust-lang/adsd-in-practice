//! Axum HTTP control plane (ADR-0007, ADR-0009).
//!
//! Wave M2.1 — exposes two SSE endpoints driven off a single 1 Hz
//! sampler that broadcasts to all subscribers via
//! `tokio::sync::broadcast`:
//!
//! - `GET /api/stats` — `event: stats\ndata: {…}\n\n` per second.
//! - `GET /api/keys`  — `event: keys\ndata: [{…},…]\n\n` per second
//!   (top 100 keys, hashbrown-ordered — matches Redis `KEYS` ordering
//!   contract: "returns no specified order").
//!
//! Wave M3.1 (ADR-0009) adds a third SSE endpoint:
//! - `GET /api/pubsub` — `event: pubsub\ndata: {"channels":[...]}\n\n`
//!   per second.  Drives the SvelteKit `/pubsub` read-only dashboard.
//!
//! F24 defence (ADR-0007 watch-out): no `tower::ServiceBuilder` stack
//! of hidden middleware.  Each route is a plain async handler; the
//! whole router fits in ~5 LOC at `router()` below.
//!
//! Shutdown coordination (ADR-0007 §Cross-references): independent
//! `tokio::signal::ctrl_c()` watch — `main.rs` `try_join!`s both
//! listeners, and Linux delivers SIGINT to the whole process so both
//! handlers fire simultaneously.

use std::convert::Infallible;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue, ORIGIN,
};
use axum::http::{HeaderMap, Response};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use futures_util::stream::StreamExt as _;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

use crate::state::{AppState, KeysSnapshot, PubsubSnapshot, StatsSnapshot};

/// `/api/keys` sample cap (ADR-0007 §Q3).
///
/// "M2.1 is a demo dashboard, not prod-grade" — first 100 keys is
/// enough to drive the UI table; larger keyspaces show truncation.
pub const KEYS_SAMPLE_LIMIT: usize = 100;

/// Sampler tick interval (ADR-0007 §Q2).  1 Hz — dashboard refresh
/// rate, not a hot path.
pub const SAMPLER_INTERVAL: Duration = Duration::from_secs(1);

const BROWSER_DEV_ALLOWED_CORS_ORIGINS: &[&str] =
    &["http://localhost:5173", "http://127.0.0.1:5173"];
const BROWSER_DEV_CORS_HEADERS: &str = "accept, cache-control";

/// Per-key payload emitted on the `/api/keys` SSE stream.  Field
/// names are LOCKED for the M2.2 frontend (ADR-0007 §Q5).
#[derive(Debug, Clone, Serialize)]
struct KeyJson {
    key: String,
    #[serde(rename = "type")]
    kind: &'static str,
    ttl_secs: i64,
}

/// Build the Axum router.  Split out so tests can mount the router
/// onto a `TestServer` if needed; production code uses [`run`].
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/stats", get(stats_sse))
        .route("/api/keys", get(keys_sse))
        .route("/api/pubsub", get(pubsub_sse))
        .with_state(state)
}

/// Bind on `addr` and serve the HTTP control plane until ctrl_c.
///
/// Spawns the 1 Hz sampler task as a sibling of the axum serve loop;
/// the sampler exits when all `Receiver`s have dropped AND the
/// `Sender` is dropped (i.e. on shutdown).
///
/// # Errors
///
/// Returns `io::Error` if the listener cannot bind.
pub async fn run(addr: SocketAddr, state: AppState) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    tracing::info!(addr = %local_addr, "HTTP listener bound");
    run_on(listener, state).await
}

/// Run the HTTP control plane on an already-bound listener.
///
/// Split out for the same reason as [`crate::server::run_on`]: tests
/// bind on `127.0.0.1:0` and read the OS-assigned port back.
///
/// # Errors
///
/// Returns `io::Error` from `axum::serve`.
pub async fn run_on(listener: TcpListener, state: AppState) -> io::Result<()> {
    // Spawn 1 Hz sampler.  The task exits naturally when every
    // sender clone is dropped (we hold one; axum router holds clones
    // via state).
    let sampler_state = state.clone();
    let sampler = tokio::spawn(sampler_loop(sampler_state));

    let app = router(state);

    let serve_result = axum::serve(listener, app)
        .with_graceful_shutdown(async {
            // Ctrl-C → graceful shutdown.  We deliberately do NOT
            // share a single signal with the RESP listener: Linux
            // delivers SIGINT to the whole process, so both ctrl_c
            // futures resolve simultaneously.
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("HTTP ctrl_c received — shutting down");
        })
        .await;

    // After axum exits, abort the sampler (it would otherwise run
    // until all broadcast senders drop — which doesn't happen until
    // `state` itself is dropped).
    sampler.abort();
    let _ = sampler.await;

    serve_result
}

/// 1 Hz sampler — computes both stats and keys snapshots, fans out
/// via the two broadcast channels.  Returns when aborted.
async fn sampler_loop(state: AppState) {
    let mut ticker = tokio::time::interval(SAMPLER_INTERVAL);
    // Skip the immediate-fire of the first tick: we want a 1 s gap
    // between subscribe and the next frame so SSE clients don't
    // race the server's first emit.  `MissedTickBehavior::Delay`
    // also avoids burst-catchup on sleep skew.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;
        // Fan-out: ignore SendError (no subscribers is fine).
        let stats_frame = state.snapshot_stats();
        let _ = state.stats_tx.send(stats_frame);

        let keys_frame = KeysSnapshot(state.store.sample_keys(KEYS_SAMPLE_LIMIT));
        let _ = state.keys_tx.send(keys_frame);

        let pubsub_frame = state.snapshot_pubsub();
        let _ = state.pubsub_tx.send(pubsub_frame);
    }
}

/// `GET /api/stats` — SSE stream of `event: stats` frames.
///
/// Each frame's `data:` line is a JSON `StatsSnapshot`.
async fn stats_sse(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response<axum::body::Body> {
    let rx = state.stats_tx.subscribe();
    // `Sse::new` requires `Stream<Item = Result<Event, _>>`; our
    // mapper produces a plain `Event` and we wrap with `Ok` here so
    // clippy's `unnecessary_wraps` stays happy inside the mapper.
    let stream = BroadcastStream::new(rx).map(|item| Ok::<_, Infallible>(map_stats_event(&item)));
    with_browser_dev_cors(&headers, Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// `GET /api/keys` — SSE stream of `event: keys` frames (JSON array).
async fn keys_sse(headers: HeaderMap, State(state): State<AppState>) -> Response<axum::body::Body> {
    let rx = state.keys_tx.subscribe();
    let stream = BroadcastStream::new(rx).map(|item| Ok::<_, Infallible>(map_keys_event(&item)));
    with_browser_dev_cors(&headers, Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// `GET /api/pubsub` — SSE stream of `event: pubsub` frames (ADR-0009).
/// Payload is `{"channels": [{"name": "...", "subscribers": N}, ...]}`.
async fn pubsub_sse(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response<axum::body::Body> {
    let rx = state.pubsub_tx.subscribe();
    let stream = BroadcastStream::new(rx).map(|item| Ok::<_, Infallible>(map_pubsub_event(&item)));
    with_browser_dev_cors(&headers, Sse::new(stream).keep_alive(KeepAlive::default()))
}

fn with_browser_dev_cors(
    request_headers: &HeaderMap,
    response: impl IntoResponse,
) -> Response<axum::body::Body> {
    let mut response = response.into_response();
    if let Some(origin) = request_headers
        .get(ORIGIN)
        .and_then(|origin| origin.to_str().ok())
        .filter(|origin| BROWSER_DEV_ALLOWED_CORS_ORIGINS.contains(origin))
    {
        response.headers_mut().insert(
            ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_str(origin).expect("allowed CORS origin is a valid header value"),
        );
        response.headers_mut().insert(
            ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static(BROWSER_DEV_CORS_HEADERS),
        );
    }
    response
}

/// Convert a `BroadcastStream` item into a SSE `Event`.  On
/// `Lagged(n)` we emit an `event: error\ndata: lagged:N\n\n` frame —
/// the client knows to reconnect.  We *don't* terminate the stream
/// here; the underlying broadcast receiver re-syncs on the next item.
fn map_stats_event(item: &Result<StatsSnapshot, BroadcastStreamRecvError>) -> Event {
    match item {
        Ok(snap) => {
            // Serialization is infallible for these plain structs;
            // fall back to "{}" rather than poisoning the whole
            // stream (no realistic failure path here).
            let payload = serde_json::to_string(snap).unwrap_or_else(|_| "{}".into());
            Event::default().event("stats").data(payload)
        }
        Err(BroadcastStreamRecvError::Lagged(n)) => {
            Event::default().event("error").data(format!("lagged:{n}"))
        }
    }
}

fn map_keys_event(item: &Result<KeysSnapshot, BroadcastStreamRecvError>) -> Event {
    match item {
        Ok(snap) => {
            let payload: Vec<KeyJson> = snap
                .0
                .iter()
                .map(|k| KeyJson {
                    key: k.key.clone(),
                    kind: k.kind,
                    ttl_secs: k.ttl_secs,
                })
                .collect();
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "[]".into());
            Event::default().event("keys").data(json)
        }
        Err(BroadcastStreamRecvError::Lagged(n)) => {
            Event::default().event("error").data(format!("lagged:{n}"))
        }
    }
}

/// Convert a pubsub broadcast item into a SSE `Event` (ADR-0009).
fn map_pubsub_event(item: &Result<PubsubSnapshot, BroadcastStreamRecvError>) -> Event {
    match item {
        Ok(snap) => {
            // `{"channels":[...]}` — empty case serialises to
            // `{"channels":[]}`, never `null`.
            let json =
                serde_json::to_string(snap).unwrap_or_else(|_| r#"{"channels":[]}"#.to_owned());
            Event::default().event("pubsub").data(json)
        }
        Err(BroadcastStreamRecvError::Lagged(n)) => {
            Event::default().event("error").data(format!("lagged:{n}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis_storage::Store;

    #[test]
    fn key_json_serializes_with_renamed_type_field() {
        // The frontend contract says the field name is "type" — but
        // Rust can't use that as a struct field, so #[serde(rename)]
        // remaps `kind` → `"type"` on the wire.
        let k = KeyJson {
            key: "foo".into(),
            kind: "string",
            ttl_secs: -1,
        };
        let json = serde_json::to_string(&k).expect("serialize");
        assert!(
            json.contains(r#""type":"string""#),
            "expected wire field 'type', got: {json}"
        );
        assert!(json.contains(r#""ttl_secs":-1"#));
        assert!(json.contains(r#""key":"foo""#));
    }

    #[test]
    fn stats_snapshot_serializes_with_locked_field_names() {
        let snap = StatsSnapshot {
            connections_active: 3,
            commands_total: 1024,
            keys_active: 42,
            mem_value_bytes: 2048,
            uptime_secs: 300,
        };
        let json = serde_json::to_string(&snap).expect("serialize");
        // All five LOCKED field names must appear verbatim.
        for needle in [
            r#""connections_active":3"#,
            r#""commands_total":1024"#,
            r#""keys_active":42"#,
            r#""mem_value_bytes":2048"#,
            r#""uptime_secs":300"#,
        ] {
            assert!(
                json.contains(needle),
                "missing locked field {needle} in {json}"
            );
        }
    }

    #[tokio::test]
    async fn router_smoke_constructs() {
        // Make sure router() actually type-checks against an
        // AppState built from a real Store.  This is the smallest
        // compile-time guard that the SSE handlers wire to State.
        // `Store::new()` requires a tokio runtime (background
        // expiry task), so this is a `#[tokio::test]`.
        let state = AppState::new(Store::new(), 4096);
        let _r: Router = router(state);
    }

    #[tokio::test]
    async fn stats_sse_reflects_allowed_browser_dev_cors_origin() {
        let response = stats_sse_response_with_origin(Some("http://localhost:5173")).await;

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("http://localhost:5173"))
        );
    }

    #[tokio::test]
    async fn stats_sse_omits_cors_for_disallowed_origin() {
        let response = stats_sse_response_with_origin(Some("https://evil.example")).await;

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN), None);
    }

    #[tokio::test]
    async fn stats_sse_omits_cors_when_origin_absent() {
        let response = stats_sse_response_with_origin(None).await;

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN), None);
    }

    async fn stats_sse_response_with_origin(
        origin: Option<&str>,
    ) -> axum::http::Response<axum::body::Body> {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt as _;

        let state = AppState::new(Store::new(), 4096);
        let mut builder = Request::builder().uri("/api/stats");
        if let Some(origin) = origin {
            builder = builder.header(ORIGIN, origin);
        }
        router(state)
            .oneshot(
                builder
                    .body(Body::empty())
                    .expect("request builder succeeds"),
            )
            .await
            .expect("router response succeeds")
    }

    // ── M3.1 (ADR-0009) Pub/Sub SSE wire shape ───────────────────────────

    #[test]
    fn pubsub_snapshot_serializes_to_locked_shape() {
        use crate::state::{PubsubChannelEntry, PubsubSnapshot};
        let snap = PubsubSnapshot {
            channels: vec![
                PubsubChannelEntry {
                    name: "chat".into(),
                    subscribers: 12,
                },
                PubsubChannelEntry {
                    name: "news".into(),
                    subscribers: 3,
                },
            ],
        };
        let json = serde_json::to_string(&snap).expect("serialize");
        // Locked wire field names.
        assert!(json.contains(r#""channels":["#), "got: {json}");
        assert!(json.contains(r#""name":"chat""#));
        assert!(json.contains(r#""subscribers":12"#));
        assert!(json.contains(r#""name":"news""#));
        assert!(json.contains(r#""subscribers":3"#));
    }

    #[test]
    fn pubsub_snapshot_empty_serializes_to_empty_array_not_null() {
        use crate::state::PubsubSnapshot;
        let snap = PubsubSnapshot::default();
        let json = serde_json::to_string(&snap).expect("serialize");
        assert_eq!(json, r#"{"channels":[]}"#);
    }
}
