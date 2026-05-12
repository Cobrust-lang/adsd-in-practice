//! Git object model: Blob / Tree / Commit / Tag.
//!
//! M1.0 scaffold — Blob first, Tree/Commit at M2/M3.

use crate::Result;

/// Object kind. v0.1.0 supports Blob; Tree/Commit at M2/M3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Blob,
    Tree,
    Commit,
    Tag,
}

impl Kind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blob => "blob",
            Self::Tree => "tree",
            Self::Commit => "commit",
            Self::Tag => "tag",
        }
    }
}

/// Serialize an object kind + payload into the `kind size\0payload` form
/// that git hashes.
///
/// Header format: `b"<kind> <ascii decimal len>\0<payload>"`.
#[must_use]
pub fn header_payload(kind: Kind, payload: &[u8]) -> Vec<u8> {
    let header = format!("{} {}\0", kind.as_str(), payload.len());
    let mut out = Vec::with_capacity(header.len() + payload.len());
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(payload);
    out
}

/// Compute the object's SHA-1 over `kind size\0payload`.
#[must_use]
pub fn hash(kind: Kind, payload: &[u8]) -> String {
    crate::hash::sha1_hex(&header_payload(kind, payload))
}

/// Loose object write path:
/// 1. compute hash of `header + payload`
/// 2. zlib-compress `header + payload`
/// 3. write to `.mg/objects/<aa>/<bb..>` (first two hex chars / rest)
///
/// M1.1 stub — implementation lands at M1.1.
///
/// # Errors
///
/// Returns IO error if `.mg/objects/` cannot be written.
pub fn write_loose(_kind: Kind, _payload: &[u8], _mg_dir: &std::path::Path) -> Result<String> {
    Err(crate::Error::InvalidObject(
        "write_loose not yet implemented (M1.1)",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_hash_hello() {
        // Same fixture as hash::tests::sha1_known_blob,
        // but going through the object header construction.
        assert_eq!(
            hash(Kind::Blob, b"hello"),
            "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0"
        );
    }
}
