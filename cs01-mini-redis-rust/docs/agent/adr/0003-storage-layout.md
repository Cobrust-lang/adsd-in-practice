---
adr: 0003
title: In-memory storage layout — hashbrown::HashMap + DelayQueue for TTL
status: accepted
date: 2026-05-12
case: cs01-mini-redis-rust
supersedes: none
last_verified_commit: pending
---

# ADR-0003: In-memory storage layout

## Context

`redis-storage::Store` 需要存:
- Key (String) → Value (Vec<u8>) 映射
- 每个 key 的可选过期时间
- 并发访问(server 多任务跟 store 交互)

ADR-0001 已经定 hashbrown 作 KV;**本 ADR 决定 lock 策略 + TTL 实现**。

约束:
- M1.2 不做 lock-free,接受 `parking_lot::RwLock<HashMap>`
- TTL 必须真"过期",不能"只在 GET 时检查"(active expiration)
- Store::execute(cmd) 返回 Reply,不暴露内部状态
- 单测可写(纯函数测命令 → Reply 关系,不依赖时间)

## Options Considered

### Option A: `parking_lot::RwLock<hashbrown::HashMap>` + `tokio::time::DelayQueue` 主动过期(选中)

```rust
pub struct Store {
    inner: Arc<parking_lot::RwLock<Inner>>,
}
struct Inner {
    map: hashbrown::HashMap<String, Entry>,
}
struct Entry { value: Vec<u8>, expires_at: Option<Instant> }
// 单独一个 tokio task 跑 DelayQueue,到点把 key 从 map 里 remove
```

- **Pros**:
  - parking_lot 比 std::sync 快 3-5×;hashbrown 是 std HashMap 底层,显式选更可控
  - DelayQueue 是 tokio 官方过期 primitive,无 sleep 抢占
  - 简单:reader/writer 锁就够,M1.2 不追求 lock-free
- **Cons**:
  - 写锁串行(多 producer 同时 SET 会串)— 但 redis-cli 单连接是 sequential,bottleneck 在 server task spawn 不在锁
  - DelayQueue 在大量短 TTL 时(如 1M 条 EX 1s)有 GC 开销 — v0.2 不优化

### Option B: `tokio::sync::RwLock<HashMap>` + 被动过期(GET 时检查)

- **Pros**:全异步;无单独 GC task
- **Cons**:
  - **被动过期违反 Redis 语义**:`KEYS *` 会列出已过期 key 直到下次 GET。**这是 F24 偷懒**(用"假装过期"模拟真过期)
  - tokio::sync::RwLock 在 contention 高时比 parking_lot 慢

### Option C: 分片 hashtable(每个 shard 自己锁)

- **Pros**:多 writer 真并行
- **Cons**:**过早优化**,v0.2 才考虑;增加 shard hash 实现复杂度

## Decision

**选 Option A**。

理由:
1. M1.2 目标是 ship,不追求 lock-free;parking_lot 锁是最简单稳的
2. DelayQueue 主动过期是真 Redis 语义,F24 防御
3. Option B 的被动过期是 F24 偷懒,**禁止**
4. Option C 是 v0.2 优化

## Consequences

### 正面

- 单测可写:把 `Instant::now()` 注入(用 trait)就可控时间
- 主动过期 → `KEYS *` 行为符合 Redis,oracle 不挂
- parking_lot 性能足够 M1.2 SLO(50k SET/s 单连接)

### 负面 / 接受的债

- 写锁串行 → 多并发 client 写同一个 key 时序列化(M1.2 接受)
- DelayQueue 内存 = 1 entry per active TTL key(高 TTL 密度场景内存放大,M3 评估)

### 不可逆性

- 中等可逆。Inner struct 在 crate 内部,改成分片是 1 周工作。Public API `Store::execute(cmd)` 不变。

## Done Criteria

- [ ] `Store::execute(Command::Set { key, value, ttl_secs: None })` 写入并返回 `Reply::Ok`
- [ ] `Store::execute(Command::Get { key })` 返回 `Reply::Bulk(Some(value))`
- [ ] `Store::execute(Command::Get { key: 不存在 })` 返回 `Reply::Bulk(None)`
- [ ] `Store::execute(Command::Del { keys: [a, b, c] })` 返回 `Reply::Integer(实际删的数量)`
- [ ] `Store::execute(Command::Incr { key })` 在不存在/存在时正确递增,**非整数 value 时返回 `Reply::Error("ERR value is not an integer")`**
- [ ] TTL 主动过期:`SET k v EX 1` 后 sleep 1.1s,`GET k` 返回 nil。**测试用 tokio::time::pause + advance**,不真 sleep
- [ ] `cargo test --workspace --locked` 全过

## Cross-references

- ADR-0001 stack choice(hashbrown + parking_lot)
- 代码:`crates/redis-storage/src/lib.rs`

## Notes

- 时间注入:用 `tokio::time::Instant` + `tokio::time::pause()`,**不引入自定义 Clock trait**(过早抽象)
- INCR/DECR 的"非整数 value" 错误必须跟真 Redis 字面对齐:`ERR value is not an integer or out of range`
