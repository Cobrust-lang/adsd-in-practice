// SPDX-License-Identifier: Apache-2.0 OR MIT
#pragma once

#include "vec2.hpp"

namespace phys2d {

struct Aabb {
    Vec2 min;
    Vec2 max;

    [[nodiscard]] constexpr bool intersects(const Aabb& o) const noexcept {
        return !(max.x < o.min.x || min.x > o.max.x || max.y < o.min.y || min.y > o.max.y);
    }
};

}  // namespace phys2d
