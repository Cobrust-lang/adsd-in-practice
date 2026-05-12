<div align="center">

# CS-06 · lockfree-queue-cpp

**SPSC + MPMC lock-free 队列 + benchmark · C++20 · 无前端**

*ADSD case study #6 — 工程纪律最考验的领域:并发原语 + 内存模型 + 高压力性能*

</div>

---

## What this is

**SPSC**(单生产者单消费者)+ **MPMC**(多生产者多消费者)lock-free 队列,基于 `std::atomic` + C++ memory ordering 实现。配 google-benchmark 性能基准和 ThreadSanitizer + 高强度 stress test。

**测什么**:**ADSD 在性能 + 正确性双高压领域是否仍然适用,还是变成 over-engineering**。LC-style 并发题(producer/consumer / single-source-of-truth)在 lock-free 上是 PhD 课题级,**F-pattern 在这里会爆发**。

## 范围

### v0.1.0 必须 ship(M4)

- ✅ `Spsc<T, Capacity>`(模板,ring-buffer 实现,Capacity 是 2 的幂)
- ✅ `Mpmc<T, Capacity>`(Bounded MPMC,Vyukov-style)
- ✅ `try_push` / `try_pop` 接口(非阻塞,失败返回 bool/optional)
- ✅ `push_blocking` / `pop_blocking`(spin + backoff,M3 提供)
- ✅ GoogleTest 单测(per-API 边界 + 多线程 stress)
- ✅ google-benchmark:对比 `std::mutex` + `std::queue`(基准),Boost lockfree,自家 SPSC/MPMC
- ✅ ThreadSanitizer 跑过 ≥1000 stress iterations 无报警
- ✅ 5 道 ADSD gate green(C++ 适配)

### Out of scope

- ❌ Unbounded queue(M4 之后)
- ❌ Wait-free(强于 lock-free,P1 才考虑)
- ❌ NUMA-aware partitioning(P1)
- ❌ MPSC / SPMC 专用(用 MPMC 替代足够 v0.1)

## ADSD 触发点

| 决策点 | 预期 ADR |
|---|---|
| Memory ordering 策略(acquire/release vs seq_cst 全用)| ADR-0002 |
| ABA 防护(version counter vs hazard pointer)| ADR-0003 |
| Capacity 是 2 的幂的约束(mask 优化)| ADR-0004 |
| SPSC vs MPMC 复用与否(同一 base class)| ADR-0005 |
| Backoff 策略(`pause` vs `yield` vs exponential)| ADR-0006 |

**预期会撞**(这是本 repo F-pattern 最爆发的 case):
- **F4** quarantine pollution(ThreadSanitizer 跑挂另一个测试,污染基线)
- **F5** silent miscompile(reorder 没保证但 x86 偶然 OK,在 arm 上爆)
- **F9** wrong root-cause(ABA 误诊成 spurious wake-up)
- **F15** single-difficulty(small queue OK,full queue 边界爆)
- **F23-A** oracle 自己写自己测(必须有 known-good lockfree 库 / Boost 做对比 oracle)
- **新候选 F-pattern**:
  - **memory_ordering 漏判**(本该 release 写成 relaxed,大多平台 OK,某些 arch fail)
  - **stress test 通过 = false confidence**(只是没撞到,不代表对)
  - **TSAN false positive on benign races**(惯用 idiom 但 TSAN 报警)

## Quick start

```bash
cd cs06-lockfree-queue-cpp
bash scripts/bootstrap.sh

# 单测 + stress
cmake -B build -S . && cmake --build build && ctest --test-dir build

# TSAN
cmake -B build-tsan -S . -DCMAKE_CXX_FLAGS="-fsanitize=thread -g -O1" \
    && cmake --build build-tsan && ctest --test-dir build-tsan

# Benchmark
cmake --build build --target bench && ./build/bench/queue_bench
```

## Architecture

```
include/lfq/
  spsc.hpp        # SPSC ring buffer (header-only template)
  mpmc.hpp        # MPMC Vyukov-style (header-only template)
  backoff.hpp     # pause / yield / exponential strategy
  detail/...      # cache-line padding, atomic helpers

tests/
  test_spsc.cpp   # boundary + roundtrip
  test_mpmc.cpp   # N producer × M consumer stress
  stress.cpp      # 10s + 1000 reps continuous

bench/
  queue_bench.cpp # google-benchmark: ours vs std::mutex+queue vs Boost
```

## Status

- 🚧 M0 scaffold
- ⬜ M1 SPSC 实现 + 单线程单测
- ⬜ M2 SPSC 多线程 stress + TSAN green
- ⬜ M3 MPMC(Vyukov)+ ABA 防护
- ⬜ M4 v0.1.0 release + benchmark + METHODOLOGY-STATUS

## License

Apache-2.0 + MIT。
