# ADR-0001 English abstract: Stack choice

> Full ADR: [docs/agent/adr/0001-stack-choice.md](../../agent/adr/0001-stack-choice.md).

## Decision

- **C++**: `C++20` (`concepts` + `consteval` to statically enforce Capacity-power-of-2)
- **Build**: `CMake` + FetchContent (auto-pulls GoogleTest / google-benchmark)
- **Tests**: `GoogleTest` (unit + stress)
- **Benchmark**: `google-benchmark` (opt-in `-DLFQ_BENCH=ON`)
- **Concurrency verification**: `ThreadSanitizer` (built into clang/gcc)
- **Oracle**: `Boost.Lockfree` (dev only, not in main deps)

## Why

- C++20 compile-time constraints → F24 defense (`static_assert((Capacity & (Capacity-1)) == 0)`)
- CMake + FetchContent gives sub-agents one-shot toolchain bootstrap
- TSAN is the de facto standard for lock-free correctness validation

## Status

`accepted` — 2026-05-12.
