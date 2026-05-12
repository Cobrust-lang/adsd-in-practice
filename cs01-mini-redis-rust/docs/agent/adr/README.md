# ADRs — cs01-mini-redis-rust

> 架构决策记录。新决策用 `_shared/adr-template.md` 模板,落在本目录 `NNNN-kebab-case-title.md`。

## Roster

| ADR | Title | Status | Date |
|---|---|---|---|
| [0001](0001-stack-choice.md) | Stack choice — tokio + Axum + hashbrown + rust-embed | accepted | 2026-05-12 |
| [0002](0002-resp-framing.md) | RESP v2 framing strategy — one-shot parse with Incomplete sentinel | accepted | 2026-05-12 |
| [0003](0003-storage-layout.md) | In-memory storage layout — hashbrown::HashMap + DelayQueue for TTL | accepted | 2026-05-12 |
| [0004](0004-command-routing.md) | Command routing — Frame → Command via match on first BulkString | accepted | 2026-05-12 |
| [0005](0005-tcp-listener.md) | RESP TCP listener — accept-loop + per-conn task + BytesMut drain | accepted | 2026-05-12 |
| [0006](0006-m1-4-commands-and-hardening.md) | M1.4 — EXPIRE/TTL/PERSIST + TYPE + KEYS glob + PING optional + max-frame-size + docker oracle | accepted | 2026-05-12 |
| [0007](0007-m2-1-axum-sse-control-plane.md) | M2.1 — Axum HTTP control plane + SSE for dashboard / keys (backend only) | accepted | 2026-05-12 |
| [0008](0008-m2-2-sveltekit-ui.md) | M2.2 — SvelteKit UI (dashboard / keys / pubsub-stub) consuming M2.1 SSE | accepted | 2026-05-12 |
| [0009](0009-m3-1-pubsub.md) | M3.1 — Pub/Sub (SUBSCRIBE / UNSUBSCRIBE / PUBLISH) + per-conn subscriber state + /pubsub UI swap | accepted | 2026-05-12 |
| [0010](0010-m3-2-aof-persistence.md) | M3.2 — AOF append-only persistence + replay-on-restart | accepted | 2026-05-12 |

## 命名规范

- 编号从 0001 起,**只增不减**(superseded 不重用编号)
- `kebab-case-title` 不超过 6 个词
- frontmatter 必填 `adr / title / status / date / case`(`last_verified_commit` 在落地时填)

## 编号空间

**本 case 独立编号**。`cs01-ADR-0001` 跟 `cs02-ADR-0001` 互不相干。

## Status 流转

`proposed → accepted → superseded`

`superseded` 时 frontmatter 必须填 `supersedes` / `superseded_by`。
