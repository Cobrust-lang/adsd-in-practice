# Finding M4.3:Tauri sidecar packaging blocker

> Agent finding: [docs/agent/findings/m4-3-tauri-sidecar-packaging-blocker.md](../../agent/findings/m4-3-tauri-sidecar-packaging-blocker.md)。

## 摘要

Tauri 桌面壳和 local-dev sidecar lifecycle 已实现,但完整 bundle-time sidecar packaging 尚不能声称 release-ready。一次把 `bundle.resources` 指向 `../../../target/debug/redis-server` 的检查失败,因为该 binary 没有被稳定 staging 到该路径。

## 影响

M4.3 可以作为本地桌面 preview 验证:通过 `CS01_REDIS_SERVER_BIN` 或本地 Cargo target binary 启动 sidecar。正式 release bundle 仍需要确定性的 sidecar staging / signing 步骤,并显式跑一次 `CS01_TAURI_FULL_BUILD=1` gate 且记录磁盘 before/after。
