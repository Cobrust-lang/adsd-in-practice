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

## CS-01 mini-redis-rust(0.1.0 / M4.4 `.app` bundle gate passed)

CS-01 是 ADSD 在 Cobrust 之外的第一个强验证 case:网络协议 + async server + persistence + web/desktop release surface。到 v0.1.0 finalization main `31b52a1`,agent ADR 共有 **13** 份(0001-0013),agent finding 共有 **8** 份,并已触发一次 **8-agent pre-release audit**(4 internal + 3 persona + 1 deep-source-read)。审计原始结果约 80 条,去重约 50 条:BLOCK 1,HIGH cross-validated 12,HIGH single-agent 14,MED 20,LOW 13。Persona 分数/结论:Mei 4/5,Aleksandr 4/5,Sarah 36/100(WATCH-FROM-DISTANCE,主要卡 release/legal/security/bus-factor)。M4.4 已在 `31b52a1` 通过 `CS01_TAURI_FULL_BUILD=1 bash scripts/tauri-gate.sh` 的 macOS `.app` bundle gate,产物为 `cs01-mini-redis-rust/web/src-tauri/target/release/bundle/macos/CS01 mini-redis.app`;DMG、signing、notarization 仍是 out-of-gate future release-engineering risk,不伪装成已完成。

| 实践 | 评价 | 改造成本 | 证据 | 备注 |
|---|---|---|---|---|
| ADR-driven 决策捕获 | ✅ 完全有效 | 约 6 h | 13 个 ADR 覆盖 stack/RESP/storage/TCP/M2/M3/M4.1/M4.2/M4.3;ADR-0011/0012 直接从审计 finding 拆 sprint | 两阶段 SOP 有效,但 ADR 写完后必须 cross-check local CLAUDE.md |
| Finding-driven 失败 | ✅ 完全有效 | 约 3 h | 8 个 finding;TTL oracle bug、CTO 写代码、lagging subscriber、AOF corruption、8-agent audit、Tauri sidecar packaging blocker 都留下证据 | 负面结果没有隐藏,成为 M4.1/M4.2/M4.3/M4.4 backlog 来源 |
| 双语 zh/en doc | 🟡 半失效 | 约 4 h | M4 audit CV-11 抓到 3 个 finding 缺双语摘要;M4.2 扩 doc-coverage gate 修复 | 规则有效,但没有 tooling enforcement 时会沉积 |
| Wave-based commit | ✅ 完全有效 | 约 1 h | M1-M4.1 commit message 均用 Wave 标记;Tx tag 帮助从 audit 回溯责任面 | 快速 wave merge 会放大 README/metadata sediment,需 M4 sweep |
| 5-gate CI | ✅ 完全有效 | 约 2 h | fmt/clippy/build/test/doc-coverage 在每个 Rust sprint 守闸;M4.1 critical fixes 后继续跑 | 前端 gate 成为 case-local 第 6 gate |
| Doc-coverage gate | 🟡 半失效→修复中 | 约 2 h | M1.1 P9 误报无 shared script;M4 audit SA-13 抓到 finding bilingual 未 enforce | M4.2 将 findings mirror 加入 `_shared/doc-coverage.sh` |
| F24 primitive 禁令 | ✅ 完全有效 | 约 2 h | 不用 BTreeMap 假装 sorted set;Pub/Sub broadcast 通过 ADR-0009 明确 trade-off;unsupported Redis structures 直接 out-of-scope | F24 需要结合 ADR 允许合理 primitive,否则会过度禁止 |

**F-pattern 增量候选**:

1. **F1.x Constitution-vs-ADR drift**:local CLAUDE.md 曾禁止 `tokio::sync::broadcast`,ADR-0009 后实际选择 broadcast;local CLAUDE.md 也曾禁止 storage→protocol edge,ADR-0010 后为了 AOF wire compatibility 引入该 edge。根因不是代码错,而是 charter doc 与 decision doc 分裂。Mitigation:ADR 模板增加 "Constitution cross-check" 段。
2. **F23-A.happy-path-only oracle gap**:Redis oracle 对 happy-path wire compatibility 很有效(TTL rounding bug 被抓),但 malformed input (`SET k v EX 60 GARBAGE`) 直到 deep-source-read 才发现。Mitigation:oracle harness 必须包含 reject-on-malformed/fuzz-style negative cases。
3. **F8 inverse under-claim**:README 状态落后不是 marketing overreach,而是 under-claim;但对 public readiness 的伤害相同(visitor 以为项目没完成)。Mitigation:release sweep 把 stale status 当 BLOCK。
4. **F18.CTO-as-implementer**:M1.3 中 CTO 亲自写实现,违反两阶段 dispatch;finding 记录后恢复 P9 dispatch。Mitigation:CTO phase 只能写 ADR/test skeleton,不能写 implementation。
5. **Audit-team leverage as evidence**:8-agent audit 抓到单 reviewer 守闸漏掉的 release/legal/security/doc/code-source 精确问题,与 ADSD/Cobrust 的 8-dimension audit pattern 对齐。

**总评**:ADSD 在 cs01 上总体成立,尤其 ADR/finding/5-gate/8-agent audit 的收益明确。破点主要是速度导致的文档沉积,以及 Cobrust-derived 规则移植到 Redis domain 时产生的 charter-vs-ADR drift。M4.2 的结论不是“减少文档”,而是把 doc-coverage 从 ADR 扩展到 finding,并把 constitution cross-check 前置到 ADR phase。

---

## CS-02 mini-git-rust(待填)

> v0.1.0 ship 后填写

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

完成所有 case 后,这一节回答:

1. **ADSD 哪些原则是真正语言/领域无关的?**
2. **哪些是 Cobrust 特化、其它项目要 drop?**
3. **哪些需要"按领域分支"出 ADSD 子方言?**(例:`ADSD-for-LLM-apps`、`ADSD-for-concurrency`)
4. **F-catalog 增量了几条?** 总条目从 F24 涨到 F?
5. **5-gate 是不是仍然是 5,还是应该变成"5 + 领域 X 道"?**

---

## 写作 checklist(每个 case 完成时)

- [x] CS-01 7 个表格行已填(M4.3 rc; finding count reconciled to 8)
- [x] CS-01 改造成本使用 sprint/commit 时间级粗估,非精确 stopwatch;最终 v0.1.0 可重测
- [x] CS-01 证据有具体 ADR/finding 文件引用
- [x] CS-01 新 F-pattern 有 high-specificity 描述
- [x] CS-01 跟 Cobrust 原始 case 做横向对比(8-agent audit pattern / ADR/finding/5-gate)
- [x] CS-01 不掩饰失效:双语 gate 与 charter-vs-ADR drift 已标半失效/候选 F-pattern
