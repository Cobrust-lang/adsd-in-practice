---
adr: 0001
title: Stack choice — C++17 + CMake + Emscripten + SvelteKit/Vite
status: accepted
date: 2026-05-12
case: cs05-phys2d-wasm-cpp
supersedes: none
last_verified_commit: pending
---

# ADR-0001: Stack choice

## Context

CS-05 是 C++ 物理引擎 + wasm + 前端可视化。要选:

- **C++ 版本**:C++17 / C++20 / C++23
- **Build system**:CMake / Meson / Bazel
- **Wasm 编译器**:Emscripten / wasi-sdk / Clang wasm32
- **Wasm 绑定层**:`emscripten::bind` (C++ glue) / 手写 C ABI extern "C"
- **前端**:SvelteKit / 纯 vite-app / React + Vite
- **C++ 测试框架**:GoogleTest / Catch2 / doctest

## Options Considered

### Option A: C++17 + CMake + Emscripten + 手写 C ABI + SvelteKit + GoogleTest(选中)

- **Pros**:
  - C++17 编译器支持最广(老 emsdk 都支持)
  - CMake 是 wasm + native 双 target 最成熟的选择
  - **手写 C ABI** 比 `emscripten::bind` 更可控,struct layout 显式锁,**符合 F24 防御**(F24 是 primitive-as-everything,bind 会引入隐藏复杂度)
  - SvelteKit 跟 cs01/cs03/Studio 对齐
  - GoogleTest 在 wasm 测试上也能跑(FetchContent)
- **Cons**:
  - 手写 C ABI 啰嗦,但 layout 显式可审计
  - C++17 vs 20 损失了 `concepts` 和 `ranges`(可接受,本 case 用不上)

### Option B: C++20 + Bazel + wasi-sdk + emscripten::bind + React

- **Pros**:C++20 现代化;Bazel scalable
- **Cons**:Bazel 学习曲线劝退个人项目;wasi-sdk 缺 DOM 绑定;`emscripten::bind` 隐藏 layout(F24 风险);React 跟其他 case 不对齐

### Option C: Rust + wasm-bindgen 改写整个 case

- **Pros**:更安全
- **Cons**:**违反 case 目的**(本 case 就是要测 C++,改 Rust 就没有跨语言验证价值)

## Decision

**选 Option A**。

理由:
1. C++17 + CMake + Emscripten 是 wasm-from-C++ 的事实标准
2. 手写 C ABI 让 wasm 边界 explicit,符合 ADSD "doc decision boundaries" 原则
3. SvelteKit 跟 cs01/cs03/Studio 栈对齐,跨 case 知识复用
4. GoogleTest 跨 native + wasm 都能跑

## Consequences

### 正面

- 5-gate 在 C++ 端可用(clang-format + clang-tidy + cmake + ctest + cppcheck)
- wasm artifact <150KB,加载快
- C ABI 显式 → 比 bind 易诊断 ABI bug

### 负面 / 接受的债

- C ABI glue 比 `bind` 写得更长(可接受)
- C++17 没 `<ranges>`,有些代码可能啰嗦

### 不可逆性

- C++17 → C++20:中等可逆(一些 API 需调整)
- 手写 ABI → emscripten::bind:**完全可逆**(可同时存在)
- CMake → Bazel:**严重不可逆**(几乎重写 build),不会做

## Done Criteria

- [ ] `cmake -B build-native -S core && cmake --build build-native && ctest --test-dir build-native` 全过
- [ ] `bash scripts/build-wasm.sh` 产出 `web/static/phys2d.{js,wasm}` ≤ 150KB
- [ ] `web/src/lib/phys.ts` 通过 wasm exports 创建 World、跑 step、读 bodies
- [ ] `cd web && pnpm dev` 网页能 60 fps 跑 demo

## Cross-references

- 参考 cs01 / cs03 的 SvelteKit 选择
- 代码:`core/CMakeLists.txt`, `core/src/wasm_glue.cpp`, `scripts/build-wasm.sh`

## Notes

- 如果 v0.2 要做 SIMD 性能优化,Emscripten 的 `-msimd128` 配 `wasm-simd` intrinsics。
- Sub-agent 注意:**任何 struct 改字段必须同步改 ts wrapper**(F-pattern 候选:cross-language ABI contract drift)。
