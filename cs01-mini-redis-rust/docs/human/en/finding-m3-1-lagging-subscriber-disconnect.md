# Finding M3.1: Lagging Pub/Sub subscriber disconnect policy

## Abstract

M3.1 Pub/Sub uses `tokio::sync::broadcast`. If a subscriber falls behind the broadcast buffer and `RecvError::Lagged(_)` occurs, this implementation writes `-ERR client lagged behind pub/sub buffer; disconnecting` before closing the connection.

Real Redis 7 usually handles comparable slow Pub/Sub clients via `client-output-buffer-limit pubsub` and resets the connection without sending this application-level error frame. The end state is the same (disconnected client), but the wire-level behavior differs.

## Why accepted

- The trigger is pathological: capacity is 128 and healthy clients should drain continuously.
- An explicit error frame is easier to diagnose during demo / case-study work.
- The divergence is documented publicly instead of being hidden behind a compatibility claim.

## Follow-up condition

If pre-`0.1.0` oracle or burst testing shows real clients are affected by the extra `-ERR`, switch to Redis-compatible silent reset or add a compatibility-mode switch.

## Cross-references

- Agent finding: [`../../agent/findings/m3-1-lagging-subscriber-disconnect.md`](../../agent/findings/m3-1-lagging-subscriber-disconnect.md)
- ADR: [`adr-0009-m3-1-pubsub.md`](adr-0009-m3-1-pubsub.md)
