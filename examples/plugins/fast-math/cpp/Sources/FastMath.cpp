#include "FastMath.hpp"

namespace plugin_dev::plugin_nexa::plugin_fast_dash_math::FastMath {

std::int32_t add(std::int32_t a, std::int32_t b) noexcept {
    return a + b;
}

std::int32_t multiply(std::int32_t a, std::int32_t b) noexcept {
    return a * b;
}

std::int64_t computeHash(std::vector<std::uint8_t> data) noexcept {
    std::uint64_t hash = 14695981039346656037ULL;
    for (std::uint8_t byte : data) {
        hash ^= byte;
        hash *= 1099511628211ULL;
    }
    return static_cast<std::int64_t>(hash);
}

std::future<std::int64_t> heavyCalculation(std::int32_t input) noexcept {
    std::promise<std::int64_t> promise;
    std::int64_t result = static_cast<std::int64_t>(input) * 42;
    promise.set_value(result);
    return promise.get_future();
}

} // namespace plugin_dev::plugin_nexa::plugin_fast_dash_math::FastMath
