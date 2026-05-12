# cs02-mini-git-rust (English user guide)

## What this is

A from-scratch git plumbing layer (object model + index + minimal porcelain commands) in pure Rust. Byte-level compatible with real git — `.mg/` is interchangeable with `.git/`.

## Quick start

```bash
cd cs02-mini-git-rust
bash scripts/bootstrap.sh
cargo install --path crates/mg-cli
mkdir demo && cd demo
mg init
echo "hello" > a.txt
mg add a.txt
mg commit -m "first"
mg log
```

## Supported commands (after M3)

**Plumbing**: `hash-object` / `cat-file` / `write-tree` / `commit-tree` / `ls-files`

**Porcelain subset**: `init` / `add` / `commit -m` / `log`

## ADR index

- [ADR-0001 Stack choice](./adr-0001-stack-choice.md): pure-Rust crypto + flate2 + clap

## License

Apache-2.0 + MIT.
