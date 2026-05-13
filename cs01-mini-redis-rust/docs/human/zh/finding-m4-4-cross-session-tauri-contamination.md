# Finding 中文摘要:M4.4 cross-session Tauri requirement contamination

> 完整 finding 见 [docs/agent/findings/m4-4-cross-session-tauri-contamination.md](../../agent/findings/m4-4-cross-session-tauri-contamination.md)。

## 观察

cs01 M4.3/M4.4 曾把 Tauri desktop packaging 当成本 repo 的 release requirement 处理。用户在 2026-05-13 澄清:这是发错 session 的需求,不属于 ADSD in Practice / cs01。

该污染进入了 README、CHANGELOG、local CLAUDE、ADR/finding 索引、human docs、`web/` 依赖、Tauri runtime helper、`web/src-tauri/` 和 gate script。

## 处理

M4.4 撤回 desktop packaging scope,把 cs01 release surface 恢复为:Rust RESP server + Axum HTTP/SSE control plane + SvelteKit browser dashboard。

Tauri-specific code、依赖、gate script 和 release claim 被移除。此前 Tauri packaging blocker 不再作为 live release debt,由本 finding 记录为 cross-session scope contamination。

## 状态

`closed by scope correction` — 2026-05-13。
