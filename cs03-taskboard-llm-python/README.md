<div align="center">

# CS-03 · taskboard-llm-python

**个人任务面板 · FastAPI + SvelteKit + LLM 自动标注**

*ADSD case study #3 — Python 生态 + Web 应用 + LLM 集成 + SSE*

</div>

---

## What this is

一个**个人 task board**:你建 task,LLM 自动给它**加标签 / 估时 / 拆子任务**。FastAPI 后端 + SvelteKit 前端 + SQLite + Anthropic/OpenAI router(借鉴 Cobrust Studio 的 `studio-router`)。

**测什么**:ADSD 在 **Python 生态**(没有 Cargo workspace lock,有 GIL,异步模型不同)和 **LLM 集成场景**(Studio 的核心商业场景)下是否仍然成立。

## 范围

### v0.1.0 必须 ship(M4)

- ✅ FastAPI 后端 + SQLite + Alembic migration
- ✅ Task CRUD:`POST /api/tasks` / `GET /api/tasks` / `PATCH /api/tasks/:id` / `DELETE /api/tasks/:id`
- ✅ LLM 自动行动:`POST /api/tasks/:id/auto-tag`(SSE 流回响应)
  - 自动标签(3-5 个 tags)
  - 估时(分钟数)
  - 拆子任务(0-5 个 subtask description)
- ✅ LLM router(Anthropic + OpenAI-compatible)+ env-key 鉴权
- ✅ SvelteKit UI:list / detail / 拖拽排序 / 自动标注按钮 + SSE 流式动画
- ✅ 5 道 ADSD gate green(Python 适配)

### Out of scope(0.1.0 不做)

- ❌ 多用户 / RBAC
- ❌ 协同编辑 / 实时同步
- ❌ 移动端
- ❌ 历史/审计 trail
- ❌ Embedding / 检索(P1)

## ADSD 触发点(预期)

| 决策点 | 预期 ADR |
|---|---|
| 同步 vs 异步 FastAPI(`def` vs `async def` 边界)| ADR-0002 |
| LLM router 设计(参考 Studio,统一 schema 还是各家自己) | ADR-0003 |
| SSE 流式返回:服务端 protocol(`text/event-stream` vs `application/json-stream`) | ADR-0004 |
| SQLite 在 async 下的访问(`aiosqlite` vs sync wrap) | ADR-0005 |
| API key 存哪:env / config file / DB(加密) | ADR-0006 |
| Python 测试 + Type check 的 5-gate 适配(pyright vs mypy) | ADR-0007 |

**预期会撞**:
- **F12** thinking-model budget 配置陷阱(Anthropic claude-opus 的 `max_tokens`)
- **F14** endpoint silent model swap(`claude-opus-4-5` → `claude-opus-4-7`)
- **F8** marketing(LLM "自动估时"准确度的诚实标注 — 不能吹"AI smart")
- **F23-A** oracle(LLM 输出非确定,怎么测?)→ 这会撞出新 F-pattern
- **新 F-pattern 候选**:**非确定 LLM 输出下的 ADSD 测试纪律**

## Quick start

```bash
cd cs03-taskboard-llm-python
bash scripts/bootstrap.sh
# 起服务
uv run uvicorn taskboard.main:app --reload --port 8000
# 另一个终端起前端
cd web && pnpm dev
```

需要 env:
```bash
export ANTHROPIC_API_KEY=sk-ant-...
# 或
export OPENAI_API_KEY=sk-...
```

## Architecture

```
┌─────────────────────────────────────┐
│  SvelteKit (Vite dev or static build)│
└────────────────┬────────────────────┘
                 │ REST + SSE
       ┌─────────▼──────────┐
       │   FastAPI (uvicorn) │
       │   /api/tasks (CRUD) │
       │   /api/tasks/:id/   │
       │     auto-tag (SSE)  │
       └────┬────────────┬───┘
            │            │
   ┌────────▼──┐  ┌─────▼──────────┐
   │ aiosqlite │  │ llm_router      │
   │ SQLite    │  │ - anthropic     │
   │           │  │ - openai-compat │
   └───────────┘  └─────────────────┘
```

## Status

- 🚧 M0 scaffold
- ⬜ M1 backend MVP — FastAPI + SQLite + 5 routes
- ⬜ M2 frontend MVP — SvelteKit + list + detail + auto-tag UI
- ⬜ M3 LLM router lift + SSE 流式打字机效果
- ⬜ M4 v0.1.0 release + METHODOLOGY-STATUS

## License

Apache-2.0 + MIT,同顶层 repo。
