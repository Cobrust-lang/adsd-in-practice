---
adr: 0012
title: M4.2 — Doc sweep + release artifacts (LICENSE / CHANGELOG / CONTRIBUTING / SECURITY / METHODOLOGY-STATUS + sediment cleanup)
status: accepted
date: 2026-05-12
case: cs01-mini-redis-rust
supersedes: none
last_verified_commit: 31b52a1
---

# ADR-0012: M4.2 doc sweep + release artifacts

## Context

M4.1 (ADR-0011, commit `d02aa55`) closed 13 critical / code-level audit items。剩下的 ~37 audit findings 大部分是 **doc sediment + release artifact gap**(LICENSE 缺失 / README §Status 过期 / bilingual mirror 不全 / METHODOLOGY-STATUS 占位 / etc.)。M4.2 holistic sweep,准备 0.1.0 release。

待处理(全部 audit-team-surfaced,bucket 化):

### Bucket A — Release artifacts (Sarah's hard blockers)
- A1. `LICENSE-APACHE` + `LICENSE-MIT` 文件 at **repo root**(Cargo.toml SPDX 已声明,但无 license text;Sarah scorecard 4/10 legal)
- A2. `CHANGELOG.md` at repo root + cs01 子目录(per case)
- A3. `CONTRIBUTING.md` at repo root(top CLAUDE.md 是 agent-facing,人类 contributor 没入口)
- A4. `SECURITY.md` at repo root(non-placeholder GitHub disclosure guidance + threat-model 简介)
- A5. `METHODOLOGY-STATUS.md` cs01 第 1 节填实(Sarah / SA-14)

### Bucket B — README sediment (BLOCK + HIGH)
- B1. README §Status update — M3.1+M3.2+M4.1 全 ✅,M4.2 进行中,M4.3 0.1.0 tag pending(BLOCK from public-readiness;triple-validated)
- B2. README §"Pub/Sub 页是 stub" rewrite — 不再是 stub,是 read-only dashboard(HIGH)
- B3. README §Quick-start drop `--aof data/dump.aof`(ENOENT)or AOF writer auto-mkdir(HIGH)
- B4. README §Architecture diagram 加 "M4 target" 标记 rust-embed(rust-embed deferred to M4.3 之后或 v0.2,确认)
- B5. README 加 §"Supported commands" subsection 列 PING/ECHO/SELECT/QUIT/GET/SET(EX)/DEL/EXISTS/INCR/DECR/EXPIRE/TTL/PERSIST/TYPE/KEYS/SUBSCRIBE/UNSUBSCRIBE/PUBLISH;明确 not implemented:INCRBY/DECRBY/PSUBSCRIBE/HSET/...(Mei + Public-readiness HIGH-5)
- B6. README 加 §"Prerequisites":Rust 1.94+ / node ≥ 20 / pnpm ≥ 9 / docker optional
- B7. README 加 §"Docs" subsection — link `docs/human/{zh,en}/` + `docs/agent/adr/` + `docs/agent/findings/`
- B8. README 加 §"Wedge / why this exists" 段:第一句明确 ADSD methodology angle(Mei 关键信号)
- B9. README 加 §"Known behavioral deltas vs real Redis":link finding `m3-1-lagging-subscriber-disconnect` + `m3-2-aof-replay-corruption-handling`(Mei 信任信号)

### Bucket C — Bootstrap + tooling
- C1. `scripts/bootstrap.sh` rewrite — "M1.0 scaffold" → 当前 working server(HIGH)
- C2. `scripts/bootstrap.sh` 加 soft `pnpm + node` 检查(non-fatal hint)

### Bucket D — Bilingual sync (constitution requires)
- D1. `docs/human/{zh,en}/finding-m1-1-p9-missed-shared-doc-coverage.md` 双语 abstract 加
- D2. `docs/human/{zh,en}/finding-m3-1-lagging-subscriber-disconnect.md` 双语 abstract 加
- D3. `docs/human/{zh,en}/finding-m3-2-aof-replay-corruption-handling.md` 双语 abstract 加
- D4. `docs/human/{zh,en}/README.md` update — 不再 "M0 scaffold",反映 M4 状态 + ADR 0001-0011 全 listed

### Bucket E — `doc-coverage.sh` 扩 enforce finding bilingual
- E1. `_shared/doc-coverage.sh` 加一段 mirror ADR check,对 `docs/agent/findings/*.md` 也强制 zh/en 双语镜像(SA-13;LOW root cause)

### Bucket F — ADR / finding metadata sweep
- F1. ADR-0005 `last_verified_commit: 3a8c58d` (32 commits stale) → 重审 ADR-0005 §Done Criteria 跟 M4.1 后 server.rs 一致,bump 到 d02aa55(CV-12 / Doc HIGH-4)
- F2. ADR-0001 Done Criteria 行 "M3 完成 rust-embed" → "M4.3+ (见 ADR-0008 deferral)"(Doc MED-5)
- F3. ADR-0009 §Done Criteria 错误 string 缺 "RESET" → 加(self-contradicting fix,Doc MED-2)
- F4. ADR-0009 §Cross-references "文件改动清单" 提 `pubsub.rs` 不存在 → 加 implementation-deltas 解释(Doc MED-1)
- F5. cs01 CLAUDE.md §3 wave order 加 closure markers(`✅ shipped @ <SHA>` 每行)(Doc MED-4)

### Bucket G — `findings/README.md` ledger 已 fix (CV-10 done in commit 7cd8ef0)— verify only

## Decision

All buckets ship in one P9 sprint(7 buckets × 5-9 items = ~30 sub-items)。P9 mechanical work:无新代码,纯 doc / text / index。

理由:
- 全部是 audit-surfaced + non-code(can't break tests)
- Atomic doc-sweep:跟 release artifacts 一起 commit 自然 release-rdy
- doc-coverage.sh 扩 enforce(E1)是 self-binding move:future findings 不会 silently 漏双语,如同 LICENSE 现在不会 silently 缺

### Wording / content 标准

- LICENSE-APACHE:standard Apache 2.0 text(rustls-tls / tokio 用同款)
- LICENSE-MIT:standard MIT text
- CHANGELOG.md:keep-a-changelog format;A14.4 finalization converts the former `0.1.0-rc` placeholder into a final `[0.1.0] - 2026-05-13` release entry without inventing a tag SHA
- CONTRIBUTING.md:ADSD-aware,链 `_shared/adr-template.md` + `_shared/finding-template.md` + 5-gate / doc-coverage / Tx-tag 流程
- SECURITY.md:disclosure 走 email + 24h ack SLA(占位),threat model 简引 finding m4-pre-release-audit-team-aggregation
- METHODOLOGY-STATUS.md cs01 节:含 wave M0→M4.1 实际进展 + 8-agent audit 数据 + F-pattern 候选(`F1.x constitution-vs-ADR drift` + `F23-A.gap happy-path-only oracle`)

### Wedge / why-this-exists 第一句

For README §1 (cs01) — 新开头:

> 这是用 ADSD multi-agent methodology 从 0 写起的 Redis 子集,**故意**离 Cobrust(编译器)domain 远,测试 methodology 是否还成立。Redis 行为兼容 `redis-cli` 实测 36/36 commands(F23-A oracle)。Methodology 工件:[13 个 ADR](docs/agent/adr/) + [8 个 finding](docs/agent/findings/) + [8-agent pre-release audit](docs/agent/findings/m4-pre-release-audit-team-aggregation.md)。

(若 zh / en README 加 mirror 句)

## Consequences

### 正面

- Sarah's WATCH-FROM-DISTANCE → 至少 EVALUATE-LATER(LICENSE + CHANGELOG + CONTRIBUTING + SECURITY 全到位)
- Mei 的 close-tab 风险全消(README sediment + quick-start ENOENT 全 fix)
- Aleksandr 的 "AI-slop tells" 之 README 部分清理(stale Status / bootstrap.sh)
- doc-coverage.sh enforcement 防 future F1 sediment 复发
- 0.1.0 release-readiness gate 全 met(剩 M4.3 tag)

### 负面 / 接受的债

- Aleksandr "AppState wrap in Arc" / Reply::Bulk → Bytes 等 perf 优化留 v0.2
- README 截图 / docker-compose / asciinema 不在本 wave(Mei wishlist,但 P0 不卡)— **flag**:M4.3 release 前补 screenshot 是 nice-to-have,但不阻塞
- AUTH / TLS / replication 全 v0.2+(已 CLAUDE.md / README out-of-scope 标 stub)

### 不可逆性

- 完全可逆。全 doc / text。

## Done Criteria(falsifiable)

### Release artifacts (Bucket A)
- [x] `LICENSE-APACHE` exists at repo root with standard Apache-2.0 text
- [x] `LICENSE-MIT` exists at repo root with repository copyright holder
- [x] `CHANGELOG.md` exists at repo root;A14.4 updated first entry to `[0.1.0] - 2026-05-13`, lists M1-M4.4 readiness evidence, and explicitly keeps DMG/signing/notarization out-of-gate
- [x] `cs01-mini-redis-rust/CHANGELOG.md` 同结构
- [x] `CONTRIBUTING.md` exists at repo root,提到 ADR/finding/5-gate/doc-coverage/Tx-tag/bilingual rule
- [x] `SECURITY.md` exists at repo root with disclosure path
- [x] `METHODOLOGY-STATUS.md` cs01 section ≥ 200 字,列实际 ADR/finding 计数 + persona scores + F-pattern 候选

### README sediment (Bucket B)
- [x] README §Status:M1 ✅ / M2 ✅ / M3 ✅ / M4.1 ✅ / M4.2 ✅ / M4.3/M4.4 ✅ Tauri `.app` bundle gate,with DMG/signing/notarization explicitly out-of-gate future release-engineering work + 各 link 对应 ADR
- [x] README §"Pub/Sub 页是 stub" 段删除,改 §"Pub/Sub 页是 read-only dashboard (M3.1)" + read-only banner doc
- [x] README §Quick-start 第一命令 `cargo run -p redis-server -- --port 6380`(无 --aof)
- [x] README §"Persistence (M3.2)" 子节单独说 `--aof` + 需 `mkdir -p data/`
- [x] README §Architecture diagram 加 "M4.3 target" 注 rust-embed line
- [x] README §"Supported commands" subsection,列 18+ 命令 + 显式 "Not implemented" list
- [x] README §"Prerequisites" 子节
- [x] README §"Docs" 子节(zh / en / agent links)
- [x] README §"Why does this exist?" 第一句 wedge claim
- [x] README §"Known behavioral deltas vs real Redis" 子节 link 2 个 finding

### Bootstrap
- [x] `scripts/bootstrap.sh` 不再说 "M1.0 scaffold",末段提示 working binary + frontend pnpm 流
- [x] `scripts/bootstrap.sh` 跑 `command -v pnpm` / `command -v node` 软检查,缺失只 warn

### Bilingual (Bucket D)
- [x] `docs/human/zh/finding-m1-1-p9-missed-shared-doc-coverage.md` 存在
- [x] `docs/human/en/finding-m1-1-p9-missed-shared-doc-coverage.md` 存在
- [x] `docs/human/{zh,en}/finding-m3-1-lagging-subscriber-disconnect.md` 各存在
- [x] `docs/human/{zh,en}/finding-m3-2-aof-replay-corruption-handling.md` 各存在
- [x] `docs/human/zh/README.md` + `docs/human/en/README.md` 更新到 M4.3 状态;列 ADR 0001-0013

### doc-coverage.sh
- [x] `_shared/doc-coverage.sh` 新增一段循环 `docs/agent/findings/*.md`,对每个 finding name 检查 `docs/human/{zh,en}/` 至少有一个 .md 提及
- [x] `bash _shared/doc-coverage.sh` 单跑 exit 0(已 8 finding 全 bilingual after D1-D4 land)
- [ ] 故意删一个 zh finding 文件再跑 → exit 1(回归 test in tests/doc-cov-self-test.sh OR 手测)

### ADR metadata
- [x] ADR-0005 `last_verified_commit` updated to `d02aa55`(or last server.rs touch)+ 加 §"Verified by M4.1 review" addendum
- [x] ADR-0001 Done Criteria "M3 完成 rust-embed" 改 "M4.3+ — 见 ADR-0008 + ADR-0011 deferral"
- [x] ADR-0009 §Done Criteria 错误 string 加 "RESET"(self-consistency)
- [x] ADR-0009 §"Implementation deltas" 加 1 行 "pubsub helpers landed in lib.rs, not separate pubsub.rs file as Cross-references suggested"
- [x] cs01 CLAUDE.md §3 wave order 每行 wave 加 `✅ shipped @ <SHA>` / `🚧 in progress`

### Gates
- [x] fmt / clippy / build / test / doc-coverage 全过 in M4.2 sweep; M4.4 release-doc fix re-ran doc-coverage + rustfmt only because this patch is docs-only
- [x] frontend-gate 全过 in M4.2/M4.3 release-surface validation; no frontend files are changed by M4.4 release-doc fix
- [x] oracle 36/36(无回归 claimed by M4.2/M4.3; M4.4 docs-only patch does not alter protocol code)
- [x] backend test count 仍 ≥ 284(不应减少;M4.2 不加 test 也不删 test)

## Cross-references

- ADR-0011 M4.1 critical fixes(parent wave)
- finding `m4-pre-release-audit-team-aggregation.md`(audit source-of-truth,本 ADR 的 ~37 个 deferred items 都从这来)
- 顶层 CLAUDE.md §1.1 双语 invariant(D1-D4 + E1 关闭本约束的 audit gap)
- top README + ADSD upstream(METHODOLOGY-STATUS 是回灌素材)
- 文件改动清单:
  - **NEW** `LICENSE-APACHE`,`LICENSE-MIT`,`CHANGELOG.md`,`CONTRIBUTING.md`,`SECURITY.md`,`METHODOLOGY-STATUS.md` (all at repo root)
  - **NEW** `cs01-mini-redis-rust/CHANGELOG.md`
  - **NEW** 6 bilingual finding abstract files
  - **MODIFY** `cs01-mini-redis-rust/README.md`(7+ section edits)
  - **MODIFY** `cs01-mini-redis-rust/CLAUDE.md`(§3 closure markers)
  - **MODIFY** `cs01-mini-redis-rust/scripts/bootstrap.sh`
  - **MODIFY** `_shared/doc-coverage.sh`(+ finding bilingual enforce)
  - **MODIFY** `docs/agent/adr/0001-stack-choice.md`,`0005-tcp-listener.md`,`0009-m3-1-pubsub.md`
  - **MODIFY** `docs/human/zh/README.md`,`docs/human/en/README.md`

## Notes

- 本 ADR 是 ADSD F1 sediment 的**第二次正面修复**(M4.1 修 constitution drift,M4.2 修 doc-vs-code drift)
- M4.3 紧接其后:0.1.0 tag + final CHANGELOG entry + METHODOLOGY-STATUS update
- 顶层 README 也可能需要 small update,但本 ADR 主要聚焦 cs01;顶层是后续 case 一起处理
- Sarah 的 "v0.1.0 tag + cs02 ship" 解锁条件中,本 wave 完成 0.1.0 tag prep,但 cs02 ship 不在本 case scope
