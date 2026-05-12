<div align="center">

# CS-05 · phys2d-wasm-cpp

**2D 刚体物理引擎 + WebAssembly + Svelte 可视化**

*ADSD case study #5 — C++17 + Emscripten + Vite + 跨语言 build system*

</div>

---

## What this is

一个**2D 刚体物理引擎**(类 Box2D 的极简版):AABB + SAT 碰撞检测,Verlet 积分。**C++17 core 用 CMake 构建,通过 Emscripten 编译到 WebAssembly,Svelte canvas 做可视化 demo**。

**测什么**:ADSD 在**跨语言、跨 build system**(cmake + emcc + vite)的项目上是否仍然成立。预期 5-gate 会从 5 涨到 ≥ 8(C++ 端 + wasm 端 + 前端各一套),**这是验证 ADSD 复杂度的关键点**。

## 范围

### v0.1.0 必须 ship(M4)

- ✅ Vec2 / AABB / 矩形刚体定义
- ✅ Verlet 积分(固定时间步 1/60s)
- ✅ AABB broad phase + SAT narrow phase 碰撞检测
- ✅ 接触点求解(impulse-based)
- ✅ Emscripten 编译到 `.wasm` + JS glue
- ✅ Svelte canvas demo:鼠标点击在场景里落矩形 + 重力 + 边界
- ✅ 端到端:网页打开 → 看 60 fps 多刚体下落 + 堆叠 demo
- ✅ 5+3 道 ADSD gate green(C++ 端 5 + 前端 3)

### Out of scope(0.1.0 不做)

- ❌ 多边形(只 AABB + 圆形)
- ❌ 连续碰撞检测(CCD)
- ❌ 软体物理 / 关节(P1)
- ❌ 3D
- ❌ 跨平台原生 binding(只 wasm,不做 Python bindings)

## ADSD 触发点

| 决策点 | 预期 ADR |
|---|---|
| Verlet vs Euler vs Runge-Kutta 积分器 | ADR-0002 |
| Broad phase 数据结构(naive O(n²) vs sweep & prune vs grid) | ADR-0003 |
| Emscripten 选项 + ABI 边界(C struct passing vs glue layer) | ADR-0004 |
| Wasm memory layout(stack vs heap allocation) | ADR-0005 |
| Svelte frame loop(rAF vs setInterval) | ADR-0006 |
| Build system 顶层:CMake 调 emcc,还是用 emcmake | ADR-0007 |

**预期会撞**:
- **F5** silent miscompile(数值漂移 / FP precision 在 wasm 跟 native 不一致,没 panic 但仿真错)
- **F15** single-difficulty(简单 demo OK,堆叠 50 个 box 不稳)
- **新候选 F-pattern**:**跨语言 ABI 边界的契约缺失**(C++ struct layout 改了,wasm bind 没同步)
- **新候选 F-pattern**:**多 build system 协同**(cmake + emcc + vite,任一改了另两个不一定 rebuild)

## Quick start

```bash
cd cs05-phys2d-wasm-cpp
bash scripts/bootstrap.sh   # 装 cmake + emsdk + pnpm
# 构建 native test
cmake -B build -S core && cmake --build build && ctest --test-dir build
# 构建 wasm + 前端 dev
bash scripts/build-wasm.sh
cd web && pnpm dev
# 打开 http://localhost:5173
```

## Architecture

```
core/                       # C++17 物理引擎
  include/phys2d/           # Public headers
    vec2.hpp / aabb.hpp / world.hpp / body.hpp
  src/                      # 实现 .cpp
  tests/                    # GoogleTest 单测(native)

web/                        # SvelteKit + Vite
  src/lib/phys.ts           # wasm binding wrapper
  src/routes/+page.svelte   # canvas demo
  static/phys2d.wasm        # ← emscripten 产物(构建时复制)
```

依赖:`core` → wasm(emcc 编译)→ `web/static/`。CMake 调 emcc 还是单独脚本,见 ADR-0007。

## Status

- 🚧 M0 scaffold
- ⬜ M1 Vec2/AABB/Body + native unit tests
- ⬜ M2 Verlet + 简单重力(单刚体下落)
- ⬜ M3 collision + emcc wasm 链
- ⬜ M4 v0.1.0 release(网页 demo + 多刚体堆叠)+ METHODOLOGY-STATUS

## License

Apache-2.0 + MIT。
