# Finding M3.1: 慢 Pub/Sub subscriber 断连策略

## 摘要

M3.1 的 Pub/Sub 使用 `tokio::sync::broadcast`。当某个 subscriber 落后超过 buffer 容量并触发 `RecvError::Lagged(_)` 时,我们的实现会先写出 `-ERR client lagged behind pub/sub buffer; disconnecting` 再关闭连接。

真 Redis 7 在类似慢客户端场景下通常通过 `client-output-buffer-limit pubsub` 重置连接,不发送这个应用层错误帧。因此最终状态相同(连接断开),但 wire-level 行为不同。

## 为什么接受

- 触发条件病态:capacity 为 128,健康客户端不应长期不读。
- 明确错误帧在 demo / case-study 阶段更容易诊断。
- 该分歧已公开记录,不是静默兼容性 claim。

## 后续条件

如果 v0.1.0 前的 oracle / burst 测试显示真实客户端因这个额外 `-ERR` 受影响,应切换到 Redis-compatible silent reset 或提供兼容模式开关。

## Cross-references

- Agent finding: [`../../agent/findings/m3-1-lagging-subscriber-disconnect.md`](../../agent/findings/m3-1-lagging-subscriber-disconnect.md)
- ADR: [`adr-0009-m3-1-pubsub.md`](adr-0009-m3-1-pubsub.md)
