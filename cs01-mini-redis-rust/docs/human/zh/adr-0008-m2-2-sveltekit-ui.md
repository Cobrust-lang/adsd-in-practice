# ADR-0008 中文摘要:M2.2 SvelteKit UI

> 完整 ADR 见 [docs/agent/adr/0008-m2-2-sveltekit-ui.md](../../agent/adr/0008-m2-2-sveltekit-ui.md)。

## 决策(紧凑式)

| 子项 | 选 |
|---|---|
| Frontend | **SvelteKit 2 + Svelte 5(rune-based)** |
| CSS | **Tailwind 4 + DaisyUI 5** |
| Adapter | `adapter-static` + SPA fallback |
| 包管理 | **pnpm**(本机 10.33.0 + node 25.9.0 已就绪) |
| Dev mode CORS | **vite proxy `/api → :6381`**,不引 tower-http::cors |
| rust-embed | **M4 才接**(M2.2 dev 用 vite,不污染 cargo rebuild) |
| pubsub 页 | **stub**(`/pubsub` route + "M3 placeholder",显式标 stub) |
| Frontend tests | **vitest unit + 手动浏览器 smoke**,**不**引 playwright |
| Frontend gate | 新 `scripts/frontend-gate.sh`(`pnpm install --frozen-lockfile && pnpm check && pnpm test && pnpm build`),不塞进 cargo gate 4 |

## 锁的 TypeScript schema

严格对齐 ADR-0007 §Q5:
- `StatsEvent` 5 字段
- `KeyInfo { key, type, ttl_secs }`,**ttl_secs 用 round-half-up 秒**(对齐 M1.4 commit `0800d86`)

## 三页

- `/` Dashboard:5 个 stat card,SSE 1Hz
- `/keys` table:前 100 sample_keys,human-friendly TTL
- `/pubsub` stub:"M3 placeholder"

## 状态

`accepted` — 2026-05-12
