---
adr: 0001
title: Stack choice — stdlib only + uv + hypothesis + black-as-oracle
status: accepted
date: 2026-05-12
case: cs04-pyfmt-mini-python
supersedes: none
last_verified_commit: pending
---

# ADR-0001: Stack choice

## Context

CS-04 是个小型 CLI 工具(<2k LOC 目标),做 Python 代码格式化的子集。需要选:

- **解析层**:`tokenize` / `ast` / 自写词法
- **运行时依赖**:零依赖 / 用 `libcst` 等高级 AST 库
- **测试**:pytest + hypothesis property-based / 仅 pytest 单测
- **Oracle**:`black` / 无 oracle / 多 formatter 投票
- **CLI**:argparse / click / typer

**约束**:小项目,**ADSD 不该是 over-engineering**,但又要展示 ADR/finding 的开销/收益比。

## Options Considered

### Option A: stdlib only + uv + hypothesis + black-as-oracle(选中)

- **Pros**:
  - **零运行时 deps** — 用户不用 pip 装一堆东西,`pip install pyfmt-mini` 即用
  - stdlib `tokenize` + `ast` 足够本 case 规模
  - hypothesis 是 Python property-based testing 标准,对 idempotency 测试是完美工具
  - `black` 是 oracle 的合理选择(它是行业事实标准)
- **Cons**:
  - stdlib `tokenize` 偶尔在新 Python 语法上滞后(3.12+ 的 type parameter 等)
  - `black` 也不是 ground truth,有自身偏好(可接受作 best-effort oracle)

### Option B: libcst + click + 不用 hypothesis

- **Pros**:`libcst` 比 stdlib `ast` 强很多(保留 whitespace / comment)
- **Cons**:**违反"零运行时 deps"** + libcst 启动慢(0.5s+)严重影响 CLI 体验

### Option C: 自写 lexer + 无 oracle

- **Pros**:最纯净
- **Cons**:**严重 yak-shaving**;无 oracle 意味着 F23-A 直接命中(自己写自己测,没参考)

## Decision

**选 Option A**。

理由:
1. 零运行时 deps 是 CLI 工具的核心 UX(`pip install` 不能挂掉别人的环境)
2. hypothesis 是验证 idempotency 不变量的天然工具(ADSD 强约束:idempotent guarantee)
3. `black` 做 oracle 是务实选择,差异处可显式声明

## Consequences

### 正面

- pip install 30s 内完成,无 C 编译
- hypothesis property test 直接验证不变量,不靠手编 corpus
- 跟 ruff/black 用户社区无缝

### 负面 / 接受的债

- 3.12+ 新语法支持要追 `tokenize` 升级
- `black --line-length 100` 的偏好我们继承(可接受)

### 不可逆性

- 完全可逆。换 `libcst` 是 1 行 import 改动 + 一些 API 调整。

## Done Criteria

- [ ] `pyproject.toml` 主依赖 list 为空 `[]`,仅 dev deps 含 pytest / hypothesis / black
- [ ] `pyfmt-mini --version` 启动 ≤ 100 ms
- [ ] `tests/test_idempotency.py` 用 hypothesis 生成 ≥ 1000 inputs
- [ ] `tests/test_black_oracle.sh` 对 corpus 跟 black 比对,差异在白名单内

## Cross-references

- 参考 cs03 ADR-0001 的 `uv` 选择(本 case 沿用)
- 代码:`pyproject.toml`, `src/pyfmt_mini/rules.py`

## Notes

- 如果 v0.2 要加 line-wrap rule(black 核心),可能需要换 libcst,届时重新审视本 ADR。
