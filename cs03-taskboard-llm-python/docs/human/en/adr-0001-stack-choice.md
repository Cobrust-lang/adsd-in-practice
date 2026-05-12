# ADR-0001 English abstract: Stack choice

> Full ADR: [docs/agent/adr/0001-stack-choice.md](../../agent/adr/0001-stack-choice.md).

## Decision

- **Backend**: `FastAPI` + `aiosqlite` + `alembic`
- **LLM client**: `anthropic` SDK + `openai` SDK (both installed, runtime picks via env)
- **Frontend**: `SvelteKit` (aligned with Studio + cs01)
- **Dep management**: `uv` (Rust-written, 10-100× faster than pip/poetry)
- **Type check**: `mypy --strict` (matches Rust clippy strictness)

## Why

- FastAPI + pydantic is the de facto combo for LLM apps (built-in schema validation)
- `uv` dramatically speeds up sub-agent feedback loops — concrete ADSD "fast feedback" principle
- SvelteKit aligns with Studio + cs01 for cross-case knowledge reuse

## Status

`accepted` — 2026-05-12.
