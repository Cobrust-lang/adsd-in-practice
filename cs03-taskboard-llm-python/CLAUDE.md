# CS-03 taskboard-llm-python — Local Agent Constitution

> Local CLAUDE.md。覆盖顶层 [`/CLAUDE.md`](../CLAUDE.md) 的 case-specific 规则。

---

## 1. 本 case 不可简化的核心约束(F24 防御)

- ❌ **不准在 LLM auto-tag 端到端测试里直接 mock LLM 响应**(那是 F24)。真测必须用一个**固定 fixture prompt + 真 LLM 调用 + 用 LLM-as-judge 评估输出是否结构合规**。
- ❌ **不准在生产代码里用 sync sqlite + asyncio.to_thread 模拟 async**——必须用 `aiosqlite`(否则违反 async-first 决策)。
- ❌ **不准把 API key 硬编码 / 存明文 config file**(M3+ 一律 env / Vault)。
- ✅ 允许在单测里 mock `httpx.AsyncClient` 替代真 LLM 调用(单测和 e2e 分层)。

## 2. 本 case 的 oracle(F23-A 防御)

**两套 oracle**:

1. **结构 oracle**(CI 强制):
   ```python
   # LLM 必须返回 {tags: list[str], estimate_minutes: int, subtasks: list[str]}
   # 用 pydantic 校验结构,违反就 retry 一次
   from taskboard.llm_router import call_with_schema
   result = await call_with_schema(
       prompt=task_description,
       schema=AutoTagOutput,  # pydantic model
       max_retries=1,
   )
   ```
   这是**确定性**oracle:LLM 输出 schema 合规,就过 gate。

2. **质量 oracle**(M3+ 跑,不强制阻塞):
   - 用另一个 LLM(对比 model)做 judge:"task 是 X,LLM 标签是 Y,你打 1-5 分"
   - 平均分 ≥ 3.5 = ok
   - 这是**非确定性**oracle,**用统计而不是相等性**判断

**新 F-pattern 候选**:F25 *LLM-output non-determinism in oracle assertion*。完成 v0.1.0 后写到 METHODOLOGY-STATUS。

## 3. Python 5-gate 跟 Rust 5-gate 的差异

| Gate | Rust | Python(本 case)|
|---|---|---|
| 1 fmt | `cargo fmt --check` | `ruff format --check` |
| 2 lint | `cargo clippy -D warnings` | `ruff check` + `mypy --strict src/` |
| 3 build | `cargo build --locked` | `python -c "import taskboard"` |
| 4 test | `cargo test --locked` | `pytest --cov=src --cov-fail-under=80` |
| 5 doc-coverage | `_shared/doc-coverage.sh` | 同 |

**关注点**:Python 没有 Cargo workspace lock,**5-gate 的 F10 cargo-lock-contention 在本 case 消失**,**会被什么新 F-pattern 取代?**——这是本 case 的核心观察点之一。

## 4. 实施顺序

**Wave M1**(FastAPI + SQLite):
1. `POST/GET/PATCH/DELETE /api/tasks`
2. SQLite 用 aiosqlite + alembic
3. 基础 pytest 单测

**Wave M2**(SvelteKit UI):
4. login(env API key 启动时读)→ task list → task detail
5. Playwright e2e:login → 建 task → 改 task → 删 task

**Wave M3**(LLM 集成):
6. llm_router(Anthropic + OpenAI-compat,从 cobrust-studio/studio-router 借鉴 schema)
7. `/api/tasks/:id/auto-tag` SSE 端点
8. UI:auto-tag 按钮 + 流式打字机效果

**Wave M4**:release + status

## 5. 性能 SLO

| 指标 | 目标 | 测法 |
|---|---|---|
| `POST /api/tasks` p99 | ≤ 50 ms | locust |
| `GET /api/tasks` (100 条) p99 | ≤ 100 ms | locust |
| LLM auto-tag 首字节延迟(TTFB)| ≤ 1.5 s | curl trace |
| LLM auto-tag 总耗时 | ≤ 8 s(claude-haiku) | curl trace |
| 启动到 ready | ≤ 1 s | wallclock |
| 内存(空 DB) | ≤ 80 MiB RSS | ps |

## 6. 双语 doc + 代码注释

同 cs01。Python 代码 docstring 用英文(PEP 257);中文写在 `docs/human/zh/`。

---

**End. 其它沿用顶层 CLAUDE.md。**
