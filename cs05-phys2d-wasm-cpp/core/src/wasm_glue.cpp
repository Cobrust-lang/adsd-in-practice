// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// C ABI glue exposed to Emscripten. Single global World for simplicity
// in M0 — multi-world dispatch lands at M4 if demand exists.
//
// ABI invariants (DO NOT CHANGE without an ADR superseder):
//   - Body layout: pos.xy, prev_pos.xy, half.xy, inv_mass, is_static (32 bytes)
//   - All floats are little-endian (wasm assumption)
//   - World_get_bodies returns a pointer into wasm linear memory;
//     the caller must read sizeof(Body)*count bytes immediately.

#include "phys2d/world.hpp"

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#define EXPORT EMSCRIPTEN_KEEPALIVE
#else
#define EXPORT
#endif

extern "C" {

static phys2d::World g_world;

EXPORT void World_step(float dt) { g_world.step(dt); }

EXPORT void World_add_body(float x, float y, float hx, float hy, float inv_mass, int is_static) {
    phys2d::Body b{};
    b.pos = {x, y};
    b.prev_pos = {x, y};
    b.half_extents = {hx, hy};
    b.inv_mass = inv_mass;
    b.is_static = (is_static != 0);
    g_world.add_body(b);
}

EXPORT const phys2d::Body* World_get_bodies(int* out_count) {
    *out_count = static_cast<int>(g_world.bodies().size());
    return g_world.bodies().data();
}

}  // extern "C"
