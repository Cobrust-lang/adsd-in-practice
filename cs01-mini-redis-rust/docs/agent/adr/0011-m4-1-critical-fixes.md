---
adr: 0011
title: M4.1 — Critical fixes (security + constitution drift + AOF hardening + parser strictness)
status: accepted
date: 2026-05-12
case: cs01-mini-redis-rust
supersedes: none
last_verified_commit: pending
---

# ADR-0011: M4.1 critical fixes

## Context

8-agent audit team(finding `m4-pre-release-audit-team-aggregation.md` at commit `7cd8ef0`)产出 ~50 unique findings + 12 cross-validated HIGH。M4 拆三波:M4.1 critical fixes(本 ADR)、M4.2 doc-sweep + release artifacts、M4.3 v0.1.0 tag。

M4.1 处理**真 bug + constitution-vs-ADR drift + security 加固**,共 13 items:

| # | Source | Issue |
|---|---|---|
| 1 | Sec HIGH-1+2 | RESP listener 无 AUTH + default `0.0.0.0` bind |
| 2 | Sec HIGH-3 | 无 max-clients cap → DoS |
| 3 | Sec HIGH-4 | `Frame::parse` 数组递归无深度限 → stack overflow attack |
| 4 | Sec MED-1 | AOF 文件 0o644 → 应 0o600 |
| 5 | Code-Q HIGH-1 | cs01 CLAUDE.md §1 broadcast pub/sub 禁令 vs ADR-0009 决策冲突 |
| 6 | Code-Q HIGH-2 | cs01 CLAUDE.md §4 layer rule 单向 vs ADR-0010 storage→protocol 边违反 |
| 7 | Aleksandr #3 | AOF `Always` policy misleading(Reply::Ok 返回早于 sync_data) |
| 8 | Deep HIGH-4 | AOF mpsc `unbounded_channel` → slow disk OOM |
| 9 | Deep MED-1 + MED-4 | `replay_from_path` 用 `std::fs::read` 全部 + 双拷贝 |
| 10 | Deep MED-6 | `SET k v EX 60 GARBAGE` 不 reject(parts.len() ≥ 5 不严)→ F23-A oracle gap |
| 11 | Deep HIGH-3 + Aleksandr | `main.rs:140-146` confused-then-corrected comment block |
| 12 | Deep LOW-1 | `Frame::Integer` encoder 评论说 itoa-style 实际 `.to_string()` alloc |
| 13 | Aleksandr #1 + #2 | pubsub `recv_any_subscription` `StreamMap` + `Store::subscribe` read-then-write |

## Decision(紧凑)

### Security / DoS

#### 1. Default bind `127.0.0.1` + 显式 `--bind 0.0.0.0` opt-out

- `main.rs` Args.bind default `"127.0.0.1"`
- README 加 "binding to all interfaces" 警告
- AUTH 命令本 wave **不实现**(scope creep),CLI 加 `--insecure-no-auth` flag 文档化"无 auth 是 0.1 已知限制"(顶层 §1.3 显式标 stub)

#### 2. `--max-clients <N>` flag(default 1000,匹配 cs01 SLO)

- 在 `server::run` accept loop 里 check `state.connections_active.load() >= max_clients`,超过则:发 `-ERR max number of clients reached\r\n` + 立即 close
- 不 spawn task — early reject
- 加 unit test:1001-th conn 收到 max-clients error

#### 3. `Frame::parse` 递归深度限制

- 私有 `parse_with_depth(input, depth)`,`MAX_FRAME_DEPTH = 32`
- 深度超限 → `Err(ProtocolError::Invalid("frame nested too deeply"))`
- 公开 `Frame::parse` 入口 depth = 0
- 加 测试:`*1\r\n*1\r\n*1\r\n... (33 层)` → error

#### 4. AOF 文件 0o600 (Unix only)

- `crates/redis-storage/src/aof.rs`:`OpenOptionsExt::mode(0o600)` 用 cfg(unix) 包,Windows 跳过
- 对齐真 Redis 行为

### Constitution drift (F1 sub-pattern fix)

#### 5. cs01 CLAUDE.md §1 update — broadcast pub/sub 接受

- 改原文 "❌ 不准用 `tokio::sync::broadcast` 替代真 Pub/Sub subscription tracking"
- 改为:"⚠️ `tokio::sync::broadcast` 接受作 Pub/Sub fan-out(ADR-0009 已论证);**但**:lagging client 必须显式 disconnect 而不是 silently drop msg;subscriber state 必须真在 per-conn 持(`ConnState::Subscribed { rxs }`),不准 stash to global。判断标准维持:`redis-cli` round-trip 跟真 Redis 行为可区分 → 是简化 → F24"
- 理由:实际 ADR-0009 broadcast 决策 trade-off 合理(`Arc<Vec<u8>>` 共享 + 自然 fan-out);constitution 当时 over-prohibit 是过严。Charter doc 跟随实际 engineering decision update 是 ADSD 默认 proceed 原则的应用

#### 6. cs01 CLAUDE.md §4 update — accept `storage → protocol` 单向边

- 改原文 "依赖单向" 子句
- 改为:"依赖单向:`server → storage` / `server → protocol` / **`storage → protocol`(仅为 AOF wire compatibility,ADR-0010 论证)**。**`protocol → storage` 反向禁止**;**`server → server` 不允许**(避免循环)"
- 理由:AOF format 复用 `Frame::to_bytes` 是 F24 defence(用真 RESP 字面,不重新发明 wire),反过来如果 server 层 copy frame 是 F1 double-source 风险。M3.2 P9 加这边 dep 是合理 trade-off,constitution 需要追上

### AOF 加固

#### 7. AOF `Always` 真同步:加新 policy variant 或 rename

**选**:加 `FsyncPolicy::AlwaysBlocking`(语义:`Reply::Ok` 返回前真 fsync),保留 `Always` 但 rename 行为:`Always` 现在表示"每命令都触发 fsync,但在 writer task,**异步**"(实质等同 `Everysec` + flush-on-each);**P0**:文档化两个 mode 区别

- `AlwaysBlocking` 实现:`Store::execute` 写 AOF 时 send `AofMsg::AppendBlocking(bytes, oneshot)`,await 完成才返回
- `Always` 保持 M3.2 行为(send + return),但 doc 明确"durability lag ≤ writer task scheduling latency"
- README + ADR-0010 addendum 同步 lock 语义

#### 8. AOF mpsc bounded

- `aof::new` 用 `mpsc::channel(8192)` 而非 unbounded
- `append` 改 `try_send`:满则 log error + 跌回 `blocking_send`(背压到 RESP path)
- 加 doc comment 说明:"under sustained slow disk,RESP path will block 等 fsync 跟上 — 这是设计的 backpressure"
- 加 test:fill the queue,assert blocking_send 触发

#### 9. `replay_from_path` streaming

- 用 `tokio::fs::File::open + BufReader` 流式读
- `BytesMut::with_capacity(4096)` 起;`read_buf` 累积;`Frame::parse` drain
- 不 hold 整个 file 进内存
- 加 test:8 GiB synthetic AOF replay 内存峰值 < 100 MiB(可选 — 测试不便,文档化即可)

### Dispatch 严格性

#### 10. `parse_set` 严格 arity:`parts.len() == 5` 才允许 EX form

- `parts.len() == 4` → 已经 reject(EX 缺 secs)
- `parts.len() == 5` 且 parts[3] EX → ttl form
- `parts.len() ∈ {6, 7, ...}` → `-ERR syntax error`(对齐真 Redis)
- 加 oracle fixture:`SET k v EX 60 GARBAGE` → 真 Redis -ERR / 我们 -ERR

### Pub/Sub perf

#### 11. `recv_any_subscription` 用 `tokio_stream::StreamMap`

- 当前:每 select-loop 迭代 box N 个 `Receiver::recv` future
- 改:`StreamMap<String, BroadcastStream<Arc<Vec<u8>>>>` 持 across iterations
- `BroadcastStream` 已经 wrap broadcast::Receiver,`StreamMap::poll_next` 不重 box
- 加 引入 `tokio-stream` workspace dep(已经存在 in `redis-server`)

#### 12. `Store::subscribe` read-then-write-on-miss

- 当前:`inner.write()` 然后 `entry().or_insert_with`
- 改:`inner.read()` + `if let Some(tx) = guard.subscribers.get(channel)` 直接 subscribe;miss 才 `drop(read)` + `inner.write()` + insert
- 不可不顾 ABA:read drop 到 write acquire 中间另一 thread 可能 insert;再 read inside write block check

### Doc / comment 清理

#### 13. main.rs:140-146 重写 + Frame::Integer 注释 fix

- `main.rs` 改为:"// Graft the AofWriter onto the just-replayed store; the writer task only owns the new file handle and shares the existing inner map via Arc."
- `Frame::Integer` 改为:"// Format i64 — single allocation via String; itoa optimisation deferred to v0.2."

## Decision summary

| # | Deliverable | Crate / file |
|---|---|---|
| 1 | bind default `127.0.0.1` + `--insecure-no-auth` placeholder doc | redis-server/src/main.rs + README |
| 2 | `--max-clients <N>` flag (default 1000) + early reject in `server::run` | redis-server/src/main.rs + server.rs + state.rs |
| 3 | `Frame::parse` `MAX_FRAME_DEPTH = 32` private recursion limit | redis-protocol/src/lib.rs |
| 4 | AOF file mode `0o600` (cfg unix) | redis-storage/src/aof.rs |
| 5 | cs01 CLAUDE.md §1 update broadcast clause | cs01-mini-redis-rust/CLAUDE.md |
| 6 | cs01 CLAUDE.md §4 update layer-rule clause | cs01-mini-redis-rust/CLAUDE.md |
| 7 | AOF `FsyncPolicy::AlwaysBlocking` variant + `Always` semantics doc | redis-storage/src/aof.rs + ADR-0010 addendum |
| 8 | AOF mpsc `channel(8192)` + `try_send`→`blocking_send` backpressure | redis-storage/src/aof.rs |
| 9 | `Store::replay_from_path` streaming `tokio::fs` | redis-storage/src/lib.rs |
| 10 | `parse_set` strict arity (==5 for EX form) + oracle trailing-token test | redis-server/src/dispatch.rs + tests/oracle.sh |
| 11 | `StreamMap` for pubsub fan-in | redis-server/src/server.rs + Cargo.toml workspace dep |
| 12 | `Store::subscribe` read-then-write-on-miss | redis-storage/src/lib.rs |
| 13 | main.rs:140-146 comment cleanup + Frame::Integer comment fix | redis-server/src/main.rs + redis-protocol/src/lib.rs |

## Consequences

### 正面

- 关 7 个 cross-validated HIGH + 多个 single-agent HIGH
- 关 F1 "constitution-vs-ADR drift" 子模式(charter doc update 到 reality)
- AOF 在生产场景下真有 backpressure(unbounded mpsc 是 OOM 等待发生)
- Frame parser 抗 stack-overflow attack
- 默认 bind 不再 LAN-exposed(0.0.0.0 → 127.0.0.1)
- F23-A oracle 加 trailing-token rejection 维度(覆盖 happy-path-only 之外)

### 负面 / 接受的债

- AUTH 命令本 wave **不**实现(scope creep);M5 / v0.2 处理
- TLS 同上
- 8 GiB+ AOF streaming replay 测试无法跑(磁盘 + 时间限制),只 doc
- `replay_from_path` 改 streaming 需要 async — 改 `pub async fn`,Wave M3.2 的 同步 signature 变 breaking(M4 是合理时机)
- Aleksandr's "MAXMEMORY policy" 不在本 wave(M5+ feature)

### 不可逆性

- 可逆。所有改动是 type / API 内部调整;`AlwaysBlocking` 是新 variant 不破现有。CLAUDE.md update 是文档,可 revert

## Done Criteria(falsifiable)

### Security

- [ ] `cargo run -p redis-server` 启动默认 bind `127.0.0.1:6380`(redis-cli `-h 127.0.0.1 -p 6380 PING` → PONG)
- [ ] `--bind 0.0.0.0` opt-out 仍 work(LAN-bind test)
- [ ] `--max-clients 5` + 6 个 client 同时连 → 第 6 个收到 `-ERR max number of clients reached\r\n` 然后 close
- [ ] `Frame::parse` 33 层深 nested `*1\r\n*1\r\n...\r\n+OK\r\n` → `Err(ProtocolError::Invalid("frame nested too deeply"))`
- [ ] AOF 文件 (cfg unix) `stat -c %a /tmp/foo.aof` = `600`

### Constitution

- [ ] cs01 CLAUDE.md §1 broadcast clause 改完 + commit msg cite 本 ADR
- [ ] cs01 CLAUDE.md §4 layer-rule clause 改完
- [ ] grep "❌ 不准用 `tokio::sync::broadcast`" 无 hit(原句删除)

### AOF

- [ ] `FsyncPolicy::AlwaysBlocking` variant exists,test:写完 5 commands,kill -9 process,重启 — 全部 commands 都 replay
- [ ] `--aof-fsync alwaysblocking` CLI value accepted
- [ ] AOF mpsc capacity 8192 + backpressure test:fill queue,assert RESP path blocks
- [ ] `Store::replay_from_path` 是 `pub async fn`(用 tokio::fs)
- [ ] 5 MiB AOF replay 测试通过,内存增量 < 10 MiB

### Dispatch

- [ ] `SET k v EX 60 X` → `-ERR syntax error`
- [ ] `SET k v EX 60` → 成功
- [ ] oracle.sh 加 1 个 trailing-token 测 case,跟真 Redis 对照

### Pub/Sub

- [ ] `recv_any_subscription` 用 StreamMap;cargo build clean
- [ ] `Store::subscribe` 已存在 channel 时只 acquire read lock(可以单测计数读 vs 写 lock counts)

### Comments

- [ ] `main.rs:140-146` 无 "we replay AGAIN" 误导文本
- [ ] `Frame::Integer` encoder 注释跟实际行为一致

### Gates

- [ ] fmt / clippy / build / test / doc-coverage 全过
- [ ] oracle.sh 35+1 = 36/36 commands match
- [ ] frontend-gate (no frontend changes; not regressed)
- [ ] backend test count ≥ 280(M3.2 baseline 269,M4.1 加 ≥ 11)

## Cross-references

- ADR-0002 RESP framing(`parse` 加 depth limit)
- ADR-0005 TCP listener(handle_conn 加 max-clients early reject)
- ADR-0007 SSE control plane(AppState 字段不动)
- ADR-0009 Pub/Sub(StreamMap 重写;Store::subscribe lock 优化)
- ADR-0010 AOF persistence(AlwaysBlocking variant + mpsc bound + streaming replay)
- finding `m4-pre-release-audit-team-aggregation.md`(audit source-of-truth)
- 顶层 CLAUDE.md §1.1 invariants(constitution drift fix 是 charter update)

## Notes

- 本 ADR 是 ADSD F1 "constitution-vs-ADR drift" sub-pattern 的**第一次正面修复**实例 — charter doc 跟 implement reality 同步 update,而不是 forcing implementation to roll back。提议作为 ADSD upstream `case-study/` 子节
- AUTH / TLS scope creep:**显式不在 M4.1**,在 cs01 CLAUDE.md §1.1 / README "out of scope" 列出
- Aleksandr 提的 "AppState wrap in `Arc<AppState>`" 是 LOW perf nit,M5+ 评估
- M4.2 (doc sweep + release artifacts) 在 M4.1 ship 之后开 ADR-0012
