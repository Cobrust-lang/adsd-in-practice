# ADR-0009 中文摘要:M3.1 Pub/Sub

> 完整 ADR 见 [docs/agent/adr/0009-m3-1-pubsub.md](../../agent/adr/0009-m3-1-pubsub.md)。

## 决策(紧凑)

| 子项 | 选 |
|---|---|
| Subscriber state | **Inner.subscribers: HashMap<String, broadcast::Sender<Arc<Vec<u8>>>>**(parking_lot RwLock 同域,跟 KV 同锁) |
| Fan-out | **tokio::sync::broadcast**(同 ADR-0007 stats/keys 模式),`Arc<Vec<u8>>` 共享 payload 避免 N 倍 copy |
| Channel 匹配 | **M3.1 精确 only**;PSUBSCRIBE glob 留 M3.2+(F22 cadence-aware) |
| sub mode | **handle_conn 持 ConnState enum**(`Normal` / `Subscribed { rxs }`),tokio::select on (read_buf, rxs.recv);sub mode 下 GET/SET/... 一律 `-ERR Can't execute ... in this context` |
| PUBLISH 返回 | **`Integer(receiver_count)`**(broadcast::Sender::send 返回 N) |
| counters | sub mode commands_total / connections_active **保持 inc**,跟真 Redis 对齐 |
| `/api/pubsub` SSE | 1Hz `{channels: [{name, subscribers}]}` snapshot;message firehose 留 M4 |
| `/pubsub` UI | **read-only dashboard**(channel/sub 表 + "Use redis-cli to publish/subscribe" banner);web→RESP bridge 留 M4 |
| F23-A oracle | 新 `tests/oracle_pubsub.py`(Python redis-py + docker redis:7-alpine)+ 6 个 stateful fixture |

## 数字目标

- backend test ≥ 220(+20)
- frontend vitest ≥ 28(+3)
- oracle 28/28(M1.4 22 + M3.1 6 stateful)

## 接受的债

- Subscriber sender 不 evict(M4 release-readiness 加 GC)
- PSUBSCRIBE 留 M3.2
- UI 是 read-only(web→pub 桥留 M4)
- Lagged subscriber 直接 disconnect(比 Redis 激进,finding 候选)

## 状态

`accepted` — 2026-05-12
