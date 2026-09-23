#pragma once

#include <cstdint>
#include <future>
#include <vector>

namespace plugin_dev::plugin_nexa::plugin_fast_dash_math::FastMath {

std::int32_t add(std::int32_t a, std::int32_t b) noexcept;
std::int32_t multiply(std::int32_t a, std::int32_t b) noexcept;
std::int64_t computeHash(std::vector<std::uint8_t> data) noexcept;
std::future<std::int64_t> heavyCalculation(std::int32_t input) noexcept;

} // namespace plugin_dev::plugin_nexa::plugin_fast_dash_math::FastMath
