---
finding: m3-1-lagging-subscriber-disconnect
date: 2026-05-12
case: cs01-mini-redis-rust
severity: low
status: accepted
adr_ref: 0009
---

# Finding M3.1: Lagging Pub/Sub subscriber → disconnect (more aggressive than Redis)

## Context

ADR-0009 §"负面 / 接受的债" already flagged that our M3.1 behaviour
when a subscriber's `tokio::sync::broadcast::Receiver` reports
`RecvError::Lagged(_)` is **more aggressive** than what real Redis 7
does for the same scenario.

- **Our M3.1 code path** (`crates/redis-server/src/server.rs`,
  `recv_any_subscription` + the `SubRecvError::Lagged` arm of
  `handle_conn`):
  on `Lagged`, we write
  `-ERR client lagged behind pub/sub buffer; disconnecting\r\n` and
  return `Ok(())` from the per-conn task, which drops the socket.
- **Real Redis 7**:
  detects the slow client via `client-output-buffer-limit pubsub`,
  resets the connection without an error reply.  Same end state
  (disconnected client) but no application-level error frame.

## Why we accept the divergence in M3.1

1. **broadcast channel capacity = 128** (`PUBSUB_BROADCAST_CAPACITY`).
   At 1 frame/second from the SSE dashboard sampler and any
   realistic publish rate, a healthy client cannot fall 128 frames
   behind.  Triggering this code path requires the client to
   stop draining for many seconds — pathological by construction.
2. **`-ERR` frame is informational, not protocol-breaking**.  Any
   conforming RESP client treats it as a regular error reply and
   then sees EOF.  The user-facing failure is identical (the
   subscription stops working until they reconnect).
3. **Cobrust-style "tell the user why"**: emitting an explicit
   error makes failure modes easier to diagnose during the demo
   phase.  Real Redis' silent reset is a 20-year-old battle scar,
   not a design we should mimic before measuring.

## Risks / when to revisit

| Risk | Likelihood | Mitigation |
|---|---|---|
| Real Redis-py / `redis-cli` mishandles the extra `-ERR` frame before EOF | Low | The oracle harness (`tests/oracle_pubsub.py`) covers the round-trip; M3.1 fixtures don't induce Lagged so the divergence is untested |
| User reports "weird message" instead of clean disconnect | Low | Banner in `/pubsub` UI tells users the dashboard is read-only; CLI users get a textual error which is arguably more helpful |
| Production load (M4 release) triggers `Lagged` at scale | Medium | Pre-release sprint should measure pubsub burst tolerance under realistic publish rates; if `Lagged` becomes a routine event the answer is to **raise capacity**, not to suppress the error |

## Decision

**Accept for M3.1.**  Plan: revisit in M4 release-readiness with
either:
- Match real Redis (silent reset, no `-ERR`), OR
- Make the error frame configurable and disable by default for
  Redis-compat mode.

This is the F22-cadence-aware path: ship M3.1, gather real-world
behaviour data, change the policy before the v0.1.0 freeze if
needed.

## Cross-references

- ADR-0009 §"负面 / 接受的债" (the original flag)
- `crates/redis-server/src/server.rs::recv_any_subscription`
  and `SubRecvError::Lagged`
- Real Redis docs: `client-output-buffer-limit pubsub` defaults
  (32 MiB hard limit / 8 MiB soft limit over 60 s)
