---
adr: 0002
title: RESP v2 framing strategy — one-shot parse with Incomplete sentinel
status: accepted
date: 2026-05-12
case: cs01-mini-redis-rust
supersedes: none
last_verified_commit: pending
---

# ADR-0002: RESP v2 framing strategy

## Context

CS-01 的 TCP layer 收到 `BytesMut` buffer,可能含 0、1、N 个 RESP 帧,也可能在帧中间被切。需要确定 parser 接口形态和复用模式。

约束:
- **不能阻塞 IO**:parser 必须可以"返回 Incomplete 然后下次再来"
- **测试要简单**:能纯函数 round-trip 测,不依赖 TCP/tokio
- **不能假设 buffer 是 single frame**:Redis pipelining 允许一个 buffer 含多个命令
- **支持 RESP v2 五种类型**:`+` SimpleString / `-` Error / `:` Integer / `$` BulkString / `*` Array

## Options Considered

### Option A: 一次性 `parse(&[u8]) -> Result<(Frame, usize), ProtocolError>`(选中)

签名:
```rust
pub fn parse(input: &[u8]) -> Result<(Frame, usize), ProtocolError>;
//  返回 (frame, bytes_consumed) 或 Err(Incomplete) / Err(Invalid)
```

- **Pros**:
  - 纯函数,易测,easy oracle
  - caller 自己管 buffer:`while let Ok((f, n)) = parse(&buf[..]) { buf.advance(n); dispatch(f); }`
  - 不引入 nom 依赖
- **Cons**:
  - 嵌套 array(array of array)需要递归调用,栈深度等于 nesting depth(实测 Redis 命令最多 2-3 层嵌套,可接受)
  - 大 bulk string(MB 级)拷贝到 Vec 是一次完整 copy(M3 优化:换 `Bytes` 零拷贝 slice)

### Option B: `nom` combinator

```rust
named!(parse_frame<&[u8], Frame>, ...);
```

- **Pros**:nom 表达力强,组合式
- **Cons**:
  - 引入 nom 依赖(~600 LOC 编译时间增加)
  - nom 7 的 API 变化大,版本锁定后期升级痛苦
  - **过度抽象**:RESP 简单,nom 是 overkill

### Option C: 手写流式状态机

```rust
pub struct StreamParser { state: State, ... }
impl StreamParser { fn feed(&mut self, bytes: &[u8]) -> Vec<Frame> { ... } }
```

- **Pros**:零 copy + 流式
- **Cons**:
  - 状态机维护成本高,测试覆盖率难做到全
  - **过早优化**,M3 之前不需要

## Decision

**选 Option A**。

理由:
1. RESP 简单(5 种 type + 行边界 `\r\n`),手写 + Incomplete sentinel 是最 idiomatic 的 Rust 协议解析
2. caller 端 loop "advance buffer + dispatch frame" 是 tokio-util Codec 的标准模式
3. 不引入 nom 减少编译时间(对 sub-agent 反馈循环 ROI 重要,ADSD "fast feedback")
4. M3 性能优化时换 `Bytes` 是 type 切换不是接口重写

## Consequences

### 正面

- pure-function parser,纯单测覆盖率可达 95%+
- 跟真 `redis-cli` round-trip 测试简单写
- 编译产物小

### 负面 / 接受的债

- Bulk string 在 v0.1 是 `Vec<u8>` 拷贝;M3 换 `Bytes` 之前大 value 性能不优
- 嵌套 array 用递归,理论可栈溢出(Redis 客户端不会发那种,接受)

### 不可逆性

- 中等可逆。Frame 类型在 public API,改成 `&[u8]` 引用版需要 lifetime 引入,**有 breaking risk**。
- 但 Frame 在 `redis-protocol` crate 内部使用为主,server crate 用 Frame 然后立刻 dispatch 命令,**改造影响范围有限**。

## Done Criteria(falsifiable)

- [ ] `Frame::parse(b"+OK\r\n")` 返回 `(Frame::SimpleString("OK"), 5)`
- [ ] `Frame::parse(b":42\r\n")` 返回 `(Frame::Integer(42), 5)`
- [ ] `Frame::parse(b"$5\r\nhello\r\n")` 返回 `(Frame::BulkString(Some(b"hello".to_vec())), 11)`
- [ ] `Frame::parse(b"$-1\r\n")` 返回 `(Frame::BulkString(None), 5)`(RESP nil)
- [ ] `Frame::parse(b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n")` 返回 Array of 2 BulkString
- [ ] `Frame::parse(b"+OK\r")` 返回 `Err(Incomplete)`(尾巴不全)
- [ ] `Frame::to_bytes(&Frame::SimpleString("OK".into()))` 返回 `b"+OK\r\n"`
- [ ] round-trip:`parse(to_bytes(f)).0 == f` 对 100 个随机生成 Frame 都成立(`hypothesis` 风格 fuzz / `proptest` strategy)
- [ ] Gate 1-5 全过

## Cross-references

- 相关 ADR:ADR-0001 stack choice(决定了 tokio + Axum + bytes 栈)
- 代码:`crates/redis-protocol/src/lib.rs`(M1.1 落地点)
- 测试:`crates/redis-protocol/tests/round_trip.rs`(M1.1 新加)
- RESP spec:<https://redis.io/docs/latest/develop/reference/protocol-spec/>

## Notes

- v0.2 性能优化方向(超出本 ADR scope):换 `Frame` 内 `BulkString` 用 `Bytes` 而不是 `Vec<u8>`,零拷贝。
- 错误格式:遇到协议错误返回 `Err(ProtocolError::Invalid("reason"))`,**不要 panic**。任何 panic 都是 F5(silent miscompile)候选。
