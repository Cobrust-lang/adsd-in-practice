//! Hash abstraction layer.
//!
//! v0.1.0 supports SHA-1 only (`.mg/HEAD` says `mg-version: 1, hash: sha1`).
//! SHA-256 lands at v0.2 via the same `Hasher` trait.
//!
//! Design: leave the abstraction in place from day 1 so v0.2 is a
//! type swap, not a re-architecture.

use sha1::{Digest, Sha1};

#[must_use]
pub fn sha1_hex(bytes: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_empty() {
        // sha1("") = da39a3ee5e6b4b0d3255bfef95601890afd80709
        assert_eq!(sha1_hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn sha1_known_blob() {
        // git blob "hello" = blob header + content
        // header: "blob 5\0", content: "hello"
        // sha1 of "blob 5\0hello" = b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0
        let data = b"blob 5\0hello";
        assert_eq!(sha1_hex(data), "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0");
    }
}
