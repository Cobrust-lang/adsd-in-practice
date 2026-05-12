//! Git object model: Blob / Tree / Commit / Tag.
//!
//! M1 implements Git-compatible blob identity plus zlib loose-object IO.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use crate::{Error, Result};

/// Object kind. v0.1.0 M1 pretty-prints Blob; Tree/Commit/Tag parsing is reserved for later waves.
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

impl std::str::FromStr for Kind {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "blob" => Ok(Self::Blob),
            "tree" => Ok(Self::Tree),
            "commit" => Ok(Self::Commit),
            "tag" => Ok(Self::Tag),
            other => Err(Error::UnsupportedKind(other.to_owned())),
        }
    }
}

/// A decoded loose object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedObject {
    pub kind: Kind,
    pub payload: Vec<u8>,
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

/// Return the filesystem path for a loose object under `<mg_dir>/objects`.
#[must_use]
pub fn loose_path(mg_dir: &Path, sha: &str) -> PathBuf {
    let (prefix, suffix) = sha.split_at(2);
    mg_dir.join("objects").join(prefix).join(suffix)
}

/// Write a zlib-compressed loose object to `.mg/objects/<aa>/<bb..>`.
///
/// Returns the SHA-1 object ID.
pub fn write_loose(kind: Kind, payload: &[u8], mg_dir: &Path) -> Result<String> {
    let object_bytes = header_payload(kind, payload);
    let sha = crate::hash::sha1_hex(&object_bytes);
    let path = loose_path(mg_dir, &sha);
    let parent = path
        .parent()
        .ok_or_else(|| Error::InvalidObject("loose object path has no parent".to_owned()))?;
    fs::create_dir_all(parent)?;

    if path.exists() {
        return Ok(sha);
    }

    let mut zlib_writer = ZlibEncoder::new(Vec::new(), Compression::default());
    zlib_writer.write_all(&object_bytes)?;
    let compressed = zlib_writer.finish()?;
    fs::write(path, compressed)?;
    Ok(sha)
}

/// Read and validate a zlib-compressed loose object by SHA-1.
pub fn read_loose(mg_dir: &Path, sha: &str) -> Result<DecodedObject> {
    validate_sha1_hex(sha)?;
    let path = loose_path(mg_dir, sha);
    let file = File::open(path)?;
    let mut decoder = ZlibDecoder::new(file);
    let mut encoded = Vec::new();
    decoder.read_to_end(&mut encoded)?;

    let actual = crate::hash::sha1_hex(&encoded);
    if actual != sha {
        return Err(Error::HashMismatch {
            expected: sha.to_owned(),
            actual,
        });
    }

    decode(&encoded)
}

/// Decode `kind size\0payload` bytes after zlib inflation.
pub fn decode(encoded: &[u8]) -> Result<DecodedObject> {
    let nul = encoded
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| Error::InvalidObject("object header missing NUL separator".to_owned()))?;
    let header = std::str::from_utf8(&encoded[..nul])
        .map_err(|_| Error::InvalidObject("object header is not UTF-8".to_owned()))?;
    let (kind_raw, size_raw) = header
        .split_once(' ')
        .ok_or_else(|| Error::InvalidObject("object header missing kind/size split".to_owned()))?;
    let kind = kind_raw.parse::<Kind>()?;
    let declared_size = size_raw
        .parse::<usize>()
        .map_err(|_| Error::InvalidObject("object size is not decimal usize".to_owned()))?;
    let payload = encoded[nul + 1..].to_vec();
    if payload.len() != declared_size {
        return Err(Error::InvalidObject(format!(
            "object size mismatch: declared {declared_size}, actual {}",
            payload.len()
        )));
    }
    Ok(DecodedObject { kind, payload })
}

/// A regular-file tree entry for flat canonical tree objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub mode: u32,
    pub name: String,
    pub object_id: [u8; 20],
}

impl TreeEntry {
    /// Construct a tree entry from an index entry.
    pub fn from_index_entry(entry: &crate::index::Entry) -> Result<Self> {
        let name = entry
            .path
            .to_str()
            .ok_or_else(|| Error::InvalidObject("tree paths must be UTF-8 in M2".to_owned()))?
            .to_owned();
        if name.is_empty() || name.contains('/') || name.as_bytes().contains(&0) {
            return Err(Error::InvalidObject(
                "M2 tree entries support only flat non-empty filenames".to_owned(),
            ));
        }
        Ok(Self {
            mode: entry.mode,
            name,
            object_id: entry.object_id,
        })
    }
}

/// Encode a flat canonical tree payload as `<mode> <name>\0<raw 20-byte oid>` entries.
pub fn tree_payload(entries: &[TreeEntry]) -> Result<Vec<u8>> {
    let mut sorted = entries.to_vec();
    sorted.sort_by_key(tree_sort_key);

    let mut out = Vec::new();
    for entry in &sorted {
        if !matches!(entry.mode, 0o100_644 | 0o100_755) {
            return Err(Error::InvalidObject(format!(
                "unsupported tree mode {mode:o}; M2 supports regular files only",
                mode = entry.mode
            )));
        }
        if entry.name.is_empty() || entry.name.contains('/') || entry.name.as_bytes().contains(&0) {
            return Err(Error::InvalidObject(
                "M2 tree entries support only flat non-empty filenames".to_owned(),
            ));
        }
        out.extend_from_slice(format!("{:o} {}", entry.mode, entry.name).as_bytes());
        out.push(0);
        out.extend_from_slice(&entry.object_id);
    }
    Ok(out)
}

fn tree_sort_key(entry: &TreeEntry) -> Vec<u8> {
    entry.name.as_bytes().to_vec()
}

fn validate_sha1_hex(sha: &str) -> Result<()> {
    if sha.len() != 40 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidObject(
            "expected a 40-character SHA-1 hex object ID".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_hash_empty_matches_git_fixture() {
        assert_eq!(
            hash(Kind::Blob, b""),
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
        );
    }

    #[test]
    fn blob_hash_hello_without_newline_matches_git_fixture() {
        assert_eq!(
            hash(Kind::Blob, b"hello"),
            "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0"
        );
    }

    #[test]
    fn decode_round_trips_blob_payload() {
        let decoded = decode(&header_payload(Kind::Blob, b"hello\n"))
            .expect("valid blob header should decode");
        assert_eq!(decoded.kind, Kind::Blob);
        assert_eq!(decoded.payload, b"hello\n");
    }

    #[test]
    fn write_and_read_loose_blob_round_trip() {
        let tmp = std::env::temp_dir().join(format!("mg-core-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("objects")).expect("test temp object dir should be created");
        let sha = write_loose(Kind::Blob, b"hello", &tmp).expect("loose write should succeed");
        assert_eq!(sha, "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0");
        assert!(loose_path(&tmp, &sha).is_file());
        let decoded = read_loose(&tmp, &sha).expect("loose read should succeed");
        assert_eq!(decoded.kind, Kind::Blob);
        assert_eq!(decoded.payload, b"hello");
        fs::remove_dir_all(&tmp).expect("test temp object dir should be removed");
    }

    #[test]
    fn tree_payload_uses_raw_object_ids() {
        let object_id = [0x11; 20];
        let payload = tree_payload(&[TreeEntry {
            mode: 0o100_644,
            name: "a.txt".to_owned(),
            object_id,
        }])
        .expect("tree payload should encode");
        let mut expected = b"100644 a.txt\0".to_vec();
        expected.extend_from_slice(&object_id);
        assert_eq!(payload, expected);
    }
}
