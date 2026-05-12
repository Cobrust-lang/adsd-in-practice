# CS-05 phys2d-wasm-cpp — Local Agent Constitution

> Local CLAUDE.md。

---

## 1. F24 防御 — 不可简化清单

- ❌ **不准用 `std::vector<Vec2>` "代替"freeform bodies 列表**(违反 broad-phase 数据结构 ADR;必须真做 spatial partition,即使简单 grid)
- ❌ **不准用 `std::list` 当主体 body container**(cache miss 极差,2D 物理引擎核心不变量)
- ❌ **不准 cheat 用 `Math.random` 在 JS 里加噪声"假装"模拟稳定性**——所有物理输出必须由 wasm core 决定
- ✅ 允许 v0.1 用 naive O(n²) broad phase(M1),但 M3 必须升级到 grid

判断:**如果用户从 native ctest 跑出的数值跟 wasm 跑出的不一致(超过 FP epsilon),那就是 bug,不是"差不多"**。

## 2. 跨语言 oracle(F23-A)

**双重 oracle**:

1. **Native vs Wasm 一致性**(自一致):
   - 相同输入 → 相同输出(在 epsilon 内)
   - `tests/cross_runtime_test.sh`:跑 native 测试,记录每帧每个 body 的 x/y/v;跑 wasm 同 fixture,逐帧比对
   - **每个 commit 都跑**,因为 emcc 优化级别不同会 introduce drift

2. **物理常识 oracle**:
   - 自由下落 1 秒 → 位移 ≈ 4.9 m(在我们标定的重力 9.8 下)
   - 弹性碰撞动量守恒
   - 这些是物理定律,可以做 invariant check

## 3. 多 build system 协同(本 case 最大新挑战)

```
                     ┌─────────────┐
                     │  cmake-native│  ← cmake -B build-native
                     └─────────────┘
                            ↑
                            │
                     ┌──────┴──────┐
                     │  core/      │
                     │  CMakeLists │
                     └──────┬──────┘
                            ↓
                     ┌─────────────┐
                     │  cmake-emcc │  ← emcmake cmake -B build-wasm
                     └──────┬──────┘
                            ↓ produces phys2d.wasm + phys2d.js
                     ┌─────────────┐
                     │  web (vite) │  ← vite imports static/phys2d.js
                     └─────────────┘
```

**改 `core/` 时,必须**:
1. native build 通过(ctest)
2. wasm build 通过(`build-wasm.sh`)
3. wasm artifact 复制到 `web/static/`
4. `web` 前端 build 通过(vite build)

**新 F-pattern 候选**:multi-build coherence failure — 改了 core,vite 里旧 wasm 没替换。在 M4 release 前必须固化解决方案(`scripts/build-all.sh` 强制顺序)。

## 4. 实施顺序

**M1**(C++ native):
1. `Vec2`, `AABB`, `Body` 头文件 + 实现
2. GoogleTest:Vec2 加减 / AABB 相交 / Body 默认值
3. `World::step(dt)` 空实现

**M2**(物理积分):
4. Verlet 积分(`Body::update_verlet(dt)`)
5. 重力作用单刚体(自由下落 1s = 4.9m)

**M3**(碰撞 + wasm):
6. AABB broad phase(M3.1 naive O(n²),M3.2 grid)
7. Penetration 解算(impulse-based)
8. emcc binding:`World_step` / `World_add_body` / `World_get_bodies` C ABI
9. Svelte canvas:鼠标点击落矩形,rAF 调 `World_step`

**M4**:release + cross-runtime test + 50-box 堆叠 demo

## 5. 性能 SLO

| 指标 | 目标 | 测法 |
|---|---|---|
| 单 body Verlet 步进(native) | ≤ 100 ns | google-benchmark |
| 50 bodies World::step(native) | ≤ 100 µs | google-benchmark |
| 50 bodies wasm 60 fps | 稳定无掉帧 | Chrome devtools |
| wasm bundle 大小 | ≤ 150 KB(brotli) | `ls -l web/static/*.wasm.br` |
| native vs wasm 数值漂移 | ≤ 1e-5(单步)| cross_runtime_test |

## 6. 双语 doc

ADR / finding 双语。C++ 代码注释英文(Doxygen 风格);中文 README 写"用户能看到什么"。

---

**End. 其它沿用顶层 CLAUDE.md。**
