# ADR-0001 中文摘要:栈选择

> 完整 ADR 见 [docs/agent/adr/0001-stack-choice.md](../../agent/adr/0001-stack-choice.md)。

## 决策

- **C++**:`C++20`(`concepts` + `consteval` 强约束 Capacity-power-of-2)
- **Build**:`CMake` + FetchContent(自动拉 GoogleTest / google-benchmark)
- **测试**:`GoogleTest`(单测 + stress)
- **基准**:`google-benchmark`(opt-in `-DLFQ_BENCH=ON`)
- **并发验证**:`ThreadSanitizer`(clang/gcc 内置)
- **Oracle**:`Boost.Lockfree`(仅 dev,不入主依赖)

## 为什么

- C++20 编译期约束 → F24 防御(`static_assert((Capacity & (Capacity-1)) == 0)`)
- CMake + FetchContent 让 sub-agent 一键跑齐工具链
- TSAN 是 lock-free 正确性的事实标准

## 状态

`accepted` — 2026-05-12
