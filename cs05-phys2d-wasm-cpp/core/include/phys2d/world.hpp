// SPDX-License-Identifier: Apache-2.0 OR MIT
#pragma once

#include <vector>
#include "body.hpp"
#include "vec2.hpp"

namespace phys2d {

/// Top-level simulation container.
///
/// M0 scaffold — `step()` is a stub. Real Verlet + collision lands at M2/M3.
class World {
public:
    void add_body(const Body& b);
    [[nodiscard]] const std::vector<Body>& bodies() const noexcept { return bodies_; }
    void step(float dt) noexcept;

    Vec2 gravity{0.0F, -9.8F};

private:
    std::vector<Body> bodies_;
};

}  // namespace phys2d
