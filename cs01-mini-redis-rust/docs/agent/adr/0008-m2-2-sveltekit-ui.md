---
adr: 0008
title: M2.2 — SvelteKit UI (dashboard / keys / pubsub-placeholder) consuming M2.1 SSE
status: accepted
date: 2026-05-12
case: cs01-mini-redis-rust
supersedes: none
last_verified_commit: pending
---

# ADR-0008: M2.2 SvelteKit UI

## Context

M2.1 (ADR-0007, commit `16abe49`) ship 后,backend `/api/stats` + `/api/keys` SSE 已稳定,event schema 锁。M2.2 是 cs01 §3 items 6-7 的 frontend 部分:三页 UI (dashboard / keys / pubsub) + dev-mode 接 SSE。

待定决策:

1. UI framework 选型(SvelteKit / Next.js / SolidStart / 纯 Vue)
2. CSS framework(Tailwind / Vanilla / DaisyUI / Skeleton)
3. SvelteKit adapter(static / node / cloudflare)
4. 包管理器(npm / pnpm / yarn / bun)
5. Dev mode CORS / proxy 策略
6. rust-embed 接入时机(M2.2 / M3 / M4)
7. pubsub 页 M3 才有真功能 — M2.2 stub 还是不放
8. Frontend tests(vitest / playwright / 无)
9. 5-gate 怎么扩到 frontend(新 gate 6 / 同 gate 4 多 step)

环境:本机 `pnpm 10.33.0` + `node 25.9.0` 已就绪。

## Decision(紧凑式 — 每子项直接给选)

| 子项 | 选 | 拒绝的方案 + 理由 |
|---|---|---|
| Frontend framework | **SvelteKit 2 + Svelte 5** | Next.js(React 心智模型重);SolidStart(社区窄);Vue(跟 cs01 没有先验缘分);**SvelteKit 跟 ADR-0001 setup expectations 对齐 + 跟 Cobrust Studio 同栈复用经验** |
| CSS | **Tailwind 4 + DaisyUI 5**(component primitives) | 纯 CSS(写 styling 是 yak-shaving);Skeleton UI(主张性强);**DaisyUI = Tailwind 上的无主张 component lib,跟 cs01 "dashboard 不是品牌" 一致** |
| Adapter | **`@sveltejs/adapter-static`** + fallback `index.html`(SPA mode) | adapter-node(M2.2 不用 SSR,backend 是 RESP/SSE);**adapter-static 出来的 `build/` 是纯静态,**M4 直接 rust-embed 即可** |
| Package manager | **pnpm**(本机已有 10.33.0)+ `engines` 锁 ≥ 9 | npm(默认但慢);yarn(legacy);bun(快但生态变动大);**pnpm 是 svelte / vite 默认推荐** |
| Dev mode CORS | **vite proxy** `/api → http://localhost:6381`,**不引 CORS header** | tower-http::cors(M2.1 显式拒);vite proxy 完全避开 CORS,prod via rust-embed same-origin |
| rust-embed 接入 | **M4 才接**(M2.2 不动 binary) | M2.2 接 → 每次 frontend rebuild → cargo rebuild,**dev 死慢**;M4 release-readiness 一次接入更对 |
| pubsub 页 | **M2.2 放 stub**(`/pubsub` route + "M3 placeholder" 文本) | 不放(M3 时改 routing 麻烦);**stub 让 nav 三个 item 完整,M3 只动 page content** |
| Frontend tests | **vitest unit 少量 + 手动浏览器 smoke**;不引 playwright | playwright(too heavy, ~150MB browsers + setup);vitest 覆盖纯逻辑(EventSource state machine, key list formatter);浏览器 smoke 在 P9 报告里截 console + screenshot 即可 |
| Build gate | **新 gate 6:`pnpm install --frozen-lockfile && pnpm check && pnpm test && pnpm build`** | 塞 gate 4(cargo test 不该跑 node);新增 `_shared/6-gate-node.sh`?**不**,案例特定,放 `cs01-mini-redis-rust/scripts/frontend-gate.sh` |

### TypeScript

- `strict: true`
- 类型来自 hand-written `src/lib/api/types.ts`,**字段名严格对齐 ADR-0007 §Q5 锁的 schema**
  - `StatsEvent = { connections_active: number; commands_total: number; keys_active: number; mem_value_bytes: number; uptime_secs: number }`
  - `KeyInfo = { key: string; type: "string" | "none"; ttl_secs: number /* -1 = no TTL, else seconds */ }`
- M2.2 不做 schema-from-Rust 自动生成(P9 在 M2.1 报告里 flag 过,**作为 ADR-0009 候选**;M2.2 手写 + 单测覆盖 mismatch 风险)

### Routing(SvelteKit file-based)

```
src/routes/
  +layout.svelte           # 全局 nav + Tailwind 全局 CSS
  +page.svelte             # / → Dashboard
  keys/+page.svelte        # /keys → Key list (live SSE)
  pubsub/+page.svelte      # /pubsub → "Pub/Sub coming in M3" placeholder
```

`+layout.svelte` 的 nav 显示三个 link,active state via `$page.url.pathname`。

### EventSource 用法

每页 `onMount` 建 `EventSource(`/api/stats` or `/api/keys`)`,parse `event.data` 为 JSON,push 到 Svelte 5 `$state` rune。`onDestroy` 调用 `eventSource.close()`。**不引外部 EventSource lib**(浏览器原生足够)。

### Dev mode 启动

```bash
# Terminal 1 — backend
cargo run -p redis-server -- --port 6380 --http-port 6381

# Terminal 2 — frontend
cd web && pnpm dev   # vite 在 5173,proxy /api → 6381
```

文档化在 `cs01-mini-redis-rust/README.md` 新 section "Dev mode (M2.2)"。

## Consequences

### 正面

- Frontend / backend 解耦,M2.2 P9 sprint 只动 `web/`(再加一点点 README + `scripts/frontend-gate.sh` + 顶层 `.gitignore`)
- 三页 nav 完整;M3 接 Pub/Sub 时只 swap pubsub page content
- vite dev 体感快(M2.2 P9 实际开发循环 < 1s HMR)
- rust-embed 不接 = 不用每次 frontend 改动 cargo rebuild

### 负面 / 接受的债

- Frontend 类型手写,M2.2 P9 必须人手校对 ADR-0007 §Q5 schema(M2.2 P9 prompt 显式列)
- 浏览器 smoke 不是 CI gate(手动 step),release readiness 时复审
- DaisyUI 锁 5.x → tailwind 4.x 兼容性边界;Tailwind 5 出来时 review
- pubsub 页是 stub:**严格遵守"显式标 stub"原则**(顶层 CLAUDE.md §1.3),页面文案 + README 同步标
- 没有 frontend i18n(顶层 doc 双语,UI 文案 M2.2 中文为主,英文为辅注释),M3 重审

### 不可逆性

- 完全可逆。`web/` 全删 = 回 M2.1 状态;Rust 侧不动

## Done Criteria(falsifiable)

### Scaffold

- [ ] `web/package.json` declared with engines: `{ "node": ">=20", "pnpm": ">=9" }`
- [ ] `web/svelte.config.js` 配置 `adapter-static` + `fallback: 'index.html'`
- [ ] `web/vite.config.ts` 配置 `server.proxy['/api'] = 'http://localhost:6381'` + `'/api SSE'` 注意 `changeOrigin: true`
- [ ] `web/tsconfig.json` strict + extends SvelteKit base
- [ ] `web/tailwind.config.ts` + `web/postcss.config.js` 配 DaisyUI plugin

### Pages

- [ ] `/` Dashboard:
  - 顶部 5 个 stat card(connections_active / commands_total / keys_active / mem_value_bytes / uptime_secs)
  - 每秒由 SSE 更新,数值变化平滑(不闪)
  - mem_value_bytes 显示 human-friendly(1024 → 1.0 KiB)
  - uptime_secs 显示 1d 2h 3m 4s 格式
- [ ] `/keys` Keys:
  - table 显示 sample_keys 前 100(key / type / ttl)
  - ttl 显示 `-1 → 永久`,`0+ → 剩余秒`,human-friendly
  - 上方 banner 标 "showing up to 100 keys; use SCAN in M3 for full keyspace"
- [ ] `/pubsub` Stub:页面只有一段文字 "Pub/Sub UI coming in Wave M3"

### Nav / Layout

- [ ] `+layout.svelte` 顶部 nav 三个 link 显示
- [ ] 当前 page active 高亮(DaisyUI tab 或自写)
- [ ] 全局深色模式(DaisyUI dark theme)

### Tests

- [ ] `pnpm check` (svelte-check + tsc): 0 errors
- [ ] vitest unit:
  - `formatBytes(0)` / `formatBytes(1024)` / `formatBytes(2 ** 30)` 正确
  - `formatUptime(0)` / `formatUptime(3661)` 正确
  - `formatTtl(-1)` / `formatTtl(0)` / `formatTtl(60)` 正确
  - SSE event parser 单测:string `event: stats\ndata: {...}` → struct
- [ ] `pnpm build`: 0 errors, output `web/build/index.html` + asset

### Backend gates(不退化)

- [ ] cargo fmt / clippy / build / test / doc-coverage 全过
- [ ] CS01_RUN_ORACLE=1 bash tests/oracle.sh 22/22 不退化
- [ ] 浏览器 smoke(P9 手动):
  - 启 backend + `pnpm dev`,打开 `http://localhost:5173`,Dashboard 看到数值
  - RESP `redis-cli -p 6380 SET foo bar` → /keys 下一刷新出现 `foo`
  - 切到 /pubsub → 看到 stub 文字
  - 报告里截 1 张 dashboard screenshot(或者文字描述每个 card 数值)

### Docs

- [ ] `cs01-mini-redis-rust/README.md` 加 "Dev mode (M2.2)" section
- [ ] ADR-0008 frontmatter `last_verified_commit` 改为 final SHA

## Cross-references

- ADR-0007 (M2.1 backend, schema 锁) — **frontend types 严格对齐**
- top-level CLAUDE.md §1.3 显式标 stub(pubsub 页)
- cs01 CLAUDE.md §3 wave order
- 新增脚本:`cs01-mini-redis-rust/scripts/frontend-gate.sh`
- 新文件清单:
  - `web/.gitignore` (`node_modules/ build/ .svelte-kit/`)
  - `web/package.json` + `pnpm-lock.yaml`(P9 必须 commit lock 文件)
  - `web/svelte.config.js`
  - `web/vite.config.ts`
  - `web/tsconfig.json`
  - `web/tailwind.config.ts` + `web/postcss.config.js`
  - `web/src/app.css` (Tailwind directives + DaisyUI)
  - `web/src/app.html` (SvelteKit shell)
  - `web/src/lib/api/types.ts`
  - `web/src/lib/api/sse.ts` (EventSource wrapper)
  - `web/src/lib/format.ts` (formatBytes / formatUptime / formatTtl)
  - `web/src/lib/format.test.ts` (vitest)
  - `web/src/routes/+layout.svelte`
  - `web/src/routes/+page.svelte` (Dashboard)
  - `web/src/routes/keys/+page.svelte`
  - `web/src/routes/pubsub/+page.svelte`

## Notes

- **不要用 `pnpm dlx create-svelte`**(interactive),P9 应该 manual scaffold(写出 `package.json` 列依赖 + 跑 `pnpm install`)。CTO 已经查过 SvelteKit 2 / Svelte 5 的标准 boilerplate,P9 可以参考 SvelteKit 官网文档但**不**互联网跑 init 命令。
- Tailwind 4 用 `@tailwindcss/vite` plugin,**不需要 PostCSS**(Tailwind 4 直接 vite plugin)。P9 自己 verify。
- Svelte 5 `$state` / `$derived` 是 rune-based,不是 Svelte 4 reactive `$:`。**P9 必须用 Svelte 5 风格**。
- `web/` 单独 `.gitignore`,但顶层 cs01 还要把 `target/` `node_modules/` `web/build/` `web/.svelte-kit/` 包进 `.gitignore`(检查有没有)
- 浏览器 smoke 不是 CI gate — M2.2 sprint 内 P9 跑一次,报告里贴结果。**Release-readiness wave 时再正规化**(M4 拉 playwright headless 跑端到端)
- ADR-0009 候选:**自动从 Rust 类型生成 TypeScript schema**(`ts-rs` 或 `specta` crate),M3 之前不接
