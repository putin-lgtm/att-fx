#include "ccp/app.hpp"

#include "App.h"

#include <atomic>
#include <chrono>
#include <cstdlib>
#include <iostream>
#include <string>
#include <string_view>
#include <thread>

#ifdef _WIN32
#include <windows.h>
#endif

namespace {

std::atomic<bool> g_stop_requested{false};

#ifdef _WIN32
BOOL WINAPI handle_console_signal(DWORD signal) {
	switch (signal) {
	case CTRL_C_EVENT:
	case CTRL_BREAK_EVENT:
	case CTRL_CLOSE_EVENT:
	case CTRL_SHUTDOWN_EVENT:
		g_stop_requested.store(true);
		return TRUE;
	default:
		return FALSE;
	}
}
#endif

std::string make_error_json(const std::string& message) {
	return std::string("{\"status\":\"error\",\"message\":\"") + message + "\"}";
}

std::string extract_json_string_field(std::string_view payload, std::string_view key) {
	const std::string needle = std::string("\"") + std::string(key) + "\":\"";
	const size_t key_start = payload.find(needle);
	if (key_start == std::string_view::npos) {
		return {};
	}

	const size_t value_start = key_start + needle.size();
	const size_t value_end = payload.find('"', value_start);
	if (value_end == std::string_view::npos) {
		return {};
	}

	return std::string(payload.substr(value_start, value_end - value_start));
}

} // namespace

namespace ccp {

std::string format_binance_ticker(std::string_view payload) {
	const std::string last_price = extract_json_string_field(payload, "c");
	const std::string bid_price = extract_json_string_field(payload, "b");
	const std::string ask_price = extract_json_string_field(payload, "a");
	const std::string volume = extract_json_string_field(payload, "v");

	if (last_price.empty() || bid_price.empty() || ask_price.empty() || volume.empty()) {
		return make_error_json("missing expected Binance ticker fields");
	}

	return std::string("{\"c\":\"") + last_price +
		   "\",\"b\":\"" + bid_price +
		   "\",\"a\":\"" + ask_price +
		   "\",\"v\":\"" + volume + "\"}";
}

int fetch_binance_ticker() {
#ifdef _WIN32
	g_stop_requested.store(false);
	SetConsoleCtrlHandler(handle_console_signal, TRUE);
#endif

	struct PerSocketData {
		uint64_t connection_id = 0;
	};

	std::atomic<uint64_t> next_connection_id{0};
	std::atomic<uint64_t> open_connections{0};
	std::atomic<uint64_t> received_messages{0};
	std::atomic<bool> listen_failed{false};
	std::atomic<bool> server_ready{false};

	std::thread heartbeat([&]() {
		uint64_t heartbeat_count = 0;
		while (!g_stop_requested.load()) {
			std::this_thread::sleep_for(std::chrono::seconds(1));
			if (g_stop_requested.load()) {
				break;
			}

			std::cerr << "[uWS] heartbeat=" << ++heartbeat_count
					  << " connections=" << open_connections.load()
					  << " messages=" << received_messages.load()
					  << " endpoint=ws://127.0.0.1:9001"
					  << '\n';
			std::cerr.flush();
		}
	});

	uWS::App()
		.get("/health", [](auto *res, auto */*req*/) {
			res->end("ok");
		})
		.ws<PerSocketData>("/*", {
			.compression = uWS::CompressOptions(uWS::DISABLED),
			.maxPayloadLength = 16 * 1024,
			.idleTimeout = 32,
			.maxBackpressure = 64 * 1024,
			.closeOnBackpressureLimit = false,
			.resetIdleTimeoutOnSend = false,
			.sendPingsAutomatically = true,
			.upgrade = nullptr,
			.open = [&](auto *ws) {
				auto *data = ws->getUserData();
				data->connection_id = ++next_connection_id;
				++open_connections;

				std::cerr << "[uWS] open connection_id=" << data->connection_id
						  << " active=" << open_connections.load() << '\n';
				std::cerr.flush();
			},
			.message = [&](auto *ws, std::string_view message, uWS::OpCode opCode) {
				++received_messages;

				std::cerr << "[uWS] message connection_id=" << ws->getUserData()->connection_id
						  << " size=" << message.size()
						  << " payload=" << message << '\n';
				std::cerr.flush();

				ws->send(message, opCode, false);
			},
			.drain = nullptr,
			.ping = nullptr,
			.pong = nullptr,
			.close = [&](auto *ws, int code, std::string_view message) {
				if (open_connections.load() > 0) {
					--open_connections;
				}

				std::cerr << "[uWS] close connection_id=" << ws->getUserData()->connection_id
						  << " code=" << code
						  << " reason=" << message
						  << " active=" << open_connections.load() << '\n';
				std::cerr.flush();
			}
		})
		.listen(9001, [&](auto *listen_socket) {
			if (listen_socket) {
				server_ready.store(true);
				std::cerr << "[uWS] listening on ws://127.0.0.1:9001" << '\n';
				std::cerr << "[uWS] health check: http://127.0.0.1:9001/health" << '\n';
				std::cerr.flush();
			} else {
				listen_failed.store(true);
				g_stop_requested.store(true);
				std::cerr << "[uWS] failed to listen on port 9001" << '\n';
				std::cerr.flush();
			}
		})
		.run();

	g_stop_requested.store(true);
	if (heartbeat.joinable()) {
		heartbeat.join();
	}

#ifdef _WIN32
	SetConsoleCtrlHandler(handle_console_signal, FALSE);
#endif

	return (server_ready.load() && !listen_failed.load()) ? EXIT_SUCCESS : EXIT_FAILURE;
}

} // namespace ccp
