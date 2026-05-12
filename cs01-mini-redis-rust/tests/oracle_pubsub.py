#!/usr/bin/env python3
"""oracle_pubsub.py — M3.1 (ADR-0009) Pub/Sub round-trip vs real Redis 7.

F23-A defence (cs01 CLAUDE.md §2 + ADR-0009 §Q9): the Pub/Sub state
machine is stateful, multi-connection, and order-sensitive — bash with
`redis-cli` would be far too brittle for it.  Python (with the
`redis` PyPI package) makes the assertions readable.

Activation
----------
Invoked from `tests/oracle.sh` AFTER the baseline 22-fixture loop.
That script is itself opt-in (CS01_RUN_ORACLE=1).

Each fixture runs against TWO endpoints:
  - real Redis 7 docker (port from CS01_ORACLE_PORT, default 6379)
  - our mini-redis-server (port from CS01_OUR_PORT, default 16380)

If the two endpoints disagree on any frame we exit 1, printing the
divergence.  If everything matches we exit 0 and print
`oracle_pubsub.py: N / N pubsub fixtures matched`.

Skip rules
----------
- Missing `redis` PyPI package    → exit 0, log "skipped (no redis pkg)"
- Either endpoint not responsive  → exit 0, log "skipped (endpoint down)"
  (the bash wrapper is responsible for starting docker; if the wrapper
  itself skipped, this script won't be invoked at all)
"""

from __future__ import annotations

import os
import sys
import time
from dataclasses import dataclass

# Skip cleanly if the `redis` package isn't available — this keeps the
# script optional even when the wrapper script decided to invoke us.
try:
    import redis  # type: ignore[import-not-found]
except ImportError:
    print("oracle_pubsub.py: redis package not installed — skipped")
    sys.exit(0)

ORACLE_PORT = int(os.environ.get("CS01_ORACLE_PORT", "6379"))
OUR_PORT = int(os.environ.get("CS01_OUR_PORT", "16380"))


@dataclass
class FixtureResult:
    """One fixture's outcome on one endpoint."""

    name: str
    sequence: list[object]
    """Observed frames in the order they arrived (Redis-py decodes them
    to dicts/strs/ints; we capture verbatim so the diff is mechanical)."""


def connect(port: int) -> "redis.Redis":
    """Open a fresh Redis-py client to `port`. Decode responses so the
    fixture comparison is plain-Python rather than `bytes` soup."""
    return redis.Redis(
        host="127.0.0.1",
        port=port,
        decode_responses=True,
        socket_connect_timeout=3,
        socket_timeout=5,
    )


def ping_works(port: int) -> bool:
    """True if the endpoint answers PING."""
    try:
        return connect(port).ping() is True
    except redis.RedisError:
        return False


# ── Fixtures ─────────────────────────────────────────────────────────────────


def fixture_subscribe_single_channel(port: int) -> FixtureResult:
    """SUBSCRIBE news → expect one subscribe ack with count=1."""
    r = connect(port)
    pubsub = r.pubsub(ignore_subscribe_messages=False)
    pubsub.subscribe("news")
    msg = pubsub.get_message(timeout=2)
    pubsub.close()
    return FixtureResult(
        name="subscribe-single",
        sequence=[(msg["type"], msg["channel"], msg["data"]) if msg else None],
    )


def fixture_subscribe_multi_channels(port: int) -> FixtureResult:
    """SUBSCRIBE a b c → expect three acks with counts 1/2/3."""
    r = connect(port)
    pubsub = r.pubsub(ignore_subscribe_messages=False)
    pubsub.subscribe("a", "b", "c")
    seq: list[object] = []
    for _ in range(3):
        m = pubsub.get_message(timeout=2)
        seq.append((m["type"], m["channel"], m["data"]) if m else None)
    pubsub.close()
    return FixtureResult(name="subscribe-multi", sequence=seq)


def fixture_unsubscribe_one_of_three(port: int) -> FixtureResult:
    """SUBSCRIBE a b c then UNSUBSCRIBE b → expect ack count=2 with channel b."""
    r = connect(port)
    pubsub = r.pubsub(ignore_subscribe_messages=False)
    pubsub.subscribe("a", "b", "c")
    # drain the three subscribe acks
    for _ in range(3):
        pubsub.get_message(timeout=2)
    pubsub.unsubscribe("b")
    msg = pubsub.get_message(timeout=2)
    pubsub.close()
    return FixtureResult(
        name="unsubscribe-one",
        sequence=[(msg["type"], msg["channel"], msg["data"]) if msg else None],
    )


def fixture_publish_with_one_subscriber(port: int) -> FixtureResult:
    """PUBLISH news hi to one subscriber → PUBLISH returns 1; subscriber
    receives `message / news / hi`."""
    r = connect(port)
    pubsub = r.pubsub(ignore_subscribe_messages=True)
    pubsub.subscribe("news")
    # Tiny grace period for the subscribe ack to settle (no event
    # signalling available to us at this protocol layer).
    time.sleep(0.2)
    pub_count = r.publish("news", "hi")
    msg = pubsub.get_message(timeout=2)
    pubsub.close()
    return FixtureResult(
        name="publish-one-sub",
        sequence=[
            ("publish_count", pub_count),
            (msg["type"], msg["channel"], msg["data"]) if msg else None,
        ],
    )


def fixture_publish_no_subscribers(port: int) -> FixtureResult:
    """PUBLISH none none with zero subs → PUBLISH returns 0."""
    r = connect(port)
    pub_count = r.publish("nobody-listens", "irrelevant")
    return FixtureResult(
        name="publish-zero-subs",
        sequence=[("publish_count", pub_count)],
    )


def fixture_ping_in_sub_mode(port: int) -> FixtureResult:
    """PING while in sub mode → expect `+PONG\\r\\n` (Redis 7 behaviour).

    ADR-0009 §Notes flags this as the *most likely* spec-vs-real
    divergence point.  We make the assertion via redis-py's
    `pubsub.ping()`, which returns the raw reply string on success.
    """
    r = connect(port)
    pubsub = r.pubsub(ignore_subscribe_messages=True)
    pubsub.subscribe("p")
    time.sleep(0.1)
    try:
        pong_reply = pubsub.ping()
    except redis.ResponseError as e:
        pong_reply = f"ERROR: {e}"
    pubsub.close()
    return FixtureResult(
        name="ping-in-sub-mode",
        sequence=[("pong", pong_reply)],
    )


FIXTURES = [
    fixture_subscribe_single_channel,
    fixture_subscribe_multi_channels,
    fixture_unsubscribe_one_of_three,
    fixture_publish_with_one_subscriber,
    fixture_publish_no_subscribers,
    fixture_ping_in_sub_mode,
]


# ── Diff loop ────────────────────────────────────────────────────────────────


def main() -> int:
    if not ping_works(ORACLE_PORT):
        print(f"oracle_pubsub.py: oracle on :{ORACLE_PORT} not responsive — skipped")
        return 0
    if not ping_works(OUR_PORT):
        print(f"oracle_pubsub.py: ours on :{OUR_PORT} not responsive — skipped")
        return 0

    failures = 0
    total = 0
    for fixture in FIXTURES:
        total += 1
        # Run on real Redis first (deterministic side: it's stable).
        try:
            oracle = fixture(ORACLE_PORT)
        except redis.RedisError as e:
            print(f"  x oracle errored on {fixture.__name__}: {e}")
            failures += 1
            continue
        try:
            ours = fixture(OUR_PORT)
        except redis.RedisError as e:
            print(f"  x ours errored on {fixture.__name__}: {e}")
            failures += 1
            continue

        if ours.sequence != oracle.sequence:
            print(f"  x DIVERGENCE on {oracle.name}")
            print(f"      ours:   {ours.sequence}")
            print(f"      oracle: {oracle.sequence}")
            failures += 1
        else:
            print(f"  + {oracle.name} -> {oracle.sequence}")

    print()
    print(f"oracle_pubsub.py: {total - failures} / {total} pubsub fixtures matched")
    return 1 if failures > 0 else 0


if __name__ == "__main__":
    sys.exit(main())
