# ADR-0001 中文摘要:栈选择

> 完整 ADR 见 [docs/agent/adr/0001-stack-choice.md](../../agent/adr/0001-stack-choice.md)。

## 决策

- **解析**:stdlib `tokenize` + `ast`(零运行时 deps)
- **CLI**:stdlib `argparse`
- **测试**:`pytest` + `hypothesis`(property-based,验证 idempotency)
- **Oracle**:`black --line-length 100`(dev dep)
- **依赖管理**:`uv`(同 cs03)

## 为什么

- CLI 工具零依赖 = 用户体验
- hypothesis 是 idempotency 不变量的最优测试工具
- black 是事实标准,做 best-effort oracle

## 状态

`accepted` — 2026-05-12
