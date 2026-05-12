// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// google-benchmark suite. Builds only with -DLFQ_BENCH=ON.
// M4 will compare against std::mutex+std::queue and (optional) Boost.
//
// M0 — stub:

#include <benchmark/benchmark.h>
#include "lfq/spsc.hpp"

static void BM_SpscPushNoop(benchmark::State& state) {
    lfq::Spsc<int, 1024> q;
    for (auto _ : state) {
        // M1.1 stub: try_push always returns false in scaffold
        benchmark::DoNotOptimize(q.try_push(42));
    }
}
BENCHMARK(BM_SpscPushNoop);

BENCHMARK_MAIN();
