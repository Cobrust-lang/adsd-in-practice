// SPDX-License-Identifier: Apache-2.0 OR MIT
#pragma once

#include "vec2.hpp"
#include "aabb.hpp"

namespace phys2d {

/// Rigid body with Verlet integrator state.
///
/// `prev_pos` is implicit velocity: `velocity = (pos - prev_pos) / dt`.
struct Body {
    Vec2 pos;
    Vec2 prev_pos;
    Vec2 half_extents;  // AABB half-size
    float inv_mass{1.0F};
    bool is_static{false};

    [[nodiscard]] Aabb aabb() const noexcept {
        return {pos - half_extents, pos + half_extents};
    }
};

}  // namespace phys2d
