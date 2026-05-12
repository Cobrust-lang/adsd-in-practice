//! Git index v2 reader/writer for the CS-02 M2 stage-0 regular-file subset.
//!
//! The on-disk format is the binary `DIRC` format used by Git, including the
//! trailing SHA-1 checksum over all preceding bytes.

use std::fs;
use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};

use crate::{Error, Result};

const SIGNATURE: &[u8; 4] = b"DIRC";
const VERSION: u32 = 2;
const SHA1_LEN: usize = 20;
const ENTRY_FIXED_LEN: usize = 62;
const FLAG_NAME_LEN_MASK: u16 = 0x0fff;
const FLAG_STAGE_MASK: u16 = 0x3000;
const REGULAR_MODE_644: u32 = 0o100_644;
const REGULAR_MODE_755: u32 = 0o100_755;

/// A stage-0 regular-file entry from a Git index v2 file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub ctime_seconds: u32,
    pub ctime_nanoseconds: u32,
    pub mtime_seconds: u32,
    pub mtime_nanoseconds: u32,
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub file_size: u32,
    pub object_id: [u8; SHA1_LEN],
    pub path: PathBuf,
}

impl Entry {
    /// Build an index entry for a flat regular worktree file.
    pub fn from_worktree_file(worktree_path: &Path, object_id_hex: &str) -> Result<Self> {
        if !is_flat_relative_path(worktree_path) {
            return Err(Error::InvalidIndex(format!(
                "M2 supports only flat repository-root file paths, got {}",
                worktree_path.display()
            )));
        }

        let metadata = fs::metadata(worktree_path)?;
        if !metadata.is_file() {
            return Err(Error::InvalidIndex(format!(
                "mg add supports regular files only in M2, got {}",
                worktree_path.display()
            )));
        }

        let object_id = decode_sha1_hex(object_id_hex)?;
        Ok(Self {
            ctime_seconds: metadata_ctime_seconds(&metadata),
            ctime_nanoseconds: metadata_ctime_nanoseconds(&metadata),
            mtime_seconds: metadata_mtime_seconds(&metadata),
            mtime_nanoseconds: metadata_mtime_nanoseconds(&metadata),
            dev: metadata_dev(&metadata),
            ino: metadata_ino(&metadata),
            mode: file_mode(&metadata),
            uid: metadata_uid(&metadata),
            gid: metadata_gid(&metadata),
            file_size: u64_to_u32_saturating(metadata.len()),
            object_id,
            path: worktree_path.to_path_buf(),
        })
    }

    /// Return the staged object ID as a lowercase hex string.
    #[must_use]
    pub fn object_id_hex(&self) -> String {
        hex::encode(self.object_id)
    }
}

/// Read a Git index v2 file, verifying its trailing SHA-1 checksum.
pub fn read(path: &Path) -> Result<Vec<Entry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(path)?;
    read_bytes(&bytes)
}

/// Write a Git index v2 file with entries sorted by Git path order.
pub fn write(path: &Path, entries: &[Entry]) -> Result<()> {
    let bytes = write_bytes(entries)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

/// Insert or replace one entry, preserving a sorted index.
#[must_use]
pub fn upsert_entry(mut entries: Vec<Entry>, entry: Entry) -> Vec<Entry> {
    entries.retain(|existing| existing.path != entry.path);
    entries.push(entry);
    entries.sort_by_key(|entry| path_bytes(&entry.path));
    entries
}

/// Decode index bytes. Exposed for tests and fuzz-style fixtures.
pub fn read_bytes(bytes: &[u8]) -> Result<Vec<Entry>> {
    if bytes.len() < 12 + SHA1_LEN {
        return Err(Error::InvalidIndex("index too short".to_owned()));
    }
    let checksum_start = bytes.len() - SHA1_LEN;
    let expected_checksum = &bytes[checksum_start..];
    let actual_checksum = Sha1::digest(&bytes[..checksum_start]);
    if expected_checksum != actual_checksum.as_slice() {
        return Err(Error::HashMismatch {
            expected: hex::encode(expected_checksum),
            actual: hex::encode(actual_checksum),
        });
    }

    if &bytes[0..4] != SIGNATURE {
        return Err(Error::InvalidIndex(
            "index signature is not DIRC".to_owned(),
        ));
    }
    let version = read_u32(bytes, 4)?;
    if version != VERSION {
        return Err(Error::InvalidIndex(format!(
            "unsupported index version {version}; M2 supports v2 only"
        )));
    }
    let entry_count = read_u32(bytes, 8)? as usize;
    let mut offset = 12usize;
    let mut entries = Vec::with_capacity(entry_count);

    for _ in 0..entry_count {
        let (entry, next_offset) = parse_entry(bytes, offset, checksum_start)?;
        entries.push(entry);
        offset = next_offset;
    }

    if offset != checksum_start {
        return Err(Error::InvalidIndex(
            "index extensions are out of scope for M2".to_owned(),
        ));
    }

    entries.sort_by_key(|entry| path_bytes(&entry.path));
    Ok(entries)
}

fn parse_entry(bytes: &[u8], mut offset: usize, checksum_start: usize) -> Result<(Entry, usize)> {
    let entry_start = offset;
    if offset + ENTRY_FIXED_LEN > checksum_start {
        return Err(Error::InvalidIndex("index entry truncated".to_owned()));
    }

    let ctime_seconds = read_u32(bytes, offset)?;
    offset += 4;
    let ctime_nanoseconds = read_u32(bytes, offset)?;
    offset += 4;
    let mtime_seconds = read_u32(bytes, offset)?;
    offset += 4;
    let mtime_nanoseconds = read_u32(bytes, offset)?;
    offset += 4;
    let dev = read_u32(bytes, offset)?;
    offset += 4;
    let ino = read_u32(bytes, offset)?;
    offset += 4;
    let mode = read_u32(bytes, offset)?;
    offset += 4;
    validate_regular_mode(mode)?;
    let uid = read_u32(bytes, offset)?;
    offset += 4;
    let gid = read_u32(bytes, offset)?;
    offset += 4;
    let file_size = read_u32(bytes, offset)?;
    offset += 4;

    let object_id_slice = bytes
        .get(offset..offset + SHA1_LEN)
        .ok_or_else(|| Error::InvalidIndex("index object id truncated".to_owned()))?;
    let object_id: [u8; SHA1_LEN] = object_id_slice
        .try_into()
        .map_err(|_| Error::InvalidIndex("invalid object id length".to_owned()))?;
    offset += SHA1_LEN;

    let flags = read_u16(bytes, offset)?;
    offset += 2;
    let declared_path_len = validate_stage0_flags(flags)?;
    let (path, path_end) = parse_path(bytes, offset, checksum_start, declared_path_len)?;
    offset = path_end + 1;
    offset = skip_padding(bytes, entry_start, offset, checksum_start)?;

    Ok((
        Entry {
            ctime_seconds,
            ctime_nanoseconds,
            mtime_seconds,
            mtime_nanoseconds,
            dev,
            ino,
            mode,
            uid,
            gid,
            file_size,
            object_id,
            path,
        },
        offset,
    ))
}

fn validate_regular_mode(mode: u32) -> Result<()> {
    if !matches!(mode, REGULAR_MODE_644 | REGULAR_MODE_755) {
        return Err(Error::InvalidIndex(format!(
            "unsupported index mode {mode:o}; M2 supports regular files only"
        )));
    }
    Ok(())
}

fn validate_stage0_flags(flags: u16) -> Result<usize> {
    if flags & FLAG_STAGE_MASK != 0 {
        return Err(Error::InvalidIndex(
            "M2 supports only stage-0 index entries".to_owned(),
        ));
    }
    let declared_path_len = usize::from(flags & FLAG_NAME_LEN_MASK);
    if declared_path_len == usize::from(FLAG_NAME_LEN_MASK) {
        return Err(Error::InvalidIndex(
            "M2 does not support >=4095-byte index paths".to_owned(),
        ));
    }
    Ok(declared_path_len)
}

fn parse_path(
    bytes: &[u8],
    offset: usize,
    checksum_start: usize,
    declared_path_len: usize,
) -> Result<(PathBuf, usize)> {
    let nul_pos = bytes[offset..checksum_start]
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| Error::InvalidIndex("index path missing NUL terminator".to_owned()))?;
    let path_end = offset + nul_pos;
    let raw_path = &bytes[offset..path_end];
    if raw_path.len() != declared_path_len {
        return Err(Error::InvalidIndex(format!(
            "index path length mismatch: flag {declared_path_len}, actual {}",
            raw_path.len()
        )));
    }
    let path = decode_path(raw_path)?;
    if !is_flat_relative_path(&path) {
        return Err(Error::InvalidIndex(format!(
            "M2 supports only flat index paths, got {}",
            path.display()
        )));
    }
    Ok((path, path_end))
}

fn skip_padding(
    bytes: &[u8],
    entry_start: usize,
    offset: usize,
    checksum_start: usize,
) -> Result<usize> {
    let consumed = offset - entry_start;
    let padded_len = align8(consumed);
    let padding = padded_len - consumed;
    if offset + padding > checksum_start {
        return Err(Error::InvalidIndex(
            "index entry padding truncated".to_owned(),
        ));
    }
    if bytes[offset..offset + padding]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(Error::InvalidIndex("index padding is not NUL".to_owned()));
    }
    Ok(offset + padding)
}

/// Encode index bytes with trailing SHA-1 checksum.
pub fn write_bytes(entries: &[Entry]) -> Result<Vec<u8>> {
    let mut sorted = entries.to_vec();
    sorted.sort_by_key(|entry| path_bytes(&entry.path));

    let entry_count = u32::try_from(sorted.len())
        .map_err(|_| Error::InvalidIndex("too many index entries".to_owned()))?;
    let mut out = Vec::new();
    out.extend_from_slice(SIGNATURE);
    push_u32(&mut out, VERSION);
    push_u32(&mut out, entry_count);

    for entry in &sorted {
        encode_entry(&mut out, entry)?;
    }

    let checksum = Sha1::digest(&out);
    out.extend_from_slice(&checksum);
    Ok(out)
}

fn encode_entry(out: &mut Vec<u8>, entry: &Entry) -> Result<()> {
    if !matches!(entry.mode, REGULAR_MODE_644 | REGULAR_MODE_755) {
        return Err(Error::InvalidIndex(format!(
            "unsupported index mode {mode:o}; M2 supports regular files only",
            mode = entry.mode
        )));
    }
    if !is_flat_relative_path(&entry.path) {
        return Err(Error::InvalidIndex(format!(
            "M2 supports only flat index paths, got {}",
            entry.path.display()
        )));
    }
    let path = path_bytes(&entry.path);
    if path.len() >= usize::from(FLAG_NAME_LEN_MASK) {
        return Err(Error::InvalidIndex(
            "M2 does not support >=4095-byte index paths".to_owned(),
        ));
    }

    let entry_start = out.len();
    push_u32(out, entry.ctime_seconds);
    push_u32(out, entry.ctime_nanoseconds);
    push_u32(out, entry.mtime_seconds);
    push_u32(out, entry.mtime_nanoseconds);
    push_u32(out, entry.dev);
    push_u32(out, entry.ino);
    push_u32(out, entry.mode);
    push_u32(out, entry.uid);
    push_u32(out, entry.gid);
    push_u32(out, entry.file_size);
    out.extend_from_slice(&entry.object_id);
    let flags = u16::try_from(path.len())
        .map_err(|_| Error::InvalidIndex("index path length exceeds u16".to_owned()))?;
    push_u16(out, flags);
    out.extend_from_slice(&path);
    out.push(0);

    let consumed = out.len() - entry_start;
    let padded_len = align8(consumed);
    out.resize(entry_start + padded_len, 0);
    Ok(())
}

fn align8(len: usize) -> usize {
    (len + 7) & !7
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| Error::InvalidIndex("u32 field truncated".to_owned()))?;
    let array: [u8; 4] = raw
        .try_into()
        .map_err(|_| Error::InvalidIndex("invalid u32 field length".to_owned()))?;
    Ok(u32::from_be_bytes(array))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| Error::InvalidIndex("u16 field truncated".to_owned()))?;
    let array: [u8; 2] = raw
        .try_into()
        .map_err(|_| Error::InvalidIndex("invalid u16 field length".to_owned()))?;
    Ok(u16::from_be_bytes(array))
}

fn decode_sha1_hex(sha: &str) -> Result<[u8; SHA1_LEN]> {
    let bytes = hex::decode(sha).map_err(|_| {
        Error::InvalidObject("expected a lowercase 40-character SHA-1 hex object ID".to_owned())
    })?;
    if bytes.len() != SHA1_LEN {
        return Err(Error::InvalidObject(
            "expected a 40-character SHA-1 hex object ID".to_owned(),
        ));
    }
    bytes
        .try_into()
        .map_err(|_| Error::InvalidObject("invalid SHA-1 byte length".to_owned()))
}

fn decode_path(path: &[u8]) -> Result<PathBuf> {
    if path.is_empty() || path.iter().any(|byte| *byte == b'/' || *byte == 0) {
        return Err(Error::InvalidIndex(
            "M2 supports only non-empty flat paths without slash".to_owned(),
        ));
    }
    let value = std::str::from_utf8(path)
        .map_err(|_| Error::InvalidIndex("M2 supports UTF-8 index paths only".to_owned()))?;
    Ok(PathBuf::from(value))
}

fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}

fn is_flat_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path.is_relative()
        && path.components().count() == 1
        && !path_bytes(path).contains(&0)
}

#[cfg(unix)]
fn metadata_ctime_seconds(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    i64_to_u32_saturating(metadata.ctime())
}

#[cfg(not(unix))]
fn metadata_ctime_seconds(_metadata: &fs::Metadata) -> u32 {
    0
}

#[cfg(unix)]
fn metadata_ctime_nanoseconds(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    i64_to_u32_saturating(metadata.ctime_nsec())
}

#[cfg(not(unix))]
fn metadata_ctime_nanoseconds(_metadata: &fs::Metadata) -> u32 {
    0
}

#[cfg(unix)]
fn metadata_mtime_seconds(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    i64_to_u32_saturating(metadata.mtime())
}

#[cfg(not(unix))]
fn metadata_mtime_seconds(metadata: &fs::Metadata) -> u32 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |duration| {
            duration.as_secs().min(u64::from(u32::MAX)) as u32
        })
}

#[cfg(unix)]
fn metadata_mtime_nanoseconds(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    i64_to_u32_saturating(metadata.mtime_nsec())
}

#[cfg(not(unix))]
fn metadata_mtime_nanoseconds(metadata: &fs::Metadata) -> u32 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.subsec_nanos())
}

#[cfg(unix)]
fn metadata_dev(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    u64_to_u32_saturating(metadata.dev())
}

#[cfg(not(unix))]
fn metadata_dev(_metadata: &fs::Metadata) -> u32 {
    0
}

#[cfg(unix)]
fn metadata_ino(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    u64_to_u32_saturating(metadata.ino())
}

#[cfg(not(unix))]
fn metadata_ino(_metadata: &fs::Metadata) -> u32 {
    0
}

#[cfg(unix)]
fn metadata_uid(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.uid()
}

#[cfg(not(unix))]
fn metadata_uid(_metadata: &fs::Metadata) -> u32 {
    0
}

#[cfg(unix)]
fn metadata_gid(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.gid()
}

#[cfg(not(unix))]
fn metadata_gid(_metadata: &fs::Metadata) -> u32 {
    0
}

#[cfg(unix)]
fn file_mode(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    if metadata.permissions().mode() & 0o111 == 0 {
        REGULAR_MODE_644
    } else {
        REGULAR_MODE_755
    }
}

#[cfg(not(unix))]
fn file_mode(_metadata: &fs::Metadata) -> u32 {
    REGULAR_MODE_644
}

fn u64_to_u32_saturating(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn i64_to_u32_saturating(value: i64) -> u32 {
    u32::try_from(value).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_round_trip_preserves_entry_and_checksum() {
        let entry = Entry {
            ctime_seconds: 1,
            ctime_nanoseconds: 2,
            mtime_seconds: 3,
            mtime_nanoseconds: 4,
            dev: 5,
            ino: 6,
            mode: REGULAR_MODE_644,
            uid: 7,
            gid: 8,
            file_size: 9,
            object_id: [0x11; SHA1_LEN],
            path: PathBuf::from("a.txt"),
        };
        let bytes = write_bytes(std::slice::from_ref(&entry)).expect("index encode should succeed");
        let decoded = read_bytes(&bytes).expect("index decode should succeed");
        assert_eq!(decoded, vec![entry]);
    }

    #[test]
    fn index_rejects_bad_checksum() {
        let entry = Entry {
            ctime_seconds: 0,
            ctime_nanoseconds: 0,
            mtime_seconds: 0,
            mtime_nanoseconds: 0,
            dev: 0,
            ino: 0,
            mode: REGULAR_MODE_644,
            uid: 0,
            gid: 0,
            file_size: 0,
            object_id: [0x22; SHA1_LEN],
            path: PathBuf::from("b.txt"),
        };
        let mut bytes = write_bytes(&[entry]).expect("index encode should succeed");
        bytes[12] ^= 0xff;
        assert!(matches!(
            read_bytes(&bytes),
            Err(Error::HashMismatch { .. })
        ));
    }
}
