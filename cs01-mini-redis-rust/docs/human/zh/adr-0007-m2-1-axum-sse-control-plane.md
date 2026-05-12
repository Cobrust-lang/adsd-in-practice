# ADR-0007 中文摘要:M2.1 Axum HTTP + SSE 控制面(后端)

> 完整 ADR 见 [docs/agent/adr/0007-m2-1-axum-sse-control-plane.md](../../agent/adr/0007-m2-1-axum-sse-control-plane.md)。

## 决策

把 M2 拆 **M2.1 后端 + M2.2 前端**。M2.1 一波:

1. **独立 HTTP listener 6381**(不跟 RESP 6380 multiplex — 协议差异大,protocol-sniff 是 over-engineering)
2. **`Store::metrics()` + `Store::sample_keys(n)`** 放 storage crate(数据 owner 自己暴露)
3. **`AppState { store, conn_count, cmd_count, started }`** 共享给 RESP + HTTP 两个 listener;RESP `handle_conn` 用 RAII guard 维护 connection count,每次 `from_frame` 后 inc `commands_total`
4. **SSE event format**:标准 `event: <type>\ndata: <json>\n\n`,1Hz 推送
5. **`/api/stats`**:推 `{connections_active, commands_total, keys_active, mem_value_bytes, uptime_secs}`
6. **`/api/keys`**:推 top 100 KeyInfo(`{key, type, ttl_secs}`)— 大 keyspace 截断
7. **`tokio::sync::broadcast` fan-out**:一个 sampler task → N 个 SSE client 共享数据,lagging client 直接断
8. **e2e 测试用 `reqwest`** 自连 SSE,assert schema + 单调 uptime
9. **F23-A 不适用 SSE 层**(没有 reference impl 对照),落 finding 标记 gap

## 拒绝的方案

- 同端口 multiplex(6380 byte-sniff RESP/HTTP)— 过度工程
- server 自己 track key count(双数据源 + 同步 bug 候选,F1)
- per-conn 独立 sampler timer(N×lock,白白浪费)
- CloudEvents 格式(EventSource client 不需要)
- M2.1 引入 CORS(M2.2 vite dev 再决定)
- M2.1 引入 playwright(过重,e2e 用 reqwest 即可)

## 测试目标

`cargo test --workspace` ≥ 195(M1.4 baseline 179,M2.1 加 ~15-20)。

## 状态

`accepted` — 2026-05-12
