# cs06-lockfree-queue-cpp (English user guide)

## What this is

C++20 lock-free queues (SPSC + MPMC), with GoogleTest unit/stress tests, ThreadSanitizer validation, and google-benchmark. **The ADSD endurance test for performance + correctness double-pressure domains.**

## Quick start

```bash
cd cs06-lockfree-queue-cpp
bash scripts/bootstrap.sh   # needs cmake + clang + cppcheck

# TSAN stress
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

## ADR index

- [ADR-0001 Stack choice](./adr-0001-stack-choice.md): C++20 + CMake + GoogleTest + google-benchmark + TSAN

## License

Apache-2.0 + MIT.
