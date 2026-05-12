# CS-06 lockfree-queue-cpp — Local Agent Constitution

> Local CLAUDE.md。本 case 是 ADSD 在**性能 + 正确性双高压**领域的考验,**纪律比 cs01-cs05 都严**。

---

## 1. F24 防御 — 不可简化清单(本 case 最严)

- ❌ **不准用 `std::mutex` "代替" lock-free 实现**——这是定义 F24 的核心反模式。整个 case 的意义就是不用 mutex。
- ❌ **不准用 `std::atomic_flag` 当 spinlock 然后说"是 lock-free"**——那是 lock-based 的轻量版,不是真 lock-free。
- ❌ **不准 stress test 用 `std::this_thread::sleep_for` 假装"并发"**——必须用真 N 线程 + 共享原子。
- ❌ **不准用 `memory_order_seq_cst` 一把梭**——必须明确每个 atomic 操作的 ordering(默认 seq_cst 是性能反模式,且掩盖 ordering bug)。
- ❌ **不准在 push 失败时返回 `T()` 而不报告**——必须 `bool try_push` 或 `std::optional<T>`,**永远不静默丢消息**。
- ✅ 允许在 MPMC 上用 hazard pointer(虽然复杂),不允许说"我们不做 ABA 防护因为 64-bit counter 够大了"——必须显式 ADR。

判断:**用 ThreadSanitizer 跑 1000 次 stress test 全 green;在 ARM64 上跑同样测试也 green**。两者都过 = 真 lock-free;只过一个 = 你只在 x86 上"偶然对"。

## 2. Oracle(F23-A 防御)

**三套 oracle**:

1. **Boost.LockFree 比对**:
   - 我们的 `Spsc<int, 1024>` 跟 `boost::lockfree::spsc_queue<int, capacity<1024>>` 在同样 stress 输入下,**消费者收到的元素 multiset 相等**(消息无丢、无重)
2. **ThreadSanitizer**:
   - `-fsanitize=thread` 跑 1000 次 stress,0 warnings = pass
3. **物理不变量**:
   - N 个 producer 各推 K 条消息,M 个 consumer 各拉若干,**总收到条数 = N*K**
   - producer 序号递增,consumer 收到的消息**FIFO 序保持**(SPSC)或**至少 partial order**(MPMC)

## 3. Memory ordering 决策模板

任何 atomic 操作 ADR 必须显式回答:

- 这是 acquire / release / acquire-release / relaxed / seq_cst?
- 为什么(给出 happens-before 链)?
- 反例:用更弱的 ordering 会撞什么 race?
- x86 / arm64 上的实际产物是什么(`objdump` 摘要)?

参考:`include/lfq/spsc.hpp` 的 `head_.store(new_head, std::memory_order_release)` 必须有上述 4 行注释。

## 4. 实施顺序(F22 cadence-aware)

**M1**(SPSC 单线程):
1. `Spsc<T, Capacity>` 头文件实现(ring buffer + 两个 atomic index)
2. 单线程单测:push/pop 边界 / 满 / 空

**M2**(SPSC 多线程):
3. 1 producer + 1 consumer stress(各 1e6 ops × 10 次,无丢消息)
4. TSAN 跑同样 stress,green
5. google-benchmark:vs `std::mutex` + `std::queue` 基准

**M3**(MPMC):
6. Vyukov bounded MPMC 实现
7. N×M stress(N=4, M=4, ops/thread=1e5)
8. ABA 防护(64-bit sequence counter,显式 ADR)
9. TSAN green

**M4**:
10. benchmark report(table + 图)
11. ARM64 cross-check(M-series Mac 上跑 + ARM linux 上 docker)
12. METHODOLOGY-STATUS 写

## 5. 性能 SLO(必须 reach,否则 ADR 解释为什么不行)

| 指标 | 目标 | 备注 |
|---|---|---|
| Spsc<int, 1024> push throughput(单线程)| ≥ 200M ops/s | mac M-series |
| Spsc<int, 1024> 单生产+单消费 | ≥ 100M msgs/s/pair | 总吞吐 |
| Mpmc<int, 1024> 4×4 | ≥ 50M msgs/s 总 | |
| vs `std::mutex` + `std::queue` | ≥ 10× faster | 1P+1C |
| L1 miss rate(perf stat) | ≤ 5% | cache-friendly check |

**达不到**:写 finding `m4-perf-shortfall-{spsc,mpmc}.md`,记录 profile 分析(perf / Instruments),不准粉饰。

## 6. ARM64 强制 cross-check(F15 single-difficulty 防御)

mac 是 ARM64,本机直接 cross-check。但 CI 上要 x86 + ARM 都跑(M4 release 必须达到)。

```bash
# 本机 arm64
cmake -B build-arm -S . && cmake --build build-arm && ctest --test-dir build-arm
# docker x86_64
docker run --rm -v $PWD:/w -w /w --platform linux/amd64 ubuntu:24.04 \
    bash -c 'apt-get update && apt-get install -y cmake build-essential && \
             cmake -B build-x86 -S . && cmake --build build-x86 && \
             ctest --test-dir build-x86'
```

任一 arch fail = release 阻塞。

## 7. 双语 doc

C++ 代码注释英文(必须详细,memory ordering 注释是法定要求);ADR 双语;README 双语。

---

**End. 本 case 是 ADSD 工程纪律最严的考验。任何"差不多对"=直接 fail。**
