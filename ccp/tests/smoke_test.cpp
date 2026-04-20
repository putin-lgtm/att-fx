#include <cstdlib>
#include <iostream>

#include "ccp/app.hpp"

int main() {
    const std::string formatted = ccp::format_binance_ticker(
        R"({"c":"74781.08000000","b":"74781.07000000","a":"74781.09000000","v":"13858.40324000"})"
    );

    if (formatted.find("\"c\":\"74781.08000000\"") == std::string::npos ||
        formatted.find("\"b\":\"74781.07000000\"") == std::string::npos ||
        formatted.find("\"a\":\"74781.09000000\"") == std::string::npos ||
        formatted.find("\"v\":\"13858.40324000\"") == std::string::npos) {
        std::cerr << "format_binance_ticker() did not keep the expected fields\n";
        return EXIT_FAILURE;
    }

    return EXIT_SUCCESS;
}
