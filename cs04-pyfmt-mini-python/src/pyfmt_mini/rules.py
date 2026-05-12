"""Formatting rules.

Wave M0 scaffold — minimal rule_trailing_whitespace stub.
Real rules (indent / quotes) land at M1+.
"""

from __future__ import annotations


def rule_trailing_whitespace(source: str) -> str:
    """Strip trailing whitespace from each line; ensure file ends with one newline.

    This is the only rule that's safe with naive line-level regex —
    other rules must go through `tokenize` (see CLAUDE.md §1 F24 defense).
    """
    lines = source.splitlines()
    stripped = [line.rstrip() for line in lines]
    out = "\n".join(stripped)
    if not out.endswith("\n"):
        out += "\n"
    return out


def format_source(source: str) -> str:
    """Apply all rules in order. M0 scaffold — only rule_trailing_whitespace.

    M1: + rule_indent (tokenize-based)
    M2: + rule_quotes (tokenize-based)
    """
    return rule_trailing_whitespace(source)
