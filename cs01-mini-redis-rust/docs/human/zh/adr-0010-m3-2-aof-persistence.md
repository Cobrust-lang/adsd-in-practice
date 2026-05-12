# ADR-0010 中文摘要:M3.2 AOF 持久化

> 完整 ADR 见 [docs/agent/adr/0010-m3-2-aof-persistence.md](../../agent/adr/0010-m3-2-aof-persistence.md)。

## 决策(紧凑)

| 子项 | 选 |
|---|---|
| AOF format | **RESP-encoded `Frame::to_bytes()`** — 跟 Redis 一致,可 `redis-cli --pipe` 重放 |
| 哪些命令进 AOF | **SET/DEL/EXPIRE/PERSIST/INCR/DECR** 共 6 个 writable;read-only 不进;SUBSCRIBE/UNSUBSCRIBE/PUBLISH 不进(volatile) |
| 写入路径 | **`Store::execute` 内部 hook** + mpsc 推到后台 writer task(不阻塞 RESP 路径) |
| fsync 策略 | `--aof-fsync` flag,**`everysec` 默认** + `always` / `no` 可选 |
| TTL 漂移 | 写原 `EXPIRE k seconds` (相对),replay 时按当下重计算;接受 < 1 秒漂移(跟真 Redis 一致) |
| 损坏尾部 | log warn + 接受 file 长度作为 truncate 点(候选 finding) |
| Replay 顺序 | **replay 完才 bind listener**(deterministic ready 信号) |
| Oracle | 新 `tests/oracle_aof.py`:重启 round-trip vs `real redis --appendonly yes`,7 fixture |

## 数字目标

- backend test ≥ 260(M3.1 baseline 243, +17+)
- oracle 矩阵 35/35(22 RESP + 6 pubsub + 7 AOF restart)

## 接受的债

- AOF rewrite(同 key 多次写压缩)留 M4 / v0.2
- 损坏 warn-and-truncate:M4 决定升级 refuse-to-start
- replay 完才 accept 意味大 AOF 启动慢(< 100MB 可接受)

## 状态

`accepted` — 2026-05-12
