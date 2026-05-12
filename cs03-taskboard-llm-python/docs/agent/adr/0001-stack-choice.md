---
adr: 0001
title: Stack choice — FastAPI + aiosqlite + SvelteKit + Anthropic/OpenAI router
status: accepted
date: 2026-05-12
case: cs03-taskboard-llm-python
supersedes: none
last_verified_commit: pending
---

# ADR-0001: Stack choice

## Context

CS-03 是个 Python 后端 + 前端 + LLM 应用,目标是验证 ADSD 在 Python 生态下的有效性。需要选择:

- **Web 框架**:FastAPI / Flask / aiohttp / Starlette 裸
- **数据库 access**:`aiosqlite` / `sqlalchemy async` / `databases`
- **migration**:`alembic` / 手写 SQL
- **LLM client**:`anthropic` SDK / `openai` SDK / 自写 httpx
- **前端**:SvelteKit(对齐 Studio) / Next.js / Vue
- **依赖管理**:`uv` / `poetry` / `pip + venv`
- **Type check**:`mypy strict` / `pyright`

## Options Considered

### Option A: FastAPI + aiosqlite + alembic + anthropic/openai + SvelteKit + uv + mypy(选中)

- **Pros**:
  - FastAPI 是 Python async web 框架的事实标准,文档生成 + pydantic 集成是行业标杆
  - `uv` 是 2024+ Python 包管理新标准(Rust 写的,比 pip/poetry 快 10-100×)
  - SvelteKit 跟 Cobrust Studio 栈对齐
  - mypy strict 是社区认可的最严格 type check
- **Cons**:
  - `uv` 还很新(部分企业环境未必接受),但 dev/CI 友好
  - mypy strict 在 FastAPI 上偶尔需要 `cast()`(可接受)

### Option B: Flask + sync sqlite + manual migration + raw httpx

- **Pros**:Flask 久经考验,sync 模型简单
- **Cons**:**违反 async-first**(LLM 调用是 IO-heavy 的典型 async 场景);手写 migration 是 yak-shaving

### Option C: Starlette 裸 + sqlalchemy async + Vue + pyright

- **Pros**:Starlette 更轻,Vue 学习曲线低
- **Cons**:Starlette 裸要自己接 pydantic / openapi(yak-shaving);Vue 跟 Studio SvelteKit 不对齐(违反"跨项目栈复用")

## Decision

**选 Option A**。

理由:
1. FastAPI + pydantic 是 LLM 应用的事实组合(schema validation 内置)
2. `uv` 速度优势在 CI / sub-agent 反馈循环上 ROI 极高(对齐 ADSD "fast feedback" 原则)
3. SvelteKit 跟 Studio + cs01 对齐,跨 case 知识复用
4. mypy strict 跟 Rust clippy `-D warnings` 严格等级对齐(5-gate 跨语言一致性)

## Consequences

### 正面

- 跟 Studio + cs01 栈对齐
- async-first,LLM 调用零摩擦
- `uv` 在 sub-agent 反馈循环上提速 10×+

### 负面 / 接受的债

- `uv` 学习曲线(开发者要适应 lockfile 行为)
- mypy strict 偶尔需要 `cast()`,接受

### 不可逆性

- 中等可逆。换 FastAPI → Starlette 是 router 重写(2-3 天)。
- 换 `uv` → `pip`/`poetry` 是一行 Makefile 改动。

## Done Criteria

- [ ] pyproject.toml 声明所有依赖
- [ ] `uv sync` 一键装齐
- [ ] `mypy --strict src/` 0 errors
- [ ] FastAPI app 起来,`GET /healthz` 返回 200
- [ ] llm_router stub 在 no-key env 下抛 `LLMError`

## Cross-references

- 参考 Cobrust Studio ADR-0001 stack choice
- 参考 cs01-mini-redis-rust ADR-0001 的"跟 Studio 对齐"理由
- 代码:`pyproject.toml`, `src/taskboard/main.py`, `src/taskboard/llm_router.py`

## Notes

- LLM client 选 Anthropic + OpenAI SDK 两个,而不是裸 httpx,因为 SDK 处理 SSE / token 计数 / retry 比裸 httpx 省事。代价是 SDK 偶尔不同步最新模型名(F14 风险)。
- pydantic 升级到 2.x 会有 breaking,锁 `pydantic>=2.9` 已对齐 FastAPI 0.115。
