// SPDX-License-Identifier: Apache-2.0 OR MIT
#include "phys2d/vec2.hpp"
#include <cmath>

namespace phys2d {

float Vec2::length() const noexcept {
    return std::sqrt(x * x + y * y);
}

}  // namespace phys2d
