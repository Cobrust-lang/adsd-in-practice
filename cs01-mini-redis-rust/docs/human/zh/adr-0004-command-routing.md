# ADR-0004 中文摘要:命令路由

> 完整 ADR 见 [docs/agent/adr/0004-command-routing.md](../../agent/adr/0004-command-routing.md)。

## 决策

`redis-server::dispatch::from_frame(Frame) -> Result<Command, Reply>`:

- 在 **server crate** 做 Frame → Command 解析(不是 storage)
- 命令名 `to_ascii_uppercase()` 后 `match`,大小写不敏感
- 未知命令 / 参数错 → `Err(Reply::Error("ERR ..."))`,严格按 Redis 错误字符串
- 一个 match,加新命令加一个 arm

## 拒绝的方案

- **新建 `redis-commands` crate**:过度分层,M1.2 不需要
- **storage 自己解析 RESP**:违反 layered architecture(storage 不该懂 protocol)

## 状态

`accepted` — 2026-05-12
