# Findings — cs06-lockfree-queue-cpp

## Ledger

| File | Severity | Status | Related ADR | F-pattern | Date |
|---|---|---|---|---|---|
| (尚无 finding) | | | | | |

## 预期会撞的新 F-pattern(本 case 最多)

- **memory_ordering-misjudgment** — 本该 release 写成 relaxed,x86 OK 但 ARM 爆
- **stress-pass-equals-false-confidence** — 1000 次 stress 没撞 race 不代表对(扩 case 数据规模 5x 后才暴露)
- **ABA-misdiagnosis-as-spurious-wakeup** — 真的是 ABA 但被当成"偶发"
- **TSAN-false-positive-on-benign-race** — 已知 idiom TSAN 报警,需要 ignorelist 但要诚实记录
- **cache-line-thrashing-from-shared-counter** — 4 producer 各递增同一 counter,产生 cache contention,但功能"对"

## 命名

`{milestone}-{slug}.md`,例:`m2-spsc-arm-release-miss.md`
