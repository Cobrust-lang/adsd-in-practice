---
finding: <slug>
date: YYYY-MM-DD
case: csXX-<short-name>
severity: P0 | P1 | P2 | P3
specificity: high | medium | candidate
related_adr: <ADR-NNNN or none>
related_f: <ADSD F-pattern e.g. F5, F23-A; or "new">
last_verified_commit: <git SHA or "live">
---

# Finding: <Title>

## Hypothesis(我们最初以为问题是什么)

<1-2 段。**用过去时态写**,记录最初的假说。如果后来发现假说错了,**不要修改这一段**,在 Method/Result/Conclusion 里说明被推翻。>

## Method(怎么验证)

<具体步骤。指令行、commit SHA、测试输入、复现脚本。>

```bash
# 复现命令
...
```

## Result(实测看到了什么)

<具体观察。错误消息全文、性能数字、行为差异。包含日期 + 硬件标签。>

```
<paste raw output here>
```

## Conclusion(根因)

<现在认为根因是什么。如果跟 Hypothesis 不同,**显式标注**:"Hypothesis was wrong; actual root cause: ..."。>

## Fix / Mitigation

- **Fix**:具体 commit SHA / PR link / patch
- **Test added**:具体测试名,确保 regression 不再发生
- **Doc updated**:CLAUDE.md / README / ADR 哪些章节同步改了

## Lessons / F-pattern mapping

- 本 finding 是 ADSD F-pattern 的哪一种?
- 如果不是已有的,是不是 ADSD F-catalog 的候选?
- 改 `METHODOLOGY-STATUS.md` 的相应表格行了吗?

## Notes

(可选)写值得记录的旁观察 / 误导 / 元教训。
