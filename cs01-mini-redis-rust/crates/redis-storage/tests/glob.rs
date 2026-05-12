//! Integration tests for `redis_storage::glob::matches`.
//!
//! ADR-0006 §"KEYS glob 实现" — covers `*` / `?` / `[a-z]` / `\` escape
//! plus negated char class.  ≥ 20 cases including pathological inputs.

use redis_storage::glob::matches;

// ── Star ─────────────────────────────────────────────────────────────────────

#[test]
fn star_alone_matches_anything_including_empty() {
    assert!(matches(b"*", b""));
    assert!(matches(b"*", b"x"));
    assert!(matches(b"*", b"hello world"));
}

#[test]
fn star_includes_slashes_unlike_path_glob() {
    // Redis KEYS '*' is NOT path-segment aware — verify slash matches.
    assert!(matches(b"*", b"a/b/c"));
    assert!(matches(b"a*c", b"a/b/c"));
}

#[test]
fn user_prefix_pattern() {
    assert!(matches(b"user:*", b"user:1"));
    assert!(matches(b"user:*", b"user:42"));
    assert!(matches(b"user:*", b"user:"));
    assert!(!matches(b"user:*", b"users:1"));
    assert!(!matches(b"user:*", b"User:1")); // case-sensitive
}

// ── Question mark ────────────────────────────────────────────────────────────

#[test]
fn question_user_single_digit() {
    assert!(matches(b"user:?", b"user:0"));
    assert!(matches(b"user:?", b"user:Z"));
    assert!(!matches(b"user:?", b"user:42"));
    assert!(!matches(b"user:?", b"user:"));
}

#[test]
fn many_questions_must_all_match() {
    assert!(matches(b"???", b"abc"));
    assert!(!matches(b"???", b"ab"));
    assert!(!matches(b"???", b"abcd"));
}

// ── Char class ───────────────────────────────────────────────────────────────

#[test]
fn class_set_prefix() {
    assert!(matches(b"[abc]*", b"apple"));
    assert!(matches(b"[abc]*", b"banana"));
    assert!(matches(b"[abc]*", b"cherry"));
    assert!(!matches(b"[abc]*", b"date"));
}

#[test]
fn class_lowercase_range() {
    assert!(matches(b"[a-z]", b"m"));
    assert!(!matches(b"[a-z]", b"M"));
}

#[test]
fn class_digit_range() {
    for d in b'0'..=b'9' {
        assert!(matches(b"[0-9]", &[d]));
    }
    assert!(!matches(b"[0-9]", b"a"));
}

#[test]
fn class_negated() {
    assert!(matches(b"[^xyz]", b"a"));
    assert!(!matches(b"[^xyz]", b"x"));
    assert!(!matches(b"[^xyz]", b"y"));
}

#[test]
fn class_with_range_and_literal_mixed() {
    // `[a-cZ]` accepts a/b/c/Z.
    assert!(matches(b"[a-cZ]", b"a"));
    assert!(matches(b"[a-cZ]", b"b"));
    assert!(matches(b"[a-cZ]", b"c"));
    assert!(matches(b"[a-cZ]", b"Z"));
    assert!(!matches(b"[a-cZ]", b"d"));
}

#[test]
fn class_reversed_range_still_works() {
    // `[z-a]` is normalised by the matcher.
    assert!(matches(b"[z-a]", b"m"));
}

// ── Escape ───────────────────────────────────────────────────────────────────

#[test]
fn escape_star_is_literal() {
    assert!(matches(b"\\*", b"*"));
    assert!(!matches(b"\\*", b"x"));
}

#[test]
fn escape_question_is_literal() {
    assert!(matches(b"\\?", b"?"));
    assert!(!matches(b"\\?", b"x"));
}

#[test]
fn escape_bracket_is_literal() {
    assert!(matches(b"\\[", b"["));
    assert!(matches(b"\\]", b"]"));
}

#[test]
fn escape_backslash_pair_matches_backslash() {
    // \\ in a Rust byte literal is one byte 0x5c; `\\\\` is two bytes 0x5c 0x5c.
    // Pattern `\\\\` (4 bytes: \ \ \ \) → first `\X` escape: X = `\`.
    assert!(matches(b"\\\\", b"\\"));
}

// ── Compound / pathological ──────────────────────────────────────────────────

#[test]
fn star_in_middle() {
    assert!(matches(b"a*b", b"ab"));
    assert!(matches(b"a*b", b"axxxb"));
    assert!(!matches(b"a*b", b"axxx"));
}

#[test]
fn multiple_stars_collapse() {
    assert!(matches(b"a**b", b"axxxb"));
    assert!(matches(b"***", b"anything"));
}

#[test]
fn empty_pattern_only_matches_empty_key() {
    assert!(matches(b"", b""));
    assert!(!matches(b"", b"x"));
}

#[test]
fn malformed_class_returns_false() {
    // `[abc` (no `]`) — strict rejection.
    assert!(!matches(b"[abc", b"a"));
}

#[test]
fn star_at_end_after_class() {
    assert!(matches(b"[abc]*", b"apple"));
}

#[test]
fn long_pattern_long_key() {
    // Worst-case style — `*a*a*a*b` against `aaab`.
    assert!(matches(b"*a*a*a*b", b"aaab"));
    assert!(matches(b"*a*a*a*b", b"xaayaazaab"));
}

#[test]
fn case_sensitive_by_default() {
    assert!(!matches(b"foo", b"FOO"));
    assert!(matches(b"[Ff]oo", b"Foo"));
    assert!(matches(b"[Ff]oo", b"foo"));
}

#[test]
fn nonascii_bytes_pass_through_verbatim() {
    // 0xC3 0xA4 = UTF-8 'ä'. Glob is byte-oriented.
    let key = "ä".as_bytes();
    assert!(matches(b"?", &key[..1])); // single byte
    assert!(matches(b"??", key)); // two bytes
}
