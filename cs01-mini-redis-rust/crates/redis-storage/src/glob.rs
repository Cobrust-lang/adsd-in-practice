//! Redis-flavoured glob pattern matcher (ADR-0006).
//!
//! Pure byte-level matcher modelled after Redis' `stringmatchlen.c` —
//! a single-string glob (NOT path-segment aware like `std::path::Pattern`).
//!
//! Supported syntax:
//!
//! - `*`        — match any sequence of bytes (possibly empty), including `/`
//! - `?`        — match exactly one byte
//! - `[abc]`    — inclusive character class (single byte)
//! - `[a-z]`    — inclusive range inside a class
//! - `[^abc]`   — negated class (anything NOT in the set)
//! - `\X`       — literal `X` (escape; `X` may be any glob metacharacter)
//! - any other byte — matched literally
//!
//! NOT supported (documented for completeness):
//!
//! - `\d` / `\w` / `\s` shorthands (real Redis doesn't have them either)
//! - POSIX classes like `[:alpha:]`
//! - `**` (path globbing has no meaning here)
//!
//! Allocation discipline (CLAUDE.md §3.3): the inner loop walks the
//! pattern and key as byte slices only — no `String` allocation.

/// Match `key` against the Redis-style glob `pattern`.
///
/// Operates on raw bytes (UTF-8 / arbitrary bytes both fine).
///
/// Time: O(pattern_len * key_len) worst case for nested `*` patterns,
/// O(pattern_len + key_len) typical (the same backtracking strategy
/// Redis itself uses; KEYS is documented as best-effort glob).
#[must_use]
pub fn matches(pattern: &[u8], key: &[u8]) -> bool {
    match_impl(pattern, key)
}

/// Inner recursive matcher.
///
/// Recursion only on `*` (one level per star); other arms tail-iterate
/// by re-slicing.  The recursion depth is bounded by the number of `*`
/// characters in the pattern, not by the key length.
fn match_impl<'a>(mut pattern: &'a [u8], mut key: &'a [u8]) -> bool {
    loop {
        match pattern.first() {
            None => return key.is_empty(),
            Some(&b'*') => return match_star(pattern, key),
            Some(&b'?') => {
                if key.is_empty() {
                    return false;
                }
                pattern = &pattern[1..];
                key = &key[1..];
            }
            Some(&b'[') => match match_class(pattern, key) {
                None => return false,
                Some((next_pat, next_key)) => {
                    pattern = next_pat;
                    key = next_key;
                }
            },
            Some(&b'\\') => match match_escape(pattern, key) {
                None => return false,
                Some((next_pat, next_key)) => {
                    pattern = next_pat;
                    key = next_key;
                }
            },
            Some(&b) => {
                if key.first() != Some(&b) {
                    return false;
                }
                pattern = &pattern[1..];
                key = &key[1..];
            }
        }
    }
}

/// Handle the `*` arm.  Collapses runs of `*` and tries each possible
/// suffix split.  Tail-recursion via direct recursive call — depth ≤
/// number of `*` chars in the pattern.
fn match_star(mut pattern: &[u8], key: &[u8]) -> bool {
    while pattern.first() == Some(&b'*') {
        pattern = &pattern[1..];
    }
    if pattern.is_empty() {
        return true;
    }
    for i in 0..=key.len() {
        if match_impl(pattern, &key[i..]) {
            return true;
        }
    }
    false
}

/// Handle the `[...]` character class arm.  Returns `Some((next_pattern,
/// next_key))` on a successful one-byte match, or `None` if either the
/// class is malformed (no closing `]`) or the current key byte fails
/// the membership test.
fn match_class<'a>(pattern: &'a [u8], key: &'a [u8]) -> Option<(&'a [u8], &'a [u8])> {
    if key.is_empty() {
        return None;
    }
    let kb = key[0];
    let mut cursor = 1; // skip the '['
    let negate = pattern.get(cursor) == Some(&b'^');
    if negate {
        cursor += 1;
    }
    let mut in_set = false;
    let mut closed = false;
    while cursor < pattern.len() {
        let b = pattern[cursor];
        if b == b']' {
            closed = true;
            cursor += 1;
            break;
        }
        // `\X` inside a class — literal X.
        if b == b'\\' && cursor + 1 < pattern.len() {
            let esc = pattern[cursor + 1];
            if kb == esc {
                in_set = true;
            }
            cursor += 2;
            continue;
        }
        // Range `a-z` (only if `-` is not immediately followed by `]`).
        if cursor + 2 < pattern.len()
            && pattern[cursor + 1] == b'-'
            && pattern[cursor + 2] != b']'
        {
            let (lo, hi) = if b <= pattern[cursor + 2] {
                (b, pattern[cursor + 2])
            } else {
                (pattern[cursor + 2], b)
            };
            if kb >= lo && kb <= hi {
                in_set = true;
            }
            cursor += 3;
            continue;
        }
        if kb == b {
            in_set = true;
        }
        cursor += 1;
    }
    if !closed {
        return None;
    }
    let matched = if negate { !in_set } else { in_set };
    if matched {
        Some((&pattern[cursor..], &key[1..]))
    } else {
        None
    }
}

/// Handle `\X` escape outside a class.  A lone trailing `\` at end of
/// pattern is treated as literal `\` (mirrors Redis' lenient behaviour).
fn match_escape<'a>(pattern: &'a [u8], key: &'a [u8]) -> Option<(&'a [u8], &'a [u8])> {
    if pattern.len() == 1 {
        if key.first() == Some(&b'\\') {
            Some((&pattern[1..], &key[1..]))
        } else {
            None
        }
    } else {
        let lit = pattern[1];
        if key.first() == Some(&lit) {
            Some((&pattern[2..], &key[1..]))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::matches;

    // ── Literals ─────────────────────────────────────────────────────────────

    #[test]
    fn empty_pattern_matches_empty_key() {
        assert!(matches(b"", b""));
    }

    #[test]
    fn empty_pattern_rejects_nonempty_key() {
        assert!(!matches(b"", b"x"));
    }

    #[test]
    fn literal_exact_match() {
        assert!(matches(b"foo", b"foo"));
    }

    #[test]
    fn literal_mismatch() {
        assert!(!matches(b"foo", b"bar"));
    }

    #[test]
    fn literal_length_mismatch() {
        assert!(!matches(b"foo", b"foobar"));
        assert!(!matches(b"foobar", b"foo"));
    }

    // ── Star ─────────────────────────────────────────────────────────────────

    #[test]
    fn star_matches_anything() {
        assert!(matches(b"*", b""));
        assert!(matches(b"*", b"anything goes"));
        assert!(matches(b"*", b"/path/with/slashes"));
    }

    #[test]
    fn prefix_star() {
        assert!(matches(b"user:*", b"user:"));
        assert!(matches(b"user:*", b"user:42"));
        assert!(!matches(b"user:*", b"users:42"));
    }

    #[test]
    fn suffix_star() {
        assert!(matches(b"*end", b"end"));
        assert!(matches(b"*end", b"the-end"));
        assert!(!matches(b"*end", b"endpoint"));
    }

    #[test]
    fn collapsed_stars_equal_one() {
        assert!(matches(b"**", b"anything"));
        assert!(matches(b"a**b", b"axxxb"));
    }

    // ── Question mark ────────────────────────────────────────────────────────

    #[test]
    fn question_matches_exactly_one() {
        assert!(matches(b"?", b"a"));
        assert!(!matches(b"?", b""));
        assert!(!matches(b"?", b"ab"));
    }

    #[test]
    fn question_in_middle() {
        assert!(matches(b"user:?", b"user:7"));
        assert!(!matches(b"user:?", b"user:42"));
    }

    // ── Character class ──────────────────────────────────────────────────────

    #[test]
    fn class_single_byte_in_set() {
        assert!(matches(b"[abc]", b"a"));
        assert!(matches(b"[abc]", b"b"));
        assert!(!matches(b"[abc]", b"d"));
    }

    #[test]
    fn class_range() {
        assert!(matches(b"[a-z]", b"m"));
        assert!(!matches(b"[a-z]", b"A"));
        assert!(matches(b"[0-9]", b"7"));
    }

    #[test]
    fn class_negated() {
        assert!(!matches(b"[^abc]", b"a"));
        assert!(matches(b"[^abc]", b"z"));
    }

    #[test]
    fn class_combined_with_literal_prefix() {
        assert!(matches(b"[abc]*", b"apple"));
        assert!(!matches(b"[abc]*", b"zebra"));
    }

    // ── Escape ───────────────────────────────────────────────────────────────

    #[test]
    fn escape_literal_star() {
        assert!(matches(b"\\*", b"*"));
        assert!(!matches(b"\\*", b"a"));
    }

    #[test]
    fn escape_literal_question() {
        assert!(matches(b"\\?", b"?"));
        assert!(!matches(b"\\?", b"x"));
    }

    #[test]
    fn escape_literal_bracket() {
        assert!(matches(b"\\[", b"["));
    }

    #[test]
    fn escape_inside_class() {
        // Inside a class, `\]` is a literal `]`.
        assert!(matches(b"[\\]]", b"]"));
    }

    // ── Pathological cases ───────────────────────────────────────────────────

    #[test]
    fn multiple_stars_around_literal() {
        assert!(matches(b"*foo*", b"hellofoothere"));
        assert!(matches(b"*foo*", b"foo"));
        assert!(!matches(b"*foo*", b"hello"));
    }

    #[test]
    fn malformed_class_no_close() {
        // Pattern `[abc` (no `]`) — strict mode: reject.
        assert!(!matches(b"[abc", b"a"));
    }

    #[test]
    fn star_matches_zero_chars() {
        assert!(matches(b"a*b", b"ab"));
    }

    #[test]
    fn ascii_byte_semantics_not_utf8() {
        // Glob is byte-oriented; high bytes are matched verbatim.
        // Two bytes of a multibyte UTF-8 sequence count as two `?` matches.
        let key = "ä".as_bytes(); // 0xc3 0xa4 — 2 bytes
        assert!(matches(b"??", key));
    }
}
