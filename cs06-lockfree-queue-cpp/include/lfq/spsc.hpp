// SPDX-License-Identifier: Apache-2.0 OR MIT
#pragma once

#include <array>
#include <atomic>
#include <cstddef>
#include <new>
#include <optional>
#include <type_traits>

namespace lfq {

inline constexpr std::size_t kCacheLineSize = 64;

/// Lock-free Single-Producer Single-Consumer ring buffer.
///
/// Capacity must be a power of two; this lets us mask instead of modulo.
///
/// M0 scaffold — concrete `try_push` / `try_pop` semantics need explicit
/// memory ordering ADR (see CLAUDE.md §3). M1.1 lands the real implementation.
template <typename T, std::size_t Capacity>
class Spsc {
    static_assert(Capacity >= 2, "Capacity >= 2");
    static_assert((Capacity & (Capacity - 1)) == 0, "Capacity must be power of two");
    static_assert(std::is_trivially_copyable_v<T>, "T must be trivially copyable for v0.1");

public:
    Spsc() = default;
    Spsc(const Spsc&) = delete;
    Spsc& operator=(const Spsc&) = delete;

    /// Producer-side: try to enqueue. Returns false if full.
    ///
    /// Memory ordering(M1.1 ADR 后填):
    ///   - load tail_: relaxed (producer owns tail)
    ///   - load head_: acquire (synchronize-with consumer's release on pop)
    ///   - store tail_: release (publish slot write before tail advance)
    [[nodiscard]] bool try_push(const T& /*v*/) noexcept {
        // M1.1 stub
        return false;
    }

    /// Consumer-side: try to dequeue. Returns nullopt if empty.
    ///
    /// Memory ordering:
    ///   - load head_: relaxed (consumer owns head)
    ///   - load tail_: acquire (synchronize-with producer's release on push)
    ///   - store head_: release (publish slot read before head advance)
    [[nodiscard]] std::optional<T> try_pop() noexcept {
        // M1.1 stub
        return std::nullopt;
    }

private:
    alignas(kCacheLineSize) std::atomic<std::size_t> head_{0};
    alignas(kCacheLineSize) std::atomic<std::size_t> tail_{0};
    alignas(kCacheLineSize) std::array<T, Capacity> buf_{};
};

}  // namespace lfq
