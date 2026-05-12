# Finding M3.2: AOF replay 损坏尾部处理

## 摘要

ADR-0010 选择的 M3.2 AOF replay 策略是:遇到 `Frame::parse` 失败时记录 warning,停止 replay,返回已成功 replay 的命令数,然后继续启动 server。实现不会自动 truncate 损坏尾部;后续 append 会继续写到文件末尾。

这与完整 Redis 生产工具链不同:我们没有 `redis-check-aof` 风格修复工具,也没有 refuse-to-start 策略。因此如果尾部损坏和非幂等命令(`INCR`/`DECR`)组合出现,存在重启后计数漂移或重复 warning 的风险。

## 为什么接受

- M3.2 的目标是 demo / case-study 可用性,优先 tell-the-user 而不是直接拒绝启动。
- 正常 `write_all` 路径出现半帧损坏的概率低。
- 风险已作为 accepted debt 公开,不声称 production-grade AOF。

## 后续条件

M4/v0.2 如果要提升 persistence 可信度,应实现 refuse-to-start + repair/truncate 工具,或在 AOF rewrite 时同时处理安全 offset truncation。

## Cross-references

- Agent finding: [`../../agent/findings/m3-2-aof-replay-corruption-handling.md`](../../agent/findings/m3-2-aof-replay-corruption-handling.md)
- ADR: [`adr-0010-m3-2-aof-persistence.md`](adr-0010-m3-2-aof-persistence.md)
