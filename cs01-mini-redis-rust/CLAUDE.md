# CS-01 mini-redis-rust — Local Agent Constitution

> Local CLAUDE.md。**覆盖**顶层 [`/CLAUDE.md`](../CLAUDE.md) 的 case-specific 规则,其它沿用顶层。

---

## 1. 本 case 不可简化的核心约束(F24 防御)

**ADSD F24 是 primitive-as-everything,本 case 必须遵守:**

- ❌ **不准用 `std::collections::HashMap` 假装 Redis hash 命令**(`HSET`/`HGET` 等)。
- ❌ **不准用 `BTreeMap` 模拟 sorted set**(`ZADD`/`ZRANGE`)。
- ⚠️ `tokio::sync::broadcast` 接受作 Pub/Sub fan-out (ADR-0009 已论证); 但: lagging client 必须显式 disconnect 而不是 silently drop msg; subscriber state 必须真在 per-conn 持 (`ConnState::Subscribed { rxs }`),不准 stash to global。判断标准维持:`redis-cli` round-trip 跟真 Redis 行为可区分 → 是简化 → F24。
- ✅ 允许用 `hashbrown::HashMap` 作 string KV(不是为了"模拟",是因为对应 Redis 内部就是 hashtable)。
- ✅ 允许用 `tokio::time::DelayQueue` 实现 TTL(对应 Redis 的 active expiration)。

判断标准:**如果用户用 `redis-cli` 跟我们对接,他能不能从行为上区分我们的实现和真 Redis**?能 → 是模拟 → F24。不能 → 是合规简化。

> M4.1 (ADR-0011 §#5) charter-doc update:M3.1 charter 原文 "❌ 不准用 `tokio::sync::broadcast` 替代真 Pub/Sub subscription tracking" 在 ADR-0009 后变成 constitution-vs-decision drift (F1 sub-pattern)。pre-M4 audit 实际确认 broadcast + `Arc<Vec<u8>>` 共享 + 自然 fan-out 的 trade-off 合理,旧约束 over-prohibit。本节按 ADSD F1 "charter doc 跟随 reality update" 原则修正。

## 2. 本 case 的 oracle(F23-A 防御)

**所有协议正确性测试必须对照真 `redis-cli`(或 docker `redis:7-alpine`)做 round-trip**:

```bash
# 启动我们的 server
cargo run -p redis-server -- --port 6380 &

# 同步起一个真 redis
docker run --rm -d -p 6379:6379 --name redis-oracle redis:7-alpine

# 对每个 command 跑 round-trip
for cmd in "PING" "SET foo bar" "GET foo" "INCR counter" "EXPIRE foo 100"; do
    our=$(redis-cli -p 6380 $cmd)
    oracle=$(redis-cli -p 6379 $cmd)
    [ "$our" = "$oracle" ] || echo "DIVERGENCE on '$cmd': ours='$our' oracle='$oracle'"
done
```

测试脚本固化在 `tests/oracle.sh`,CI gate 4 必须跑它。

## 3. 命令实现的优先顺序

按 `redis-cli --help` 出现频率倒推 + ADSD F22 cadence-aware(先把第一波修好再扩):

**Wave M1**(协议 + 基础 KV) — ✅ shipped @ e24a8d2:
1. `PING` / `ECHO` / `QUIT` / `SELECT 0`(server 必备)
2. `SET key val` / `GET key` / `DEL key` / `EXISTS key`
3. `INCR` / `DECR` / `INCRBY` / `DECRBY`
4. `EXPIRE key seconds` / `TTL key` / `PERSIST key`
5. `TYPE key` / `KEYS pattern`

**Wave M2**(SvelteKit UI 接 SSE) — ✅ shipped @ bf4307c:
6. `/api/stats` / `/api/keys` SSE
7. UI:dashboard / keys / pubsub 三页

**Wave M3**(Pub/Sub + AOF) — ✅ shipped @ bf4307c:
8. `SUBSCRIBE channel` / `UNSUBSCRIBE` / `PUBLISH`
9. AOF append + replay on restart

**Wave M4**(release):
10. ✅ M4.1 critical fixes shipped @ d02aa55(ADR-0011)
11. ✅ M4.2 release artifacts + doc sweep(ADR-0012)
12. ✅ M4.3 Tauri desktop frontend source + managed `redis-server` sidecar lightweight gate(ADR-0013);full bundle/signing verification 仍是 release-readiness work;rust-embed 不再是 v0.1.0 primary blocker

## 4. 引用结构

- `redis-protocol` crate:**纯函数**,RESP encode/decode。无 IO。
- `redis-storage` crate:存储 + AOF + expiry。无网络。
- `redis-server` crate:Axum + tokio + RESP TCP listener。依赖前两者。
- `web/` 是 SvelteKit project;M4.3 起 primary frontend surface 转为 Tauri desktop app + managed `redis-server` sidecar(ADR-0013)。rust-embed 不再是 v0.1.0 blocker。

依赖单向:`server → storage` / `server → protocol` / **`storage → protocol`(仅为 AOF wire compatibility,ADR-0010 论证)**。**`protocol → storage` 反向禁止**;**`server → server` 不允许**(避免循环)。

> M4.1 (ADR-0011 §#6) charter-doc update:原文 "依赖单向, 不允许反向 import" 在 M3.2 (ADR-0010) 后变成 constitution-vs-decision drift — `redis-storage` 拉 `redis-protocol` 复用 `Frame::to_bytes` 给 AOF 编码,这是 F24 defence (不重新发明 RESP wire),合理。本节明确接受这条单向边,反向 (`protocol → storage`) 仍然禁止。

## 5. 性能 SLO(不是必须达成,但要测出来)

| 指标 | 目标 | 测法 |
|---|---|---|
| `SET` ops/s(单连接) | ≥ 50k | `redis-benchmark -p 6380 -t set -n 100000` |
| `GET` ops/s(单连接) | ≥ 100k | 同上 |
| 内存占用(空 server) | ≤ 10 MiB | RSS |
| 启动到 listening | ≤ 200 ms | wallclock |
| TCP 连接数 | ≥ 1000 | `redis-benchmark -c 1000` |

达不到不算 fail,但**必须在 `docs/agent/findings/m4-perf-baseline.md` 实测落盘**。

## 6. 双语 doc 边界

- ADR / finding:**双语**(zh + en)
- README / CLAUDE.md(本文件):双语不强制,**优先中文**
- 代码注释:**英文**(Rust 生态惯例)
- commit message:**英文优先,scope 可以中文**

---

**End. 其它沿用顶层 CLAUDE.md。**
