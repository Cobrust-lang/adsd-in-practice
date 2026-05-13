---
adr: 0001
title: Stack choice — pure-Rust crypto (sha1/sha2) + flate2 + clap
status: accepted
date: 2026-05-12
case: cs02-mini-git-rust
supersedes: none
last_verified_commit: cd352e6eefdc6cd0af461523e022e11b341c0484
---

# ADR-0001: Stack choice — pure-Rust crypto + flate2 + clap

## Context

CS-02 是 git plumbing 实现。必须做选择:

- **哈希实现**:`sha1` crate(RustCrypto) / `ring`(BoringSSL bindings) / `openssl-sys`
- **zlib**:`flate2`(纯 Rust + zlib-ng 后端) / `libz-sys`(C 绑定)
- **CLI 框架**:`clap` / `argh` / 手写
- **错误模型**:`anyhow + thiserror` / `eyre` / `Box<dyn Error>`

约束:
- **必须能 cross-compile**(用户可能在 windows / arm / wasm 跑)→ 排除 C 绑定
- **必须跟真 git 哈希字节级一致**(否则 oracle 失败)
- **5-day MVP 时间内做完**

## Options Considered

### Option A: RustCrypto + flate2 + clap + anyhow/thiserror(选中)

- **Pros**:
  - 全纯 Rust,cross-compile 零摩擦
  - RustCrypto 是 sha1/sha2/blake3 的标准实现,经过审计
  - flate2 默认 backend 就是 miniz_oxide(纯 Rust),也可切 zlib-ng for perf
  - clap derive 模型干净,跟 Cobrust Studio CLI 风格一致
- **Cons**:
  - sha1 crate 比 OpenSSL 慢 ~30%(对 git 这种大量小 hash 场景影响小)

### Option B: ring + libz-sys + argh

- **Pros**:ring 性能更好;argh 启动快
- **Cons**:**违反 cross-compile 约束**(ring 在 windows-msvc 上多次出 build 问题);argh 生态小

### Option C: openssl-sys + libz-sys + 手写 CLI

- **Pros**:跟 C 生态对接最好
- **Cons**:违反 cross-compile;手写 CLI 是 yak-shaving

## Decision

**选 Option A**。

理由:
1. cross-compile 是硬约束(用户场景多样)
2. RustCrypto sha1 性能对 git 用例够用
3. clap + thiserror 跟 Cobrust Studio 栈对齐,知识复用

## Consequences

### 正面

- cross-compile 到 wasm/arm/windows 零摩擦
- 单 binary 无外部 .so/.dll 依赖

### 负面 / 接受的债

- SHA-1 哈希比 OpenSSL 慢 ~30%,但因为 git 用例每个对象只 hash 一次,且对象通常 < 1MB,影响可忽略
- 后续如果想加 SHA-256 全面替换,RustCrypto 已有,不增成本

### 不可逆性

- 完全可逆。换 sha1 crate 是 cargo replace 一行的事。
- 换 flate2 backend 是 features 切换。

## Done Criteria

- [x] Cargo.toml workspace 声明 sha1/sha2/flate2/clap/anyhow/thiserror
- [x] `mg-core::hash::sha1_hex(b"")` 只验证原始 SHA-1;Git 对象 SHA 必须通过 `mg-core::object::hash(Kind::Blob, payload)` 验证
- [x] `mg hash-object hello.txt`(M1.1 后)跟 `git hash-object hello.txt` 输出一致

## Cross-references

- 参考 cs01-mini-redis-rust ADR-0001 的 stack 选择风格
- 代码:`crates/mg-core/src/hash.rs`, `crates/mg-cli/src/main.rs`

## Notes

如果 v0.2 加 SHA-256 默认,frontmatter `last_verified_commit` 必须更新;当前实现仍以 SHA-1 为唯一 public path,后续升级需要显式设计 hash abstraction 而不是假设它已存在。
