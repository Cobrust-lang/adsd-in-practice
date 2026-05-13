# cs02-mini-git-rust (English user guide)

## What this is

A from-scratch git plumbing layer (object model + index + minimal porcelain commands) in pure Rust. The v0.1 supported subset is Git-compatible: loose objects, index, trees, commits, and first-parent logs are verified against the real Git oracle. This is not a claim that `.mg/` and `.git/` are fully interchangeable outside that supported subset.

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

## Build and verify

### Bootstrap the toolchain

```bash
cd cs02-mini-git-rust
bash scripts/bootstrap.sh
```

### Install the CLI locally

```bash
cargo install --path crates/mg-cli
mg --help
```

### Run the full verification flow

```bash
bash ../_shared/doc-coverage.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash tests/oracle.sh
```

This verifies docs sync, Rust gates, and the real Git oracle including the M4 hardening negative cases.

### Manual smoke flow

```bash
mkdir -p /tmp/cs02-demo && cd /tmp/cs02-demo
mg init
echo "hello" > a.txt
mg add a.txt
mg commit -m "first"
mg log
```

## Supported commands (M4 hardened v0.1 subset)

**Plumbing**: `hash-object` / `cat-file` / `write-tree` / `commit-tree`

**Porcelain subset**: `init` / `add` / `commit -m` / `log`

## ADR index

- [ADR-0001 Stack choice](./adr-0001-stack-choice.md): pure-Rust crypto + flate2 + clap
- [ADR-0002 Object identity and loose object store](./adr-0002-object-identity-loose-store.md): Git-compatible blob IDs and zlib loose objects
- [ADR-0003 Index v2 and canonical tree compatibility](./adr-0003-index-tree-compatibility.md): Git index/tree compatibility
- [ADR-0004 Repository state, commits, and log](./adr-0004-repository-state-commits-log.md): minimal repository state and first-parent log
- [ADR-0005 M4 release filesystem hardening](./adr-0005-release-filesystem-hardening.md): filesystem hardening and documentation honesty

## Finding abstracts

- [M4 pre-release filesystem hardening](./finding-m4-pre-release-filesystem-hardening.md)

## License

Apache-2.0 + MIT.
