# Findings — cs01-mini-redis-rust

> Negative result ledger。任何"我以为 X,结果是 Y"都进这里。
>
> 用 `_shared/finding-template.md` 起一份新 finding,落在本目录 `<milestone>-<slug>.md`。

## Ledger

| File | Severity | Status | Related ADR | F-pattern | Date |
|---|---|---|---|---|---|
| [m1-1-p9-missed-shared-doc-coverage](m1-1-p9-missed-shared-doc-coverage.md) | P2 | closed by mitigation | — | F17.x(新 sub-case 候选) | 2026-05-12 |
| [m1-3-cto-wrote-code-instead-of-dispatching](m1-3-cto-wrote-code-instead-of-dispatching.md) | P1 | fix in progress (P9 redispatch) | 0005 | new candidate: CTO-as-implementer (F18 sub-pattern) | 2026-05-12 |

## 命名规范

- `m1-resp-parser-incomplete-frame.md`(M1 阶段,RESP parser 边界 case)
- `m2-sse-flush-buffering.md`(M2 阶段,SSE 缓冲行为问题)
- `m3-aof-replay-utf8.md`(M3 阶段,AOF 重播时 UTF-8 校验)

## 何时写 finding

1. Bug 修了 → 写 finding 总结 root cause + lesson
2. 性能数字跟预期差 > 2× → 写 finding
3. 测试不稳(flaky)→ 写 finding,记录原因
4. 任何"被某事卡了 30 分钟以上"的事 → 写 finding

## 何时**不**写 finding

- Typo / 格式问题
- 直接的 build error,搜一下就能修
- Sub-agent KPI 自报问题(那是 F17,不是 finding)
