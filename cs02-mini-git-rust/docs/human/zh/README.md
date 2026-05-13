# cs02-mini-git-rust(中文用户指南)

## 这是什么

从零实现 git 的 plumbing layer(底层对象 + 索引 + 简化命令),用纯 Rust 实现 v0.1 支持子集的 Git 兼容性:loose object / index / tree / commit / first-parent log 可由真 git oracle 验证。这里不是宣称 `.mg/` 与 `.git/` 在支持子集之外可完全互换。

## 快速开始

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

## 支持的命令(M4 hardened v0.1 子集)

**Plumbing**:`hash-object` / `cat-file` / `write-tree` / `commit-tree`

**Porcelain 子集**:`init` / `add` / `commit -m` / `log`

## ADR 索引

- [ADR-0001 栈选择](./adr-0001-stack-choice.md):纯 Rust crypto + flate2 + clap
- [ADR-0002 Object identity and loose object store](./adr-0002-object-identity-loose-store.md):Git-compatible blob ID 与 zlib loose object
- [ADR-0003 Index v2 and canonical tree compatibility](./adr-0003-index-tree-compatibility.md):Git index/tree 兼容性
- [ADR-0004 Repository state、commit 与 log](./adr-0004-repository-state-commits-log.md):最小仓库状态与 first-parent log
- [ADR-0005 M4 release filesystem hardening](./adr-0005-release-filesystem-hardening.md):filesystem hardening 与文档诚实

## Finding 摘要

- [M4 pre-release filesystem hardening](./finding-m4-pre-release-filesystem-hardening.md)

## License

Apache-2.0 + MIT。
