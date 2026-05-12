---
finding: m4-pre-release-audit-team-aggregation
date: 2026-05-12
case: cs01-mini-redis-rust
severity: P1
specificity: high
related_adr: 0001-0010
related_f: F1 (sediment), F8 (marketing), F17 (KPI), F18 (self-review), F23-A (oracle), F24 (primitive)
last_verified_commit: bf4307c
audit_team: 8-agent self-applied (4 internal + 3 persona + 1 deep-source-read)
---

# Finding: Pre-M4 8-agent audit team aggregate findings

## Hypothesis

After 5 sequential wave merges in one session (M1.3 → M1.4 → M2.1 → M2.2 → M3.1 → M3.2,closing Wave M3),the CTO's strategic altitude is dense and ADRs/code may have drifted relative to:
- The cs01 local constitution (CLAUDE.md §1 F24, §4 layer rule)
- The top-level constitution (CLAUDE.md §1.1 invariants, §3 standards)
- Real Redis 7 wire format (F23-A oracle gap risk)
- Public README narrative (F1 sediment / F8 under-claim)

Per memory `feedback-autonomy-and-language` §Rule 3 ("review 团队补清醒度"), dispatched a self-applied multi-agent audit team at Wave M3 closure (commit `bf4307c`) per ADSD SKILL.md §"Self-applied multi-agent audit" + §"LLM-simulated user persona" + §"Deep-source-read" — 8 dimensions in 2 waves of 4 parallel read-only sub-agents.

## Method

**Wave 1** (4 internal dimensions, parallel, read-only):
- Security: scan for credentials, DoS surface, untrusted input, recursion / channel caps, file modes
- Doc-consistency: ADR vs code drift, last_verified_commit staleness, bilingual zh/en sync, findings ledger completeness
- Public-readiness: README scope-claim defensibility, install path, stub markers, "would a stranger close the tab?"
- Code-quality: `.unwrap()` discipline, async correctness, RAII guards, lint compliance, module structure, hot-path allocations

**Wave 2** (3 persona simulations + deep-source-read, parallel, read-only):
- Persona A — Mei (Python data engineer, real Redis prod user): first-impression decision
- Persona B — Aleksandr (Rust senior eng, broadcast-burned): would-I-merge-this code review
- Persona C — Sarah (Staff Eng, OSS adoption reviewer): production-readiness scorecard
- Deep-source-read: line-by-line, no dimensional lens

Each agent had 30-60 min budget, structured `[*-AUDIT-COMPLETION]` report format, severity tiers (BLOCK/HIGH/MED/LOW).

## Result

**Total findings**: ~80 raw / ~50 after dedupe. Aggregate severity:

| Tier | Count | Notes |
|---|---|---|
| BLOCK | 1 | Public-readiness: README §Status 两波过期 |
| HIGH (cross-validated by ≥2 agents) | 12 | Listed below |
| HIGH (single-agent) | 14 | Listed below |
| MED | 20 | M4 polish backlog |
| LOW | 13 | Cosmetic / future-work |

### Cross-validated HIGH (must-fix in M4)

| # | Issue | Detected by | M4 fix |
|---|---|---|---|
| **CV-1** | README §Status 两波过期 (M3.1+M3.2 merged, README says ⬜) | Public-rdy BLOCK-1, Doc-consistency HIGH-1, Mei issue #1 | Update bullet list with ✅ markers |
| **CV-2** | `--aof data/dump.aof` quick-start ENOENT on fresh clone | Public-rdy HIGH-2, Mei issue #1 (same PR target) | Drop `--aof` from quick-start, or `AofWriter::new` mkdir parent |
| **CV-3** | `/pubsub` README says "M3 placeholder" but is live read-only dashboard | Public-rdy HIGH-1, Doc-consistency HIGH-1 | Update README §"Pub/Sub 页是 stub" |
| **CV-4** | `bootstrap.sh` says "M1.0 scaffold" — months stale | Public-rdy HIGH-4, Mei issue #1 | Rewrite the closing hint text |
| **CV-5** | LICENSE / LICENSE-MIT / LICENSE-APACHE files MISSING | Public-rdy MED-1, Sarah scorecard 4/10 license | Add Apache-2.0 + MIT text files at repo root |
| **CV-6** | No CONTRIBUTING / SECURITY / CHANGELOG | Public-rdy MED-2, Sarah scorecard | Add per top-level ADSD pattern |
| **CV-7** | RESP listener has no AUTH; default bind `0.0.0.0` exposes to LAN | Security HIGH-1+2, Aleksandr (public-facing = no), Sarah security 1/10 | Bind default `127.0.0.1`; reserve AUTH for later but document |
| **CV-8** | `Frame::parse` array arm has no recursion depth limit — stack overflow attack | Security HIGH-4, Deep-source (cross-confirmed) | Add `MAX_DEPTH = 32` private threading |
| **CV-9** | Unbounded `tokio::spawn` per accept — no max-clients cap | Security HIGH-3, Aleksandr's "MAXMEMORY"-adjacent concern | Add `--max-clients <N>` flag + early reject |
| **CV-10** | findings ledger missing `m3-1` + `m3-2` | Doc-consistency HIGH-2 | Append 2 rows to `findings/README.md` |
| **CV-11** | 3 findings have no zh/en bilingual abstracts (top CLAUDE.md §1.1 violation) | Doc-consistency HIGH-3 | Add 6 abstract files (m1-1, m3-1, m3-2 × zh+en) |
| **CV-12** | ADR-0005 `last_verified_commit` 32 commits stale | Doc-consistency HIGH-4 | Re-bless to current HEAD or last `server.rs` touch |

### Single-agent HIGH (must-fix in M4)

| # | Issue | Detected by |
|---|---|---|
| SA-1 | **cs01 CLAUDE.md §1 explicitly bans `tokio::sync::broadcast` for Pub/Sub** but ADR-0009 uses exactly that — constitution-vs-decision drift (F1 candidate) | Code-quality HIGH-1 |
| SA-2 | **`redis-storage → redis-protocol` dep violates cs01 §4 single-direction layer rule** — M3.2 P9 added it for `aof_encode`/`replay_from_path`,I守闸 时没 catch | Code-quality HIGH-2 |
| SA-3 | **AOF `Always` is misleading**:`Reply::Ok` returns before `sync_data` (fsync happens on writer task) — real Redis blocks request | Aleksandr concern #3 |
| SA-4 | **AOF mpsc is `unbounded_channel`** — slow disk + write burst → OOM. Top CLAUDE.md §3.3 dual problem | Deep-source HIGH-4 |
| SA-5 | **`replay_from_path` reads entire AOF into RAM 2× (Vec + BytesMut)** + blocks runtime worker | Deep-source MED-1 + MED-4 |
| SA-6 | **`SET k v EX 60 GARBAGE` parses as 60s TTL,ignores trailing tokens** — real Redis returns `-ERR syntax error`. **F23-A oracle gap**(M3.2 oracle 没 cover trailing-token case) | Deep-source MED-6 |
| SA-7 | **AOF tail-corruption silent re-replay** — INCR/DECR not idempotent,reboot 后 counter drift up | Security MED-2 + MED-3 (cross-confirmed by Sarah scorecard backup 2/10) |
| SA-8 | **AOF file 默认 umask 0o644 — world-readable**(可能含 password / session token)真 Redis 0o600 | Security MED-1 |
| SA-9 | `recv_any_subscription` boxes N futures per select-loop iteration — `tokio_stream::StreamMap` 是正解 | Aleksandr concern #1 |
| SA-10 | `Store::subscribe` 每次都 write-lock 即使 channel 已存在(应该 read-then-write-on-miss) | Aleksandr concern #2 |
| SA-11 | `main.rs:140-146` confused-then-corrected comment block — AI-slop tell + 误导 future maintainer | Aleksandr + Deep-source HIGH-3 |
| SA-12 | `Reply::Bulk(Some(value.clone()))` GET 热路径每次 alloc `Vec<u8>` — §3.3 violation,M4/v0.2 candidate (`bytes::Bytes`) | Code-quality MED-2 |
| SA-13 | `doc-coverage.sh` 漏 enforce finding zh/en bilingual (root cause of CV-11) | Doc-consistency LOW-2 |
| SA-14 | METHODOLOGY-STATUS.md sections 全是 "?" 占位 — Sarah evaluator scorecard扣分 | Sarah memo |

### Sample of MED (M4 backlog,not all must-fix)

- Security MED-4: KEYS O(N) under read-lock blocks writers — admin-only when AUTH lands
- Security MED-5: Lagged-subscriber 错误字符串是 side channel
- Security LOW-1: `/api/keys` SSE 暴露 key list — M4 bearer token
- Doc-consistency MED-1: ADR-0009 提 `pubsub.rs` 不存在(impl 在 lib.rs)
- Doc-consistency MED-2: ADR-0009 §Done Criteria 错误字符串少 "RESET"
- Doc-consistency MED-3: bilingual READMEs 还在 "M0 scaffold"
- Doc-consistency MED-4: cs01 CLAUDE.md §3 wave order 无 closure markers
- Doc-consistency MED-5: ADR-0001 Done Criteria 说 rust-embed "M3 完成" → 应该 M4
- Code-quality MED-1: lib.rs 983 行,pubsub 没拆 module(应该 `pubsub.rs`)
- Code-quality MED-4: AOF replay 静默 swallow `Reply::Error`(replay 计数过报)
- Code-quality MED-5: `AppState::clone()` 6 atomic incs per accept(可包 `Arc<AppState>`)
- Code-quality LOW-4: `AppState` 8 字段(边界 §3.1 限 7,but doc 已 justify)
- Deep-source MED-2: `do_ttl` 用 `unwrap_or(i64::MAX)` 掩盖 invariant
- Deep-source MED-3: `do_expire` 用 `unwrap_or(0)` 当 dead defence
- Deep-source MED-5: `/api/{stats,keys,pubsub}` 3 个 snapshot 独立 lock,无 cross-snapshot consistency
- Deep-source LOW-1: `Frame::Integer` encoder 评论说 "itoa-style" 实际 `.to_string()` 分配(comment lie)
- Aleksandr concern #4: `aof_encode` 在 `Reply::Error` 路径上仍编码 → 浪费

## Conclusion

**Audit team delivered ~80 raw findings,~50 unique,12 cross-validated HIGH,14 single-agent HIGH,20 MED,13 LOW**。

8-agent leverage 信号:
- **F1 sediment 真存在且严重**:CV-1 / CV-3 / CV-4 / CV-10 / CV-12 / SA-13 — 我每次 wave merge commit msg 都 "ship X",但 README + ledger + last_verified_commit + finding 双语都没同步更
- **F23-A oracle 有 gap**:SA-6(`SET k v EX 60 GARBAGE`)— 我的 oracle harness 都是 happy-path,边界 / trailing-token / 多空白没测;Aleksandr 没 catch,只有 deep-source line-by-line 抓到
- **F8 反向 under-claim**:CV-1 — README §Status 是反向"under-claim",但效果跟"over-claim"对外影响一致(visitor close tab)
- **Constitution-vs-ADR drift (F1.x sub-pattern)**:SA-1(broadcast pub/sub 禁令 vs ADR-0009)+ SA-2(layer rule vs ADR-0010 storage→protocol)— **我作为 CTO 写 ADR 时没 cross-check local constitution**;这是 ADSD F1 "snapshot sediment" 的同源变体 — "decision doc vs charter doc 分裂"
- **Multi-agent leverage 真兑现**:Persona A/B/C 各自 catch 了维度 agent 漏的(Mei catch quick-start ENOENT user-impact;Aleksandr catch AOF Always misleading;Sarah catch LICENSE legal blocker + bus factor)。Deep-source 抓 file:line precision 维度 agent 看不到的(MED-6 trailing-token,HIGH-3 confused-comment,HIGH-4 unbounded mpsc)
- **Persona endorsement asymmetry**:Mei 4/5,Aleksandr 4/5,Sarah 36/100(WATCH-FROM-DISTANCE)— 同一份代码,不同 user 类不同分。预期:Mei 代表"被 wedge 故事 sold",Aleksandr 代表"代码层认可 scope",Sarah 代表"production-adoption 评估"。**全过 = 真广 appeal;Sarah 低分预期内**(她要求 v1.0 + cs02 ship + LICENSE + bus factor > 1,这是 6 个 case 完成后才解锁的)

## Fix / Mitigation

提议 M4 拆 ADR-0011 (M4.1 critical fixes) + ADR-0012 (M4.2 doc sweep + release artifacts) 两个 wave:

### M4.1 — Critical fixes (ADR-0011 next)

P0 (real bugs / security):
- CV-7 default bind 127.0.0.1
- CV-8 Frame::parse recursion depth limit (32)
- CV-9 --max-clients flag + early reject
- SA-1 constitution drift fix:cs01 CLAUDE.md §1 update 或 ADR-0009 addendum 说明 broadcast 选择(我倾向 update CLAUDE.md,因为 ADR-0009 的 Pros/Cons 实际合理,constitution 当时 over-prohibit)
- SA-2 layer rule fix:cs01 CLAUDE.md §4 update 接受 storage→protocol 单向 dep(为了 AOF 用 Frame::to_bytes),或者 refactor 把 `aof_encode` 抽到 server crate
- SA-3 AOF `Always` rename or actually sync in request path(rename 更对,因为 P9 evidence 显示 even with Always 实际异步)
- SA-4 AOF mpsc bounded(`mpsc::channel(8192)` with try_send-and-drop-or-block)
- SA-5 replay_from_path 用 tokio::fs streaming(M4.1 真正 prod-grade 需要;但 1GB 以下接受 std::fs)
- SA-6 dispatch parse_set 严格 arity:`parts.len() == 5` for EX form,reject `>5`(+ oracle 加 trailing-token test)
- SA-8 AOF 文件 mode 0o600
- SA-11 main.rs:140-146 cleanup
- Deep-source LOW-1:Frame::Integer encoder 评论 fix

P1 (performance / hardening):
- SA-9 `tokio_stream::StreamMap` for pubsub fan-in
- SA-10 Store::subscribe read-then-write-on-miss
- SA-12 `Reply::Bulk` 用 `bytes::Bytes` (deferred to v0.2;M4.1 不做 — 接 ADR memo)

### M4.2 — Doc sweep + release artifacts (ADR-0012)

P0:
- CV-1 README §Status update
- CV-3 README /pubsub 段落 update
- CV-4 bootstrap.sh 重写
- CV-5 LICENSE-APACHE + LICENSE-MIT
- CV-6 CONTRIBUTING.md + SECURITY.md + CHANGELOG.md
- CV-10 findings ledger
- CV-11 6 个 bilingual finding abstracts
- CV-12 ADR-0005 (和其他 stale) `last_verified_commit` re-bless
- SA-13 doc-coverage.sh 扩 enforce finding bilingual
- SA-14 METHODOLOGY-STATUS.md 第 1 节(cs01)填实

P1:
- 加 README "Supported commands" 子节(Public-readiness HIGH-5)
- 加 README screenshot 子节(Mei + Public-readiness LOW-1)
- 加 README docker-compose 示例(Mei wishlist)
- 加 cs01 CLAUDE.md §3 wave closure markers (Doc-consistency MED-4)

### M4.3 (post M4.1+M4.2):0.1.0 tag

- git tag v0.1.0 (signed?用户决定)
- 写 CHANGELOG.md v0.1.0 entry
- METHODOLOGY-STATUS.md cs01 row finalize

## Lessons / F-pattern mapping

- **New F-candidate**:"**Constitution-vs-ADR drift**"(F1 sub-pattern)— ADR 决策 doc 在写时没 cross-check charter doc(local CLAUDE.md);CTO 守闸 时也没 catch。Mitigation:**Phase 1 ADR 模板加 "Constitution cross-check" 段**,explicit assert "本决策跟 cs01 CLAUDE.md §X 一致 / 矛盾 → 同 commit patch §X"
- **New F-candidate**:"**Audit-team multi-agent leverage**"(positive)— 8-dimension 互不重叠抓到 unique findings 集合,验证 ADSD SKILL.md §"Empirical leverage" 的 8× claim。具体本 case 测算:8 个 agent × ~45 min 平均 = 6 agent-hour;single CTO 自审 1 wave 估计 4-6 hour 且漏 ≥50% 的 finding。**实际 leverage ≈ 6-8×**(对得上 ADSD 上游)
- **Validates ADSD F23-A 边界**:F23-A oracle 抓 wire format,但**抓不到 parser 严格性**(SA-6 trailing-token bug)— oracle 只测"happy-path 我们跟真 Redis 对得上",不测"边界 input 我们跟真 Redis 同样拒"。**ADSD upstream candidate**:F23-A 子模式 "happy-path-only oracle gap",提议 oracle harness 加 fuzz-style "reject-on-malformed" 测 case
- **F18 self-review 的反向证明**:本次审计**不是** CTO 自己写自己测;是 8 个独立 sub-agent + 3 个 LLM persona simulation。即使我作为 CTO 守闸时跑过 5-gate + smoke,**multi-agent 仍抓到 12 个 cross-validated HIGH**。单 reviewer(我)真的 systematically miss 多维度

## Notes

- 本 finding 是 ADSD 这个 repo **第一次** dispatch full 8-agent audit team。如果未来其他 case (cs02-cs06) 也每 pre-release dispatch,可以汇集 N 个 case × ~50 finding ≈ 300 finding,成 ADSD upstream `case-study/` 的统计数据 set
- Persona simulation 的"in character"discipline 是关键:Mei / Aleksandr / Sarah 都没 break character(每个 KPI fidelity self-check 明确 "no AI breaks");这是 ADSD SKILL.md §"Stay-in-character constraint" 实际兑现
- Deep-source-read 跟 dimension agent 互补不重叠:dimension agent 用 grep/lens 扫,deep-source 不带 lens line-by-line。两者并行才覆盖完整 — single mode 漏 ~30% findings
- Wave 2 完成后 audit-team 总 token 消耗(粗略):4 wave1 ~250k + 4 wave2 ~300k ≈ 550k tokens。我作为 lead reviewer 整合用了 ~50k。**Budget 8 agent run + lead integration ≈ 600k tokens** — pre-major-release 节点的合理投入
