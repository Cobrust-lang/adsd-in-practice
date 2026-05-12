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

## 下一步

ADR-0013 是 Phase 1 strategic anchor。Phase 2 由 P9 实现 Tauri scaffold + sidecar lifecycle + gate/docs reconciliation,CTO 不直接写实现代码。
