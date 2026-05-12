<div align="center">

# CS-01 · mini-redis-rust

**Redis-compatible 内存 KV + web 管理控制台 · Rust 实现**

*ADSD case study #1 — 网络服务 + 协议解析 + 持久化 + 实时监控*

</div>

---

## What this is

一个从零实现的 Redis 兼容子集,**故意**做完整命令路由 / RESP 协议 / AOF 持久化 / Pub-Sub / 内存键过期 / SvelteKit 监控 UI。

**测什么**:ADSD 方法论从 Cobrust(编译器领域)迁到 **网络服务 + 协议层 + 持久化 + 前端** 这种完全不同的领域,是否还成立。

## 范围

### v0.1.0 必须 ship(M4)

- ✅ RESP 协议解析(parse + serialize round-trip)
- ✅ 命令路由:`PING / GET / SET / DEL / EXISTS / INCR / DECR / EXPIRE / TTL / TYPE / KEYS`
- ✅ 内存存储 + 过期(自己实现 hashtable,**禁止用 `BTreeMap` 假装,F24 反模式禁令**)
- ✅ AOF append-only file 持久化(简化版,无 rewrite)
- ✅ Pub/Sub:`SUBSCRIBE / UNSUBSCRIBE / PUBLISH`
- ✅ SvelteKit 监控 UI:实时连接数 / qps / 内存占用 / key 列表(SSE)
- ✅ Axum HTTP control plane(`/api/stats` / `/api/keys` SSE)
- ✅ 单 binary 部署(rust-embed 嵌入 web)
- ✅ 与 `redis-cli` 互通(对照 oracle,F23-A 防御)
- ✅ 5 道 ADSD gate green

### Out of scope(0.1.0 不做)

- ❌ Cluster / replication
- ❌ Lua scripting / Streams / Modules
- ❌ Transaction `MULTI/EXEC` 完整语义(P1 后续)
- ❌ 持久化 RDB(只做 AOF)
- ❌ ACL / TLS(deferred)

## ADSD 触发点(预期会撞出的 ADR / finding)

| 决策点 | 预期 ADR |
|---|---|
| 协议解析器分层(SIMD vs scalar)| ADR-0002 |
| 命令路由(trait dispatch vs match)| ADR-0003 |
| 内存 hashtable 选择(自己写 vs `hashbrown` 哪些用)| ADR-0004 |
| AOF 格式 + flush 策略 | ADR-0005 |
| Pub/Sub broadcast 模型(broadcast::channel vs 自己写)| ADR-0006 |
| SSE 监控数据格式 | ADR-0007 |
| 单 binary 嵌入策略(rust-embed vs include_bytes!) | ADR-0008 |

**预期会撞**:
- **F2** layer divergence(parser 和 router 各跑自己的边界 case)
- **F5** silent miscompile 类(RESP 序列化某些 edge case 跟 `redis-cli` 不一致但没 panic)
- **F10** cargo lock contention(M3+ 并发 sprint 必撞)
- **F23-A** oracle 自己写自己测(必须对照真 `redis-cli` 做端到端)
- **新 F-pattern 候选**:网络 IO 测试的非确定性(SSE 缓冲 / TCP buffering)

## Quick start

```bash
cd cs01-mini-redis-rust
bash scripts/bootstrap.sh
cargo run -p redis-server -- --port 6380 --aof data/dump.aof
# 另一个终端
redis-cli -p 6380 PING
```

监控 UI:`http://localhost:6380/_studio`(从 server binary 暴露)

## Architecture

```
┌──────────────────────────────────────────┐
│ SvelteKit UI (embedded via rust-embed)   │
│ - /_studio/dashboard                      │
│ - /_studio/keys                           │
│ - /_studio/pubsub                         │
└─────────────────┬─────────────────────────┘
                  │ REST + SSE
        ┌─────────▼──────────┐
        │   redis-server     │  Axum + tokio
        │   - /api/stats     │
        │   - /api/keys SSE  │
        │   - TCP :6380 RESP │
        └────┬──────────┬────┘
             │          │
     ┌───────▼──┐  ┌───▼────────────┐
     │ redis-   │  │ redis-storage  │
     │ protocol │  │ - hashtable    │
     │ (RESP)   │  │ - expiry wheel │
     │          │  │ - AOF writer   │
     └──────────┘  └────────────────┘
```

## Status

- 🚧 M0 scaffold(本阶段)
- ⬜ M1 backend MVP(RESP + 5 commands + AOF)
- ⬜ M2 frontend MVP(SvelteKit + 3 pages + Vitest + Playwright)
- ⬜ M3 single binary + dogfood
- ⬜ M4 v0.1.0 release + METHODOLOGY-STATUS 更新

## License

Apache-2.0 + MIT dual,同顶层 repo。
