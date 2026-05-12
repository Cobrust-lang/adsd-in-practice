# ADR-0001 中文摘要:栈选择

> 完整 ADR 见 [docs/agent/adr/0001-stack-choice.md](../../agent/adr/0001-stack-choice.md)。

## 决策

- **后端**:`FastAPI` + `aiosqlite` + `alembic`
- **LLM client**:`anthropic` SDK + `openai` SDK(都装,运行时按 env 选)
- **前端**:`SvelteKit`(跟 Studio + cs01 对齐)
- **依赖管理**:`uv`(Rust 写的,比 pip/poetry 快 10-100×)
- **Type check**:`mypy --strict`(对齐 Rust clippy 严格度)

## 为什么

- FastAPI + pydantic 是 LLM 应用事实组合(schema validation)
- `uv` 在 sub-agent 反馈循环上提速极大,ADSD "fast feedback" 原则的具体落地
- SvelteKit 跟 Studio + cs01 对齐,跨 case 知识复用

## 状态

`accepted` — 2026-05-12
