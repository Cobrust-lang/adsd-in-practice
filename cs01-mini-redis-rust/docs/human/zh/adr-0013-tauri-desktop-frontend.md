# ADR-0013 中文摘要:M4.3 Tauri desktop frontend

> 完整 ADR 见 [docs/agent/adr/0013-tauri-desktop-frontend.md](../../agent/adr/0013-tauri-desktop-frontend.md)。

## 决策

cs01 的前端发布形态从 rust-embed 浏览器管理页转向 **Tauri 桌面应用**。现有 SvelteKit UI 不推倒重来,继续作为页面源码;Tauri 负责桌面壳、local sidecar lifecycle、日志/错误可见性和 release surface。

## 选择

v0.1.0 采用 **Tauri app + managed `redis-server` sidecar**:

- 复用现有 `redis-server` CLI、RESP listener 和 `/api/*` HTTP/SSE control plane。
- 保持 `redis-cli` oracle 兼容性证据可比,因为 sidecar 仍是同一个 server binary。
- 避免在 M4.3 同时做 backend lifecycle library refactor;in-process backend 留 v0.2 ADR。
- rust-embed 不再是 v0.1.0 blocker,后续可删除、保留或改为 optional web deploy target。

## 低磁盘约束

Tauri / Rust / pnpm 构建会产生大量 `target/`、bundle、cache。P9 实现必须:

- heavy build 前后报告 `df -h .`。
- inner loop 不反复跑 `pnpm tauri build`。
- gate 默认轻量化;完整 Tauri bundle build 只在 release-readiness 跑一次。
- 清理不需要保留的 `web/src-tauri/target/` / bundle output / vite cache。

## Phase 2 实现说明

M4.3 在 `web/src-tauri/` 增加 Tauri v2 app。桌面壳复用现有 SvelteKit 页面,默认启动 loopback `redis-server` sidecar:

- RESP:`127.0.0.1:6380`
- HTTP/SSE:`127.0.0.1:6381`
- UI 失败状态:共享 layout 显示 Tauri sidecar banner,覆盖 `starting` / `running` / `failed` / `stopped`。
- 开发覆盖:如果 sidecar binary 不在默认 Cargo target 路径,设置 `CS01_REDIS_SERVER_BIN=/absolute/path/to/redis-server`。

`scripts/tauri-gate.sh` 默认轻量化:跑 SvelteKit check/test/build + 定向 `cargo check --manifest-path web/src-tauri/Cargo.toml`。完整 desktop bundle 需要显式设置 `CS01_TAURI_FULL_BUILD=1`,并记录前后磁盘状态。

## M4.3 守闸补丁

CTO 守闸退回后补两项 runtime hardening:

- sidecar 不再把 stdout/stderr 接到未 drain 的 pipe,改为 `Stdio::null()`,避免长跑或异常日志填满 pipe buffer 阻塞 `redis-server`。
- `/api/stats`、`/api/keys`、`/api/pubsub` SSE response 改为最小 CORS allowlist,不再使用 wildcard。允许的 dev browser origin 是 `http://localhost:5173` / `http://127.0.0.1:5173`;允许的 Tauri v2 app origin 是 `tauri://localhost` 以及 Tauri/wry documented workaround `http://tauri.localhost` / `https://tauri.localhost`。无 `Origin` 时不返回 CORS header;非 allowlist origin 不返回 `Access-Control-Allow-Origin`。这样保留 Tauri `EventSource(http://127.0.0.1:6381/api/*)` 能力,同时避免任意网页跨源读取本机 loopback control plane(尤其 `/api/keys`)。
