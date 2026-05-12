# ADR-0001 English abstract: Stack choice

> Full ADR: [docs/agent/adr/0001-stack-choice.md](../../agent/adr/0001-stack-choice.md).

## Decision

- **C++**: `C++17` (broadest compiler support)
- **Build**: `CMake` (dual native + wasm targets)
- **Wasm**: `Emscripten` (`emcmake cmake`), hand-written C ABI (no `emscripten::bind`)
- **Frontend**: `SvelteKit` (aligned with cs01/cs03/Studio)
- **C++ tests**: `GoogleTest` (via FetchContent)

## Why

- C++17 + CMake + Emscripten is the de facto wasm-from-C++ standard
- **Hand-written C ABI** is easier to audit than bind, with explicit struct layout — aligned with ADSD F24 defense
- SvelteKit for cross-case knowledge reuse

## Status

`accepted` — 2026-05-12.
