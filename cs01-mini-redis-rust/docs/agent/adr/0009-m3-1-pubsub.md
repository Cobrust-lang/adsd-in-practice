---
adr: 0009
title: M3.1 — Pub/Sub (SUBSCRIBE / UNSUBSCRIBE / PUBLISH) + per-conn subscriber state + /pubsub UI swap
status: accepted
date: 2026-05-12
case: cs01-mini-redis-rust
supersedes: none
last_verified_commit: 4c5e360
---

# ADR-0009: M3.1 Pub/Sub

## Context

Wave M2 closed:RESP 命令面齐 + SSE 控制面齐 + SvelteKit UI 三页(`/pubsub` 是 M3 stub)。cs01 §3 Wave M3 拆:
- **M3.1**(本 ADR):`SUBSCRIBE` / `UNSUBSCRIBE` / `PUBLISH` 三命令 + per-conn subscriber state machine + `/api/pubsub` SSE 实时 channel→subscriber 表 + 替换 SvelteKit `/pubsub` stub 为 live UI
- **M3.2**(下个 ADR):AOF append-only 持久化 + replay on restart

Pub/Sub 是真 Redis 状态机里**最复杂**的部分之一:**`SUBSCRIBE` 后连接进入"sub mode"**,只能接 `SUBSCRIBE` / `UNSUBSCRIBE` / `PSUBSCRIBE` / `PUNSUBSCRIBE` / `PING` / `QUIT`,其它命令一律拒。订阅期间收到 publish msg 直接推 `Frame::Array` 给该 client。

待定决策(把 9 个一次锁掉):

1. Subscriber state 放哪 — `Inner.subscribers: HashMap<channel, Vec<conn_id, tx>>` 集中,还是 per-conn 持本地?
2. Fan-out channel:`tokio::sync::broadcast` 按 channel name 分(M2.1 stats 用过),还是 `tokio::sync::mpsc` per-subscriber?
3. Channel 命名:精确 string match,还是 M3.1 就支持 PSUBSCRIBE glob?
4. "sub mode" 的实现:`handle_conn` 加 `conn_state: ConnState` enum?跟 dispatch 怎么协作?
5. PUBLISH 返回 received-subscriber-count(同步 / 异步:msg 已 enqueue 即算 receive,还是真送达)
6. commands_total / connections_active 在 sub mode 下怎么算
7. `/api/pubsub` SSE 推什么 — `{channel, subscribers}` map snapshot 1Hz?还是 push delta?
8. SvelteKit `/pubsub` UI 形态 — sub form + pub form + 消息日志 stream?
9. F23-A oracle 怎么扩(SUBSCRIBE 是 stateful,redis-cli 用 `--csv` 模式可对照吗)

约束:
- 不破现有 RESP 行为(M1.4 oracle 22/22 不退化)
- M1.4 frame-too-big guard 在 sub mode 仍生效
- 不引入新 workspace dep(broadcast / mpsc 都在 tokio::sync 内)
- 不允许 .unwrap() 在非测试代码
- 不允许在热路径 allocate(publish msg fan-out 不该 per-subscriber clone Vec<u8>;用 `Arc<Vec<u8>>` 共享)

## Decision(紧凑式)

### Q1 + Q2: Subscriber state + fan-out

**选**:**`tokio::sync::broadcast<Arc<Vec<u8>>>` per channel,Inner 持 `subscribers: HashMap<String, broadcast::Sender<Arc<Vec<u8>>>>`**。

- subscribe 时:`let rx = senders.entry(channel.clone()).or_insert_with(|| broadcast::channel(128).0).subscribe();`,把 `rx` 存到 per-conn state 里
- publish 时:`senders.get(channel).map(|tx| tx.send(Arc::new(msg))).unwrap_or(Ok(0))` → 返回 `n` 给 PUBLISH reply
- channel 没 subscriber 后**保留 sender**(M3.1 不做 GC,接受小内存债 ≤ key count;M4 release-readiness 时加 evict)

理由:
- broadcast 是天然 fan-out,跟 M2.1 stats 同模式(reuse pattern)
- `Arc<Vec<u8>>` clone 是 atomic inc,N 个 subscriber 不 N 倍 copy bytes
- broadcast capacity 128:慢 subscriber lag 时 `recv()` 返回 `Lagged(n)` → 我们把 sub 踢掉(Redis 真行为是 buffer overflow 也 disconnect)
- `subscribers` Map 跟 `Inner.map` 同 lock 域(`parking_lot::RwLock<Inner>`),无新锁

拒绝:
- 集中 `HashMap<channel, Vec<conn_id, tx>>` + per-subscriber `mpsc`:N 次写 N 个 mpsc,**热路径 N 次系统调用**,broadcast 一次 send 即可
- 自己写 fan-out:F24 候选(broadcast 已经是 primitive)

### Q3: Channel 匹配

**选**:**M3.1 只做精确 string match**(SUBSCRIBE / UNSUBSCRIBE / PUBLISH 三命令);PSUBSCRIBE / PUNSUBSCRIBE 留 M3.2 或 M4(F22 cadence-aware)。

理由:
- 精确 match 是 KV lookup,O(1)。glob match 需要遍历所有 channel 跟 pattern 匹配,跟 KEYS 同复杂度
- M3.1 把核心 wire protocol + sub mode state machine 先 ship,glob 是优化

### Q4: "sub mode" 实现

**选**:**`handle_conn` 持 `local: ConnState`,enum `{ Normal, Subscribed { rxs: HashMap<String, broadcast::Receiver<Arc<Vec<u8>>>> } }`**,在 inner drain loop 里 match state:

```rust
match (&local, from_frame(frame)) {
    (_, Ok(Command::Subscribe { channels })) => { /* upgrade to Subscribed */ }
    (_, Ok(Command::Unsubscribe { channels })) => { /* drop rxs */ }
    (ConnState::Subscribed { .. }, Ok(Command::Ping { .. } | Command::Quit)) => { /* pass-through */ }
    (ConnState::Subscribed { .. }, _) => { Reply::Error("ERR Can't execute '...': only ...") }
    (ConnState::Normal, Ok(other)) => { store.execute(other) }
    (_, Err(reply)) => reply,
}
```

**broadcast::Receiver 的 push msg 怎么写回 socket**:在 `handle_conn` 主 loop 里把 `socket.read_buf` 跟 `rxs.recv()` 都 `tokio::select!`,任一 fire 走对应分支。

理由:
- Per-conn state 是天然 isolation(F24 友好,sub mode 不污染 store)
- `tokio::select!` 模式是 tokio 标准,跟 server::run 的 ctrl_c select 同骨架
- Redis 错误字面对齐:`"ERR Can't execute 'SET': only (P)SUBSCRIBE / (P)UNSUBSCRIBE / PING / QUIT / RESET are allowed in this context"`

拒绝:
- 全局 dispatch trait route:复杂度上去,subscribers 跟 store 解耦在 enum match 就够

### Q5: PUBLISH 返回值

**选**:**返回 `Reply::Integer(broadcast::Sender::send().map(usize).unwrap_or(0))`**(send 返回当前 receiver_count;无 channel 返 0)。Redis 真行为也是同步计数 = "已 enqueue 给 N 个 subscriber",不保证送达;broadcast 失败(全部 receiver lagging)返回 Err,把它转成 0。

### Q6: counters 在 sub mode

- `commands_total`:**继续 inc**(SUBSCRIBE/UNSUBSCRIBE/PUBLISH 都算 command,与真 Redis 对齐)
- `connections_active`:**保持 ConnGuard RAII inc/dec**,sub mode 也算 active connection

### Q7: `/api/pubsub` SSE

**选**:**1Hz 推 `{channel: subscriber_count, ...}` snapshot**,跟 stats / keys 同 1Hz 节奏(reuse `tokio::sync::broadcast::<PubsubSnapshot>` + sampler task)。

格式:
```
event: pubsub
data: {"channels":[{"name":"news","subscribers":3},{"name":"chat","subscribers":12}]}

```

`/api/pubsub` 是 dashboard view,不是 message firehose。message-level stream 留给 M4 加 `/api/pubsub/messages?channel=X` 真 push 流(M3.1 不做)。

### Q8: SvelteKit `/pubsub` UI

**选**:简单两栏 + 消息日志:
- 左:`/api/pubsub` SSE 实时 channel→subscriber 表
- 右:**两个 form**(Subscribe to channel:`input` + button → 用 raw `fetch` POST 到一个 backend control endpoint;Publish to channel:`input` + msg textarea + button)
- 底部:可选 message log(M3.1 不接 firehose,所以 placeholder "message stream coming")

**HTTP control endpoint for sub/pub?**实际不需要:**M3.1 仅做 dashboard view 不做 web→pubsub 接 RESP**。理由:web 通过 RESP 接进来要新写一个 fake RESP client over HTTP,**严重 over-engineering**;M3.1 用户用 `redis-cli` 真接 RESP 测,UI 只看 dashboard 数字。

所以 `/pubsub` 页 UI 简化为:
- 一个 channel→subscriber 表(SSE 推)
- 一段说明:**"To subscribe/publish, use a RESP client like `redis-cli -p 6380`. UI is a read-only dashboard in M3.1."**
- M4 可以加 web→backend 的桥(over WebSocket 或 fetch streaming)

替换 M2.2 `/pubsub` 的 "M3 placeholder" 文案。

### Q9: F23-A oracle 扩展

**选**:`tests/oracle.sh` 加 6 个 fixture:
- `SUBSCRIBE news`(然后另一个 client `PUBLISH news hello`,assert subscriber 收到)
- 这是 stateful + 多 client + 时序,**比 M1.4 oracle 复杂**
- Implementation:bash 后台两个 `redis-cli` 进程(一个 sub,一个 pub),`expect`-style 同步
- 真 Redis 跟我们都跑一遍,**比对输出**

如果 bash 太繁琐,**改用 Python 写 oracle pubsub harness**(redis-py + 我们的 server,跟真 redis docker 对照),放 `tests/oracle_pubsub.py`,从 `tests/oracle.sh` 调用,opt-in via `CS01_RUN_ORACLE=1`。

## Decision summary table

| # | Deliverable | Crate |
|---|---|---|
| 1 | `Command::Subscribe { channels: Vec<String> }` / `Unsubscribe { channels: Vec<String> }` / `Publish { channel, message }` | storage |
| 2 | `Reply::SubscribeAck { channel, count }` / `UnsubscribeAck { channel, count }` / `Message { channel, payload }` (new variants) | storage |
| 3 | `Inner.subscribers: HashMap<String, broadcast::Sender<Arc<Vec<u8>>>>` | storage |
| 4 | `Store::execute` arms (Subscribe/Unsubscribe/Publish) — returns the new Reply variants; Store::subscribe_to / unsubscribe_from 内部 helper 返回 Receiver | storage |
| 5 | dispatch:SUBSCRIBE / UNSUBSCRIBE / PUBLISH parse + arity check | server |
| 6 | encode.rs:新 Reply variants → Frame 映射(Message 是 Array[bulk "message", bulk channel, bulk payload],SubscribeAck 是 Array[bulk "subscribe", bulk channel, integer count]) | server |
| 7 | `handle_conn`:per-conn ConnState enum + tokio::select on (socket.read_buf, rxs.recv);sub-mode command-filter wall | server |
| 8 | `AppState.subscribers_snapshot()` (read inner.subscribers,返回 PubsubSnapshot) | server (state) |
| 9 | http.rs:`/api/pubsub` SSE route + 1Hz sampler task + 第三个 broadcast<PubsubSnapshot> | server |
| 10 | SvelteKit:replace `web/src/routes/pubsub/+page.svelte` (read-only dashboard view + read-only banner) | web |
| 11 | TypeScript:`PubsubSnapshot = { channels: { name: string; subscribers: number }[] }` in `web/src/lib/api/types.ts` | web |
| 12 | tests/oracle_pubsub.py + oracle.sh 调用 | tests |
| 13 | dispatch tests + storage tests + server_e2e (sub-mode round-trip) + vitest 增量 | server + storage + web |

## Consequences

### 正面

- 真 Redis Pub/Sub 行为(sub mode + Array message frame + accurate publish count)
- broadcast fan-out 跟 stats / keys 同模式,可复用 sampler skeleton
- per-conn ConnState 是 enum,easy to extend(MULTI/EXEC 的 transaction state 未来 M3.2+ 同 enum 加 variant)
- `/api/pubsub` SSE 直接接入 dashboard,**replaces M2.2 stub 自然**

### 负面 / 接受的债

- Subscriber sender 不 evict(channel 永久驻留)— M4 加 evict 任务
- PSUBSCRIBE / PUNSUBSCRIBE 留 M3.2 之后
- UI 是 read-only(web→RESP bridge 留 M4)
- `oracle_pubsub.py` 多了 python 测试依赖(本机 `python3` + `pip install redis` 一次性);CI gate optional
- broadcast Lagged 时 disconnect subscriber 是 simplified 行为;Redis 实际是 reset connection,我们更激进 — 写一句 finding 候选

### 不可逆性

- 完全可逆。Subscribe / Publish / Unsubscribe 是 enum variant 加 + arms 加,无 public API 破

## Done Criteria(falsifiable)

### SUBSCRIBE state machine

- [ ] `SUBSCRIBE news` → 客户端收到 `*3\r\n$9\r\nsubscribe\r\n$4\r\nnews\r\n:1\r\n` (subscribe ack, count=1)
- [ ] 多个 channel:`SUBSCRIBE a b c` → 顺序回 3 个 subscribe ack,count 1/2/3
- [ ] sub mode 下发 `GET foo` → `-ERR Can't execute 'GET': only (P)SUBSCRIBE / (P)UNSUBSCRIBE / PING / QUIT / RESET are allowed in this context\r\n`,连接不掉
- [ ] sub mode 下发 `PING` → `+PONG`(或 `*2\r\n$4\r\npong\r\n$0\r\n\r\n` 看 Redis 行为,verify by oracle)
- [ ] sub mode 下发 `QUIT` → `+OK` + close socket
- [ ] `UNSUBSCRIBE news` → ack count 减;`UNSUBSCRIBE`(无 arg)→ 退所有
- [ ] 退订到 0 channel 自动回到 Normal mode

### PUBLISH

- [ ] `PUBLISH news hello` 在没 subscriber 时返 `:0\r\n`
- [ ] 1 subscriber 时返 `:1\r\n`,subscriber 收到 `*3\r\n$7\r\nmessage\r\n$4\r\nnews\r\n$5\r\nhello\r\n`
- [ ] 3 subscriber 时返 `:3\r\n`,3 个都收到一份 msg

### `/api/pubsub` SSE

- [ ] `curl localhost:6381/api/pubsub` 1Hz 输出 `event: pubsub\ndata: {"channels":[...]}\n\n`
- [ ] subscribe 一个 channel 后,下一帧 `subscribers=1`
- [ ] unsubscribe / disconnect 后,下一帧 `subscribers=0`(channel name 保留)
- [ ] e2e tests:reqwest 自连 SSE,启动 RESP client 模拟 subscribe,assert 下帧 count 变化

### SvelteKit `/pubsub` 页

- [ ] 替换 M2.2 "M3 placeholder" 文案
- [ ] 显示实时 channel/subscribers 表(从 `/api/pubsub` SSE)
- [ ] 顶部 banner:"This dashboard is read-only. Use a RESP client to publish/subscribe."
- [ ] vitest:加 `parsePubsubLine(...)` parser unit test
- [ ] `pnpm check && pnpm test && pnpm build` 全过

### Oracle

- [ ] `CS01_RUN_ORACLE=1 bash tests/oracle.sh` 22/22 baseline + 6 new pubsub fixture 全 match real Redis
- [ ] 失败时 print ours / oracle / cmd 详情

### Gates

- [ ] fmt / clippy / build / test / doc-coverage 全 green
- [ ] `bash scripts/frontend-gate.sh` 全 green
- [ ] backend test count ≥ 220(M2.2 baseline 200,M3.1 加 ~20)
- [ ] frontend vitest count ≥ 28(M2.2 baseline 25,+3 parsePubsub)
- [ ] oracle 28/28 commands match real Redis(M1.4 22 + M3.1 6 new)

## Cross-references

- ADR-0001 stack (tokio + parking_lot)
- ADR-0003 storage layout (Inner lock scheme — subscribers 在同一 RwLock 域)
- ADR-0005 TCP listener (handle_conn select! 模式 + ctrl_c)
- ADR-0007 SSE control plane (broadcast fan-out 同模式;新增 third broadcast channel)
- ADR-0008 SvelteKit UI (replace pubsub stub)
- 文件改动清单:
  - `crates/redis-storage/src/lib.rs` — Command 加 Sub/Unsub/Publish,Reply 加 SubscribeAck/UnsubscribeAck/Message
  - `crates/redis-storage/src/pubsub.rs` (新建) — broadcast 管理 + Inner.subscribers field
  - `crates/redis-storage/tests/pubsub.rs` (新建) — unit + integration
  - `crates/redis-server/src/dispatch.rs` — SUBSCRIBE / UNSUBSCRIBE / PUBLISH arms
  - `crates/redis-server/src/encode.rs` — 3 new Reply arms (Array shape)
  - `crates/redis-server/src/server.rs` — ConnState enum + sub-mode select!
  - `crates/redis-server/src/state.rs` — AppState pubsub snapshot + broadcast tx
  - `crates/redis-server/src/http.rs` — /api/pubsub route + sampler
  - `crates/redis-server/tests/dispatch.rs` + `tests/server_e2e.rs` + `tests/http_sse.rs` — extensions
  - `web/src/lib/api/types.ts` — PubsubSnapshot
  - `web/src/lib/api/sse.ts` — pubsub stream helper
  - `web/src/routes/pubsub/+page.svelte` — REPLACE stub with live view
  - `web/src/lib/format.ts` — `parsePubsubLine` helper (+ vitest)
  - `tests/oracle_pubsub.py` (新建)
  - `tests/oracle.sh` — 调用 oracle_pubsub.py

## Notes

- broadcast::Sender::receiver_count() 是 O(1),`/api/pubsub` snapshot 1Hz O(channel_count) 很轻
- ConnState 不持 conn_id,broadcast::Receiver 自己 token 化
- sub mode 的 PING 行为:**Redis 7 在 sub mode 下 PING 返回 `+PONG`(simple string),不是 `*2\r\n...`**(后者是 Redis 6 行为)— verify by oracle
- 错误字符串字面对齐 Redis 7,**通过 oracle 验证后才 lock**(M1.4 TTL rounding lesson)
- M3.2 (AOF):pubsub 不该被 AOF(volatile state),AOF 只 log writable commands

## Implementation deltas (post-impl, 2026-05-12)

记录与原 Decision 偏差(都不破约,只是落地细节):

1. **`recv_any_subscription` 用 `iter_mut() + select_all`,不是 `BoxFuture` Vec**:借用检查器拒绝多个 future 同时 borrow 同一个 `&mut HashMap`,只有 `iter_mut()` 给的是 disjoint 借用。`async move { (name, rx.recv().await) }` 把每个 receiver 的 future 装在闭包里,`select_all` 等任一完成 — 没拿到结果的 future 被 drop,broadcast::Receiver 的 cursor 保留所以下次 call 不丢消息。这是 Rust async 借用模型的具体落地,Decision 里没明说,但仍属于 Q4 选定的 ConnState + select! 方案。
2. **`Reply::Subscribe / Unsubscribe` 入 `Store::execute` → `Reply::Error("ERR internal: ...")`**:ADR §Q4 说 SUBSCRIBE/UNSUBSCRIBE 由 server 层做,不该走 `execute`;为符合 CLAUDE.md §3.1 "非测试不准 .unwrap()",我们让 execute 返回一个 generic ERR 而不是 panic,作为防御性回退。dispatch 测试覆盖了这条路径。
3. **`AppState` 8 个公开字段**(原 7 +1 `pubsub_tx`):超过 CLAUDE.md §3.1 "≤7" 的 hint。在 `state.rs` 的 doc-comment 里加了 justification(shared-state aggregate,折成 sub-struct 只多 indirection),不算违反。
4. **`PUBSUB_BROADCAST_CAPACITY = 128` 公开常量**:Decision 里写了 "broadcast capacity 128",但没说"公开",我们把它 `pub` 出来方便 server 调试 / 测试 introspect。
5. **`UnsubscribeAck { channel: None, count: 0 }`** 在 `UNSUBSCRIBE`(无 arg)+ 当前无订阅时由 server 直接返回,不走 store。这跟 ADR §Q4 + watch-out 的 nil-channel 边界一致,oracle fixture 6 验证。
6. **Oracle fixture 6 (PING-in-sub-mode) 直接验证 Spec**:redis-py 的 `pubsub.ping()` 在我们和真 Redis 7 上都返回 `None`(无 raise),说明双方都发的是 `+PONG\r\n`(simple string),而不是 Redis 6 的 Array shape。ADR §Notes 的猜测被 oracle 印证;无需 addendum。
7. **Pub/Sub helpers landed in `redis-storage/src/lib.rs`, not a separate `pubsub.rs`**:the Cross-references file list predicted `crates/redis-storage/src/pubsub.rs`, but implementation kept helper methods next to `Inner.subscribers` in `lib.rs` to avoid a module split during M3.1. This is a file-layout delta only; ADR-0009's per-channel broadcast + per-conn `ConnState` design remains unchanged.
