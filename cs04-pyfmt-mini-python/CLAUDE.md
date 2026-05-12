# CS-04 pyfmt-mini-python — Local Agent Constitution

> Local CLAUDE.md。

---

## 1. F24 防御 — 不可简化清单

- ❌ **不准用 regex 实现 quote 替换**——必须用 `tokenize` 模块(否则 `f"hello {x}"` 跟 `'a "b" c'` 这种 case 会爆)
- ❌ **不准用 `str.replace("\t", "    ")` 实现缩进**——必须用 token 层处理(否则字符串字面量里的 tab 会被误改)
- ✅ 允许 trailing whitespace 用 regex(`.rstrip()` 行级别),那只是字符串操作不是结构改写

判断:**如果输入是 `s = "tab\there"` 这种字符串字面量,我的 formatter 应该原样保留字面量内容**。

## 2. Idempotency 不变量(F2 防御)

```python
assert pyfmt(pyfmt(source)) == pyfmt(source)
```

**M3 起,每个 commit 都要在 corpus 上验证 idempotency**。corpus:

- 我们自己的 `src/` 全部 .py
- pytest 的部分公开源码(取 10 个 .py)
- 当前 Python stdlib 5 个 .py(`pathlib.py` / `argparse.py` 等)

测试脚本 `tests/test_idempotency.py`,gate 4 跑它。

## 3. Black oracle(F23-A 防御)

```bash
# 对 corpus 里每个文件:
black --line-length 100 --quiet - < input.py > black_out.py
pyfmt-mini < input.py > our_out.py
diff black_out.py our_out.py
# 期望:diff 内容仅在我们**故意不支持**的规则(line wrap 等)上有差异
```

**故意分歧**列表必须在 README + 测试白名单显式声明,不能"悄悄不一致"。

## 4. 实施顺序

**M1**:
- `rule_trailing_whitespace`:每行 `.rstrip()` + 文件末尾保 1 个 `\n`
- `rule_indent`:tokenize 看 INDENT/DEDENT,统一 4 空格

**M2**:
- `rule_quotes`:tokenize 看 STRING token,单引号 → 双引号(docstring 用 `"""`)

**M3**:
- Idempotency property tests(用 hypothesis 做 fuzzing,**≥1000 inputs**,对齐 ADSD 约束)
- Black oracle 比对脚本

**M4**:release

## 5. 性能 SLO

| 指标 | 目标 |
|---|---|
| 单文件 1k LOC 格式化 | ≤ 50 ms |
| `pyfmt-mini --check src/`(本 repo 整个)| ≤ 500 ms |
| 启动延迟 | ≤ 100 ms |

跟 `black` 速度可比,但**不准用 `black --fast`,要全规则**。

## 6. 5-gate(Python 适配)

同顶层 [`_shared/5-gate-python.sh`](../_shared/5-gate-python.sh)。

---

**End. 其它沿用顶层 CLAUDE.md。**
