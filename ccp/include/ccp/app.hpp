#pragma once

#include <string>
#include <string_view>

namespace ccp {

int fetch_binance_ticker();
std::string format_binance_ticker(std::string_view payload);

} // namespace ccp
