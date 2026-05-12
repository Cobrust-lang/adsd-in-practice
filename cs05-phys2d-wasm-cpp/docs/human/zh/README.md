# cs05-phys2d-wasm-cpp(中文用户指南)

## 这是什么

C++17 写的 2D 刚体物理引擎,Emscripten 编译到 wasm,SvelteKit canvas 做可视化。验证 ADSD 在跨语言 + 多 build system 项目上是否仍然成立。

## 快速开始

```bash
cd cs05-phys2d-wasm-cpp
bash scripts/bootstrap.sh   # 需要 cmake + emsdk + pnpm
cd web && pnpm dev          # 起前端
# 打开 http://localhost:5173
```

## ADR 索引

- [ADR-0001 栈选择](./adr-0001-stack-choice.md):C++17 + CMake + Emscripten + 手写 C ABI + SvelteKit

## License

Apache-2.0 + MIT。
