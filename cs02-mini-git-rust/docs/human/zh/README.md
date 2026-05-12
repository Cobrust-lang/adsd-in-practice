# cs02-mini-git-rust(中文用户指南)

## 这是什么

从零实现 git 的 plumbing layer(底层对象 + 索引 + 简化命令),用纯 Rust,跟真 git 字节级兼容(`.mg/` 跟 `.git/` 可互换)。

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

## 支持的命令(M3 后)

**Plumbing**:`hash-object` / `cat-file` / `write-tree` / `commit-tree` / `ls-files`

**Porcelain 子集**:`init` / `add` / `commit -m` / `log`

## ADR 索引

- [ADR-0001 栈选择](./adr-0001-stack-choice.md):纯 Rust crypto + flate2 + clap

## License

Apache-2.0 + MIT。
