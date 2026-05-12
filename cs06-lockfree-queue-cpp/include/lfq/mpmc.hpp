// SPDX-License-Identifier: Apache-2.0 OR MIT
#pragma once

#include <array>
#include <atomic>
#include <cstddef>
#include <optional>
#include <type_traits>

#include "spsc.hpp"  // for kCacheLineSize

namespace lfq {

/// Bounded MPMC queue, Vyukov-style.
///
/// Each cell holds a sequence counter that encodes "ready to write" / "ready to read".
/// ABA-safe via 64-bit sequence rotation.
///
/// M0 scaffold — implementation lands at M3.
template <typename T, std::size_t Capacity>
class Mpmc {
    static_assert(Capacity >= 2, "Capacity >= 2");
    static_assert((Capacity & (Capacity - 1)) == 0, "Capacity must be power of two");
    static_assert(std::is_trivially_copyable_v<T>, "T must be trivially copyable for v0.1");

public:
    Mpmc() {
        for (std::size_t i = 0; i < Capacity; ++i) {
            cells_[i].seq.store(i, std::memory_order_relaxed);
        }
    }

    Mpmc(const Mpmc&) = delete;
    Mpmc& operator=(const Mpmc&) = delete;

    /// Try to enqueue. Returns false if full.
    [[nodiscard]] bool try_push(const T& /*v*/) noexcept {
        // M3 stub — implement Vyukov push CAS loop with ABA-safe sequence counter.
        return false;
    }

    /// Try to dequeue. Returns nullopt if empty.
    [[nodiscard]] std::optional<T> try_pop() noexcept {
        // M3 stub
        return std::nullopt;
    }

private:
    struct Cell {
        std::atomic<std::size_t> seq{0};
        T value{};
    };

    alignas(kCacheLineSize) std::array<Cell, Capacity> cells_{};
    alignas(kCacheLineSize) std::atomic<std::size_t> enqueue_pos_{0};
    alignas(kCacheLineSize) std::atomic<std::size_t> dequeue_pos_{0};
};

}  // namespace lfq
