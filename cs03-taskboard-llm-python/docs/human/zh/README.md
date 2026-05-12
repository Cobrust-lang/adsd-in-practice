# cs03-taskboard-llm-python(中文用户指南)

## 这是什么

个人任务面板,LLM 自动给 task 加标签 / 估时 / 拆子任务。Python 后端 + SvelteKit 前端。

## 快速开始

```bash
cd cs03-taskboard-llm-python
bash scripts/bootstrap.sh
export ANTHROPIC_API_KEY=sk-ant-...   # 或 OPENAI_API_KEY
uv run uvicorn taskboard.main:app --port 8000
# 另一个终端
cd web && pnpm dev
```

## 功能(M3 后)

- 建 / 改 / 删 task
- "Auto-tag" 按钮:LLM 流式生成 tags + 估时 + 子任务建议
- Server-Sent Events 实时显示 LLM 打字效果

## ADR 索引

- [ADR-0001 栈选择](./adr-0001-stack-choice.md):FastAPI + aiosqlite + SvelteKit + Anthropic/OpenAI SDK + uv

## License

Apache-2.0 + MIT。
