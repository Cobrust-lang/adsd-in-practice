# ADR-0003 中文摘要:存储布局

> 完整 ADR 见 [docs/agent/adr/0003-storage-layout.md](../../agent/adr/0003-storage-layout.md)。

## 决策

`redis-storage::Store` 用:

- `parking_lot::RwLock<hashbrown::HashMap<String, Entry>>` — 共享可变状态
- `Entry { value: Vec<u8>, expires_at: Option<Instant> }`
- `tokio::time::DelayQueue` 跑独立 task **主动过期**(到点真删 key)

## 拒绝的方案

- **被动过期(GET 时才检查 TTL)**:F24 偷懒,**违反 Redis 语义**(`KEYS *` 会列已过期 key)
- **分片 hashtable**:过早优化,v0.2 再考虑

## 状态

`accepted` — 2026-05-12
