<div align="center">

# CS-02 · mini-git-rust

**git plumbing layer 从零实现 · 纯 Rust 库 + CLI · 无前端**

*ADSD case study #2 — 系统工具 + 对象存储 + diff/merge*

</div>

---

## What this is

实现 git 的 **plumbing layer**(底层对象模型 + 索引 + commit/log repository state)。v0.1.0 同时包含最小 porcelain 子集 `mg init` / `mg add` / `mg commit -m` / `mg log`,但不实现 `git status` / branch / merge / remote 等完整用户体验命令。发布承诺是 **Git-compatible v0.1 子集**,不是 `.mg/` 与 `.git/` 的完整双向可替换实现。

跟 cs01 的对比:
- cs01 = 网络服务 + 前端 + 实时
- cs02 = **本地工具 + 纯库 + 文件 IO + 加密哈希**

故意选**完全不重合**的领域,验证 ADSD 跨领域。

## 范围

### v0.1.0 必须 ship(M4)

- ✅ Blob / Tree / Commit 对象的 SHA-1 序列化(支持 SHA-256 后置升级)
- ✅ Object store:`.mg/objects/aa/bb...` loose objects(不实现 packfile 的 v0.1)
- ✅ Index(staging area):`.mg/index` 文件格式
- ✅ `mg hash-object` / `mg cat-file` / `mg write-tree` / `mg commit-tree`(plumbing 命令)
- ✅ `mg init` / `mg add <path>` / `mg commit -m <msg>` / `mg log`(porcelain 子集,共 8 个已实现命令)
- ✅ 与真 `git` 的 v0.1 子集兼容:支持 loose object / index / tree / commit / first-parent log,并由真实 Git oracle 验证
- ✅ 5 道 ADSD gate green

### Out of scope

- ❌ Packfile / 远程 push/fetch(P1 后续)
- ❌ Diff 算法(P1)
- ❌ Branch / merge / rebase(P1)
- ❌ Submodule / LFS / 部分 clone

## ADSD 触发点

| 决策点 | 预期 ADR |
|---|---|
| 对象哈希算法(SHA-1 vs SHA-256 兼容) | ADR-0002 |
| Index 文件格式(严格兼容 `git-format-index`) | ADR-0003 |
| Repository state / commit / log / discovery | ADR-0004 |
| M4 release hardening 与文档诚实 | ADR-0005 |

**预期会撞**:
- **F23-A** oracle authorship(自己写自己测,必须对照真 `git` 二进制)
- **F2** layer divergence(`hash-object` 跟 `cat-file` 两个命令对同一对象算的哈希不一样)
- **F5** silent miscompile(zlib 流写完没 flush,文件 0 字节但 git 不报错)
- **新 F-pattern 候选**:**文件系统竞态**(并发 `mg add` 在同一 index 上 → 损坏)

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

The bootstrap checks `cargo` and `git`, fetches dependencies, builds the workspace, and runs a smoke test pass.

### Install the CLI locally

```bash
cargo install --path crates/mg-cli
mg --help
```

### Run the full release-verification flow

```bash
bash ../_shared/doc-coverage.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash tests/oracle.sh
```

What each step proves:

- `doc-coverage.sh`: ADR/finding zh-en-agent sync is intact.
- `cargo fmt` / `cargo clippy`: Rust style and lint discipline stay clean.
- `cargo test`: unit/integration behavior stays green.
- `tests/oracle.sh`: real Git can read the supported subset we write, and M4 hardening negative cases reject unsafe paths and malformed local state.

### Manual smoke flow

```bash
mkdir -p /tmp/cs02-demo && cd /tmp/cs02-demo
mg init
echo "hello" > a.txt
mg add a.txt
mg commit -m "first"
mg log
```

## Architecture

```
┌──────────────────────────────────┐
│       mg-cli (clap-based)        │
│       binary: `mg`               │
└────────────┬─────────────────────┘
             │
   ┌─────────▼──────────┐
   │      mg-core       │
   │  - object::*       │  (Blob/Tree/Commit/Tag)
   │  - store::loose    │  (.mg/objects/aa/bb)
   │  - index::*        │  (.mg/index reader/writer)
   │  - repo::discover  │  (find .mg/ upward)
   │  - hash::*         │  (SHA-1 + SHA-256 abstraction)
   └────────────────────┘
```

## Status

- ✅ M0 scaffold
- ✅ M1 object model + `hash-object` / `cat-file`: Git-compatible blob identity, zlib loose-object IO, minimal `.mg/objects` init, and real Git oracle with 1000 randomized blobs
- ✅ M2 index + `add` / `write-tree`: Git index v2 + canonical tree compatibility per ADR-0003
- ✅ M3 commit + log + repo discovery: upward `.mg` discovery, recursive regular-file add/write-tree, Git-compatible `commit-tree`, `commit -m`, and first-parent `log` per ADR-0004
- ✅ M4 release hardening: ADR-0005 hardening landed with atomic repository-owned writes, index lock cleanup, allocation caps, symlink-path rejection, and expanded oracle negatives for the Git-compatible v0.1 subset
- ✅ METHODOLOGY-STATUS: cs02 methodology conclusions are recorded at repo top level

## License

Apache-2.0 + MIT,同顶层 repo。
