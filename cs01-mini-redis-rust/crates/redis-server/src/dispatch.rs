//! RESP Frame → `Command` dispatcher (ADR-0004).
//!
//! Entry point: [`from_frame`].  Returns `Ok(Command)` on success or
//! `Err(Reply::Error(...))` for unknown commands / wrong arity — the
//! caller serialises the `Reply` back to the client without inspecting
//! the error further.
//!
//! Layer contract:
//! - This module **may** depend on `redis_protocol` (frames come from there).
//! - This module **may** depend on `redis_storage` (it produces `Command` + consumes `Reply`).
//! - `redis_storage` must **never** know about `Frame` (ADR-0004 §Consequences).

use redis_protocol::Frame;
use redis_storage::{Command, Reply};

/// Convert a RESP `Frame` into a `Command`.
///
/// The frame must be `Frame::Array(Some(parts))` where `parts[0]` is a
/// `BulkString` containing the command name (case-insensitive).
///
/// # Errors
///
/// Returns `Err(Reply::Error(...))` for:
/// - Non-array or nil-array frames
/// - Unknown command names
/// - Wrong number of arguments
pub fn from_frame(f: Frame) -> Result<Command, Reply> {
    let Frame::Array(Some(parts)) = f else {
        return Err(Reply::Error("ERR wrong arity".to_owned()));
    };

    let cmd_name =
        bulk_to_string(parts.first()).ok_or_else(|| Reply::Error("ERR wrong arity".to_owned()))?;

    match cmd_name.to_ascii_uppercase().as_str() {
        "PING" => parse_ping(&parts),
        "GET" => parse_get(&parts),
        "SET" => parse_set(&parts),
        "DEL" => parse_del(&parts),
        "EXISTS" => parse_exists(&parts),
        "INCR" => parse_single_key_cmd(&parts, "incr", |key| Command::Incr { key }),
        "DECR" => parse_single_key_cmd(&parts, "decr", |key| Command::Decr { key }),
        // ── M1.3 (ADR-0005) ─────────────────────────────────────────────
        "ECHO" => parse_echo(&parts),
        "SELECT" => parse_select(&parts),
        "QUIT" => parse_quit(&parts),
        // ── M1.4 (ADR-0006) ─────────────────────────────────────────────
        "EXPIRE" => parse_expire(&parts),
        "TTL" => parse_single_key_cmd(&parts, "ttl", |key| Command::Ttl { key }),
        "PERSIST" => parse_single_key_cmd(&parts, "persist", |key| Command::Persist { key }),
        "TYPE" => parse_single_key_cmd(&parts, "type", |key| Command::Type { key }),
        "KEYS" => parse_keys(&parts),
        unknown => Err(Reply::Error(format!("ERR unknown command '{unknown}'"))),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Extract a UTF-8 string from a `Frame::BulkString(Some(_))`.
/// Returns `None` for nil bulk strings, non-bulk frames, or invalid UTF-8.
fn bulk_to_string(frame: Option<&Frame>) -> Option<String> {
    match frame {
        Some(Frame::BulkString(Some(bytes))) => String::from_utf8(bytes.clone()).ok(),
        _ => None,
    }
}

/// Extract raw bytes from a `Frame::BulkString(Some(_))`.
fn bulk_to_bytes(frame: Option<&Frame>) -> Option<Vec<u8>> {
    match frame {
        Some(Frame::BulkString(Some(bytes))) => Some(bytes.clone()),
        _ => None,
    }
}

// ── Per-command parsers ───────────────────────────────────────────────────────

fn parse_ping(parts: &[Frame]) -> Result<Command, Reply> {
    // PING (no args)  → Ping { message: None }
    // PING <message>  → Ping { message: Some(bytes) }    (M1.4, ADR-0006)
    // PING X Y ...    → ERR wrong number of arguments
    match parts.len() {
        1 => Ok(Command::Ping { message: None }),
        2 => {
            let bytes = bulk_to_bytes(parts.get(1)).ok_or_else(|| {
                Reply::Error("ERR wrong number of arguments for 'ping' command".to_owned())
            })?;
            Ok(Command::Ping {
                message: Some(bytes),
            })
        }
        _ => Err(Reply::Error(
            "ERR wrong number of arguments for 'ping' command".to_owned(),
        )),
    }
}

fn parse_get(parts: &[Frame]) -> Result<Command, Reply> {
    if parts.len() != 2 {
        return Err(Reply::Error(
            "ERR wrong number of arguments for 'get' command".to_owned(),
        ));
    }
    let key = bulk_to_string(parts.get(1)).ok_or_else(|| {
        Reply::Error("ERR wrong number of arguments for 'get' command".to_owned())
    })?;
    Ok(Command::Get { key })
}

fn parse_set(parts: &[Frame]) -> Result<Command, Reply> {
    // Minimum: SET key value (3 parts)
    // With EX:  SET key value EX secs (5 parts)
    if parts.len() < 3 {
        return Err(Reply::Error(
            "ERR wrong number of arguments for 'set' command".to_owned(),
        ));
    }
    let key = bulk_to_string(parts.get(1)).ok_or_else(|| {
        Reply::Error("ERR wrong number of arguments for 'set' command".to_owned())
    })?;
    let value = bulk_to_bytes(parts.get(2)).ok_or_else(|| {
        Reply::Error("ERR wrong number of arguments for 'set' command".to_owned())
    })?;

    // Parse optional EX <secs> suffix.
    let ttl_secs = if parts.len() >= 5 {
        let opt_name = bulk_to_string(parts.get(3))
            .ok_or_else(|| Reply::Error("ERR syntax error".to_owned()))?;
        if !opt_name.eq_ignore_ascii_case("EX") {
            return Err(Reply::Error("ERR syntax error".to_owned()));
        }
        let secs_str = bulk_to_string(parts.get(4)).ok_or_else(|| {
            Reply::Error("ERR value is not an integer or out of range".to_owned())
        })?;
        let secs: u64 = secs_str
            .parse()
            .map_err(|_| Reply::Error("ERR value is not an integer or out of range".to_owned()))?;
        Some(secs)
    } else if parts.len() == 4 {
        // 4 parts means EX is present but secs is missing.
        return Err(Reply::Error(
            "ERR wrong number of arguments for 'set' command".to_owned(),
        ));
    } else {
        None
    };

    Ok(Command::Set {
        key,
        value,
        ttl_secs,
    })
}

fn parse_del(parts: &[Frame]) -> Result<Command, Reply> {
    if parts.len() < 2 {
        return Err(Reply::Error(
            "ERR wrong number of arguments for 'del' command".to_owned(),
        ));
    }
    let keys: Vec<String> = parts[1..]
        .iter()
        .filter_map(|f| bulk_to_string(Some(f)))
        .collect();
    if keys.len() != parts.len() - 1 {
        return Err(Reply::Error(
            "ERR wrong number of arguments for 'del' command".to_owned(),
        ));
    }
    Ok(Command::Del { keys })
}

fn parse_exists(parts: &[Frame]) -> Result<Command, Reply> {
    if parts.len() < 2 {
        return Err(Reply::Error(
            "ERR wrong number of arguments for 'exists' command".to_owned(),
        ));
    }
    let keys: Vec<String> = parts[1..]
        .iter()
        .filter_map(|f| bulk_to_string(Some(f)))
        .collect();
    if keys.len() != parts.len() - 1 {
        return Err(Reply::Error(
            "ERR wrong number of arguments for 'exists' command".to_owned(),
        ));
    }
    Ok(Command::Exists { keys })
}

/// Parse commands that take exactly one key argument: INCR, DECR.
fn parse_single_key_cmd(
    parts: &[Frame],
    name: &str,
    mk_cmd: impl Fn(String) -> Command,
) -> Result<Command, Reply> {
    if parts.len() != 2 {
        return Err(Reply::Error(format!(
            "ERR wrong number of arguments for '{name}' command"
        )));
    }
    let key = bulk_to_string(parts.get(1)).ok_or_else(|| {
        Reply::Error(format!(
            "ERR wrong number of arguments for '{name}' command"
        ))
    })?;
    Ok(mk_cmd(key))
}

// ── M1.3 (ADR-0005) parsers ──────────────────────────────────────────────────

fn parse_echo(parts: &[Frame]) -> Result<Command, Reply> {
    if parts.len() != 2 {
        return Err(Reply::Error(
            "ERR wrong number of arguments for 'echo' command".to_owned(),
        ));
    }
    let message = bulk_to_bytes(parts.get(1)).ok_or_else(|| {
        Reply::Error("ERR wrong number of arguments for 'echo' command".to_owned())
    })?;
    Ok(Command::Echo { message })
}

fn parse_select(parts: &[Frame]) -> Result<Command, Reply> {
    if parts.len() != 2 {
        return Err(Reply::Error(
            "ERR wrong number of arguments for 'select' command".to_owned(),
        ));
    }
    let db_str = bulk_to_string(parts.get(1))
        .ok_or_else(|| Reply::Error("ERR value is not an integer or out of range".to_owned()))?;
    let db: i64 = db_str
        .parse()
        .map_err(|_| Reply::Error("ERR value is not an integer or out of range".to_owned()))?;
    Ok(Command::Select { db })
}

fn parse_quit(parts: &[Frame]) -> Result<Command, Reply> {
    if parts.len() != 1 {
        return Err(Reply::Error(
            "ERR wrong number of arguments for 'quit' command".to_owned(),
        ));
    }
    Ok(Command::Quit)
}

// ── M1.4 (ADR-0006) parsers ──────────────────────────────────────────────────

fn parse_expire(parts: &[Frame]) -> Result<Command, Reply> {
    if parts.len() != 3 {
        return Err(Reply::Error(
            "ERR wrong number of arguments for 'expire' command".to_owned(),
        ));
    }
    let key = bulk_to_string(parts.get(1)).ok_or_else(|| {
        Reply::Error("ERR wrong number of arguments for 'expire' command".to_owned())
    })?;
    let secs_str = bulk_to_string(parts.get(2))
        .ok_or_else(|| Reply::Error("ERR value is not an integer or out of range".to_owned()))?;
    let seconds: i64 = secs_str
        .parse()
        .map_err(|_| Reply::Error("ERR value is not an integer or out of range".to_owned()))?;
    Ok(Command::Expire { key, seconds })
}

fn parse_keys(parts: &[Frame]) -> Result<Command, Reply> {
    if parts.len() != 2 {
        return Err(Reply::Error(
            "ERR wrong number of arguments for 'keys' command".to_owned(),
        ));
    }
    let pattern = bulk_to_string(parts.get(1)).ok_or_else(|| {
        Reply::Error("ERR wrong number of arguments for 'keys' command".to_owned())
    })?;
    Ok(Command::Keys { pattern })
}
