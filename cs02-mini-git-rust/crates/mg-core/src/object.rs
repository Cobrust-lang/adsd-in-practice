//! Git object model: Blob / Tree / Commit / Tag.
//!
//! M3 implements Git-compatible loose-object IO plus recursive tree and commit
//! payload construction for the minimal repository workflow.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use crate::{Error, Result};

/// Object kind.
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

/// A regular-file tree entry or subtree entry for canonical tree objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub mode: u32,
    pub name: String,
    pub object_id: [u8; 20],
}

/// Commit author/committer identity used in the raw Git commit payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub name: String,
    pub email: String,
    pub date: String,
}

impl Signature {
    /// Render `Name <email> seconds timezone`.
    #[must_use]
    pub fn render(&self) -> String {
        format!("{} <{}> {}", self.name, self.email, self.date)
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

/// Write recursive tree objects for all staged entries and return the root tree SHA.
pub fn write_tree_from_index(entries: &[crate::index::Entry], mg_dir: &Path) -> Result<String> {
    let mut root = TreeNode::default();
    for entry in entries {
        root.insert(entry)?;
    }
    write_tree_node(&root, mg_dir)
}

/// Encode one canonical tree payload as `<mode> <name>\0<raw 20-byte oid>` entries.
pub fn tree_payload(entries: &[TreeEntry]) -> Result<Vec<u8>> {
    let mut sorted = entries.to_vec();
    sorted.sort_by(tree_entry_cmp);

    let mut out = Vec::new();
    for entry in &sorted {
        if !matches!(entry.mode, 0o100_644 | 0o100_755 | 0o040_000) {
            return Err(Error::InvalidObject(format!(
                "unsupported tree mode {mode:o}; M3 supports regular files and trees only",
                mode = entry.mode
            )));
        }
        if entry.name.is_empty() || entry.name.contains('/') || entry.name.as_bytes().contains(&0) {
            return Err(Error::InvalidObject(
                "tree entries require flat non-empty names".to_owned(),
            ));
        }
        out.extend_from_slice(format!("{:o} {}", entry.mode, entry.name).as_bytes());
        out.push(0);
        out.extend_from_slice(&entry.object_id);
    }
    Ok(out)
}

/// Build a raw Git commit payload.
pub fn commit_payload(
    tree: &str,
    parents: &[String],
    author: &Signature,
    committer: &Signature,
    message: &str,
) -> Result<Vec<u8>> {
    validate_sha1_hex(tree)?;
    let mut out = String::new();
    out.push_str("tree ");
    out.push_str(tree);
    out.push('\n');
    for parent in parents {
        validate_sha1_hex(parent)?;
        out.push_str("parent ");
        out.push_str(parent);
        out.push('\n');
    }
    out.push_str("author ");
    out.push_str(&author.render());
    out.push('\n');
    out.push_str("committer ");
    out.push_str(&committer.render());
    out.push_str("\n\n");
    out.push_str(message);
    if !message.ends_with('\n') {
        out.push('\n');
    }
    Ok(out.into_bytes())
}

/// Parse the first parent from a commit payload.
pub fn first_parent(payload: &[u8]) -> Result<Option<String>> {
    let text = std::str::from_utf8(payload)
        .map_err(|_| Error::InvalidObject("commit payload is not UTF-8".to_owned()))?;
    for line in text.lines() {
        if line.is_empty() {
            break;
        }
        if let Some(parent) = line.strip_prefix("parent ") {
            validate_sha1_hex(parent)?;
            return Ok(Some(parent.to_owned()));
        }
    }
    Ok(None)
}

/// Parse the subject line from a commit payload.
pub fn commit_subject(payload: &[u8]) -> Result<String> {
    let text = std::str::from_utf8(payload)
        .map_err(|_| Error::InvalidObject("commit payload is not UTF-8".to_owned()))?;
    let Some((_, message)) = text.split_once("\n\n") else {
        return Ok(String::new());
    };
    Ok(message.lines().next().unwrap_or("").to_owned())
}

/// Validate a 40-character SHA-1 hex object ID.
pub fn validate_sha1_hex(sha: &str) -> Result<()> {
    if sha.len() != 40 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidObject(
            "expected a 40-character SHA-1 hex object ID".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug, Default)]
struct TreeNode {
    files: BTreeMap<String, TreeEntry>,
    dirs: BTreeMap<String, TreeNode>,
}

impl TreeNode {
    fn insert(&mut self, entry: &crate::index::Entry) -> Result<()> {
        crate::index::validate_relative_path(&entry.path)?;
        let parts = path_parts(&entry.path)?;
        self.insert_parts(&parts, entry)
    }

    fn insert_parts(&mut self, parts: &[String], entry: &crate::index::Entry) -> Result<()> {
        match parts {
            [] => Err(Error::InvalidObject("empty tree path".to_owned())),
            [file] => {
                if self.dirs.contains_key(file) {
                    return Err(Error::InvalidObject(format!(
                        "path conflict: {file} is both file and directory"
                    )));
                }
                self.files.insert(
                    file.clone(),
                    TreeEntry {
                        mode: entry.mode,
                        name: file.clone(),
                        object_id: entry.object_id,
                    },
                );
                Ok(())
            }
            [dir, rest @ ..] => {
                if self.files.contains_key(dir) {
                    return Err(Error::InvalidObject(format!(
                        "path conflict: {dir} is both file and directory"
                    )));
                }
                self.dirs
                    .entry(dir.clone())
                    .or_default()
                    .insert_parts(rest, entry)
            }
        }
    }
}

fn write_tree_node(node: &TreeNode, mg_dir: &Path) -> Result<String> {
    let mut entries = Vec::new();
    for file in node.files.values() {
        entries.push(file.clone());
    }
    for (name, child) in &node.dirs {
        let child_sha = write_tree_node(child, mg_dir)?;
        let object_id = decode_sha1_hex(&child_sha)?;
        entries.push(TreeEntry {
            mode: 0o040_000,
            name: name.clone(),
            object_id,
        });
    }
    let payload = tree_payload(&entries)?;
    write_loose(Kind::Tree, &payload, mg_dir)
}

fn path_parts(path: &Path) -> Result<Vec<String>> {
    let text = path
        .to_str()
        .ok_or_else(|| Error::InvalidObject("tree paths must be UTF-8".to_owned()))?;
    Ok(text.split('/').map(ToOwned::to_owned).collect())
}

fn tree_entry_cmp(left: &TreeEntry, right: &TreeEntry) -> std::cmp::Ordering {
    tree_sort_name(left).cmp(&tree_sort_name(right))
}

fn tree_sort_name(entry: &TreeEntry) -> Vec<u8> {
    let mut key = entry.name.as_bytes().to_vec();
    if entry.mode == 0o040_000 {
        key.push(b'/');
    }
    key
}

fn decode_sha1_hex(sha: &str) -> Result<[u8; 20]> {
    validate_sha1_hex(sha)?;
    let bytes = hex::decode(sha).map_err(|_| {
        Error::InvalidObject("expected a lowercase 40-character SHA-1 hex object ID".to_owned())
    })?;
    bytes
        .try_into()
        .map_err(|_| Error::InvalidObject("invalid SHA-1 byte length".to_owned()))
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

    #[test]
    fn commit_payload_matches_git_header_shape() {
        let sig = Signature {
            name: "A U Thor".to_owned(),
            email: "a@example.com".to_owned(),
            date: "1700000000 +0000".to_owned(),
        };
        let payload = commit_payload(
            "0123456789abcdef0123456789abcdef01234567",
            &["89abcdef0123456789abcdef0123456789abcdef".to_owned()],
            &sig,
            &sig,
            "msg",
        )
        .expect("commit payload should encode");
        let text = String::from_utf8(payload).expect("payload should be utf8");
        assert!(text.contains("parent 89abcdef0123456789abcdef0123456789abcdef\n"));
        assert!(text.ends_with("\n\nmsg\n"));
    }
}
