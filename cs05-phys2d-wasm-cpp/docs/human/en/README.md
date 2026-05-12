# cs05-phys2d-wasm-cpp (English user guide)

## What this is

A 2D rigid body physics engine in C++17, compiled to wasm via Emscripten, visualized with SvelteKit canvas. Validates ADSD across language boundary + multi-build-system projects.

## Quick start

```bash
cd cs05-phys2d-wasm-cpp
bash scripts/bootstrap.sh   # needs cmake + emsdk + pnpm
cd web && pnpm dev          # start frontend
# open http://localhost:5173
```

## ADR index

- [ADR-0001 Stack choice](./adr-0001-stack-choice.md): C++17 + CMake + Emscripten + hand-written C ABI + SvelteKit

## License

Apache-2.0 + MIT.
