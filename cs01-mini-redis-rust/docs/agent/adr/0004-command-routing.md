---
adr: 0004
title: Command routing — Frame → Command parse via match on first BulkString
status: accepted
date: 2026-05-12
case: cs01-mini-redis-rust
supersedes: none
last_verified_commit: pending
---

# ADR-0004: Command routing

## Context

`redis-server` 收到 RESP `Frame`,需要分发到 `Store::execute(Command)`。需要决定:

- Frame → Command 的解析在哪 crate(server / storage / 新 crate)
- 命令名匹配大小写(Redis 协议 case-insensitive:`SET` = `set` = `SeT`)
- 未知命令 / 参数错误 的响应格式

## Options Considered

### Option A: 在 `redis-server` crate 里有 `dispatch.rs`,`fn from_frame(Frame) -> Result<Command, Reply>` 解析(选中)

```rust
pub fn from_frame(f: Frame) -> Result<Command, Reply> {
    let Frame::Array(Some(parts)) = f else { return Err(Reply::Error(...)); };
    let cmd_name = parts.first().and_then(bulk_to_str).ok_or(...)?;
    match cmd_name.to_ascii_uppercase().as_str() {
        "PING" => Ok(Command::Ping),
        "GET"  => parse_get(parts),
        // ...
        unknown => Err(Reply::Error(format!("ERR unknown command '{}'", unknown))),
    }
}
```

- **Pros**:
  - **dispatch 是 server 的责任**(它有 socket / 协议感),不是 storage 的(storage 只 execute)
  - 一处 match,易测易加新命令
  - 错误返回 `Reply` 而不是 panic,让 caller 直接序列化回 client
- **Cons**:
  - 不易扩展到"插件命令"(无所谓,v0.1 不做)

### Option B: 新建 `redis-commands` crate 专门做 parse

- **Pros**:解耦干净
- **Cons**:**过度分层**,M1.2 不需要;增加 build complexity

### Option C: storage 自己解析 Frame

- **Pros**:更"oop"
- **Cons**:storage 不该懂 RESP(它该只关心 Command enum),违反 layered architecture

## Decision

**选 Option A**。

理由:
1. dispatch 是 protocol concern(server crate 的天然责任)
2. 一个 `match` 简单清晰,新命令加一个 arm
3. 错误用 `Reply::Error(...)` 跟 client 通信,严格按 Redis 行为:`-ERR ...` 字符串

## Consequences

### 正面

- 加新命令 = 加一个 match arm + 一个 Command variant(M3 加 SUBSCRIBE 时小手术)
- 单测可在 server crate 写:`from_frame(parse("*1\r\n$4\r\nPING\r\n").0).unwrap() == Command::Ping`
- 命令名大小写不敏感,符合 Redis

### 负面 / 接受的债

- 命令多到 50+ 时 match 长,后期可改 phf static map(v0.2)
- 错误消息字符串硬编码(M3 把它们 const 化)

### 不可逆性

- 完全可逆。从 server 抽到独立 crate 是 cargo new + 1 个文件 move。

## Done Criteria

- [ ] `from_frame(parse("*1\r\n$4\r\nPING\r\n").0)` → `Ok(Command::Ping)`
- [ ] `from_frame(parse("*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n").0)` → `Ok(Command::Get { key: "foo" })`
- [ ] `from_frame(parse("*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n").0)` → `Ok(Command::Set { key: "foo", value: b"bar".to_vec(), ttl_secs: None })`
- [ ] `from_frame(parse("*5\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n$2\r\nEX\r\n$2\r\n60\r\n").0)` → `Ok(Command::Set { ..., ttl_secs: Some(60) })`
- [ ] 大小写不敏感:`set` / `Set` / `SET` 都 work
- [ ] 未知命令 → `Err(Reply::Error("ERR unknown command 'XYZ'"))`
- [ ] 参数数量错 → `Err(Reply::Error("ERR wrong number of arguments for 'set' command"))`
- [ ] `cargo test --workspace --locked` 全过

## Cross-references

- ADR-0002 RESP framing(本 ADR 的上游;Frame 是它的输出)
- ADR-0003 Storage layout(本 ADR 的下游;Command 进 Store::execute)
- 代码:`crates/redis-server/src/dispatch.rs`(M1.2 新建)

## Notes

- M3 加 PubSub 时,`Command::Subscribe(channels)` 解析逻辑放同一文件。
- 命令的"参数数量"检查在 `parse_get` 等 helper 函数里,不在 top-level match,**保持 match 干净**。
