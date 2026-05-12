# ADR-0001 中文摘要:栈选择

> 这是 [docs/agent/adr/0001-stack-choice.md](../../agent/adr/0001-stack-choice.md) 的人类可读摘要。完整 ADR(包括 3 个候选方案对比、不可逆性分析、done criteria)请看 agent 版本。

## 决策

CS-01 mini-redis-rust 采用:

- **异步运行时**:`tokio`
- **HTTP 框架**:`Axum`
- **内存 KV 存储**:`hashbrown::HashMap`
- **单 binary 嵌入**:`rust-embed`
- **TCP 编解码**:`tokio_util::codec` + `bytes::BytesMut`

## 为什么

- **跟 Cobrust Studio 栈对齐**,sub-agent 经验/工具/snippet 跨项目复用
- tokio + Axum 是 Rust 异步生态最成熟、招外部贡献者门槛最低的组合
- rust-embed 是单 binary 部署的标准做法,Cobrust Studio M3 已经验证

## 拒绝的方案

- `async-std + tide`:生态萎缩,长尾维护风险
- 纯 hyper + 自写 router:严重 yak-shaving,5-day MVP 做不完

## 影响

- 性能 ceiling 受 tokio 模型限制(每连接 task),v0.2 后考虑 tokio-uring
- hashbrown 锁版本 0.15.x(RawTable API 不稳)

## 状态

`accepted` — 2026-05-12
