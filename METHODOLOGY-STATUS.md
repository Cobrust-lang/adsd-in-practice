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

## CS-01 mini-redis-rust(待填)

> v0.1.0 ship 后填写

| 实践 | 评价 | 改造成本 | 证据 | 备注 |
|---|---|---|---|---|
| ADR-driven 决策捕获 | ? | ? h | ? | |
| Finding-driven 失败 | ? | ? h | ? | |
| 双语 zh/en doc | ? | ? h | ? | |
| Wave-based commit | ? | ? h | ? | |
| 5-gate CI | ? | ? h | ? | |
| Doc-coverage gate | ? | ? h | ? | |
| F24 primitive 禁令 | ? | ? h | ? | |

**新 F-pattern**:待填

**总评**:待填

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

- [ ] 7 个表格行都填了(不能空)
- [ ] 改造成本是真实测量(stopwatch 或 commit 时间差),不是估计
- [ ] 证据有具体 commit SHA / finding 文件 / PR 链接
- [ ] 新 F-pattern 有 high-specificity 描述(commit / 时间戳引用)
- [ ] 跟 Cobrust 原始 case 做横向对比(同样实践在 Cobrust 上工作得怎样)
- [ ] 不掩饰失效,**ADSD 在本 case 上越破,本 repo 越有价值**
