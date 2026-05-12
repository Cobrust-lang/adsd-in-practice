# cs03-taskboard-llm-python (English user guide)

## What this is

Personal task board where the LLM auto-tags, estimates duration, and suggests subtasks. Python backend + SvelteKit frontend.

## Quick start

```bash
cd cs03-taskboard-llm-python
bash scripts/bootstrap.sh
export ANTHROPIC_API_KEY=sk-ant-...   # or OPENAI_API_KEY
uv run uvicorn taskboard.main:app --port 8000
# in another terminal
cd web && pnpm dev
```

## Features (after M3)

- Create / edit / delete tasks
- "Auto-tag" button → LLM streams tags + estimated minutes + subtask suggestions
- Server-Sent Events for live typewriter effect

## ADR index

- [ADR-0001 Stack choice](./adr-0001-stack-choice.md): FastAPI + aiosqlite + SvelteKit + Anthropic/OpenAI SDK + uv

## License

Apache-2.0 + MIT.
