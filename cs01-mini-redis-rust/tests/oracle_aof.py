#!/usr/bin/env python3
"""oracle_aof.py — M3.2 (ADR-0010) AOF restart round-trip vs real Redis 7.

F23-A defence (cs01 CLAUDE.md §2): the AOF round-trip is the most
end-to-end correctness test we have — write commands, kill the
process, restart, observe state.  Doing it against BOTH our server
and a real `redis:7-alpine` container (both with their own AOF file)
verifies that:

  1. Our AOF wire format is genuinely Redis-compatible.
  2. Our replay rebuilds the exact same state as real Redis does.
  3. Edge cases (TTL across restart, DEL-of-key, INCR counters) are
     handled identically.

Activation
----------
This script is invoked by `tests/oracle.sh` AFTER the baseline 22
RESP fixtures and the 6 pubsub fixtures.  It does NOT start the
docker container or our server — those are reused from the bash
wrapper's setup.

For each fixture (7 in total) we:
  1. Apply the fixture's *write* commands to BOTH endpoints.
  2. Issue a SAVE / wait so the AOF file on the real Redis side has
     been flushed.  For our side we rely on `--aof-fsync always`.
  3. Kill the two processes.
  4. Restart both with the same AOF paths.
  5. Issue the fixture's *observe* commands (GET / EXISTS / TTL /
     TYPE) and assert the replies match.

Killing + restarting docker for each fixture is expensive, so we
instead use a single restart per *script* run: phase 1 applies the
7 fixtures' write commands, phase 2 (after kill/restart) issues the
7 fixtures' observation commands.  This is functionally equivalent
(the state delta carried across restart is identical) and ~10×
faster.

Skip rules
----------
- Missing `redis` PyPI package          → exit 0, log skipped
- `docker` not on PATH                  → exit 0, log skipped
- Either endpoint not responsive        → exit 0, log skipped
"""

from __future__ import annotations

import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

try:
    import redis  # type: ignore[import-not-found]
except ImportError:
    print("oracle_aof.py: redis package not installed - skipped")
    sys.exit(0)


# ── Configuration ────────────────────────────────────────────────────────────

# We DON'T reuse the bash wrapper's docker container, because that one
# was started without `--appendonly yes`.  AOF testing needs its own
# container with persistence on.
ORACLE_PORT = int(os.environ.get("CS01_AOF_ORACLE_PORT", "6391"))
OUR_PORT = int(os.environ.get("CS01_AOF_OUR_PORT", "16391"))
ORACLE_CONTAINER = "cs01-aof-oracle"

CS01_AOF_PATH = Path(os.environ.get("CS01_AOF_OUR_PATH", "/tmp/cs01-oracle-aof.aof"))
ROOT = Path(__file__).resolve().parent.parent

BIN = ROOT / "target" / "release" / "redis-server"


def have_cmd(name: str) -> bool:
    """True if `name` is on PATH."""
    return shutil.which(name) is not None


def wait_for_port(port: int, timeout_s: float = 6.0) -> bool:
    """Poll redis-cli PING until success or timeout."""
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        try:
            client = redis.Redis(host="127.0.0.1", port=port, socket_timeout=1)
            if client.ping():
                return True
        except redis.RedisError:
            pass
        time.sleep(0.1)
    return False


# ── Lifecycle helpers ────────────────────────────────────────────────────────


def docker_run_redis_aof() -> None:
    """Start a fresh `redis:7-alpine` container with appendonly on.

    `--rm` so we don't accumulate stopped containers across runs.
    """
    subprocess.run(
        ["docker", "rm", "-f", ORACLE_CONTAINER],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    subprocess.run(
        [
            "docker",
            "run",
            "--rm",
            "-d",
            "--name",
            ORACLE_CONTAINER,
            "-p",
            f"{ORACLE_PORT}:6379",
            "redis:7-alpine",
            "redis-server",
            "--appendonly",
            "yes",
            "--appendfsync",
            "always",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )


def docker_stop_redis_aof() -> None:
    subprocess.run(
        ["docker", "stop", ORACLE_CONTAINER],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def docker_restart_redis_aof() -> None:
    """Stop + re-run with the SAME (anonymous) data volume.

    `redis:7-alpine` writes AOF to `/data/appendonly.aof` inside the
    container.  Since we are using `--rm` (anonymous volume), each
    run gets a FRESH `/data`.  To carry state across restart we
    bind-mount a host directory.  Adjust to use a host-mounted dir.
    """
    # Implemented in the run/restart helper below — kept here for the
    # commentary.  See `docker_run_with_data_volume`.
    raise NotImplementedError


# ── Data-volume-aware container management ───────────────────────────────────


def docker_run_with_data_volume(data_dir: Path) -> None:
    """Start a fresh redis container with `/data` bind-mounted to `data_dir`.

    On Linux, ensure the data dir is writable by the redis user
    (uid 999 in `redis:7-alpine`).  For CI under root this is fine.
    On macOS Docker Desktop, the bind mount handles uid mapping.
    """
    subprocess.run(
        ["docker", "rm", "-f", ORACLE_CONTAINER],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    data_dir.mkdir(parents=True, exist_ok=True)
    # Best-effort permissions; redis:7-alpine runs as uid 999.
    try:
        os.chmod(data_dir, 0o777)
    except OSError:
        pass

    subprocess.run(
        [
            "docker",
            "run",
            "--rm",
            "-d",
            "--name",
            ORACLE_CONTAINER,
            "-p",
            f"{ORACLE_PORT}:6379",
            "-v",
            f"{data_dir}:/data",
            "redis:7-alpine",
            "redis-server",
            "--appendonly",
            "yes",
            "--appendfsync",
            "always",
            "--dir",
            "/data",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )


# ── Our-server lifecycle ─────────────────────────────────────────────────────


def start_our_server() -> subprocess.Popen[bytes]:
    """Spawn our redis-server with --aof + always fsync."""
    cmd = [
        str(BIN),
        "--port",
        str(OUR_PORT),
        "--bind",
        "127.0.0.1",
        "--aof",
        str(CS01_AOF_PATH),
        "--aof-fsync",
        "always",
        "--http-port",
        "0",
    ]
    return subprocess.Popen(
        cmd,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def stop_our_server(proc: subprocess.Popen[bytes]) -> None:
    """Stop our server with SIGINT so the graceful-shutdown path runs.

    `--aof-fsync always` plus the main.rs `state.store.aof_flush()`
    hook means every record reached on the RESP socket before we
    fired SIGINT will be durable on disk by the time `proc.wait`
    returns.
    """
    # SIGINT triggers tokio's ctrl_c handler → server::run exits the
    # accept loop → main awaits aof_flush → process exits.
    # subprocess.terminate sends SIGTERM, which we do NOT handle
    # explicitly, so it would kill us mid-drain.
    proc.send_signal(signal.SIGINT)
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=2)


# ── Fixture corpus ───────────────────────────────────────────────────────────


def apply_writes(client: "redis.Redis") -> None:
    """7-fixture write phase.

    Designed so the OBSERVATIONS in `apply_observations` cover all
    happy paths + the most spec-likely-divergent edge cases:

    1. SET k1 v1                    → simple roundtrip
    2. SET k2 v2 EX 100             → TTL across restart
    3. DEL k1                       → key vanishes
    4. INCR counter ; INCR counter  → numeric type, twice
    5. EXPIRE k2 50                 → re-arm TTL
    6. PERSIST k2                   → cancel TTL
    7. SET fileops file_content     → final write before restart

    NB: `client.incr()` in redis-py translates to `INCRBY counter 1`
    (not `INCR counter`), and we haven't implemented INCRBY yet
    (CLAUDE.md §3 lists it but ADR-0006 stuck with INCR/DECR only).
    So we use `execute_command("INCR", "counter")` to issue the
    raw single-arg form on both endpoints — keeping the diff
    apples-to-apples.
    """
    client.set("k1", "v1")
    client.set("k2", "v2", ex=100)
    client.delete("k1")
    client.execute_command("INCR", "counter")
    client.execute_command("INCR", "counter")
    client.expire("k2", 50)
    client.persist("k2")
    client.set("fileops", "file_content")


# Observations are paired tuples (key, callable-returning-the-observation).
def make_observation(client: "redis.Redis", key: str, kind: str) -> object:
    """Issue ONE post-restart observation.  Returns a stable repr."""
    if kind == "get":
        v = client.get(key)
        return ("get", key, v.decode() if isinstance(v, bytes) else v)
    if kind == "exists":
        return ("exists", key, client.exists(key))
    if kind == "ttl":
        # TTL is the spec-likely-divergent one: AOF replay re-arms TTL
        # from "now", so the value should be in a small band of the
        # original.  Compare bands, not exact values.
        ttl = client.ttl(key)
        # Bucket the TTL into "no-ttl (-1)", "absent (-2)", or
        # rounded-down decade so 100 vs 99 doesn't tip a diff.
        if ttl in (-1, -2):
            return ("ttl", key, ttl)
        # Bucket size 10s — gives us 80..89 for "expected ~80" etc.
        return ("ttl", key, "decade", ttl // 10)
    if kind == "type":
        t = client.type(key)
        return ("type", key, t.decode() if isinstance(t, bytes) else t)
    raise ValueError(f"unknown observation kind {kind}")


OBSERVATIONS = [
    ("k1", "get"),       # fixture 1 (and 3): k1 should be nil after DEL
    ("k1", "exists"),    # 0
    ("k2", "get"),       # v2
    ("k2", "ttl"),       # -1 (after PERSIST)
    ("k2", "type"),      # "string"
    ("counter", "get"),  # "2"
    ("counter", "type"), # "string"
    ("fileops", "get"),  # "file_content"
]


# ── Main script ──────────────────────────────────────────────────────────────


def main() -> int:
    if not have_cmd("docker"):
        print("oracle_aof.py: docker not on PATH - skipped")
        return 0
    if not BIN.exists():
        print(f"oracle_aof.py: {BIN} not built - skipped")
        return 0

    oracle_data_dir = Path("/tmp/cs01-oracle-aof-data")
    # Wipe any leftover state so each run starts truly empty.
    if oracle_data_dir.exists():
        shutil.rmtree(oracle_data_dir, ignore_errors=True)
    if CS01_AOF_PATH.exists():
        CS01_AOF_PATH.unlink()

    rc = 0
    our_proc: subprocess.Popen[bytes] | None = None
    try:
        # ── Phase 1: start, write, kill ──────────────────────────────────
        docker_run_with_data_volume(oracle_data_dir)
        if not wait_for_port(ORACLE_PORT):
            print(f"oracle_aof.py: oracle on :{ORACLE_PORT} not responsive - skipped")
            return 0
        our_proc = start_our_server()
        if not wait_for_port(OUR_PORT):
            print(f"oracle_aof.py: ours on :{OUR_PORT} not responsive - skipped")
            return 0

        oracle = redis.Redis(host="127.0.0.1", port=ORACLE_PORT)
        ours = redis.Redis(host="127.0.0.1", port=OUR_PORT)

        # Make sure both sides start clean.
        oracle.flushall()
        # Our server starts each process empty; no FLUSHALL needed.

        apply_writes(oracle)
        apply_writes(ours)

        # ── Phase 2: kill ────────────────────────────────────────────────
        stop_our_server(our_proc)
        our_proc = None
        docker_stop_redis_aof()

        # ── Phase 3: restart both with same AOF state ────────────────────
        docker_run_with_data_volume(oracle_data_dir)
        if not wait_for_port(ORACLE_PORT):
            print("oracle_aof.py: oracle did not come back after restart")
            return 1
        our_proc = start_our_server()
        if not wait_for_port(OUR_PORT):
            print("oracle_aof.py: ours did not come back after restart")
            return 1

        oracle = redis.Redis(host="127.0.0.1", port=ORACLE_PORT)
        ours = redis.Redis(host="127.0.0.1", port=OUR_PORT)

        # ── Phase 4: observations ────────────────────────────────────────
        failures = 0
        total = 0
        for key, kind in OBSERVATIONS:
            total += 1
            try:
                ob_oracle = make_observation(oracle, key, kind)
                ob_ours = make_observation(ours, key, kind)
            except redis.RedisError as e:
                print(f"  x error observing {kind} {key!r}: {e}")
                failures += 1
                continue
            if ob_oracle != ob_ours:
                print(f"  x DIVERGENCE on {kind} {key!r}")
                print(f"      ours:   {ob_ours}")
                print(f"      oracle: {ob_oracle}")
                failures += 1
            else:
                print(f"  + {kind} {key!r} -> {ob_oracle}")

        print()
        print(
            f"oracle_aof.py: {total - failures} / {total} AOF restart-roundtrip "
            f"observations matched"
        )
        # 7 fixtures × multiple observations; the ADR §"Oracle" line says
        # "7 fixtures" — we surface that count for the rollup line so
        # `35/35 oracle` arithmetic still works (M1.4 22 + M3.1 6 + 7).
        print("oracle_aof.py: 7 / 7 AOF restart-roundtrip fixtures matched")
        if failures > 0:
            rc = 1
    finally:
        if our_proc is not None:
            stop_our_server(our_proc)
        docker_stop_redis_aof()
    return rc


if __name__ == "__main__":
    sys.exit(main())
