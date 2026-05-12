# ADR-0001 English abstract: Stack choice

> Full ADR: [docs/agent/adr/0001-stack-choice.md](../../agent/adr/0001-stack-choice.md).

## Decision

- **Parsing**: stdlib `tokenize` + `ast` (zero runtime deps)
- **CLI**: stdlib `argparse`
- **Testing**: `pytest` + `hypothesis` (property-based, idempotency validation)
- **Oracle**: `black --line-length 100` (dev dep only)
- **Dep management**: `uv` (same as cs03)

## Why

- Zero-dep CLI = best user experience
- hypothesis is the optimal tool for idempotency invariants
- black is the de facto standard, best-effort oracle

## Status

`accepted` — 2026-05-12.
