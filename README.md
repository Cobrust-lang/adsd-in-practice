<div align="center">

# ADSD in Practice

**多语言、多领域的 ADSD 方法论实战 case study 集合**

*Validation set for [Agent-Driven Software Development](https://github.com/Cobrust-lang/agent-driven-development) beyond the original Cobrust compiler.*

[![License](https://img.shields.io/badge/license-Apache--2.0%20%2F%20MIT-blue.svg)](#license)
[![Status](https://img.shields.io/badge/status-scaffolding-orange.svg)](#case-studies)

</div>

---

## 为什么有这个 repo

ADSD 方法论原本只在 **Cobrust(一个 Rust 编译器)** 这一个 case 上做过验证。`README` 里作者自己写了:

> single validated case study, actively seeking design partners for second-project validation

**这个 repo 就是那个 second-project validation,而且不止一个**——跨 Rust / Python / C++ 三种语言,跨「网络服务 / 系统工具 / Web 应用 / 算法库 / 物理仿真 / 并发原语」六种领域,跨「带前端 / 纯后端」两种形态,故意把 ADSD 拉到它训练数据外的领域上压力测试。

**目标不是证明 ADSD 普适,是找出它在哪里破**。每个 case 完成后,在 `METHODOLOGY-STATUS.md` 写一份"哪些 ADSD 原则在本 case 上失效 / 半失效 / 完全有效"的诚实报告,作为 ADSD 的 N+1 case study 反哺。

## Case Studies

| # | 名字 | 语言 | 前端 | 领域 | 状态 |
|---|---|---|---|---|---|
| **CS-01** | [mini-redis-rust](cs01-mini-redis-rust/) | Rust | SvelteKit | 网络服务 + 协议解析 + 存储 + 实时监控 | scaffold |
| **CS-02** | [mini-git-rust](cs02-mini-git-rust/) | Rust | — | 系统工具 + 对象存储 + diff/merge | scaffold |
| **CS-03** | [taskboard-llm-python](cs03-taskboard-llm-python/) | Python | SvelteKit | Web 应用 + LLM 集成 + SSE | scaffold |
| **CS-04** | [pyfmt-mini-python](cs04-pyfmt-mini-python/) | Python | — | 算法工具(formatter)+ AST 操作 | scaffold |
| **CS-05** | [phys2d-wasm-cpp](cs05-phys2d-wasm-cpp/) | C++ | wasm + Svelte | 物理仿真 + 跨语言整合 | scaffold |
| **CS-06** | [lockfree-queue-cpp](cs06-lockfree-queue-cpp/) | C++ | — | 并发原语 + 高压力工程纪律 | scaffold |

每个 case study 都是**独立可构建、独立 5-gate CI、独立 ADR/finding 命名空间**的子项目。

## 共享设施

- **[`_shared/adr-template.md`](_shared/adr-template.md)** — ADR 模板(跨 case 一致)
- **[`_shared/finding-template.md`](_shared/finding-template.md)** — finding 模板
- **[`_shared/5-gate-rust.sh`](_shared/5-gate-rust.sh)** — Rust 五道闸门(fmt + clippy + build + test + doc-coverage)
- **[`_shared/5-gate-python.sh`](_shared/5-gate-python.sh)** — Python 五道闸门(ruff + mypy + pytest + coverage + doc-coverage)
- **[`_shared/5-gate-cpp.sh`](_shared/5-gate-cpp.sh)** — C++ 五道闸门(clang-format + clang-tidy + ctest + cppcheck + doc-coverage)
- **[`_shared/doc-coverage.sh`](_shared/doc-coverage.sh)** — 双语 doc-coverage 强制脚本
- **[`_shared/tx-tag-spec.md`](_shared/tx-tag-spec.md)** — conventional commits + Tx tag 规范

## 跨 case 元产出

- **[`CLAUDE.md`](CLAUDE.md)** — 顶层 agent 宪法,适用于所有 case;每个 case 可在自己目录下加 `CLAUDE.md` 覆盖
- **[`METHODOLOGY-STATUS.md`](METHODOLOGY-STATUS.md)** — **本 repo 的核心 IP**:实测 ADSD 哪些约束在每个 case 上失效 / 半失效 / 完全有效;**每个 case 完成后必须更新**
- **[`.github/workflows/`](.github/workflows/)** — 每个 case 独立 CI workflow

## 三个不变量(任何 case 不能违反)

1. **每个 case 独立 5-gate green 才能 merge** — 跟 ADSD 一致
2. **每个 ADR/finding 双语双向 sync** — 中文写完立即英文,反之亦然
3. **不允许「primitive-as-everything」(F24 反模式)** — 见 ADSD failure-modes-catalogue F24。例:CS-06 不准用 `std::mutex` 模拟 lock-free,CS-01 不准用 `BTreeMap` 假装 Redis hash

## 启动一个 case

```bash
# clone 整个 repo
git clone https://github.com/Cobrust-lang/adsd-in-practice && cd adsd-in-practice

# 进入某个 case
cd cs01-mini-redis-rust

# 一键启动(每个 case 自带 bootstrap.sh)
bash scripts/bootstrap.sh
```

## 关系链

```
Cobrust (语言, case study #0)
   └─ catalyzes ─→  ADSD (方法论 IP)
                     └─ products ─→ Cobrust Studio (SaaS 产品)
                     └─ validates with ─→ ADSD in Practice (本 repo, N+1 ~ N+6)
```

## License

Dual-licensed under Apache-2.0 + MIT at your option.

---

*Repo created 2026-05-XX. CS-01 → CS-06 will be scaffolded then implemented sequentially (F22-aware: 不并行,每个 case 写完 status 再下一个).*
