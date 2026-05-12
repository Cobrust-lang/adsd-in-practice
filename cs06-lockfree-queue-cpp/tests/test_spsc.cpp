// SPDX-License-Identifier: Apache-2.0 OR MIT
#include <gtest/gtest.h>
#include "lfq/spsc.hpp"

using lfq::Spsc;

TEST(Spsc, ConstructEmpty) {
    Spsc<int, 8> q;
    auto v = q.try_pop();
    EXPECT_FALSE(v.has_value());
}

TEST(Spsc, CapacityIsPowerOfTwoAtCompileTime) {
    // Compile-time static_assert catches non-power-of-2 at static_assert site;
    // this test just confirms the public API compiles for a valid Capacity.
    Spsc<int, 16> q;
    (void)q;
    SUCCEED();
}

// M1.1 will add:
//   - PushPopRoundTrip
//   - FullThenEmpty
//   - Wraparound behavior
//   - ThreadSanitizer stress (M2)
