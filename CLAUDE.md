# ADSD in Practice — Top-Level Agent Constitution

> 顶层 agent constitution。适用于所有 case study;每个 case 可在自己目录下加 `CLAUDE.md` 覆盖局部约定。
>
> 沿用 [Cobrust Studio CLAUDE.md](https://github.com/Cobrust-lang/cobrust-studio/blob/main/CLAUDE.md) 设计思路,**对 sub-agents 是 read-only 参考,不要修改本文件除非有 ADR 授权**。

---

## 0. Identity

- **Repo name**: ADSD in Practice
- **One-line pitch**: 多语言多领域的 ADSD 方法论 case study 集合,故意覆盖 ADSD 原始 case(Cobrust)外的语言/领域,**找出 ADSD 在哪里破**。
- **Relationship to ADSD**: 是 ADSD 的 N+1 ~ N+6 验证 source。每个 case 完成后产出一份 `METHODOLOGY-STATUS.md` 增量。
- **License**: Apache-2.0 + MIT dual。
- **Default branch**: `main`。

## 1. 核心原则

### 1.1 不可妥协的三不变量

1. **每个 case 5-gate green 才能 merge**
2. **每个 ADR/finding 双语双向 sync**(`docs/human/zh/` + `docs/human/en/` + `docs/agent/`)
3. **F24 primitive-as-everything 禁令** — 不允许用 list 模拟 hashmap、用 mutex 模拟 lock-free 这类偷懒。每个 case 在自己的 `CLAUDE.md` §1 列出"核心约束不能简化的清单"。

### 1.2 默认 proceed

- 当本文件 / case CLAUDE.md / ADR 都没说怎么办时:**默认 proceed + 写 ADR**,不要请示。
- 只对**不可逆**决策请示:license、name、public API 冻结、breaking changes after v0.1.0。

### 1.3 显式标 stub

- 任何 stub / mock / 简化实现,**必须在 README + commit msg + ADR(若涉及决策)同时标注**。
- 参考 Cobrust Studio README "M2 status note: AES-GCM stub blob" 这种主动声明范式。

## 2. 方法论 dogfood

| 实践 | 在本 repo 怎么 dogfood |
|---|---|
| ADR-driven 决策捕获 | 每个 case `docs/agent/adr/NNNN-*.md`,顶层用 `_shared/adr-template.md` |
| Finding-driven 失败捕获 | 每个 case `docs/agent/findings/*.md` |
| 双语 zh/en + agent doc 三轨 | 每个 case `docs/human/{zh,en}/` + `docs/agent/`,doc-coverage 脚本卡死 |
| Wave-based 提交批 | Tx commit tags `feat(scope): A1.3 ...`,wave 用 `(Wave X.Y)` 后缀 |
| Doc-coverage CI gate | 每个 case `scripts/doc-coverage.sh`,顶层 `_shared/doc-coverage.sh` 是基准 |
| 5 CI gates | 每个 case 独立 workflow,语言对应 `_shared/5-gate-{rust,python,cpp}.sh` |
| External review cycle | review-claude 模板可后期加(参考 ADSD `.github/`) |

## 3. 工程标准

### 3.1 Elegant

- 公共 API 该用 newtype 就用 newtype。
- Rust:非测试代码不准 `.unwrap()`,改用 `.expect("rationale")`。
- Python:非测试代码不准裸 `except:`,改用 `except SpecificError as e:`。
- C++:不准 `using namespace std;` 于头文件。
- 每个公共结构 ≤7 个公开字段,超过要文档说明。

### 3.2 Scientific

- 每个设计决策都有 ADR。
- 每个基准测试可重现:脚本化、固定种子、硬件标签。
- 负面结果归档在 `findings/`,不要丢。

### 3.3 Efficient

- 每个 case 单 binary / 单 wheel / 单 wasm bundle 部署优先。
- 测试不准 sleep,用 condvar / 事件。
- 不准在热路径里 allocate(C++/Rust);Python 热路径不准用 `pandas.iterrows()` 之类的反模式。

## 4. 命名约定

- 文件夹:`cs01-mini-redis-rust`(case_id-名字-语言)
- 文件:Rust `snake_case.rs`、Python `snake_case.py`、C++ `snake_case.cpp` / `snake_case.hpp`
- ADR:`NNNN-kebab-case-title.md`(从 0001 起,**每个 case 独立编号空间**,不跨 case)
- Finding:`{milestone}-{slug}.md`(例:`m2-spa-fallback-extractor.md`)
- Commit:`<type>(<scope>): <Tx tag> <subject> (Wave <X.Y>)`
  - 例:`feat(server): A1.4 wire SSE dispatch route (Wave A1)`
- Tx tag 字母:**每个 case 独立空间**(cs01-A1.1 跟 cs02-A1.1 不冲突,因为路径不同)

## 5. Sub-agent 操作指南

- **默认 proceed**:可逆决策做了就做,写 ADR 解释。
- **仅请示不可逆**:license / name / public API freeze / breaking change after 0.1.0。
- **每次代码改动同步双语 doc**:CI doc-coverage 卡死。
- **每个 Tx 自成 commit**,wave 合并 = git merge of Tx branches。
- **5-gate green 才能 merge**,无例外。

## 6. 跨 case 优先级

1. **先 ship scaffold(本阶段)** — 所有 case 目录、CLAUDE.md、ADR-0001 stack choice、5-gate 脚本、bootstrap 都齐
2. **逐个 case 实现到 0.1.0**(F22-aware,**串行不并行**,每个 case M4 release 后写 status)
3. **写 `METHODOLOGY-STATUS.md` 增量**,每个 case 各 1 节
4. **最终回灌 ADSD repo**(开 PR 把 status 提炼成 ADSD 的新 F-pattern 或 case-study)

## 7. 反模式自省清单

每完成一个 case 之前自问:

- [ ] 我有没有 F24 偷懒(primitive-as-everything)?
- [ ] 我有没有 F8 marketing overreach(自陈数字 vs 实测对得上吗)?
- [ ] 我有没有 F19 onboarding 没测(`bootstrap.sh` 在 clean shell 跑过吗)?
- [ ] 我有没有 F22 coverage-fix-cadence(先扩 case 后修第一波 bug)?
- [ ] 我有没有 F23-A oracle 自己写自己测(有第三方 reference 对照吗)?

---

**End of top-level constitution. Each case's local CLAUDE.md takes priority for case-specific rules.**
