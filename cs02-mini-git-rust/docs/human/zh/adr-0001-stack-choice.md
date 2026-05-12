# ADR-0001 中文摘要:栈选择

> 完整 ADR 见 [docs/agent/adr/0001-stack-choice.md](../../agent/adr/0001-stack-choice.md)。

## 决策

- **哈希**:RustCrypto (`sha1` / `sha2` crate)
- **zlib**:`flate2`(默认 miniz_oxide backend,纯 Rust)
- **CLI**:`clap` derive
- **错误**:`anyhow` + `thiserror`

## 为什么

- **必须 cross-compile**(到 wasm/arm/windows),所以**全纯 Rust 路线**,排除 C 绑定(`ring` / `openssl-sys` / `libz-sys`)
- 跟 cs01 / Cobrust Studio 栈对齐
- RustCrypto sha1 比 OpenSSL 慢 30% 但对 git 场景够用

## 状态

`accepted` — 2026-05-12
