# ADR-0005 中文摘要:RESP TCP listener

> 完整 ADR 见 [docs/agent/adr/0005-tcp-listener.md](../../agent/adr/0005-tcp-listener.md)。

## 决策

经典 `TcpListener::accept` + 每连接 `tokio::spawn(handle_conn)` + `BytesMut` 累积 + 手动 drain `Frame::parse` 直到 `Incomplete`:

- 每连接独立 task,故障隔离天然
- 不引入 `tokio_util::codec::Framed`(那是把 ADR-0002 已经设计好的 pure-function parser 再包一层,F24 候选)
- `Reply → Frame` 映射放在 `redis-server::encode` free function(storage 不该懂 RESP,跟 ADR-0004 layer 论证同款)
- 协议错误:发 `-ERR ...` 后 close socket
- 优雅停机:M1.3 简化版 — `tokio::signal::ctrl_c()` 停 accept,在飞 task 自然结束;M3 升级 drain 模式
- E2E 测试用 in-process `tokio::net::TcpStream::connect`,不依赖 docker;真 oracle 留 M1.4
- 顺手实现 `ECHO / SELECT 0 / QUIT`(cs01 CLAUDE.md §3 列入 M1)

## 拒绝的方案

- **`tokio_util::codec::Framed`**:多一层抽象,违反 cs01 §1 "不允许 framework 掩盖 primitive"
- **`monoio` / `tokio-uring` 零拷贝**:跟 ADR-0001(tokio 锁)冲突,留给 v0.2

## 接受的债

- 单 accept loop:50k+ 长连接前没事,M3 评估
- `BytesMut` 无上限:恶意 `$<u64::MAX>` 可触发大 alloc,**M3 加 max-frame-size guard**(已开 finding 候选)
- 优雅停机不做 in-flight deadline,M3 升级

## 状态

`accepted` — 2026-05-12
