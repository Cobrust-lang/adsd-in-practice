---
finding: m3-2-aof-replay-corruption-handling
date: 2026-05-12
case: cs01-mini-redis-rust
severity: low
status: accepted
adr_ref: 0010
---

# Finding M3.2: AOF replay 损坏尾部 — warn-and-truncate vs refuse-to-start

## Context

ADR-0010 §"AOF 损坏" 锁定了我们的 replay 策略:对**任何**
`Frame::parse` 失败(`Incomplete` / `Invalid` / `Utf8`),
`Store::replay_from_path` 会:

1. log warn(`tracing::warn!`,字段含 `tail_bytes`、`error`)
2. **停止 replay**,丢弃剩余字节
3. 返回 `Ok(count)`(count = 在错误之前成功 replay 的命令数)
4. **继续 startup**:server 正常 bind,从这一刻起向同一个 AOF
   文件 append 新命令

下一次写入时,`tokio::fs::OpenOptions::new().append(true)` 把新
字节追加到文件**当前长度**末尾(就是损坏字节后面)。**我们并没
有 truncate(2) 文件**;损坏的尾部字节保持原位,只是会被新 append
盖过去。如果损坏部分本身只是 *截断* 的合法 frame(`Incomplete`
路径),下一次 replay 时会再次 warn 然后跳过,但 valid prefix
那部分 commands 会被正确 replay 两次 — 这是 INCR / DECR
**not idempotent** 的命令的实际风险点。

`Frame::parse` 返回 `Invalid("...")` 的场景(整个 frame 本身格式
就乱了,比如 RESP type byte 不对):同样 warn-and-stop,但因为我
们没有 truncate 文件,损坏字节会一直保留 — 每次重启都 warn,直
到用户手动 truncate 或我们升级到 refuse-to-start。

## Why we accept the divergence for M3.2

| 理由 | 论据 |
|---|---|
| **Cobrust 风格的 "tell the user, don't crash"** | 用户在 M3.2 用户里多半是 demo / dev 场景,startup 直接退出不友好;打 log 让人看见即可 |
| **真 Redis 行为也是 warn-and-continue** | `loadAppendOnlyFile` 在 EOF 时打 `Bad file format reading the append only file` warning 但继续启动(参考 `redis/src/aof.c`) |
| **AOF 写入路径有 lock 保证** | mpsc 单 consumer,任何 partial-write 只可能发生在 `write_all` 中断时,而 `write_all` 实际上在 tokio 里要么全成功要么 IO 错误 — 损坏 frame 在合规 host 上几乎不可能出现 |
| **F22 cadence-aware** | M3.2 先 ship,M4 release-readiness 再决定是否升级到 refuse-to-start |

## 已知风险

| 风险 | 概率 | 缓解 |
|---|---|---|
| INCR / DECR 在 truncated tail 重启时被 replay 多次 → counter 漂高 | Low:`write_all` 原子级别失败 → tail 一致截断 → 没有"部分 INCR" frame | M4 可加 `transaction marker` (Frame::Error 前缀 `"INCR-COMMITTED"`)区分 — 但这违 F24,真 Redis 也没做 |
| invalid-byte 损坏后,**所有后续真正 valid 的 frames 也丢了**(因为 stop-at-first-error) | Low:损坏后 append-onlt 流的"后续"逻辑只是 truncate 后新写 | M4 升级到 refuse-to-start + 提供 `redis-check-aof` 风格的修复工具 |
| 损坏字节永远不被擦除 | Medium:每次 restart 都 warn | M4 升级 `OpenOptions::write(true).truncate_at(safe_offset)`(需要 trait or unsafe API) |

## 升级条件 — M4 复盘清单

- [ ] 有 user 报告 "我的 INCR 数字飘了"(粗略 F23-A 信号)
- [ ] benchmark / soak test 出现 invalid frame(实际生产负载怎么撞的)
- [ ] AOF rewrite(M4 计划任务)需要 truncate 能力时顺手把 corruption truncate 也加上

## Decision

**M3.2 accepts warn-and-continue.**  refuse-to-start + repair tool
留 M4。

## Cross-references

- ADR-0010 §"AOF 损坏" — 原始决策
- `crates/redis-storage/src/lib.rs::Store::replay_from_path` —
  实际代码路径
- 真 Redis 行为:
  [`redis/src/aof.c::loadAppendOnlyFile`](https://github.com/redis/redis/blob/7.4/src/aof.c)
- M3.1 lagging-subscriber finding(相邻的 "M3 接受的债" 模式)
