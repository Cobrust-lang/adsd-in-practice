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
- 🚧 Tauri desktop frontend + managed `redis-server` sidecar(M4.3,ADR-0013)
- ⬜ rust-embed 单 binary 部署(ADR-0013 后不再是 v0.1.0 blocker)
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

监控 UI 的 primary release surface 已在 ADR-0013 转向 Tauri desktop app + managed `redis-server` sidecar。浏览器 dev 模式继续走 vite,见下一节;rust-embed 单 binary 不再是 v0.1.0 blocker。

## Dev mode (M2.2)

M2.2(Wave M2.2,ADR-0008)ship 了 SvelteKit 前端(`web/`),三页:Dashboard / Keys / Pub/Sub。M2.2 阶段是 vite dev + axum HTTP control plane 两进程协作;ADR-0013 后,M4.3 primary release target 改为 Tauri desktop app 管理 `redis-server` sidecar。

```bash
# Terminal 1 — backend(RESP :6380 + HTTP/SSE :6381)
cargo run -p redis-server -- --port 6380 --http-port 6381

# Terminal 2 — frontend(vite :5173,proxy /api → 6381)
cd web
pnpm install
pnpm dev
# 打开 http://localhost:5173
```

Frontend 是 SPA(`@sveltejs/adapter-static` + `fallback: 'index.html'`),`pnpm build` 输出到 `web/build/`。

### Pub/Sub 页是 **stub**(显式标记)

`/pubsub` route 现在只显示 "M3 placeholder" 横幅;真正的 SUBSCRIBE / UNSUBSCRIBE / PUBLISH UI 在 Wave M3 实现(顶层 CLAUDE.md §1.3 显式标 stub 原则)。

### Frontend gate

`scripts/frontend-gate.sh` 跑 install (frozen-lockfile) → svelte-check + tsc → vitest → adapter-static build,作为 M2.2 之后的新 gate 6(case-local,非 _shared)。

```bash
bash scripts/frontend-gate.sh
```

Required tooling:`node ≥ 20`, `pnpm ≥ 9`(本机 `node v25.9.0` + `pnpm 10.33.0` 已 verified)。

## Architecture

```
┌──────────────────────────────────────────┐
│ SvelteKit UI (Tauri desktop shell target)│
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

- ✅ M0 scaffold
- ✅ M1 backend MVP(RESP + 11 commands + docker oracle 22/22)
- ✅ M2 frontend MVP(M2.1 Axum HTTP/SSE control plane shipped; M2.2 SvelteKit UI shipped)
- ✅ M3 Pub/Sub + AOF
- ✅ M4.1 critical fixes(security + AOF + dispatch + Pub/Sub)
- 🚧 M4.2 doc sweep + release artifacts(ADR-0012)
- 🚧 M4.3 Tauri desktop frontend + managed sidecar(ADR-0013);rust-embed 单 binary deferred

## License

Apache-2.0 + MIT dual,同顶层 repo。
