# Findings — cs04-pyfmt-mini-python

## Ledger

| File | Severity | Status | Related ADR | F-pattern | Date |
|---|---|---|---|---|---|
| (尚无 finding) | | | | | |

## 预期会撞的 F-pattern

- F2 layer divergence — token 层和 ast 层语义不一致(比如 f-string)
- F23-A oracle authorship — black 不完美做 oracle
- F24 primitive-as-everything — 用 str.replace 偷懒不准
- **新候选**:**idempotency 边界 case 爆炸** — hypothesis 找到的反例怎么变成 regression test
