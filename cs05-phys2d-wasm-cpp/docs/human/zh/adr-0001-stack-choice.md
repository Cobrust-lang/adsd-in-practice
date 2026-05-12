# ADR-0001 中文摘要:栈选择

> 完整 ADR 见 [docs/agent/adr/0001-stack-choice.md](../../agent/adr/0001-stack-choice.md)。

## 决策

- **C++**:`C++17`(编译器支持最广)
- **Build**:`CMake`(native + wasm 双 target)
- **Wasm**:`Emscripten`(`emcmake cmake`),手写 C ABI(不用 `emscripten::bind`)
- **前端**:`SvelteKit`(对齐 cs01/cs03/Studio)
- **C++ 测试**:`GoogleTest`(via FetchContent)

## 为什么

- C++17 + CMake + Emscripten 是 wasm-from-C++ 事实标准
- **手写 C ABI** 比 bind 易审计,struct layout 显式锁,符合 ADSD F24 防御
- SvelteKit 跨 case 知识复用

## 状态

`accepted` — 2026-05-12
