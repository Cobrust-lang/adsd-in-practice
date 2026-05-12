# Finding M1.1: P9 漏查 shared doc-coverage

## 摘要

M1.1 中 P9 implementation agent 曾报告“没有 doc-coverage 脚本”,但实际仓库根目录 `_shared/doc-coverage.sh` 已存在。问题不是脚本缺失,而是 sub-agent 只在 case-local 路径搜索,没有检查顶层 shared tooling。

## 影响

这暴露了 ADSD F17 sub-agent KPI/self-report fidelity 风险:agent 的“已检查”不能等于事实。P10/P9 守闸必须重新运行 5-gate,不能只采信完成报告。

## 修复 / 经验

- 记忆中固化:从 case dir 调用 `bash ../_shared/doc-coverage.sh`。
- P9/P10 守闸必须亲自跑 doc-coverage。
- M4.2 进一步扩展 `_shared/doc-coverage.sh`,把 finding 的 zh/en mirror 也纳入强制检查。

## Cross-references

- Agent finding: [`../../agent/findings/m1-1-p9-missed-shared-doc-coverage.md`](../../agent/findings/m1-1-p9-missed-shared-doc-coverage.md)
- Shared gate: [`../../../../_shared/doc-coverage.sh`](../../../../_shared/doc-coverage.sh)
