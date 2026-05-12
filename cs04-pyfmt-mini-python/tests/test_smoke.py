"""M0 smoke + idempotency property test stub."""

from __future__ import annotations

from pyfmt_mini import format_source


def test_trailing_whitespace_stripped() -> None:
    src = "x = 1   \ny = 2\t \n"
    out = format_source(src)
    assert out == "x = 1\ny = 2\n"


def test_ends_with_newline() -> None:
    assert format_source("x = 1").endswith("\n")
    assert format_source("x = 1\n").endswith("\n")
    # Idempotent: no double newline
    out = format_source("x = 1")
    assert out.count("\n") == 1


def test_idempotent_basic() -> None:
    """M0 stub: idempotency over a tiny corpus.

    M3 will replace this with hypothesis-based ≥1000-input fuzzing.
    """
    samples = [
        "x = 1\n",
        "def f():\n    return 1\n",
        "x   = 1   \n",
    ]
    for s in samples:
        once = format_source(s)
        twice = format_source(once)
        assert once == twice, f"non-idempotent on input: {s!r}"
