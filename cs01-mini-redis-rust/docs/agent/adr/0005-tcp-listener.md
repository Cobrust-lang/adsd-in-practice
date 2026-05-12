---
adr: 0005
title: RESP TCP listener — accept-loop + per-conn task + BytesMut buffer drain
status: accepted
date: 2026-05-12
case: cs01-mini-redis-rust
supersedes: none
last_verified_commit: 3a8c58d
---

# ADR-0005: RESP TCP listener

## Context

M1.2 收尾后,`redis-protocol::Frame`、`redis-storage::Store` 和 `redis-server::dispatch::from_frame` 都齐了,但 `redis-server/src/main.rs` 还是 M1.0 的 scaffold,只 `println!` 一句话就退。

M1.3 必须把以下闭环跑通:

```
TcpListener::accept
  → spawn(handle_conn)
    → loop {
        read_buf into BytesMut
        while let Ok((frame, n)) = Frame::parse(&buf) {
            buf.advance(n);
            let cmd = dispatch::from_frame(frame);
            let reply = store.execute(cmd?)?;
            socket.write_all(&reply_to_frame(reply).to_bytes()).await;
        }
      }
```

待定的决策面:

1. **TCP accept 模型**:`TcpListener::accept` + `tokio::spawn` 每连接一个 task,还是 `tokio_util::codec::Framed`?
2. **per-conn buffer 增长策略**:`BytesMut::with_capacity(4096)` + `read_buf` 一直 grow?有没有上限?
3. **`Reply → Frame` 映射放哪个 crate**:`redis-storage` 自己 to_frame?还是 `redis-server` 里的 free function?
4. **协议错误后处理**:发 `-ERR ...` 然后关 socket?还是 keep-alive?
5. **优雅停机**:监听 `SIGINT` 后 accept 停手,已建立连接继续服务还是立刻 close?
6. **E2E 测试 oracle**:M1.3 测试用 in-process `TcpStream` 自连接,还是直接拉 `redis:7-alpine` docker?
7. **`SELECT 0`、`QUIT`、`ECHO`**:cs01 CLAUDE.md §3 列了它们;M1.3 是否就实现完?

约束:
- 不允许 panic(F5 silent miscompile 防御);所有 IO 错误走 `?` 到 task 顶层 log + drop
- 不准在热路径 alloc(每连接的 buffer reuse,parse 完 advance 不 reallocate)
- 测试不准 sleep(用 `tokio::net::TcpStream` 自连接 + `oneshot::channel` 同步)
- F23-A:oracle 测必须能跟真 redis-cli round-trip(放 `tests/oracle.sh`,CI gate optional)

## Options Considered

### Option A: 经典 `accept → spawn(handle_conn)` + `BytesMut` + 手动 drain(选中)

```rust
let listener = TcpListener::bind(addr).await?;
loop {
    let (socket, peer) = listener.accept().await?;
    let store = store.clone();
    tokio::spawn(async move {
        if let Err(e) = handle_conn(socket, store).await {
            tracing::warn!(peer = %peer, error = %e, "conn closed with error");
        }
    });
}

async fn handle_conn(mut socket: TcpStream, store: Store) -> io::Result<()> {
    let mut buf = BytesMut::with_capacity(4096);
    loop {
        if socket.read_buf(&mut buf).await? == 0 {
            return Ok(()); // peer closed
        }
        loop {
            match Frame::parse(&buf[..]) {
                Ok((frame, n)) => {
                    let reply = process(&store, frame);
                    let bytes = reply_to_frame(reply).to_bytes();
                    socket.write_all(&bytes).await?;
                    buf.advance(n);
                }
                Err(ProtocolError::Incomplete) => break, // need more bytes
                Err(ProtocolError::Invalid(msg)) => {
                    let bytes = Frame::Error(format!("ERR {msg}")).to_bytes();
                    let _ = socket.write_all(&bytes).await;
                    return Ok(()); // close conn after sending error
                }
            }
        }
    }
}
```

- **Pros**:
  - 跟 ADR-0002 的 "caller manages buffer + Incomplete sentinel" 完全对齐,**零额外抽象**
  - 单测可写:用 `TcpStream::connect` 自连,纯 in-process,不 sleep
  - per-conn task 是 tokio 标准模式,故障隔离天然(一个连接 panic 不影响别的)
  - `Store::clone()` 内部是 `Arc::clone`(ADR-0003),共享开销 = 1 个 atomic inc
- **Cons**:
  - 单 reactor 单 accept loop:超过 ~50k 长连接前没事,M3 Pub/Sub 大订阅时再评估
  - 手写 drain loop:每个连接的 buffer 累积要小心,但 ADR-0002 已经把 `parse` 设计成 pure function,drain 模式标准
  - `Reply → Frame` 映射要写一遍,但只 6 个 variant

### Option B: `tokio_util::codec::Framed` + 实现 `Decoder + Encoder`

```rust
struct RespCodec;
impl Decoder for RespCodec { ... fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Frame>, _> ... }
impl Encoder<Frame> for RespCodec { ... }

let mut framed = Framed::new(socket, RespCodec);
while let Some(frame) = framed.next().await { ... framed.send(reply).await? ... }
```

- **Pros**:
  - 把 buffer 管理交给 `tokio-util`,代码更短
  - `Sink + Stream` 接口干净,容易加 backpressure
- **Cons**:
  - **多一层抽象**:`Decoder::decode` 需要把 ADR-0002 的 `(Frame, usize)` 适配到 `Option<Frame>`,要拷贝 split 逻辑
  - 引入 `tokio_util` 的 codec feature(虽然已经有 time feature 用 DelayQueue,但 codec 是另一个 feature flag)
  - F24 候选:把已经设计好的 pure-function parser 包成 codec,**用框架替代手写,等于 primitive-as-everything 的反面 — 抽象屏蔽细节**;调试时排查 buffer drain 边界要剥两层
  - testing 时 `Framed<TcpStream, RespCodec>` 没法用现成 `TcpStream` 客户端测,要拉一个真的 `redis-cli` 才能验

### Option C: `tokio_util::codec::Framed` + per-conn dedicated thread

跳过,**严重 over-engineering**,M1.3 不评估。

### Option D: `monoio` / `tokio-uring` 零拷贝

跳过,跟 ADR-0001(tokio 锁定)冲突,**v0.2 再说**。

## Decision

**选 Option A**。

理由:

1. **抽象层数最少**:ADR-0002 已经把 parser 设计成 pure function 返回 `(Frame, usize)`,Option A 是最 natural 的消费形态;Option B 在它上面再加 `Decoder` trait 等于多一层。
2. **测试 ROI 最高**:in-process `TcpStream::connect` 测试无需 docker、无需真 client,gate 4 可以 cover 整个 E2E 闭环。
3. **故障隔离自然**:per-conn `tokio::spawn`,task panic 不串台。
4. **跟下游决策不冲突**:M3 加 Pub/Sub 时,subscriber state per task,Option A 是天然容器;Option B 要把 codec 拆开重新组装 Sink/Stream split。
5. **F24 防御**:Option B 是"用框架抽象掉协议细节",违反 cs01 §1 "不允许用 framework 掩盖 primitive";Option A 明确暴露 drain loop。

### 关键子决策一并定下

| 子项 | 决策 |
|---|---|
| `Reply → Frame` 映射 | 放 `redis-server` crate 里 free function `reply_to_frame(Reply) -> Frame`(storage 不该懂 RESP,ADR-0004 同款 layer 论证) |
| 协议错误后 | 发 `-ERR ...` 然后 close socket(对齐真 Redis 行为) |
| 优雅停机 | M1.3 简化:`tokio::signal::ctrl_c()` → 立刻 break accept loop;在飞连接靠 `tokio::spawn` 自然结束 — M3 Pub/Sub 时升级为 drain 模式 |
| Buffer 上限 | M1.3 不做硬上限;`BytesMut` 默认增长策略;**注释里标 TODO(M3) max-frame-size guard** — F5 hardening 候选 |
| `SELECT 0` 行为 | 接受任意 `SELECT <int>`,返回 `+OK`(单 DB 实现,只为 redis-cli 兼容);非整数返回 `-ERR invalid DB index` |
| `QUIT` | 返回 `+OK` 然后 close socket(标准 Redis) |
| `ECHO msg` | 返回 `$<len>\r\n<msg>\r\n`(`Reply::Bulk(Some(msg))`) |
| E2E 测试 | M1.3 用 in-process `TcpStream` 客户端做 round-trip;`tests/oracle.sh` 留 stub,真 docker oracle 留 M1.4 实装 |

## Consequences

### 正面

- 关键路径只有一层 abstraction(parser → dispatch → store → encoder),栈深度浅,排查友好
- 用 `tokio::net::TcpStream` 自连测试 = gate 4 cover E2E,gate 4 跑 < 5s
- per-conn 故障不串台
- `reply_to_frame` 在 server crate,后期加 Pub/Sub 的 push frame 直接同文件加 variant

### 负面 / 接受的债

- 单 accept loop 在 50k+ 长连接时会被 listener queue 排队;v0.2 评估
- BytesMut 无上限 → 恶意客户端发 ~$<u64::MAX>\r\n 可触发大 alloc;**M3 加 max-frame-size guard**(开 finding)
- 优雅停机 M1.3 只到 "停 accept,等 task 自然死",未做 in-flight RESP 命令的 deadline → 升级 finding 候选

### 不可逆性

- **完全可逆**。后期换 `Framed` codec 是 single-file refactor,不动 protocol/storage crate;换 monoio 是异步 runtime 切换,影响 Cargo workspace 但不触 public API。

## Done Criteria(falsifiable)

- [ ] `cargo run -p redis-server -- --port 6380` 后台启动,`tracing::info!` 显示 listening
- [ ] `redis-cli -p 6380 PING` → `PONG`
- [ ] `redis-cli -p 6380 SET foo bar` → `OK`;`GET foo` → `"bar"`
- [ ] `redis-cli -p 6380 SET k v EX 1` → `OK`;sleep 1.1s;`GET k` → `(nil)`
- [ ] `redis-cli -p 6380 DEL a b c` 返回实际删除数
- [ ] `redis-cli -p 6380 INCR counter` 多次,返回递增整数
- [ ] `redis-cli -p 6380 ECHO hi` → `"hi"`
- [ ] `redis-cli -p 6380 SELECT 0` → `OK`;`SELECT 9` → `-ERR invalid DB index`(单 DB 但兼容协议)
- [ ] `redis-cli -p 6380 QUIT` → `OK` 后连接断开
- [ ] 未知命令 → `-ERR unknown command 'XYZ'`,连接不掉
- [ ] Pipelining:一次 send 3 个命令,server 按顺序回 3 个 reply
- [ ] 半包:客户端分 2 次 send `*1\r\n` 然后 `$4\r\nPING\r\n`,server 正确组帧
- [ ] 协议错误:发 `garbage\r\n` → server 回 `-ERR ...` 然后断开
- [ ] Ctrl-C → process 退出码 0,日志显示 "shutting down"
- [ ] 集成测试 `tests/server_e2e.rs` 用 `TcpStream::connect` 测以上 12 条
- [ ] `bash tests/oracle.sh` 是 placeholder,exit 0(M1.4 真接 docker)
- [ ] Gate 1-5 全过

## Cross-references

- ADR-0001 stack choice(tokio + Axum 锁)
- ADR-0002 RESP framing(parse / to_bytes API)
- ADR-0003 storage layout(Store::execute / Reply)
- ADR-0004 command routing(from_frame / Command)
- 代码新增 / 修改:
  - `crates/redis-server/src/main.rs`(重写,从 scaffold 到 real server)
  - `crates/redis-server/src/server.rs`(新增,accept + handle_conn)
  - `crates/redis-server/src/encode.rs`(新增,`reply_to_frame`)
  - `crates/redis-server/tests/server_e2e.rs`(新增,12+ E2E 测试)
  - `tests/oracle.sh`(新增 placeholder)

## Notes

- M1.4 把 `EXPIRE / TTL / PERSIST / TYPE / KEYS` 接入,顺手把 oracle.sh 接真 docker。
- 客户端 pipelining 测试用 `TcpStream::write_all` 一次写多个 `*..\r\n` block,断言 server `read` 出多个 reply。`tokio::io::BufReader` 在 server 端,read_buf 自然 batch。
- `Reply::Error` 到 `Frame::Error` 时,**Error 的内容不应该带 `-` 前缀**,前缀由 `Frame::to_bytes` 加。
- "SELECT 非整数" 测试:`redis-cli` 不会发非整数,所以用 raw `TcpStream` 直发 `*2\r\n$6\r\nSELECT\r\n$3\r\nabc\r\n` 测。
