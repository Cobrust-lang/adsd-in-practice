// SPDX-License-Identifier: Apache-2.0 OR MIT
#pragma once

#include <cstddef>

namespace phys2d {

/// 2D vector. Trivially copyable; passed by value at API boundaries.
struct Vec2 {
    float x{0.0F};
    float y{0.0F};

    constexpr Vec2() = default;
    constexpr Vec2(float xv, float yv) : x{xv}, y{yv} {}

    constexpr Vec2 operator+(Vec2 o) const noexcept { return {x + o.x, y + o.y}; }
    constexpr Vec2 operator-(Vec2 o) const noexcept { return {x - o.x, y - o.y}; }
    constexpr Vec2 operator*(float s) const noexcept { return {x * s, y * s}; }

    [[nodiscard]] constexpr float dot(Vec2 o) const noexcept { return x * o.x + y * o.y; }
    [[nodiscard]] float length() const noexcept;
};

static_assert(sizeof(Vec2) == 8, "Vec2 must be 8 bytes for wasm ABI stability");

}  // namespace phys2d
