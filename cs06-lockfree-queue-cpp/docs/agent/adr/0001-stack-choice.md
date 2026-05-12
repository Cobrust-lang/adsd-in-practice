---
adr: 0001
title: Stack choice — C++20 + CMake + GoogleTest + google-benchmark + TSAN
status: accepted
date: 2026-05-12
case: cs06-lockfree-queue-cpp
supersedes: none
last_verified_commit: pending
---

# ADR-0001: Stack choice

## Context

CS-06 是 lock-free 数据结构 + 高压并发 + 性能基准。必须做选:

- **C++ 标准**:C++17 / C++20 / C++23
- **Build**:CMake / Bazel
- **测试**:GoogleTest / Catch2 / doctest
- **基准**:google-benchmark / nanobench / Catch2 bench
- **并发验证**:ThreadSanitizer / Helgrind / cppcoreguidelines-concurrency / Loom
- **Boost lockfree 当 oracle 是否依赖**:依赖 / 不依赖

约束:
- **arm + x86 都要 work**(mac M-series 是 arm,Linux CI 多数 x86)
- **TSAN 必须能跑**(并发正确性的核心验证)

## Options Considered

### Option A: C++20 + CMake + GoogleTest + google-benchmark + TSAN(选中)

- **Pros**:
  - C++20 的 `std::atomic_ref` 在某些 lock-free 模式下有用(可选)
  - C++20 `consteval` / `concepts` 强类型约束
  - CMake + FetchContent 拉 GTest + bench(零外部 deps install)
  - TSAN 是 clang/gcc 内置(macOS clang 也支持)
- **Cons**:
  - C++20 在老编译器(GCC 9 / Clang 11 之前)支持不全
  - GoogleTest 加 1 分钟编译时间(可忽略)

### Option B: C++17 + Bazel + Catch2 + nanobench + Loom

- **Pros**:Loom 是 lock-free 验证的圣杯(C-Reduce style state-space search)
- **Cons**:Bazel 个人项目 overhead 高;Loom 跨编译器支持差;Catch2 启动慢

### Option C: 用 Boost.Lockfree 当主要实现 + 我们写薄包装

- **Pros**:快速 ship
- **Cons**:**违反 case 目的**(本 case 就是要自己写,Boost 只能当 oracle)

## Decision

**选 Option A**。

理由:
1. C++20 `consteval`/`concepts` 让 Capacity-power-of-2 约束在编译期严格(F24 防御)
2. CMake + FetchContent 让 sub-agent 一键跑齐 GoogleTest / google-benchmark
3. TSAN 是 lock-free 正确性的事实标准 oracle
4. Boost 不入主依赖(避免运行时绑定),只作 oracle 对比测试(可选)

## Consequences

### 正面

- 单 binary 全静态,可移植
- TSAN green 是真硬证据
- 跨 arm/x86 都跑(macOS native + docker x86)

### 负面 / 接受的债

- C++20 限制最老编译器(GCC 11+,Clang 13+),不支持老 RHEL 7 等
- TSAN 是 stress / fuzz 工具,**不是 proof**(可能漏)

### 不可逆性

- C++20 → C++17:小可逆(去掉 concepts / consteval)
- CMake → Bazel:**强不可逆**,不做

## Done Criteria

- [ ] `cmake -B build -S . && cmake --build build && ctest --test-dir build` 全过
- [ ] `cmake -B build-tsan ... -fsanitize=thread` 跑 ctest green
- [ ] `cmake -B build -DLFQ_BENCH=ON && cmake --build build --target queue_bench` 输出 benchmark 数字
- [ ] mac ARM64 + Linux x86_64(docker)双 arch green

## Cross-references

- 参考 cs05 C++ stack(同 CMake + GoogleTest 风格,但 C++20 升级)
- 代码:`CMakeLists.txt`, `include/lfq/spsc.hpp`

## Notes

- TSAN 在 mac 上需要 `-Wl,-undefined,dynamic_lookup` 在某些 toolchain(若失败,readme 加一行说明)。
- 未来要加 stress fuzzer,考虑 `relacy` 或 `Loom`(后者要换 Rust,违反 case 目的),保留观察。
