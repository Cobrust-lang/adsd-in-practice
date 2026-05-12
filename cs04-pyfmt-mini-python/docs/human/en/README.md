# cs04-pyfmt-mini-python (English user guide)

## What this is

A minimal Python code formatter: unifies indentation, quotes, trailing whitespace. A subset of `black`, but **zero runtime dependencies** and startup ≤ 100 ms.

## Quick start

```bash
cd cs04-pyfmt-mini-python
bash scripts/bootstrap.sh
uv run pyfmt-mini --check src/
echo 'x   = 1  ' | uv run pyfmt-mini -
```

## ADR index

- [ADR-0001 Stack choice](./adr-0001-stack-choice.md): stdlib only + uv + hypothesis + black-as-oracle

## License

Apache-2.0 + MIT.
