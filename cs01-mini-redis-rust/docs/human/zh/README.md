# cs01-mini-redis-rust(中文用户指南)

## 这是什么

一个用 Rust 从零实现的 Redis 兼容子集,带 SvelteKit 监控控制台;M4.3 起 primary frontend surface 转向 Tauri 桌面应用 + managed `redis-server` sidecar。目标是验证 ADSD 方法论在网络服务 + 持久化 + 前端发布领域是否仍然有效。

## 快速开始

```bash
cd cs01-mini-redis-rust
bash scripts/bootstrap.sh
cargo run -p redis-server -- --port 6380
```

监控页:M4.3 target 是 Tauri desktop app;浏览器 dev mode 用 `cd web && pnpm dev` 打开 `http://localhost:5173`。

AOF 持久化需显式创建目录:

```bash
mkdir -p data
cargo run -p redis-server -- --port 6380 --aof data/dump.aof
```

## 支持的命令

- `PING / ECHO / SELECT 0 / QUIT`
- `SET key val [EX seconds] / GET key / DEL key... / EXISTS key...`
- `INCR / DECR / INCRBY / DECRBY`
- `EXPIRE key seconds / TTL key / PERSIST key`
- `TYPE key / KEYS pattern`
- `SUBSCRIBE / UNSUBSCRIBE / PUBLISH`

## 跟真 Redis 兼容

我们对照 `redis:7-alpine`(docker)做 round-trip 测试,见 `tests/oracle.sh`。已知分歧记录在 findings 中,尤其是 Pub/Sub 慢 subscriber 断连和 AOF 损坏尾部处理。

## ADR 索引

完整架构决策见 [`docs/agent/adr/`](../../agent/adr/),中文摘要:

- [ADR-0001 栈选择](./adr-0001-stack-choice.md):tokio + Axum + hashbrown;rust-embed 已 deferred,见 ADR-0013
- [ADR-0002 RESP framing](./adr-0002-resp-framing.md):RESP parse / serialize 策略
- [ADR-0003 storage layout](./adr-0003-storage-layout.md):内存存储布局
- [ADR-0004 command routing](./adr-0004-command-routing.md):命令路由
- [ADR-0005 TCP listener](./adr-0005-tcp-listener.md):RESP TCP accept loop
- [ADR-0006 M1.4 commands](./adr-0006-m1-4-commands-and-hardening.md):命令扩展与 hardening
- [ADR-0007 Axum SSE control plane](./adr-0007-m2-1-axum-sse-control-plane.md):HTTP/SSE 控制面
- [ADR-0008 SvelteKit UI](./adr-0008-m2-2-sveltekit-ui.md):前端 dashboard
- [ADR-0009 Pub/Sub](./adr-0009-m3-1-pubsub.md):SUBSCRIBE / UNSUBSCRIBE / PUBLISH
- [ADR-0010 AOF persistence](./adr-0010-m3-2-aof-persistence.md):append-only 持久化
- [ADR-0011 M4.1 critical fixes](./adr-0011-m4-1-critical-fixes.md):pre-release critical fixes
- [ADR-0012 M4.2 doc sweep](./adr-0012-m4-2-doc-sweep-release-artifacts.md):release artifacts + sediment cleanup
- [ADR-0013 Tauri desktop frontend](./adr-0013-tauri-desktop-frontend.md):M4.3 前端发布形态转向 Tauri 桌面应用 + managed sidecar

## Finding 摘要

- [M1.1 P9 missed shared doc-coverage](./finding-m1-1-p9-missed-shared-doc-coverage.md)
- [M1.3 CTO wrote code instead of dispatching](./finding-m1-3-cto-wrote-code-instead-of-dispatching.md)
- [M1.4 F23-A oracle caught TTL rounding](./finding-m1-4-f23a-oracle-caught-ttl-rounding-spec-bug.md)
- [M2.1 no F23-A on control plane](./finding-m2-1-no-f23a-on-control-plane.md)
- [M3.1 lagging subscriber disconnect](./finding-m3-1-lagging-subscriber-disconnect.md)
- [M3.2 AOF replay corruption handling](./finding-m3-2-aof-replay-corruption-handling.md)
- [M4 pre-release audit aggregation](./finding-m4-pre-release-audit-team-aggregation.md)

## 状态

✅ M1 backend MVP;✅ M2 frontend/control-plane MVP;✅ M3 Pub/Sub + AOF;✅ M4.1 critical fixes;✅ M4.2 doc sweep + release artifacts;✅ M4.3 Tauri desktop frontend source + managed sidecar lightweight gate。完整 bundle/signing 验证仍是 release-readiness work;rust-embed 单 binary 已 deferred,不再是 v0.1.0 blocker。

## License

Apache-2.0 + MIT。
