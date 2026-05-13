# METHODOLOGY-STATUS — ADSD 在多语言/多领域的实测有效性报告

> **本 repo 的核心 IP**。每个 case study 完成 v0.1.0 后,在此添加一节,诚实标注 ADSD 哪些约束**完全有效 / 半失效 / 完全失效**。
>
> 失效不是坏事——是 ADSD 的 N+1 case study 反哺数据。**作者诚信 > 方法论体面**。

---

## 评分维度(每个 case 都按此打)

每条 ADSD 实践打三档:

- **✅ 完全有效** — 在本 case 上跟 Cobrust 一样运转,无额外成本
- **🟡 半失效** — 需要本地化改造才能用,或者收益边际下降
- **❌ 完全失效 / 适得其反** — 在本 case 上反而是 noise,建议 ADSD 标 "domain-specific exception"

每条还要回答:
- **改造成本**(小时):为让 ADSD 在本 case 跑起来花了多少时间
- **收益证据**:具体抓到了什么 bug / 防住了什么 regression
- **F-pattern 增量**:本 case 是否撞出 ADSD F-catalog 里没有的新失败模式

---

## CS-01 mini-redis-rust(0.1.0 / M4.4 scope-corrected)

CS-01 是 ADSD 在 Cobrust 之外的第一个强验证 case:网络协议 + async server + persistence + browser monitoring surface。到 v0.1.0 finalization,agent ADR 共有 **12** 份 live accepted(0001-0012),agent finding 共有 **8** 份 live ledger entries,并已触发一次 **8-agent pre-release audit**(4 internal + 3 persona + 1 deep-source-read)。审计原始结果约 80 条,去重约 50 条:BLOCK 1,HIGH cross-validated 12,HIGH single-agent 14,MED 20,LOW 13。Persona 分数/结论:Mei 4/5,Aleksandr 4/5,Sarah 36/100(WATCH-FROM-DISTANCE,主要卡 release/legal/security/bus-factor)。M4.4 额外暴露一个方法论失败:wrong-session Tauri desktop packaging requirement 被误接收到 cs01 scope,随后由 `m4-4-cross-session-tauri-contamination` finding 撤回。最终有效 release surface 是 Rust RESP server + Axum HTTP/SSE control plane + SvelteKit browser dashboard;desktop packaging / installers / signing / notarization 不属于 cs01 `0.1.0` readiness。

| 实践 | 评价 | 改造成本 | 证据 | 备注 |
|---|---|---|---|---|
| ADR-driven 决策捕获 | ✅ 完全有效 | 约 6 h | 12 个 live ADR 覆盖 stack/RESP/storage/TCP/M2/M3/M4.1/M4.2;ADR-0011/0012 直接从审计 finding 拆 sprint | 两阶段 SOP 有效,但 ADR 写完后必须 cross-check local CLAUDE.md 和当前 case scope |
| Finding-driven 失败 | ✅ 完全有效 | 约 3 h | TTL oracle bug、CTO 写代码、lagging subscriber、AOF corruption、8-agent audit、cross-session Tauri contamination 都留下证据 | 负面结果没有隐藏,成为 M4.1/M4.2/M4.4 backlog 来源 |
| 双语 zh/en doc | 🟡 半失效 | 约 4 h | M4 audit CV-11 抓到 3 个 finding 缺双语摘要;M4.2 扩 doc-coverage gate 修复 | 规则有效,但没有 tooling enforcement 时会沉积 |
| Wave-based commit | ✅ 完全有效 | 约 1 h | M1-M4.1 commit message 均用 Wave 标记;Tx tag 帮助从 audit 回溯责任面 | 快速 wave merge 会放大 README/metadata sediment,需 M4 sweep |
| 5-gate CI | ✅ 完全有效 | 约 2 h | fmt/clippy/build/test/doc-coverage 在每个 Rust sprint 守闸;M4.1 critical fixes 后继续跑 | 前端 browser gate 成为 case-local 第 6 gate |
| Doc-coverage gate | 🟡 半失效→修复中 | 约 2 h | M1.1 P9 误报无 shared script;M4 audit SA-13 抓到 finding bilingual 未 enforce | M4.2 将 findings mirror 加入 `_shared/doc-coverage.sh` |
| F24 primitive 禁令 | ✅ 完全有效 | 约 2 h | 不用 BTreeMap 假装 sorted set;Pub/Sub broadcast 通过 ADR-0009 明确 trade-off;unsupported Redis structures 直接 out-of-scope | F24 需要结合 ADR 允许合理 primitive,否则会过度禁止 |

**F-pattern 增量候选**:

1. **F1.x Constitution-vs-ADR drift**:local CLAUDE.md 曾禁止 `tokio::sync::broadcast`,ADR-0009 后实际选择 broadcast;local CLAUDE.md 也曾禁止 storage→protocol edge,ADR-0010 后为了 AOF wire compatibility 引入该 edge。根因不是代码错,而是 charter doc 与 decision doc 分裂。Mitigation:ADR 模板增加 "Constitution cross-check" 段。
2. **F23-A.happy-path-only oracle gap**:Redis oracle 对 happy-path wire compatibility 很有效(TTL rounding bug 被抓),但 malformed input (`SET k v EX 60 GARBAGE`) 直到 deep-source-read 才发现。Mitigation:oracle harness 必须包含 reject-on-malformed/fuzz-style negative cases。
3. **F8 inverse under-claim**:README 状态落后不是 marketing overreach,而是 under-claim;但对 public readiness 的伤害相同(visitor 以为项目没完成)。Mitigation:release sweep 把 stale status 当 BLOCK。
4. **F18.CTO-as-implementer**:M1.3 中 CTO 亲自写实现,违反两阶段 dispatch;finding 记录后恢复 P9 dispatch。Mitigation:CTO phase 只能写 ADR/test skeleton,不能写 implementation。
5. **Audit-team leverage as evidence**:8-agent audit 抓到单 reviewer 守闸漏掉的 release/legal/security/doc/code-source 精确问题,与 ADSD/Cobrust 的 8-dimension audit pattern 对齐。
6. **Cross-session requirement contamination**:自主 loop 中,一个来自其他 session 的 Tauri desktop requirement 被误当作当前 case scope,并传播到 code/docs/gates。Mitigation:任何 orthogonal new release surface 必须做 scope-contamination check,即使它看起来技术上合理。

**总评**:ADSD 在 cs01 上总体成立,尤其 ADR/finding/5-gate/8-agent audit 的收益明确。破点主要是速度导致的文档沉积、Cobrust-derived 规则移植到 Redis domain 时产生的 charter-vs-ADR drift,以及 M4.4 暴露的 cross-session requirement contamination。M4.2/M4.4 的结论不是“减少文档”,而是把 doc-coverage 从 ADR 扩展到 finding,把 constitution/scope cross-check 前置到 ADR phase,并在发现 scope 污染时用 finding 公开撤回。

---

## CS-02 mini-git-rust(0.1.0 / M4 hardening closed)

CS-02 是 ADSD 在本地工具 + binary compatibility + filesystem state 领域的强验证 case。到 v0.1.0 local-readiness 收口,agent ADR 共有 **5** 份 live accepted(0001-0005),agent finding 共有 **1** 个关键 pre-release finding:`m4-pre-release-filesystem-hardening`。M1-M3 功能面先后闭合了 Git-compatible blob/loose-object、index/tree、repository state/commit/log/discovery；M4 没有继续扩 feature,而是按 pre-release 审计结果进入 release-hardening sprint,补上 atomic write、symlink ancestry 防护、`.mg/index.lock`、bounded parsing、repository-internal path rejection、lowercase SHA validation、README honesty 和 oracle 负例。该 case 没有撞出 BLOCK 级功能错误,但确实验证了 ADSD 在“功能已绿后仍需 hardening/doc honesty 二次收口”的节奏价值。真正让 `0.1.0` 可信的不是 M3 功能完成,而是 M4 把本地文件系统风险从“已知但未封口的 HIGH/MED finding”收敛成通过 oracle 和 5-gate 的发布证据。

| 实践 | 评价 | 改造成本 | 证据 | 备注 |
|---|---|---|---|---|
| ADR-driven 决策捕获 | ✅ 完全有效 | 约 4 h | ADR-0002/0003/0004 把 object/index/repo state 逐 wave 固定;ADR-0005 把 release hardening 范围压回 Option A,防止 M4 feature creep | 本 case 再次证明两阶段 SOP 能防止“功能做完后顺手扩 scope” |
| Finding-driven 失败 | ✅ 完全有效 | 约 2 h | `m4-pre-release-filesystem-hardening` 把 direct writes、unbounded inflate、index allocation、internal paths、docs overclaim 一次性落盘 | finding 直接成为 M4 sprint backlog,没有被“测试都绿了”掩盖 |
| 双语 zh/en doc | ✅ 完全有效 | 约 2 h | ADR-0005 与 finding 在 agent/en/zh 三轨同时落地;doc-coverage 始终维持 green | 相比 cs01,本 case 双语沉积明显更轻 |
| Wave-based commit | ✅ 完全有效 | 约 1 h | M1→M4 都以 Wave/ADR 锚点推进,功能层和 hardening 层边界清楚 | 尤其适合 plumbing/tooling 这类可严格分层的问题 |
| 5-gate CI | ✅ 完全有效 | 约 2 h | fmt/clippy/test/oracle/doc-coverage 在每波后都可直接判定 readiness | 对 binary-compat/tooling case,oracle 的价值甚至高于普通集成测试 |
| Real-oracle discipline (F23-A defense) | ✅ 完全有效 | 约 3 h | `tests/oracle.sh` 用真实 Git 双向验证 M1-M3,并在 M4 增加 negative hardening cases | 这是 cs02 最强证据:不是“自己写自己测”,而是一直对 Git 二进制对齐 |
| F24 primitive 禁令 | ✅ 完全有效 | 约 1 h | 没有退回 JSON/sqlite/uncompressed object 这类假兼容方案 | 说明 ADSD 在格式兼容型项目上能约束 agent 不偷懒 |

**F-pattern 增量候选**:

1. **F25 hardening-after-green gap**:功能 oracle 全绿并不等于 release-ready。对于 filesystem/stateful tooling,第一波“能跑通”之后仍可能残留 direct-write、symlink ancestry、allocation bound、lock cleanup 这类发布级问题。Mitigation:在 `0.1.0` 前强制一次 pre-release hardening audit,不能因为 functional gates 已绿就跳过。
2. **F26 documentation honesty as compatibility boundary**:当项目卖点是“兼容 X”时,README 的措辞本身就是一条 compatibility boundary。cs02 M4 证明 `.mg/.git 完全可互换` 这种一句话 overclaim 会把本来可接受的子集实现变成不诚实发布。Mitigation:把 doc honesty 放进 ADR done criteria 和 oracle narrative,不是留给 release note 润色。
3. **F23-A.negative-case extension**:真实 oracle 最初只覆盖 happy-path 双向兼容还不够;到 M4 才把 internal path、uppercase SHA、symlink ancestry、lock cleanup、inflation cap 等 negative paths 纳入 oracle。Mitigation:当 case 核心域是 local state/unsafe input 时,oracle 必须同时验证正向兼容和负向拒绝语义。

**总评**:ADSD 在 cs02 上表现得比 cs01 更“工科化”:object/index/commit 这类 binary compatibility 问题非常适合 ADR 分波、Git oracle 守闸和 finding 驱动 hardening。真正有价值的经验不是“agent 能把 mini-git 写出来”,而是方法论逼着我们在功能全绿后继续做 M4 honesty/hardening 收口,没有把本地文件系统风险伪装成可接受的 v0.1 debt。对 ADSD upstream 的启发是:凡是核心域涉及文件系统/对象格式/解析器边界,都应把 pre-release hardening audit 视为标配而不是可选项。

---

## CS-03 taskboard-llm-python(待填)

> v0.1.0 ship 后填写。**重点观察**:Python 生态(没 Cargo 那种 workspace 锁机制)下 5-gate 的 F10 cargo-lock-contention 是否消失,被什么新 F-pattern 取代?

---

## CS-04 pyfmt-mini-python(待填)

> v0.1.0 ship 后填写。**重点观察**:小型工具型项目(代码量 < 2k LOC),ADR/finding 的开销/收益比是否仍合算?

---

## CS-05 phys2d-wasm-cpp(待填)

> v0.1.0 ship 后填写。**重点观察**:跨语言/跨 build system(cmake + emcc + vite)下,5-gate 是不是变成 15-gate,还能不能 hold 住?

---

## CS-06 lockfree-queue-cpp(待填)

> v0.1.0 ship 后填写。**重点观察**:F-pattern 是否会从 catalog 里 F5(silent miscompile)扩展出 F5.1(memory ordering 漏判)、F5.2(ABA 误诊)等并发专属子项。

---

## 跨 case 总观察(预填占位)

> 等 CS-03+ 完成后更新。
