# ADR-0002 中文摘要:RESP 协议帧解析策略

> 完整 ADR 见 [docs/agent/adr/0002-resp-framing.md](../../agent/adr/0002-resp-framing.md)。

## 决策

`redis-protocol` 提供**一次性 + Incomplete sentinel** 风格的 parser:

```rust
pub fn parse(input: &[u8]) -> Result<(Frame, usize), ProtocolError>;
```

- 成功 → `(Frame, 消耗字节数)`
- buffer 不足 → `Err(Incomplete)`
- 协议错误 → `Err(Invalid("reason"))`

caller(`redis-server`)循环 `parse + advance` 直到 Incomplete,然后等更多字节。

## 拒绝的方案

- **nom**:RESP 太简单,nom 是 overkill,且增加编译时间
- **流式状态机**:过早优化,M3 之前不需要

## v0.2 优化方向

把 `BulkString(Option<Vec<u8>>)` 换成 `BulkString(Option<Bytes>)`,零拷贝大 value。

## 状态

`accepted` — 2026-05-12
