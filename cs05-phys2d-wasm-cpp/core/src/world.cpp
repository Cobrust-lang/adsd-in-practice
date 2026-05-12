// SPDX-License-Identifier: Apache-2.0 OR MIT
#include "phys2d/world.hpp"

namespace phys2d {

void World::add_body(const Body& b) { bodies_.push_back(b); }

void World::step(float dt) noexcept {
    // M0 scaffold — actual Verlet + collision land at M2/M3.
    // For now: apply gravity as a velocity tweak so a smoke test
    // can verify the API surface.
    for (auto& b : bodies_) {
        if (b.is_static) continue;
        b.pos = b.pos + (gravity * (dt * dt));
    }
}

}  // namespace phys2d
