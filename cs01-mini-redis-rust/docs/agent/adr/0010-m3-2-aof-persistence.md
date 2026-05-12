---
adr: 0010
title: M3.2 — AOF append-only persistence + replay-on-restart
status: accepted
date: 2026-05-12
case: cs01-mini-redis-rust
supersedes: none
last_verified_commit: 35f4257
---

# ADR-0010: M3.2 AOF persistence

## Context

M3.1 closed(Pub/Sub + UI swap)。cs01 Wave M3 最后一块是 AOF(append-only file)持久化 + restart replay,关闭整个 Wave M3。M4 是 release-readiness。

Redis AOF 设计核心:
- 把每个**写命令**(SET / DEL / EXPIRE / PERSIST / INCR / DECR / SUBSCRIBE 不算)以 RESP 编码 append 到磁盘文件
- 启动时把 AOF 重放 — 重建内存 state
- fsync 策略:`always` / `everysec` / `no`
- AOF rewrite 把 N 个 SET 同一 key 压成 1 个(M3.2 **不做**,M4 / v0.2 评估)

cs01 main.rs 已经 reserved `--aof <path>` CLI flag(M1.3 起标 "M3 reserved")。

待定决策:

1. **AOF format**:RESP-encoded(跟 Redis 一致,oracle 能读)还是自己定义?
2. **哪些命令进 AOF**:writable only?ECHO / PING / SELECT / QUIT / KEYS / TTL / TYPE / EXISTS 这些都 read-only,不进。SUBSCRIBE / UNSUBSCRIBE / PUBLISH **不进**(volatile, no replay 价值)
3. **写入路径**:在 `Store::execute` 内部 hook,还是 server 层把 dispatch 后的 frame copy 一份?
4. **fsync 策略**:always(每命令)/ everysec(1Hz 后台 task)/ no(让 OS 决定)
5. **TTL 漂移**:重启时 EXPIRE 100 已经过了 50 秒,replay 用绝对时间还是相对剩余?
6. **空 / 不存在的 AOF**:cold start
7. **AOF 损坏处理**:replay 半截 truncated file 怎么办?
8. **Replay 期间能否接受新 RESP 连接**:全 replay 完才 accept,还是同时?
9. **Oracle 扩展**:重启 round-trip 验证 — 复杂度?

约束:
- AOF 不引入热路径阻塞 I/O(用 `tokio::sync::mpsc` 把命令推后台 writer task)
- 不引入新 workspace dep(`tokio::fs` 已经能 async write file)
- 不允许 .unwrap() / 不准热路径 alloc
- F23-A oracle:重启后 RESP 行为完全等同 baseline + Redis 真行为

## Decision(紧凑)

| 子项 | 选 | 拒绝 |
|---|---|---|
| **AOF format** | **RESP-encoded `Frame`** — 直接复用 `Frame::to_bytes(&Frame::Array(...))` | 自定义 binary(F24:重复造轮子);JSON 每行(不便 redis-cli inspect) |
| **哪些命令进 AOF** | **SET / DEL / EXPIRE / PERSIST / INCR / DECR / SET-with-EX**;ECHO/PING/SELECT/QUIT/KEYS/TTL/TYPE/EXISTS/GET 不进;SUBSCRIBE/UNSUBSCRIBE/PUBLISH 不进 | 全部命令(无意义,会爆) |
| **写入路径** | **`Store::execute` 内部 hook** — 持 `aof: Option<AofWriter>`,写命令成功后 push 命令到 mpsc | server 层 copy frame(双 codec 路径,F1 候选;且 dispatch 后才知道是不是 writable) |
| **fsync 策略** | **`everysec` 默认**(1Hz 后台 task fsync)+ CLI `--aof-fsync always/everysec/no` | `always`(性能差 10×);`no`(数据丢) |
| **TTL 漂移** | **写绝对时间** — AOF 里 `EXPIRE k <abs-unix-ms>` 自创变种 `EXPIREAT` 风格;replay 时如果绝对时间已过去就 `DEL k` 而非 SET-EX。**但**:这违反 F24(自定义 wire 命令)。**改方案**:仍写原 `EXPIRE k seconds`,replay 时这条命令把"几秒前的 seconds"当真当下重计算 — 接受 **TTL 延长 = AOF 滞后时间**(典型 < 1 秒,跟真 Redis 一致行为) | 自创 EXPIREAT 变种(F24);存绝对 instant(replay 时无对应 RESP 命令) |
| **空 / 不存在 AOF** | `--aof <path>` 但 path 不存在 → 创建空文件;`--aof` 不传 → 无持久化 | 启动失败(刚部署用户体验差) |
| **AOF 损坏** | replay 时 `Frame::parse` 失败 → log warn + truncate-on-write(写新命令时把损坏尾部覆盖) | panic / refuse to start(P1 deploy 痛点) |
| **Replay 期间 accept**:**先 replay 完再 bind listener** | 提供 deterministic "ready" 信号;部分 replay 状态接 client 会让 GET 间歇返回错值 |
| **Oracle 扩展** | 新 `tests/oracle_aof.py`:启 our server with `--aof /tmp/...`,跑命令,kill,重启 same `--aof`,assert state 等同 + 跟真 Redis 7 also-aof-restart 行为对照 | 单 bash 重启不便 |

### writable command set(锁)

| Command | AOF? | 备注 |
|---|---|---|
| SET (含 EX) | ✅ | AOF 写 `*3..SET k v` 或 `*5..SET k v EX n` |
| DEL | ✅ | |
| EXPIRE | ✅ | `*3..EXPIRE k n` |
| PERSIST | ✅ | `*2..PERSIST k` |
| INCR / DECR | ✅ | |
| ECHO / PING / SELECT / QUIT | ❌ | read-only |
| GET / EXISTS / KEYS / TYPE / TTL | ❌ | read-only |
| SUBSCRIBE / UNSUBSCRIBE / PUBLISH | ❌ | volatile state, no value to replay |

### Async writer 模型

```rust
pub struct AofWriter {
    tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    // task handle kept so writer doesn't get cancelled
    _task: Arc<JoinHandle<()>>,
}
impl AofWriter {
    pub fn new(path: PathBuf, fsync: FsyncPolicy) -> io::Result<Self> { /* spawn writer task */ }
    pub fn append(&self, encoded_frame: Vec<u8>) { self.tx.send(encoded_frame).ok(); }
}
```

writer task:
```rust
async move {
    let mut file = tokio::fs::OpenOptions::new().create(true).append(true).open(&path).await?;
    let mut buf = Vec::with_capacity(4096);
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Some(bytes) => { file.write_all(&bytes).await?; if matches!(fsync, FsyncPolicy::Always) { file.sync_data().await?; } }
                    None => break,
                }
            }
            _ = interval.tick(), if matches!(fsync, FsyncPolicy::Everysec) => { file.sync_data().await?; }
        }
    }
}
```

### Replay 流程

`main.rs` cold start:
1. Build `Store::new()`
2. If `--aof <path>` provided AND path exists:
   - Open file for reading
   - Read all bytes into `BytesMut`
   - Loop `Frame::parse(&buf)` → `from_frame` → if writable command, `store.execute` silently (no AOF append during replay)
   - Tail invalid frame → log warn, abandon rest
3. Create `AofWriter` (mode: append to same path)
4. Set `store.aof = Some(writer)`
5. Bind listeners + start accept loop

**During replay, AOF writer is OFF**(`store.aof = None`)。
**After replay**,`store.aof = Some(writer)` 然后 new writes 进 AOF。

### Oracle for AOF

新 `tests/oracle_aof.py`:
1. 启 our server with `--aof /tmp/cs01-aof-ours.aof`
2. 启 real redis container with `--appendonly yes --appendfilename "appendonly.aof"`
3. 对两个跑 7 个写命令(SET / SET-EX / EXPIRE / DEL / INCR / DECR / PERSIST)
4. kill both
5. 重启 our server with same `--aof`,重启 real redis
6. assert GET / EXISTS / TTL / TYPE 对每个 key 结果一致

完整 oracle 矩阵:M1.4 22 RESP + M3.1 6 pubsub + M3.2 7 AOF restart-roundtrip = **35 oracle commands**(每个 fixture 还含多 RESP step,所以 effective test 数 > 35)。

## Decision summary

| # | Deliverable | Crate |
|---|---|---|
| 1 | `redis-storage::aof::{AofWriter, FsyncPolicy}` | storage |
| 2 | `Store::with_aof(path, fsync)` constructor + `store.execute()` hook for writable commands | storage |
| 3 | `Store::replay_from_path(&Path)` — pure replay, no AOF append during | storage |
| 4 | `redis-server::main.rs` — `--aof <path>` + `--aof-fsync always/everysec/no` CLI flags + replay before bind | server |
| 5 | `crates/redis-storage/tests/aof.rs` — AOF write + replay unit + integration | storage |
| 6 | `crates/redis-server/tests/server_e2e.rs` — restart-roundtrip e2e via spawn | server |
| 7 | `tests/oracle_aof.py` — real redis container vs our server restart-roundtrip | tests |
| 8 | `tests/oracle.sh` — add oracle_aof.py call + cleanup |
| 9 | findings:`m3-2-aof-replay-corruption-handling.md`(损坏尾部 warn-and-truncate 行为) | docs |

## Consequences

### 正面

- 跟 Redis AOF 同 RESP format → 用户可以 `cat appendonly.aof | redis-cli --pipe` 玩
- `everysec` 默认是合理的 perf/durability 平衡
- restart-roundtrip oracle 是 F23-A 最强应用(全 round-trip)
- 替换 M1.3 main.rs reserved 的 `--aof` 占位

### 负面 / 接受的债

- 无 AOF rewrite:long-running 写多次同 key 会 file growth(M4 / v0.2)
- TTL 漂移 ≤ AOF 队列等待时间(典型 < 1 sec)
- 损坏尾部 warn-and-truncate 行为:**finding 候选**(M4 决定是否改 refuse-to-start)
- Replay 完才 accept connections 意味着大 AOF 启动慢,但 dev 用 size 都 < 100MB 接受
- `everysec` fsync 失败时 silent log → finding 候选(M4 升级 P0 error)
- AOF 跟 Pub/Sub 完全分离(对的);跟 SSE 控制面也无 interaction

### 不可逆性

- 完全可逆。`--aof` 不传 = M3.1 行为。AofWriter / Store::with_aof 加 / 删都不影响 public API。

## Done Criteria(falsifiable)

### Write path

- [ ] `Store::with_aof("/tmp/.../foo.aof", Everysec)` 创建 AofWriter
- [ ] `Store::execute(Command::Set {k, v, None})` 写完 → AOF 文件含 `*3\r\n$3\r\nSET\r\n$<n>\r\n<k>\r\n$<n>\r\n<v>\r\n`
- [ ] `Store::execute(Command::Get {k})` **不进 AOF**(GET 是 read-only)
- [ ] SUBSCRIBE / PUBLISH **不进 AOF**
- [ ] AOF tail bytes 跟 RESP encode 完全一致(可以用 redis-cli pipe)

### Replay

- [ ] `Store::replay_from_path("/tmp/...")` 读 AOF + 重建 state + AOF writer 在 replay 期间 disabled
- [ ] Empty AOF replay 不 panic
- [ ] Non-existent path replay 不 panic
- [ ] Corrupted tail(truncated frame)→ replay 处理前面 valid frames + log warn + 接受 file 长度作为 truncate 点

### CLI

- [ ] `cargo run -p redis-server -- --aof /tmp/cs01.aof` 启动正常
- [ ] `--aof-fsync always` / `everysec` / `no` 各跑过 e2e
- [ ] 没 `--aof` 时 in-memory only(M3.1 行为)

### TTL across restart

- [ ] SET k v EX 60 → kill → wait 5s → restart → GET k 返 v + TTL 约 55 (±2)
- [ ] SET k v EX 1 → kill → wait 2s → restart → GET k 返 nil(key 已过期被 PostRestart 立刻清理 — replay 时 EXPIRE 重计算 = 1 sec since restart,active expiry 1 sec 后 fire)

### Restart round-trip

- [ ] Server-A 跑 SET k1 v1 / SET k2 v2 EX 100 / DEL k1 → kill
- [ ] Server-B 重启 same `--aof` → `GET k1` = nil,`GET k2` = v2,`TTL k2` ≈ 100

### Oracle

- [ ] `tests/oracle_aof.py` 7 fixture 对照 real Redis,each fixture 重启 round-trip 一致
- [ ] `tests/oracle.sh` 调用 oracle_aof.py(opt-in via CS01_RUN_ORACLE=1)
- [ ] 全 oracle 矩阵 22 RESP + 6 pubsub + 7 AOF = **35/35 match**

### Gates

- [ ] fmt / clippy / build / test / doc-coverage 全过
- [ ] frontend-gate(无新改动,baseline 不退化)
- [ ] backend test count ≥ 260(M3.1 baseline 243,M3.2 加 ~20+)

## Cross-references

- ADR-0001 stack(tokio::fs already in `["full"]` features)
- ADR-0003 storage layout(`Inner` 加 `aof: Option<Arc<AofWriter>>` 字段,跟 subscribers 同层)
- ADR-0005 TCP listener(server::run startup 顺序变:先 replay 再 bind)
- ADR-0006 max-frame-size(AOF replay 时 frame parser 同上限)
- ADR-0009 Pub/Sub(SUBSCRIBE 等不入 AOF — 文档化)
- 文件:
  - `crates/redis-storage/src/aof.rs`(新建)
  - `crates/redis-storage/src/lib.rs` 扩 Store::with_aof / replay_from_path
  - `crates/redis-storage/tests/aof.rs`(新建)
  - `crates/redis-server/src/main.rs` --aof / --aof-fsync flags + replay step
  - `crates/redis-server/tests/server_e2e.rs` restart-roundtrip
  - `tests/oracle_aof.py`(新建)
  - `tests/oracle.sh` 增加 aof harness call
  - `docs/agent/findings/m3-2-aof-replay-corruption-handling.md`(新建)

## Notes

- AOF 文件路径默认无,**显式开启**:`--aof /var/lib/cs01/appendonly.aof`(用户必须显式选 path,跟真 Redis `appendonly yes` 不同)
- replay 期间用 `Frame::parse` + max-frame-size guard 保护(file 损坏 / malicious file)
- writer task 用 `BufWriter` 减少 syscalls(可选,M3.2 不强求)
- `Store::with_aof` 是 alternative constructor;原 `Store::new()` 保持 backward-compatible(无 AOF)

## Implementation deltas (post-impl, 2026-05-12)

记录原 Decision 没明说但落地时定下来的 6 个细节,都在 Decision sub-decisions 的边界内:

1. **`AofMsg` 双 variant 消息**(`Append(Vec<u8>)` / `Flush(oneshot::Sender<()>)`):原 ADR 写 mpsc 的 message 是 `Vec<u8>`,但 graceful shutdown / 测试都需要一个 "checkpoint" 路径。改成 enum 比开第二条 channel 简单,writer task 的 `select!` arity 不变。`Flush` ack 是 oneshot,sync 后 send。`Store::aof_flush().await` + `main.rs` 在 ctrl_c 之后 await 它,作为durability anchor — 不然 `subprocess.terminate()` 把 process 杀掉时 mpsc backlog 丢失(oracle 第一次跑被这个咬过)。
2. **`Store::attach_aof(self, ...).await -> Self` 取代直接 `with_aof + 二次 replay`**:原 ADR 的 main flow 是 "new → replay → with_aof",but `with_aof` 内部又是 `new + open writer`,导致 replay 的 Inner 状态被丢弃。`attach_aof` 接受一个 already-populated store,只 graft AofWriter,不重建 Inner。`with_aof` 现在是 `new().attach_aof(...)` 的 convenience wrapper。
3. **`Store::execute` = `aof_encode → execute_no_aof → 条件 append`**:原 ADR 说"hook in execute",落地时 split 成两个方法。`execute_no_aof` 是 public(replay 用),`execute` 在 reply 不是 `Reply::Error(_)` 时 append。INCR-on-non-integer 这种 user-error 不进 AOF — 不然 replay 会重新返回相同 error,浪费 IO。整数解码 test `incr_error_does_not_append_to_aof` 守这条。
4. **`parse_writable_frame` 私有 helper 在 storage 里复制 dispatch 的 6 个 verb 解析**:storage 不依赖 server::dispatch(layer rule, cs01 CLAUDE.md §4)。落地复制了 SET / DEL / EXPIRE / PERSIST / INCR / DECR 的 RESP-array 解析。Hand-edited AOF 含 GET / TYPE 之类的 read-only 命令时,parser 返回 `None` 并被 replay 静默跳过 — 安全但被 `replay_skips_non_writable_frames_without_failing` test 覆盖。
5. **SIGINT-only graceful shutdown**(non-Windows):tokio 默认只 handle `ctrl_c`(SIGINT),不接 SIGTERM。`subprocess.Popen.terminate()` 发 SIGTERM 直接 kill process,mpsc queue 丢失。Oracle harness (`tests/oracle_aof.py::stop_our_server`) 改为 `proc.send_signal(signal.SIGINT)`,文档 + 注释里都说明了。M4 release-readiness 可加 SIGTERM handler。
6. **`commands_total` 计数器在 replay 期间 不 增长**:`Store::execute_no_aof` 不触发 handle_conn 的 fetch_add(那是 server crate per-conn 计数),所以 replay 完后 `state.commands_total == 0`。测试 `replay_does_not_inflate_commands_total` 守这条 — 否则 dashboard 在重启后会看到"启动就 N 个命令"的迷惑数字。

## Final oracle matrix verification (post-impl, 2026-05-12)

本地 `CS01_RUN_ORACLE=1 bash tests/oracle.sh` 跑 35 / 35 通过:

- 22 RESP fixtures (M1.4 baseline)
- 6 pubsub fixtures (M3.1)
- 7 AOF restart-roundtrip fixtures (M3.2,8 observations vs real `redis:7-alpine --appendonly yes`)

`oracle_aof.py` 用独立 docker container + 独立 port(避免和 baseline 容器抢 `/data`),kill-and-restart 双方,然后 diff GET / EXISTS / TTL / TYPE 8 项 — 全 match。
