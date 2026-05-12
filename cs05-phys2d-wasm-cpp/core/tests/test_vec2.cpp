// SPDX-License-Identifier: Apache-2.0 OR MIT
#include <gtest/gtest.h>
#include "phys2d/vec2.hpp"

using phys2d::Vec2;

TEST(Vec2, AddSub) {
    Vec2 a{1.0F, 2.0F};
    Vec2 b{3.0F, 4.0F};
    auto c = a + b;
    EXPECT_FLOAT_EQ(c.x, 4.0F);
    EXPECT_FLOAT_EQ(c.y, 6.0F);
    auto d = b - a;
    EXPECT_FLOAT_EQ(d.x, 2.0F);
    EXPECT_FLOAT_EQ(d.y, 2.0F);
}

TEST(Vec2, DotProduct) {
    Vec2 a{1.0F, 0.0F};
    Vec2 b{0.0F, 1.0F};
    EXPECT_FLOAT_EQ(a.dot(b), 0.0F);
    EXPECT_FLOAT_EQ(a.dot(a), 1.0F);
}

TEST(Vec2, Length) {
    Vec2 a{3.0F, 4.0F};
    EXPECT_FLOAT_EQ(a.length(), 5.0F);
}

TEST(Vec2, SizeIsStable) {
    // Tested at compile time too — see vec2.hpp static_assert.
    EXPECT_EQ(sizeof(Vec2), 8U);
}
