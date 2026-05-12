# ADR-0011 中文摘要:M4.1 critical fixes

> 完整 ADR 见 [docs/agent/adr/0011-m4-1-critical-fixes.md](../../agent/adr/0011-m4-1-critical-fixes.md)。

## 决策

M4 拆三波:M4.1 critical fixes(本 ADR)、M4.2 doc-sweep + release artifacts、M4.3 v0.1.0 tag。

本 wave 13 个 critical items,来自 8-agent audit:

**Security / DoS**:
1. 默认 bind `127.0.0.1`(从 `0.0.0.0`)+ `--insecure-no-auth` 文档化 stub
2. `--max-clients <N>` flag (default 1000) + early reject
3. `Frame::parse` 数组递归深度限 `MAX_FRAME_DEPTH = 32`
4. AOF 文件 `0o600` 权限(cfg unix)

**Constitution drift fix**(F1 sub-pattern,第一次正面修复):
5. cs01 CLAUDE.md §1 接受 broadcast pub/sub(over-prohibit → 实际 ADR-0009 决策合理)
6. cs01 CLAUDE.md §4 接受 `storage → protocol` 单向边(为 AOF wire compat)

**AOF 加固**:
7. `FsyncPolicy::AlwaysBlocking` variant(`Reply::Ok` 返回前真 sync_data)+ `Always` 语义文档化
8. AOF mpsc `channel(8192)` + `try_send`→`blocking_send` 背压
9. `Store::replay_from_path` 改 `tokio::fs` 流式(不读整个文件进内存)

**Dispatch 严格**:
10. `parse_set` `parts.len() == 5` 才允许 EX form(reject trailing token + oracle fixture)

**Pub/Sub perf**:
11. `recv_any_subscription` 用 `tokio_stream::StreamMap`(no box-per-poll)
12. `Store::subscribe` read-then-write-on-miss

**注释清理**:
13. `main.rs:140-146` confused comment 重写 + `Frame::Integer` encoder 注释 fix

## Scope creep 显式拒

- AUTH 命令:**不**实现(scope M5+)
- TLS:同上
- AOF rewrite:M5+
- MAXMEMORY policy:M5+

## 数字目标

- backend test ≥ 280(M3.2 baseline 269,+11)
- oracle 36/36(35 + 1 trailing-token)

## 状态

`accepted` — 2026-05-12
