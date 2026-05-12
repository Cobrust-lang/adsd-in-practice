# Findings — cs05-phys2d-wasm-cpp

## Ledger

| File | Severity | Status | Related ADR | F-pattern | Date |
|---|---|---|---|---|---|
| (尚无 finding) | | | | | |

## 预期会撞的新 F-pattern

- **cross-language ABI contract drift** — 改了 C++ struct,wasm bind 没同步,运行时静默错位
- **multi-build coherence** — cmake 改了,vite 里旧 wasm 没替换
- **FP determinism: native vs wasm** — 编译器对 fma/round/denormal 处理差异导致仿真分歧
- **emcc memory growth + GC** — wasm 内存超出导致 `_malloc` 失败但 JS 端不感知
