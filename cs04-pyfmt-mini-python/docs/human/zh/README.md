# cs04-pyfmt-mini-python(中文用户指南)

## 这是什么

极简的 Python 代码格式化器:统一缩进 / 引号 / 行尾。是 `black` 的子集,但**零运行时依赖**,启动 ≤ 100 ms。

## 快速开始

```bash
cd cs04-pyfmt-mini-python
bash scripts/bootstrap.sh
uv run pyfmt-mini --check src/
echo 'x   = 1  ' | uv run pyfmt-mini -
```

## ADR 索引

- [ADR-0001 栈选择](./adr-0001-stack-choice.md):stdlib only + uv + hypothesis + black-as-oracle

## License

Apache-2.0 + MIT。
