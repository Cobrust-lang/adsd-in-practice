# ADR-0008 English abstract: M2.2 SvelteKit UI

> Full ADR: [docs/agent/adr/0008-m2-2-sveltekit-ui.md](../../agent/adr/0008-m2-2-sveltekit-ui.md).

## Decision (compact)

| Sub-item | Choice |
|---|---|
| Frontend | **SvelteKit 2 + Svelte 5 (rune-based)** |
| CSS | **Tailwind 4 + DaisyUI 5** |
| Adapter | `adapter-static` + SPA fallback |
| Package mgr | **pnpm** (local has 10.33.0 + node 25.9.0) |
| Dev CORS | **vite proxy `/api → :6381`**, no `tower-http::cors` |
| rust-embed | **deferred to M4** (M2.2 dev runs vite; avoids cargo-rebuild noise) |
| pubsub page | **stub** (`/pubsub` route + "M3 placeholder", explicitly marked) |
| Tests | **vitest unit + manual browser smoke**, no Playwright |
| Frontend gate | new `scripts/frontend-gate.sh` (`pnpm install --frozen-lockfile && pnpm check && pnpm test && pnpm build`), separate from cargo gate 4 |

## Locked TypeScript schema

Strictly aligned with ADR-0007 §Q5:
- `StatsEvent` 5 fields
- `KeyInfo { key, type, ttl_secs }`, **`ttl_secs` uses round-half-up seconds** (matches M1.4 commit `0800d86`)

## Three pages

- `/` Dashboard: 5 stat cards, SSE 1Hz
- `/keys` table: top-100 sample_keys, human-friendly TTL
- `/pubsub` stub: "M3 placeholder"

## Status

`accepted` — 2026-05-12.
