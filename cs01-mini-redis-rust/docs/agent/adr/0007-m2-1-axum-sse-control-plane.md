---
adr: 0007
title: M2.1 — Axum HTTP control plane + SSE for dashboard / keys (backend only)
status: accepted
date: 2026-05-12
case: cs01-mini-redis-rust
supersedes: none
last_verified_commit: 633b7aa
---

# ADR-0007: M2.1 Axum HTTP control plane + SSE

## Context

Wave M1 全部 ship 后,cs01 有完整的 RESP 子集 + docker oracle 验证。M2 的目标(cs01 CLAUDE.md §3 items 6-7):

- `/api/stats` / `/api/keys` SSE 实时推送
- SvelteKit UI:dashboard / keys / pubsub 三页
- 单 binary embed(rust-embed)

整体 wave 太大。**ADR-0007 只锁 M2.1 — 后端 Axum + SSE,纯 Rust**;前端 SvelteKit 留 ADR-0008。Rationale:
- M2.1 跟 frontend 完全解耦(SSE 是协议,任何 EventSource client 都行)
- M2.1 后,我们手测可以用 `curl localhost:6381/api/stats` 看 event 流
- 这给 M2.2 UI sprint 一个稳定的 backend mock 可以对照

待定决策面:

1. **HTTP listener 端口**:跟 RESP 共用 6380 multiplex,还是单独 6381?
2. **`/api/stats` 推什么 + 频率**:connections / commands / keys / memory?1Hz / 100ms?
3. **`/api/keys` 推什么**:全量 snapshot 还是 incremental delta?有大 keyspace 时怎么办?
4. **Stats metrics 在哪个 crate**:`redis-storage` 加 `metrics()` 公开方法?还是 server 自己 track?
5. **SSE event 格式**:plain JSON / CloudEvents / 自定义 schema?
6. **Connection / command counter 怎么累加**:`AtomicU64` shared across tasks?
7. **SSE 多 client subscribe**:`tokio::sync::broadcast` 还是 per-conn poll?
8. **CORS / dev mode**:M2.1 不需要(前端在 M2.2),但要不要早 enable?
9. **测试 oracle 怎么 align**:SSE 没有"真 Redis SSE"对照,F23-A 不适用 — 接受 gap,写 finding 标记

约束:
- 不破现有 RESP listener(M1.3 拿到 100% 没退化)
- HTTP listener 跟 RESP 同 `Store` 实例(同进程,共享内存,无 IPC)
- Stats 不能引入热路径开销(connection count 是 `AtomicU64::fetch_add`)
- 测试不用 playwright,用 `reqwest` 直连 SSE 跑 e2e
- F24:不准用 `axum::serve` 黑盒 — 但 axum 是 framework 整个就在它上面;**实际 F24 边界 = 不准用 `tower::ServiceBuilder` 堆隐藏 middleware**,显式列每一层
- 顶层 §3.1:不准 `.unwrap()` 在非测试代码

## Options Considered

### Q1: HTTP 端口 multiplex 还是分开

#### Option A: 单独 listener,默认 6381(选中)

- **Pros**:
  - RESP 和 HTTP 协议完全不同(byte stream vs HTTP/1.1 framing),multiplexer 要做 protocol-sniff,**严重 over-engineering**
  - 两个 listener task 在 tokio 下 = 两个 spawn,开销可忽略
  - 端口分离 = 部署时防火墙规则清晰(RESP 内网 only,HTTP 暴露给监控)
  - 默认 6381 已经在 main.rs CLI flag 里 reserved,这次接通
- **Cons**:
  - 两个 port 配置 = 用户多记一个数字(可接受)

#### Option B: 同端口 6380 multiplex via byte-sniff

- **Pros**:单端口
- **Cons**:
  - 要写一个 sniff handler(`peek` 第一 byte → `*` 走 RESP / `G P O ...` 走 HTTP),额外 100+ LOC
  - HTTP/2 prior knowledge 直接挂(no peekable startup byte)
  - **过度工程化**

#### Option C: Axum extractor + middleware 实现 RESP-over-HTTP

跳过 — 那是 HTTP/2 fanfic,不是 Redis。

**选 A**。

### Q2: stats 推什么 + 频率

- **推送项**:
  - `connections_active`: u64(当前 RESP 活连接)
  - `commands_total`: u64(累计 cumulative)
  - `keys_active`: u64(`Store::metrics().key_count`)
  - `mem_value_bytes`: u64(`Store::metrics().total_value_bytes`,粗略)
  - `uptime_secs`: u64(server 启动到现在)
- **频率**:**1Hz(每秒一次)**。理由:
  - dashboard UI 是给人看,人眼 sampling rate < 10Hz 已经足够
  - 1Hz 减小 SSE write 流量(每 client/s ~120 bytes JSON)
  - M3 加 burst 推送选项时再升级
- **格式**:`event: stats\ndata: {"connections_active":3,"commands_total":1024,...}\n\n`(SSE 标准 event/data,JSON payload)

### Q3: `/api/keys` 推什么 + 频率

- **推送项**:`Vec<KeyInfo { key: String, type: "string", ttl_secs: Option<i64> }>`
- **频率**:**1Hz**(对齐 stats)
- **大 keyspace 处理**:M2.1 截断到前 100 key(`Store::sample_keys(100)`),UI 上提示 "showing first 100 keys"
- **理由**:M2 是 demo dashboard,不是 prod-grade,F22 (coverage-fix-cadence) — 先 ship,再优化

### Q4: Stats metrics 在哪个 crate

#### Option A: `redis-storage::Store` 加 `metrics()` 方法(选中)

```rust
pub struct StoreMetrics { pub key_count: u64, pub total_value_bytes: u64 }
impl Store { pub fn metrics(&self) -> StoreMetrics { ... } }
```

- **Pros**:Store 是数据 owner,metrics 跟 Store 数据生死同步,无 stale 风险
- **Cons**:每次 metrics call 锁 read,~50µs;1Hz 频率无感

#### Option B: server 维护自己的 counter,每次 SET / DEL 时 inc / dec

- **Cons**:**双数据源 + 同步 bug 候选**(F1 候选),强烈不选

**选 A**。

`connections_active` 和 `commands_total` 是 server-side state(Store 不知道连接),由 server 自己 `AtomicU64` 维护,在 `handle_conn` 里增减。

### Q5: SSE event 格式

- **选**:JSON payload + SSE `event: <type>` line(标准 SSE protocol)
- 每个 SSE message:
  ```
  event: stats
  data: {"connections_active":3,"commands_total":1024,"keys_active":42,"mem_value_bytes":2048,"uptime_secs":300}

  ```
  (两个空行结尾,符合 EventSource 协议)
- `event: keys` 同样格式,data 是 `[{"key":"foo","type":"string","ttl_secs":-1}, ...]`
- **Reject CloudEvents**:over-engineering,EventSource 客户端不需要它

### Q6: Counter 实现

- `connections_active`: `Arc<AtomicU64>`,`handle_conn` 入口 `fetch_add(1)`,出口 `fetch_sub(1)`(via RAII guard)
- `commands_total`: `Arc<AtomicU64>`,每次 `from_frame` 后 `fetch_add(1)`(即使 unknown command 也算 — Redis 行为)
- 两个 counter 跟 `Store` 一起 包成 `pub struct AppState { store: Store, conn: Arc<AtomicU64>, cmd: Arc<AtomicU64>, started: Instant }` 共享给 RESP 和 HTTP 两个 listener

### Q7: SSE 多 client subscribe

#### Option A: `tokio::sync::broadcast` channel(选中)

- 一个 sampler task 1Hz tick → 计算 metrics + 写 channel
- 每个 SSE GET handler `subscribe()` 拿 receiver,async loop 读 + 写 socket
- **Pros**:fan-out free,新 client join 立刻能收到下一帧
- **Cons**:慢 client lag 行为 = `broadcast::Receiver::recv()` 返回 `Lagged(n)`;handler 看到就 SSE 写 `event: error\ndata: lagged\n\n` 然后 break(client 重连)

#### Option B: per-conn 独立 sampler

- 每个 SSE GET handler 自己 spawn 1Hz timer
- **Cons**:N client = N timer + N 次 metrics() lock acquisition;1000 client × 1Hz = 1000 lock/s,**不必要**

**选 A**。

### Q8: CORS / dev mode

- M2.1:**先不 enable CORS**。理由:
  - 测试用 `reqwest` 直连无 CORS 问题
  - 生产 path:M4 rust-embed serve UI 跟 API same-origin,无需 CORS
  - dev mode 跑 vite 5173 → 6381 跨域,**M2.2 frontend ADR 再决定**(`tower-http::cors` permissive for `localhost:5173` 是常见做法)
- M2.1 不引入 `tower-http` 依赖

### Q9: F23-A oracle 怎么办

- HTTP/SSE 这一层**没有 reference impl 对照**(真 Redis 没有 SSE control plane)
- **接受 gap**:M2.1 测试 = 自己写 e2e(`reqwest` 自连 SSE,assert 前 N 个 event 的 schema + 数值合理)
- **写 finding** `m2-1-no-f23a-on-control-plane.md` 标记这是 F23-A 不适用的领域,**作为正面 doc(知道 gap 在哪) 不是 negative finding**

## Decision

Bundle into one ADR,M2.1 wave landing:

| # | Deliverable | Crate |
|---|---|---|
| 1 | `redis-storage::Store::metrics() -> StoreMetrics` + tests | storage |
| 2 | `AppState { store, conn_count, cmd_count, started }` + 在 RESP listener inject + handle_conn RAII counter | server |
| 3 | Axum HTTP listener `server::http_run(addr, app_state)` + ctrl_c shutdown 协同 | server |
| 4 | `/api/stats` SSE route + 1Hz sampler + `tokio::sync::broadcast` fan-out | server |
| 5 | `/api/keys` SSE route + sample top 100 | server |
| 6 | `Store::sample_keys(limit) -> Vec<KeyInfo>` | storage |
| 7 | main.rs 同时启动 RESP + HTTP listener,select! 等任何一个退出 | server (main) |
| 8 | e2e tests: `reqwest::get` SSE stream, parse first 3 events, assert schema | server |
| 9 | finding `m2-1-no-f23a-on-control-plane.md` 标记 gap | docs |

## Consequences

### 正面

- Backend 跟前端解耦:M2.2 之前可以用 `curl --no-buffer localhost:6381/api/stats` 看实时 event 流
- `Store::metrics()` 是 pure read,无写锁竞争
- broadcast channel fan-out:N client 不增加 sampling 开销
- `tower-http` / `cors` 不引入(M2.2 frontend 再决定)

### 负面 / 接受的债

- HTTP listener 引入 `axum` workspace dep(已经在 workspace.dependencies 但 redis-server 还没 enable;启用 macros feature 即可)
- broadcast lagging 简化处理(直接断开 lagging client),后续可加 buffer 调优
- `/api/keys` 限制 100:大 keyspace 时 UI 不准,**接受 finding 候选**
- 没有 `/api/pubsub` route(留 M3,Pub/Sub 也是 M3 实现)
- HTTP path 没有 auth / rate limit(M4 release-readiness 再考虑)

### 不可逆性

- 完全可逆。HTTP listener 是新代码,RESP listener 不动。

## Done Criteria(falsifiable)

- [ ] `cargo run -p redis-server -- --port 6380 --http-port 6381` 同时启 2 个 listener
- [ ] `curl --no-buffer localhost:6381/api/stats` 持续输出 SSE events,每秒 1 个
- [ ] event 格式:`event: stats\ndata: {"connections_active":N,"commands_total":N,"keys_active":N,"mem_value_bytes":N,"uptime_secs":N}\n\n`
- [ ] 接 RESP client + SET 3 keys → SSE stats 立刻显示 `connections_active=1,keys_active=3` 在下一帧
- [ ] disconnect RESP client → SSE 下一帧 `connections_active=0`
- [ ] `curl --no-buffer localhost:6381/api/keys` 输出 `event: keys\ndata: [{...}]\n\n` per second
- [ ] `Store::metrics()` 单测:空 store / 加入 key / del key 后数值正确
- [ ] `Store::sample_keys(100)` 单测:< 100 key 全列 / > 100 key 截断
- [ ] e2e 测试 `tests/http_sse.rs`:启动 server,`reqwest::Client::get("http://.../api/stats")` 拿前 3 个 event,assert schema 完整 + uptime_secs 单调递增
- [ ] e2e 测试:同时 2 个 `reqwest` SSE client,都拿到同样的 stats event(broadcast fan-out)
- [ ] Ctrl-C 同时关掉 RESP 和 HTTP listener,exit 0
- [ ] `bash tests/oracle.sh` 不退化(RESP 行为不变)
- [ ] 全 5 gate green + test count ≥ 195(M1.4 是 179,M2.1 加 ~15-20 个)
- [ ] finding `m2-1-no-f23a-on-control-plane.md` 落档

## Cross-references

- ADR-0001 stack choice(tokio + Axum + tracing 已锁)
- ADR-0003 storage layout(Store 锁策略 — 新增 `metrics()` 是 read 路径,跟 GET 同锁层级)
- ADR-0005 TCP listener(server::run 的 ctrl_c shutdown 协同需要扩到双 listener)
- ADR-0006 max-frame-size(HTTP 不受影响,但 main.rs 同时把 --max-frame-size 传 RESP listener)
- 代码新增 / 修改:
  - `crates/redis-storage/src/metrics.rs`(新增,`StoreMetrics` struct + `Store::metrics`/`sample_keys` impls)
  - `crates/redis-storage/src/lib.rs`(扩 pub re-export)
  - `crates/redis-storage/tests/metrics.rs`(新增 unit + integration)
  - `crates/redis-server/src/state.rs`(新增 `AppState`)
  - `crates/redis-server/src/http.rs`(新增 axum router + SSE handlers + sampler task)
  - `crates/redis-server/src/server.rs`(handle_conn 加 connection counter RAII + command counter inc)
  - `crates/redis-server/src/lib.rs`(`pub mod state; pub mod http;`)
  - `crates/redis-server/src/main.rs`(`tokio::try_join!` RESP + HTTP listener)
  - `crates/redis-server/tests/http_sse.rs`(新增 e2e via reqwest)
  - `crates/redis-server/Cargo.toml`(enable axum macros + add reqwest as dev-dep)
  - `docs/agent/findings/m2-1-no-f23a-on-control-plane.md`(新增 gap finding)

## Notes

- `reqwest` 作 dev-dep + `default-features = false, features = ["json", "stream"]` 避免拉 OpenSSL
- SSE 推送 keys snapshot 大 payload 时考虑 `chunked` 但 Axum SSE 已经默认 chunked
- 1Hz sampler 用 `tokio::time::interval(Duration::from_secs(1))` + `interval.tick().await`,不用 sleep
- broadcast channel capacity:32(短缓冲,lagging client 立刻 disconnect 不积压)
- `--http-port` 设 0 → disabled(用户可以 opt-out HTTP listener)— 但 M2.1 默认 enabled
