# Finding M2.1 中文摘要(gap acknowledgement):HTTP/SSE control plane 没有 F23-A oracle

> 完整 finding 见 [docs/agent/findings/m2-1-no-f23a-on-control-plane.md](../../agent/findings/m2-1-no-f23a-on-control-plane.md)。

## 一句话

M2.1 在 cs01 接入 Axum HTTP + SSE control plane(`/api/stats` / `/api/keys`)时,**找不到同类 reference impl 作 F23-A oracle 对照** — 真 Redis 没有 SSE 控制面,redis-stat / Redis Insight / redis_exporter 三家 wire 都不一样。**F23-A 在这一层不适用**;接受 gap,改用 ADR-0007 锁 schema + `tests/http_sse.rs` 自测 + 跨 sprint mitigation(M2.2 前端 ADR-0008 必须与 backend 同步改 schema)。

## 关键判定

- F23-A 不是 universal:RESP 协议层有真 Redis 作强 oracle,HTTP/SSE 控制面**没有**
- 不是失败,是 **acceptance gap**(severity P4 + positive: false 中性标记)
- mitigation:`StatsSnapshot` / `KeyJson` 5+3 字段在 ADR-0007 §Done Criteria 锁定;前端 ADR-0008 强制 cite 本 finding

## 结论

**F23-A 的边界**:在没有 reference impl 的子领域(SSE 控制面 / 自定义 admin API / 私有 RPC),F23-A 不适用,应当**显式标 gap**(本 finding 模式)。跟 M1.4 那条正面 F23-A finding 是镜像 — 那里 oracle 抓 bug,这里显式 oracle 缺位 acknowledged。

## 状态

`P4`,acceptance gap。M2.2 frontend ADR-0008 候选 cite。
