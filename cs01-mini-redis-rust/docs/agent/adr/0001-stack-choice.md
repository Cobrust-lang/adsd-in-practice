---
adr: 0001
title: Stack choice — tokio + Axum + hashbrown + rust-embed
status: accepted
date: 2026-05-12
case: cs01-mini-redis-rust
supersedes: none
last_verified_commit: pending
---

# ADR-0001: Stack choice — tokio + Axum + hashbrown + rust-embed

## Context

CS-01 是一个 Redis 兼容子集 + web 监控控制台。我们需要选择:

- **异步运行时**:tokio / async-std / smol
- **HTTP 框架**(control plane):Axum / Actix / warp / hyper-only
- **内存 hashtable**(存 KV):标准 `HashMap` / `hashbrown::HashMap` / 自写 robin-hood
- **前端 build 嵌入策略**:`rust-embed` crate / 手写 `include_bytes!` / 启动时从磁盘读
- **TCP 编解码**:`tokio_util::codec::Framed` / 手写状态机 / `bytes::BytesMut` 直接

需要在 5-day MVP 内 ship,**避免 yak-shaving**,同时**为 v0.1.0 后期性能优化留口子**。

## Options Considered

### Option A: tokio + Axum + hashbrown + rust-embed(选中)

- **Pros**:
  - tokio 是 Rust 生态最成熟的 async runtime,生态库最齐;Cobrust Studio 也选 tokio,知识/工具复用
  - Axum 跟 tokio 一家(都是 tokio-rs org),tracing 集成好,extractor 模型清晰
  - hashbrown 是 std `HashMap` 的底层,直接用更快 + 显式选择(避免被 std 升级影响)
  - rust-embed 是单 binary 部署的标准做法,Cobrust Studio 验证过
- **Cons**:
  - tokio 的 spawn-per-connection 在 100k 连接以下都 OK,超出后要 reactor 调优
  - Axum extractor 系统对新手有学习曲线
  - hashbrown 的 `Send` 性质需要包 `Mutex` / 用 `parking_lot::RwLock`,decisions 还要做

### Option B: async-std + tide + std::HashMap + include_bytes!

- **Pros**:async-std API 更接近 std;tide 极简
- **Cons**:async-std 生态严重萎缩(2024 后基本不更新),tide 不再活跃维护;偏离 Cobrust Studio 的栈;**违反"知识复用"目标**

### Option C: 纯 hyper + 自写 router + 自写 hashtable

- **Pros**:最极致的控制,no-dep 路线美学加分
- **Cons**:**严重 yak-shaving**,5-day MVP 做不完;偏 educational 而非 ADSD 验证

## Decision

**选 Option A**。

理由:
1. **跟 Cobrust Studio 栈对齐**(tokio + Axum + rust-embed),sub-agent 复用 ADR-0005 经验,ADSD 在跨项目复用上的红利就是这种
2. 5-day MVP 严守时间约束,Option C 砍掉
3. async-std/tide(Option B)生态信号差,长尾风险高

## Consequences

### 正面

- 跟 Cobrust Studio 的开发经验/工具/Cursor snippet 复用
- tokio + Axum + tracing 是行业标准,招外部贡献者门槛低
- 单 binary 路径明确(rust-embed)

### 负面 / 接受的债

- 性能 ceiling 受限于 tokio 模型(每连接 task,vs io_uring 的 zero-copy),v0.1.0 不解决
- Axum 的 extractor 编译错误信息长,新手可能花时间适应
- hashbrown `Send + Sync` 需要 `parking_lot::RwLock<HashMap<...>>` 而不是 lock-free,Pub/Sub 高并发场景会撞瓶颈(留给 v0.2 优化)

### 不可逆性

- 中等可逆。换 Axum → Actix 需要重写 router(2-3 天工作)。
- hashbrown → 自写 robin-hood 完全可逆(只是 type alias 切换)。
- rust-embed → include_bytes! 完全可逆(build.rs 替换)。

## Done Criteria

- [ ] `Cargo.toml` workspace 声明 tokio / Axum / hashbrown / rust-embed
- [ ] `redis-server` binary 用 tokio runtime 启动
- [ ] `redis-storage` 用 hashbrown 当 KV
- [ ] `redis-server` 用 rust-embed 嵌入 SvelteKit build(M4.3+ deferred; see ADR-0008 deferral and ADR-0013 Tauri sidecar direction)
- [ ] 在 README "Architecture" 章节标注栈选择

## Cross-references

- 相关 ADR:无(本 case 的 ADR-0001)
- 参考 Cobrust Studio ADR-0001 stack-choice 和 ADR-0003 single-binary-deployment
- 代码:`Cargo.toml`, `crates/redis-server/src/main.rs`

## Notes

- 如果 v0.2 要换 io_uring,建议 follow tokio-uring 而不是 monoio(后者社区还小)。
- hashbrown 高版本(0.15+)的 `RawTable` API 不稳,锁版本到 0.15.x。
