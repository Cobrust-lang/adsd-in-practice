<div align="center">

# CS-02 · mini-git-rust

**git plumbing layer 从零实现 · 纯 Rust 库 + CLI · 无前端**

*ADSD case study #2 — 系统工具 + 对象存储 + diff/merge*

</div>

---

## What this is

实现 git 的 **plumbing layer**(底层对象模型 + 索引 + 简化的 push/fetch over HTTP),**不做 porcelain**(不实现 `git status` 这种用户体验命令)。

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
- ✅ `mg init` / `mg add <path>` / `mg commit -m <msg>` / `mg log`(porcelain 子集,核心 7 命令)
- ✅ 与真 `git` 互通:**我们的 `.mg/` 跟 `.git/` 完全兼容**(对照 oracle)
- ✅ 5 道 ADSD gate green

### Out of scope

- ❌ Packfile / 远程 push/fetch(P1 后续)
- ❌ Diff 算法(P1)
- ❌ Branch / merge / rebase(P1)
- ❌ Submodule / LFS / 部分 clone

## ADSD 触发点

| 决策点 | 预期 ADR |
|---|---|
| 对象哈希算法(SHA-1 vs SHA-256 兼容)| ADR-0002 |
| Loose object 压缩(zlib vs zstd)| ADR-0003 |
| Index 文件格式(自陈格式 vs `git-format-index` 严格兼容)| ADR-0004 |
| Worktree → index → HEAD 状态机的实现 | ADR-0005 |
| Repository discovery(从 cwd 向上找 `.mg/`)| ADR-0006 |

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

- 🚧 M0 scaffold
- ⬜ M1 object model + hash-object/cat-file
- ⬜ M2 index + add/write-tree
- ⬜ M3 commit + log + repo discovery
- ⬜ M4 v0.1.0 release + METHODOLOGY-STATUS

## License

Apache-2.0 + MIT,同顶层 repo。
