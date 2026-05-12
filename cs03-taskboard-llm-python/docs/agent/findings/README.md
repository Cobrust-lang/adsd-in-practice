# Findings — cs03-taskboard-llm-python

## Ledger

| File | Severity | Status | Related ADR | F-pattern | Date |
|---|---|---|---|---|---|
| (尚无 finding) | | | | | |

## 预期会撞的 F-pattern

- F8 marketing overreach — LLM 自动估时准确度的诚实标注
- F12 thinking-model budget — claude-opus 的 `max_tokens` 配错导致 finish_reason=length
- F14 endpoint silent model swap — `claude-opus-4-5` → `4-7` 没显式锁版本
- F23-A oracle authorship — LLM 输出非确定,怎么写"正确"测试
- **新候选**:F25 *LLM-output non-determinism in oracle assertion* — 完成 v0.1.0 后定型

## 命名

`m{milestone}-{slug}.md`(例:`m3-anthropic-max-tokens-finish-length.md`)
