# cs01-mini-redis-rust(中文用户指南)

## 这是什么

一个用 Rust 从零实现的 Redis 兼容子集,带 SvelteKit 监控控制台;M4.3 起 primary frontend surface 转向 Tauri 桌面应用 + managed `redis-server` sidecar。目标是验证 ADSD 方法论在网络服务 + 桌面发布领域是否仍然有效。

## 快速开始

```bash
cd cs01-mini-redis-rust
bash scripts/bootstrap.sh
cargo run -p redis-server -- --port 6380
```

监控页:M4.3 target 是 Tauri desktop app;浏览器 dev mode 用 `cd web && pnpm dev` 打开 `http://localhost:5173`。

## 支持的命令(M1 后)

- `PING / ECHO / QUIT`
- `SET key val [EX seconds] / GET key / DEL key... / EXISTS key...`
- `INCR / DECR / INCRBY / DECRBY`
- `EXPIRE key seconds / TTL key / PERSIST key`
- `TYPE key / KEYS pattern`

M3 后追加:`SUBSCRIBE / UNSUBSCRIBE / PUBLISH`。

## 跟真 Redis 兼容

我们对照 `redis:7-alpine`(docker)做 round-trip 测试,见 `tests/oracle.sh`(M1.3 后可用)。

## ADR 索引

完整架构决策见 [`docs/agent/adr/`](../../agent/adr/),关键决策中文摘要:

- [ADR-0001 栈选择](./adr-0001-stack-choice.md):用 tokio + Axum + hashbrown + rust-embed,跟 Cobrust Studio 栈对齐
- [ADR-0013 Tauri desktop frontend](./adr-0013-tauri-desktop-frontend.md):M4.3 前端发布形态转向 Tauri 桌面应用 + managed sidecar

## 状态

✅ M1-M3 shipped;✅ M4.1 critical fixes;🚧 M4.2 doc sweep;🚧 M4.3 Tauri desktop frontend。详见根目录 [README](../../../README.md) 的 "Status" 节。

## License

Apache-2.0 + MIT。
