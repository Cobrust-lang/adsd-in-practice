---
finding: m2-1-no-f23a-on-control-plane
date: 2026-05-12
case: cs01-mini-redis-rust
severity: P4
specificity: high
related_adr: 0007
related_f: F23-A
last_verified_commit: live
positive: false
---

# Finding(gap acknowledgement):M2.1 HTTP/SSE control plane 没有 F23-A oracle

## Hypothesis(预期)

ADSD F23-A "oracle 不能跟实现同源" 是 cs01 §2 的硬性要求 — RESP 子集靠 `tests/oracle.sh` round-trip `redis:7-alpine` 满足。在 M2.1(ADR-0007)接 Axum HTTP + SSE control plane 时,**我预期能找一个同类 reference 对照**(类似 `redis-stat` / Redis Insight / `redis-cli --stat`),验证 `/api/stats` / `/api/keys` 的事件 schema 和数值在 wire 层与真 Redis 工具链一致。

## Method

调研社区可能的 SSE / HTTP 控制面参考实现:

1. **真 Redis 服务端**:**没有** SSE / HTTP control plane。`INFO` / `CLIENT LIST` 走 RESP,不是 HTTP。
2. **redis-stat**(Ruby 工具):有 `--server` 模式提供 web dashboard,但 wire 是 HTML + JS polling,**不是 SSE**,且后端 schema 私有。
3. **Redis Insight**(官方 GUI):闭源前端,通信走 WebSocket + 私有 JSON-RPC,**不是 SSE**,schema 不公开。
4. **redis_exporter**(Prometheus):exposes `/metrics` Prometheus text format,**不是 JSON SSE**,语义不同(scrape-pull vs subscribe-push)。

没有任何 reference impl 跟 ADR-0007 §Q5 选定的格式("event: stats / data: <JSON> / \\n\\n")相同。

## Result

**F23-A 在 M2.1 HTTP/SSE 这一层不适用** — 没有同类 oracle 可对照。

接受的实测策略(ADR-0007 §Q9 + cs01 CLAUDE.md §2):
1. **自测覆盖**:`tests/http_sse.rs` 用 `reqwest` 跑 4 个 e2e
   - schema 字段名锁定(`connections_active` / `commands_total` / `keys_active` / `mem_value_bytes` / `uptime_secs`)
   - SSE wire 格式(`event:` + `data:` + 空行)
   - broadcast fan-out(2 个 client 都收到事件)
   - cross-listener 联动(RESP 写 3 keys → SSE 下一帧 `keys_active=3` + `connections_active>=1`;disconnect → 下下帧 `connections_active=0`)
2. **schema lock**:`StatsSnapshot` / `KeyJson` 的 5+3 字段在 ADR-0007 §Done Criteria 显式落档为 frontend M2.2 contract,**P9 不准改名**;前端 ADR-0008 / cs01 v0.2.0 才能引入 versioned schema
3. **gap 监控**:M3 加 Pub/Sub control 时如果出现 Redis 6 的 RESP3 push-style protocol 替代品,可作为弱 oracle(只能对照 channel 名 / payload 结构,不能对照 SSE wire)

## Conclusion

**F23-A 不能机械适用所有层**。RESP 协议层有真 Redis 作 oracle(强 F23-A);HTTP/SSE 控制面层**没有同类 oracle**(F23-A 不适用)。

接受的债:
- M2.1 测试是 "自己写自己测"(F23-A 本来要防的反模式),但**没有第三方 wire reference 可比**
- 风险窗口:M2.2 前端真正消费这些 SSE 事件时,**schema 错误可能被发现**(那时仍在 sprint 内,代价低)
- M4 release 前要在 README 写 "SSE schema is project-internal, not Redis-compat"

## Pattern lessons

这是 **ADSD F23-A 的边界 case** — 不是"做不到所以放弃 F23-A",而是"领域没有 reference impl,F23-A 不适用,改用 schema lock + 双方 (server + frontend) 同源 schema 同步" 这种 **mitigation**。

具体路径:
1. P10 (CTO) 在 Phase 1 ADR-0007 §Q9 已经预判 "F23-A 不适用",写了这条 sub-decision
2. P9 (本 sprint) 实现时在 `state.rs` 注释 + `http.rs::KeyJson` doc 上**双重标注** "field names LOCKED for M2.2 frontend"
3. M2.2 ADR-0008(前端)会引用本 finding,强制 frontend 与 backend schema 一起改、一起 ship

## Fix / Mitigation

- 不修。这是 acceptance gap(P4)
- 跨 sprint mitigation:M2.2 前端 ADR-0008 必须 cite 本 finding 作为 schema 不可单边改名的依据
- 长期 mitigation:M3 引入 OpenAPI / JSON Schema generator 自动从 `StatsSnapshot` / `KeyJson` 派生 schema 文件,与前端 typescript 类型自动对齐(ADR-0009 候选)

## Lessons / F-pattern mapping

- **F23-A 不是 universal**:在没有 reference impl 的子领域(SSE control plane / 自定义 RPC / 私有 admin API),F23-A 不适用,**应当显式标 gap**(本 finding 模式)
- 提议加入 ADSD upstream `failure-modes-catalogue.md` 作 **F23-A.gap** sub-pattern:*"acknowledge missing oracle when domain lacks reference impl"*
- 跟 M1.4 的正面 F23-A finding(`m1-4-f23a-oracle-caught-ttl-rounding-spec-bug`)是**镜像**:那里 oracle 抓 bug,这里**显式 oracle 缺位 acknowledged**;**两条 finding 一起看** 才是 F23-A 的完整图

## Notes

写本 finding 的形式 (P4 + positive: false + neutral wording) 是按 ADSD findings ledger 模式 — 不是 negative result,也不是 positive(没救回什么),是 **gap doc**:让以后做 review 的人知道 "M2.1 这一层故意没接 F23-A,因为接不上"。

CTO Phase 1 ADR-0007 §Q9 已经预先签字接受;P9 sprint 本任务的最大 action 是**把它落档到 findings ledger**,而不是补一个不存在的 oracle。
