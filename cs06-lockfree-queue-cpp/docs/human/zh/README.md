# cs06-lockfree-queue-cpp(中文用户指南)

## 这是什么

C++20 lock-free 队列(SPSC + MPMC),配 GoogleTest 单测、TSAN 验证、google-benchmark 性能基准。**ADSD 在性能 + 正确性双高压领域的考验**。

## 快速开始

```bash
cd cs06-lockfree-queue-cpp
bash scripts/bootstrap.sh   # 需要 cmake + clang + cppcheck

# TSAN 跑 stress
cmake -B build-tsan -S . -DCMAKE_CXX_FLAGS="-fsanitize=thread -g -O1"
cmake --build build-tsan && ctest --test-dir build-tsan

# Benchmark
cmake -B build -S . -DLFQ_BENCH=ON && cmake --build build --target queue_bench
./build/queue_bench
```

## API

```cpp
#include <lfq/spsc.hpp>

lfq::Spsc<int, 1024> q;
if (q.try_push(42)) { /* ok */ }
if (auto v = q.try_pop()) { /* got *v */ }
```

## ADR 索引

- [ADR-0001 栈选择](./adr-0001-stack-choice.md):C++20 + CMake + GoogleTest + google-benchmark + TSAN

## License

Apache-2.0 + MIT。
