<div align="center">

# CS-04 · pyfmt-mini-python

**极简 Python 代码格式化器 · CLI 工具 · 无前端**

*ADSD case study #4 — 小型工具型项目 + AST 操作 + 跟 black 做 oracle 对照*

</div>

---

## What this is

一个**极简的 Python 代码格式化器**(类 `black` 的子集),只做三件事:

1. **缩进统一**(4 空格;tab → 4 空格)
2. **引号统一**(默认双引号,docstring 三双引号)
3. **行尾空格 + 末尾换行**

**故意**做得小(<2k LOC 目标),验证 ADSD 在**小型工具型项目**上的开销/收益比 — ADR / finding / 双语 doc 在这种规模下是不是 over-engineering?

## 范围

### v0.1.0 必须 ship(M4)

- ✅ `pyfmt-mini <file>` 把单文件格式化(in-place 或 stdout)
- ✅ `pyfmt-mini --check <file>` 不修改,只返回退出码 0/1
- ✅ `pyfmt-mini --diff <file>` 打印 unified diff
- ✅ Idempotent guarantee:`pyfmt(pyfmt(x)) == pyfmt(x)`(formatter 的核心不变量)
- ✅ 跟 `black --line-length 100 --quiet -` 在我们支持的子集上输出一致(F23-A oracle)
- ✅ 5 道 ADSD gate green

### Out of scope(0.1.0 不做)

- ❌ 行长度切分(black 的核心难点,我们暂不做)
- ❌ import 排序(那是 isort/ruff 的活)
- ❌ 复杂 AST 重写(单行 if → if-else 之类)

## ADSD 触发点

| 决策点 | 预期 ADR |
|---|---|
| Token 层 vs AST 层处理(F2 layer divergence 直接撞)| ADR-0002 |
| Idempotency 怎么测(fixed point search vs 数学证明)| ADR-0003 |
| 跟 black 兼容多少(全兼容子集 vs 主动分歧)| ADR-0004 |

**预期会撞**:
- **F2** layer divergence(`tokenize` 看到的引号 vs `ast` 看到的字符串字面量,两层语义不一致)
- **F23-A** oracle(black 也不能完全信,有些 case 它的输出是有争议的)
- **F24** primitive-as-everything 风险:用单纯字符串替换"模拟"AST 重写 → 必须避免
- **新 F-pattern 候选**:**Idempotency 测试的边界 case 爆炸**(测多少次才算"稳定"?)

## Quick start

```bash
cd cs04-pyfmt-mini-python
bash scripts/bootstrap.sh
uv run pyfmt-mini --check src/
echo 'x = 1' | uv run pyfmt-mini -
```

## Architecture

```
┌────────────────────────────────────┐
│         pyfmt-mini (CLI)           │
│   src/pyfmt_mini/__main__.py       │
└─────────────────┬──────────────────┘
                  │
       ┌──────────▼───────────┐
       │  pyfmt_mini.rules    │
       │  - rule_indent       │
       │  - rule_quotes       │
       │  - rule_trailing     │
       └──────────────────────┘
```

无 lib dep 外部生态(只 stdlib `tokenize` + `ast`)。

## Status

- 🚧 M0 scaffold
- ⬜ M1 rule_indent + rule_trailing(简单的)
- ⬜ M2 rule_quotes(`tokenize` 层)
- ⬜ M3 idempotency 测试 + black oracle 对照
- ⬜ M4 v0.1.0 release + METHODOLOGY-STATUS

## License

Apache-2.0 + MIT。
