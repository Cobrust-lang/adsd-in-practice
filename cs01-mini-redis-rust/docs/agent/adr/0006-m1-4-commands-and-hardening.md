---
adr: 0006
title: M1.4 — EXPIRE/TTL/PERSIST + TYPE + KEYS glob + PING optional + max-frame-size + docker oracle
status: accepted
date: 2026-05-12
case: cs01-mini-redis-rust
supersedes: none
last_verified_commit: 0800d86
---

# ADR-0006: M1.4 commands + hardening

## Context

M1.3(ADR-0005)关闭后,82 tests 全过,核心 RESP 闭环 OK。剩余的 Wave M1 命令(cs01 CLAUDE.md §3 Wave M1 items 4-5)还没落地:

- `EXPIRE key seconds` — 修改已存 key 的 TTL
- `TTL key` — 返回剩余秒数 / -1 (无 TTL) / -2 (key 不存在)
- `PERSIST key` — 清掉 TTL
- `TYPE key` — 返回类型 simple string
- `KEYS pattern` — glob 匹配返回 array

加上 P9 在 M1.3 完成报告里点的 followup 和 ADR-0005 留的债:

- `tests/oracle.sh` 真接 docker `redis:7-alpine` round-trip(F23-A oracle)
- `max-frame-size` guard 防恶意 `$<u64::MAX>` 大 alloc(ADR-0005 §"接受的债")
- `PING hello` 可选 message(Redis 标准,M1.3 parse_ping 直接拒了 parts>1)

M1.4 把这 8 件事打包,作为本 case M1 wave 收尾。

待定决策:

1. **EXPIRE/PERSIST 怎么跟 DelayQueue 协作**?DelayQueue 不支持按 key 取消已 enqueue 的 entry。要么:(a) 直接再 insert 一条,让旧 entry 到点 fire 时 expiry task 看到 entry.expires_at 已变 / 已无 → skip(M1.2 现有逻辑),(b) 引入 `delay_queue::Key` 句柄 + 维护 key→Key map 来真正 cancel,(c) 自己写个 BTreeMap<Instant, key>。
2. **TTL 整数语义**:-2 / -1 / 剩余秒数(`i64`)— 跟真 Redis 对齐。剩余秒数向下取整还是向上?
3. **KEYS glob 实现**:自己写 ~50 行 matcher,引入 `globset` crate,或退化只支持 `*` / `?`?
4. **TYPE 返回值**:v0.1.0 只有 string + none;`Reply` 需要 SimpleString 还是 BulkString?
5. **PING optional message 怎么表示在 `Command::Ping`**?
6. **max-frame-size 阈值 + 配置面**:固定 512 MiB?CLI flag?per-frame 还是 buffer 总长?
7. **docker oracle 脚本**:shell + docker CLI 还是 docker-compose?如何控制启动顺序?CI 跑不跑?

约束:
- 不改 Frame / Store::execute / from_frame 的现有签名(不 breaking)
- F24 不允许:KEYS 不准用 `BTreeMap` "假装" `KEYS *`(目前底层就是 hashbrown,O(n) scan 是 Redis 真行为)
- F23-A oracle 必须真 docker,不能用 mock client
- 不准热路径 alloc:KEYS 内部 collect pattern matches 用 `Vec::with_capacity` 预估
- 不准 sleep 测 TTL,用 `tokio::time::pause` + `advance`(M1.2 已经做对)

## Options Considered

### EXPIRE / PERSIST × DelayQueue 协作

#### Option A: 直接 re-insert,旧 entry fire 时 skip-by-mismatch(选中)

逻辑:
- `EXPIRE k 100` → 改写 `entry.expires_at = now + 100s` + send 新 (key, 100s) 到 ttl_tx
- 旧 DelayQueue entry 到点 fire,task 检查 `entry.expires_at <= now`,**不匹配 → skip,但不删 key**(M1.2 现有逻辑就这样)
- `PERSIST k` → `entry.expires_at = None` + 不动 DelayQueue;旧 entry 到点 fire 时检查 `expires_at == None` → skip

- **Pros**:
  - 不改 DelayQueue 接口,M1.2 已有逻辑刚好兼容(check 已经在 `entry.expires_at.is_some_and(|t| t <= Instant::now())`)
  - 极简实现,Store::execute 加 EXPIRE/PERSIST 两个 arm 即可
- **Cons**:
  - 短期内多次 EXPIRE 会在 DelayQueue 堆 N 条 stale entry,GC 开销 O(N) 周期性扫;v0.2 优化
  - 内存:每条 EXPIRE 一个 entry,极端场景下 1M EXPIRE/s 会撑爆,M3 评估

#### Option B: 维护 `HashMap<String, delay_queue::Key>` 真 cancel

- **Pros**:精确控制
- **Cons**:Inner 多一层数据结构,Store::execute 都要持有 `&mut Inner`,**lock 占用时长拉长**;DelayQueue 的 `remove(&Key)` 文档不保证 O(1);**过早优化**

#### Option C: 抛弃 DelayQueue,自己写 `BTreeMap<Instant, Vec<String>>`

- **Pros**:简单,可控
- **Cons**:tokio 的 DelayQueue 已经做了 hierarchical timer wheel,自己写就是重发明轮子;F24 边缘(用 BTreeMap 模拟 timer wheel)

**选 A** — Option A 是 M1.2 现有逻辑的自然延伸,不破契约。

### TTL 整数语义

- **选**:严格按 Redis:`-2 = key 不存在`,`-1 = key 存在但无 TTL`,否则剩余秒数(`i64`,向下取整 `floor((expires_at - now).as_secs())`)
- 实现:`Reply::Integer(value)`,无歧义

> **Addendum (2026-05-12, commit `0800d86`)** — Phase 2 跑 docker oracle 时发现真 Redis 7 用的是 **round-half-up**(`(pttl_ms + 500) / 1000`,见 redis 源 `src/expire.c`),不是 floor。P9 在 sprint 内修正为 round-half-up,oracle 22/22 match。本节 §"选" 行的 `floor(...)` 表述**保留作 audit 轨**,但实际语义按 Addendum 为准。详见 finding `m1-4-f23a-oracle-caught-ttl-rounding-spec-bug.md`,F23-A 在 sprint 内捕获 spec-vs-real-Redis 偏差的**正面案例**。

### KEYS glob 实现

#### Option A: 自己写 ~50 行 matcher 支持 `*` / `?` / `[a-z]` / `\` 转义(选中)

- **Pros**:
  - 控制完全;无新依赖
  - cs01 §1 F24 防御 — KEYS 是 Redis 内部的 stringmatchlen.c,自己实现是 spec 还原而不是简化
  - 测试容易:input/expect 对照 cases
- **Cons**:
  - ~50 LOC + ≥20 tests;P9 工作量适中

#### Option B: 引入 `globset` crate

- **Pros**:成熟 + fast
- **Cons**:
  - `globset` 是路径风格 glob(支持 `**`,Redis KEYS 不支持),语义偏差需要 disable 部分功能 + wrapper
  - 引入 600+ LOC 依赖只为 KEYS,**过度**

#### Option C: 退化只支持 `*` / `?`

- **Pros**:超简单
- **Cons**:`KEYS user:*` / `KEYS [abc]*` 在真 Redis 都 work,我们退化 = oracle 失败 = F23-A 红旗

**选 A**。Glob matcher 单文件 `redis-storage::glob`,公开 `pub fn matches(pattern: &str, key: &str) -> bool`。

### TYPE 返回值

- `TYPE existing-string-key` → `+string\r\n`(SimpleString)
- `TYPE missing-key` → `+none\r\n`(SimpleString)
- 后续 wave 加 list / hash / set / zset 时,arm 扩展

实现:新 `Reply::SimpleString(String)` variant?或者用 `Reply::Bulk(Some(b"string".to_vec()))`?

**选**:复用现有 `Reply` 表面,加一个新 variant **`Reply::SimpleString(String)`**(对齐 RESP 的 `+string\r\n`)。`encode::reply_to_frame` 加一个 arm。

理由:不滥用 Bulk(Some) 假装 SimpleString — RESP 这两个 type 的字节都不一样,语义清楚区分。

### PING optional message

- 选:`Command::Ping { message: Option<Vec<u8>> }`
- `parse_ping`:1 part → `Ping { None }`;2 parts → `Ping { Some(bytes) }`;3+ → error
- `Store::execute(Ping { None })` → `Reply::Pong`(已有);`Ping { Some(b) }` → `Reply::Bulk(Some(b))`
- Reply::Pong → +PONG\r\n;Reply::Bulk(Some("hello")) → $5\r\nhello\r\n
- 真 Redis:`PING hello` 返回 `"hello"` bulk string,我们对齐

### max-frame-size guard

- **阈值**:`512 MiB`(对齐 Redis `proto-max-bulk-len` 默认)
- **位置**:在 `redis_server::server::handle_conn` 的 read_buf loop,**每次 read_buf 后**检查 `buf.len() > MAX_FRAME_SIZE`,超过 → 发 `-ERR Protocol error: frame too big`(同 Redis 字面)→ close
- **CLI flag**:`--max-frame-size <bytes>`,default 512 MiB,M1.4 接受
- **理由**:在 read_buf 后立刻检查,可以在还没 fully accumulated $<u64::MAX>\r\n 时就杀掉

### docker oracle 脚本

- **形式**:`tests/oracle.sh` shell + docker CLI(无 docker-compose 依赖,bootstrap 简单)
- **流程**:
  1. `docker run --rm -d -p 6379:6379 --name cs01-oracle redis:7-alpine`,等 1 秒
  2. `cargo run -p redis-server -- --port 6380 &` 后台,等 0.5 秒
  3. 对每个 cmd `(PING / SET k v / GET k / DEL k / EXISTS k / INCR / EXPIRE / TTL / PERSIST / TYPE / KEYS / ECHO / SELECT)` 用 `redis-cli -p 637X` 跑,**比较 stdout 完全相等**
  4. cleanup:`kill` 我们的 server,`docker stop cs01-oracle`
- **CI 集成**:opt-in via env `CS01_RUN_ORACLE=1`,默认 CI 不跑(避免 docker 依赖污染主 CI);本地 + nightly job 跑
- **fail-fast**:第一个 mismatch 立刻 exit 1,打印 ours / oracle / cmd
- **F23-A 真兑现** — 不再是 placeholder

## Decision

Single ADR landing all 8 deliverables:

| # | Deliverable | Crate |
|---|---|---|
| 1 | `Command::Expire / Ttl / Persist`,arms in dispatch + Store::execute (Option A re-insert DelayQueue) | storage + server |
| 2 | `Command::Type` + `Reply::SimpleString` variant | storage + server |
| 3 | `Command::Keys { pattern }` + `glob::matches` (Option A 自写 matcher) | storage |
| 4 | `Command::Ping { message: Option<Vec<u8>> }`(扩 existing variant) | storage + server |
| 5 | `max-frame-size` guard in `server::handle_conn`,512 MiB 默认 + `--max-frame-size` CLI | server |
| 6 | `tests/oracle.sh` 真 docker round-trip,opt-in CI gate | tests |
| 7 | TTL 语义严格对齐 Redis(-2 / -1 / 剩余秒数) | storage |
| 8 | KEYS pattern 支持 `*` / `?` / `[a-z]` / `\` 转义 | storage |

理由汇总:
- 8 件事互相高度耦合(Command enum 改 + dispatch + Store::execute + tests),拆 ADR 反而碎
- 全部可逆(Command variant 加 / 删都是 type change)
- M1.4 是 wave 收尾,closes Wave M1

## Consequences

### 正面

- 完成 cs01 CLAUDE.md §3 Wave M1 全部命令
- F23-A oracle 真兑现 — 命令字面跟真 Redis 对得上
- F5 max-frame-size 防御落地
- KEYS glob 自写 — 跟 ADR-0002 RESP parser 一样"primitive first"

### 负面 / 接受的债

- DelayQueue stale entry 堆积场景:M1.4 不优化,Option A 接受
- `KEYS *` O(n) scan:Redis 标准行为,**不修**;`SCAN` cursor 接到 M3
- `max-frame-size` 是 buffer total len,不是单 frame;v0.2 优化(可以 frame-size only)
- docker oracle 需要本地 docker;CI 默认不跑,**不是强制 gate**

### 不可逆性

- 完全可逆。每个 Command variant 加 / 删都是 type change,跨 case 不影响。

## Done Criteria(falsifiable)

### EXPIRE / TTL / PERSIST

- [ ] `Store::execute(Set { ttl_secs: None })` 然后 `Expire { key, seconds: 60 }` → `Reply::Integer(1)`(1 = success)
- [ ] `Expire { key: 不存在, seconds: 60 }` → `Reply::Integer(0)`
- [ ] `Ttl { key: 不存在 }` → `Reply::Integer(-2)`
- [ ] `Ttl { key: 存在但无 TTL }` → `Reply::Integer(-1)`
- [ ] `Set { ttl_secs: Some(100) }` + `Ttl { key }` → `Reply::Integer(~100)`(允许 ±1 秒漂移)
- [ ] `Persist { key: 有 TTL }` → `Reply::Integer(1)` + 之后 `Ttl` 返回 -1
- [ ] `Persist { key: 无 TTL }` → `Reply::Integer(0)`
- [ ] `Persist { key: 不存在 }` → `Reply::Integer(0)`
- [ ] EXPIRE 后 1 秒(用 `tokio::time::pause + advance`)key 真消失(`Get` 返回 nil)
- [ ] EXPIRE 多次同一 key,**最后那次生效**;stale DelayQueue entry 到点 skip

### TYPE

- [ ] `Type { key: existing-string }` → `Reply::SimpleString("string")`
- [ ] `Type { key: missing }` → `Reply::SimpleString("none")`
- [ ] 过期已 fire 的 key → `"none"`(不是 `"string"`)

### KEYS

- [ ] `Keys { pattern: "*" }` → array of all live keys
- [ ] `Keys { pattern: "user:*" }` → 只匹配 prefix
- [ ] `Keys { pattern: "user:?" }` → 单字符匹配
- [ ] `Keys { pattern: "[abc]*" }` → 字符集匹配
- [ ] `Keys { pattern: "\\*" }` → 字面 `*`(escape)
- [ ] 过期 key 不出现在 KEYS 结果里
- [ ] 空 DB → `Reply::Array(Some(vec![]))`
- [ ] 单元测试 `glob::matches` ≥ 20 case 含边界

### PING optional message

- [ ] `Ping { message: None }` → `Reply::Pong`
- [ ] `Ping { message: Some(b"hello".to_vec()) }` → `Reply::Bulk(Some(b"hello".to_vec()))`
- [ ] dispatch `*1\r\n$4\r\nPING\r\n` → `Command::Ping { message: None }`
- [ ] dispatch `*2\r\n$4\r\nPING\r\n$5\r\nhello\r\n` → `Command::Ping { message: Some(b"hello") }`
- [ ] dispatch 3+ parts → error

### max-frame-size

- [ ] CLI `--max-frame-size 1024` 启动后,client 发 `$2000\r\n<2000 bytes>\r\n` → server 回 `-ERR Protocol error: frame too big` 然后 close
- [ ] 默认 512 MiB 下,小 frame 完全不受影响
- [ ] e2e 测试 `frame_too_big_protocol_error`

### docker oracle

- [ ] `bash tests/oracle.sh` 启动 docker redis:7-alpine + 我们的 server + 跑 ≥ 15 个命令对比 + cleanup
- [ ] 没 docker 时 exit 0 with skip message(不阻塞 CI)
- [ ] env `CS01_RUN_ORACLE=1` 才真跑;否则 skip
- [ ] mismatch 时 exit 1,打印 `ours='X' oracle='Y'` 详情

### 全 5 gate green

- [ ] fmt / clippy -D / build / test / `bash ../_shared/doc-coverage.sh` 全过
- [ ] 总 test count ≥ 100(M1.3 是 82;M1.4 至少 +20)

## Cross-references

- ADR-0001 stack choice
- ADR-0002 RESP framing
- ADR-0003 storage layout(DelayQueue active expiry)
- ADR-0004 command routing(from_frame layer rule)
- ADR-0005 TCP listener(max-frame-size 在 server::handle_conn,oracle.sh placeholder)
- 代码新增 / 修改:
  - `crates/redis-storage/src/glob.rs`(新增,~50 LOC matcher + tests)
  - `crates/redis-storage/src/lib.rs`(扩 Command + execute arms + Reply::SimpleString)
  - `crates/redis-storage/tests/glob.rs`(新增,≥ 20 cases)
  - `crates/redis-storage/tests/store_basic.rs`(扩 EXPIRE/TTL/PERSIST/TYPE/KEYS tests)
  - `crates/redis-server/src/dispatch.rs`(扩 EXPIRE/TTL/PERSIST/TYPE/KEYS/PING-arg arms)
  - `crates/redis-server/src/encode.rs`(加 Reply::SimpleString arm)
  - `crates/redis-server/src/server.rs`(加 max-frame-size guard)
  - `crates/redis-server/src/main.rs`(加 --max-frame-size CLI flag)
  - `crates/redis-server/tests/server_e2e.rs`(扩 frame-too-big test)
  - `crates/redis-server/tests/dispatch.rs`(扩新命令解析 tests)
  - `tests/oracle.sh`(替换 placeholder,真 docker round-trip)

## Notes

- KEYS pattern matcher 别参考 Rust `std::path` glob — 那是 path-segment-aware 的,Redis KEYS 是单字符串。
- `Reply::SimpleString` 加入后,encode.rs 的 match 是 exhaustive,自动覆盖。
- DelayQueue Option A 的 stale entry 数学:N 个 key 各 EXPIRE M 次 → 队列内 N×M entry。M1.4 接受,M3 评估。
- oracle.sh `CS01_RUN_ORACLE=1` 是为了让 5-gate test (gate 4) 默认快;真要跑就 env 显式开。
- M1.4 完成后 cs01 进入 M2:SvelteKit UI + SSE control plane。M2 不写 Redis 命令,纯 UI 层。
