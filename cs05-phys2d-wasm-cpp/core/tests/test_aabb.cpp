// SPDX-License-Identifier: Apache-2.0 OR MIT
#include <gtest/gtest.h>
#include "phys2d/aabb.hpp"

using phys2d::Aabb;
using phys2d::Vec2;

TEST(Aabb, IntersectsTrueOnOverlap) {
    Aabb a{{0.0F, 0.0F}, {1.0F, 1.0F}};
    Aabb b{{0.5F, 0.5F}, {1.5F, 1.5F}};
    EXPECT_TRUE(a.intersects(b));
}

TEST(Aabb, IntersectsFalseOnSeparated) {
    Aabb a{{0.0F, 0.0F}, {1.0F, 1.0F}};
    Aabb b{{2.0F, 0.0F}, {3.0F, 1.0F}};
    EXPECT_FALSE(a.intersects(b));
}

TEST(Aabb, IntersectsTrueOnTouchingEdge) {
    // M0 decision: touching edges count as intersecting.
    // Revisit in ADR if Verlet step needs to distinguish.
    Aabb a{{0.0F, 0.0F}, {1.0F, 1.0F}};
    Aabb b{{1.0F, 0.0F}, {2.0F, 1.0F}};
    EXPECT_TRUE(a.intersects(b));
}
