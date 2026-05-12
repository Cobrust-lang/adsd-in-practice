# ADRs — cs01-mini-redis-rust

> 架构决策记录。新决策用 `_shared/adr-template.md` 模板,落在本目录 `NNNN-kebab-case-title.md`。

## Roster

| ADR | Title | Status | Date |
|---|---|---|---|
| [0001](0001-stack-choice.md) | Stack choice — tokio + Axum + hashbrown + rust-embed | accepted | 2026-05-12 |
| [0002](0002-resp-framing.md) | RESP v2 framing strategy — one-shot parse with Incomplete sentinel | accepted | 2026-05-12 |
| [0003](0003-storage-layout.md) | In-memory storage layout — hashbrown::HashMap + DelayQueue for TTL | accepted | 2026-05-12 |
| [0004](0004-command-routing.md) | Command routing — Frame → Command via match on first BulkString | accepted | 2026-05-12 |

## 命名规范

- 编号从 0001 起,**只增不减**(superseded 不重用编号)
- `kebab-case-title` 不超过 6 个词
- frontmatter 必填 `adr / title / status / date / case`(`last_verified_commit` 在落地时填)

## 编号空间

**本 case 独立编号**。`cs01-ADR-0001` 跟 `cs02-ADR-0001` 互不相干。

## Status 流转

`proposed → accepted → superseded`

`superseded` 时 frontmatter 必须填 `supersedes` / `superseded_by`。
