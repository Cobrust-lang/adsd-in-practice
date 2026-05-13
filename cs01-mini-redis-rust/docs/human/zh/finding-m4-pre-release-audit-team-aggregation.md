# Finding M4 中文摘要:Pre-M4 8-agent audit team 整合

> 完整 finding 见 [docs/agent/findings/m4-pre-release-audit-team-aggregation.md](../../agent/findings/m4-pre-release-audit-team-aggregation.md)。

## 一句话

Wave M3 关闭后,按 ADSD §"Self-applied multi-agent audit" + §"LLM-simulated user persona" + §"Deep-source-read",派了 **8 个 read-only audit sub-agent**(4 internal dimension + 3 persona + 1 deep-source)。**~80 raw findings,~50 unique**:1 BLOCK / 12 cross-validated HIGH / 14 single-agent HIGH / 20 MED / 13 LOW。

## 关键发现

**Constitution-vs-ADR drift**(F1 新子模式):
- cs01 §1 明文禁 broadcast pub/sub,但 ADR-0009 选了 broadcast(SA-1)
- cs01 §4 single-direction layer rule,但 M3.2 让 storage → protocol(SA-2)

**F23-A oracle gap**:`SET k v EX 60 GARBAGE` 我们接受,真 Redis reject(SA-6)— deep-source line-by-line 抓到,dimension agent 和 oracle 都漏

**真 P0**:
- `Frame::parse` 数组递归无深度限制(stack overflow attack)(CV-8)
- AOF mpsc `unbounded_channel`(slow disk OOM)(SA-4)
- `--max-clients` cap 不存在(DoS)(CV-9)
- AOF `Always` policy misleading(`Reply::Ok` 返回早于 `sync_data`)(SA-3)
- AOF tail 损坏静默 re-replay → INCR/DECR counter drift(SA-7)

**Sediment** (F1):
- README §Status 两波过期(CV-1)
- bootstrap.sh "M1.0 scaffold"(CV-4)
- findings ledger 漏 m3-1+m3-2(CV-10)
- 3 finding 无 zh/en(CV-11)
- LICENSE 文件缺失(CV-5)

## Persona 评分

- Mei(Python user):4/5,BOOKMARK + RECOMMEND-ON-SLACK
- Aleksandr(Rust senior):4/5,"would merge this"
- Sarah(OSS evaluator):36/100,WATCH-FROM-DISTANCE(等 cs02 ship + LICENSE + v0.1.0 tag)

## 提议 M4 拆 wave

- **M4.1** ADR-0011 critical fixes(real bugs + constitution drift)
- **M4.2** ADR-0012 doc sweep + release artifacts(LICENSE + CHANGELOG + CONTRIBUTING + SECURITY + METHODOLOGY-STATUS + README sediment 修)
- **M4.3** 0.1.0 tag

## 状态

`P1`,partial-closed:M4.1 已关闭 critical code/security 项,M4.2 已关闭 release-doc/artifact 阻塞。M4.3/M4.4 Tauri follow-up 后续确认是 wrong-session scope contamination,已由 `m4-4-cross-session-tauri-contamination.md` 撤回,不再是 live cs01 release debt。**ADSD upstream `case-study/` 候选**:第一次 quantified 8-agent audit team leverage(≈ 6-8× single CTO 自审)。
