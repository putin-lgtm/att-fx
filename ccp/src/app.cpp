#include "ccp/app.hpp"

#include "App.h"

#include <atomic>
#include <chrono>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <sstream>
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

std::string make_log_prefix() {
	const auto now = std::chrono::system_clock::now();
	const auto now_time = std::chrono::system_clock::to_time_t(now);
	std::tm local_time{};
#ifdef _WIN32
	localtime_s(&local_time, &now_time);
#else
	localtime_r(&now_time, &local_time);
#endif

	const auto milliseconds = std::chrono::duration_cast<std::chrono::milliseconds>(
		now.time_since_epoch()
	) % 1000;

	std::ostringstream builder;
	builder << '['
			<< std::put_time(&local_time, "%Y-%m-%d:%H:%M:%S")
			<< ':' << std::setw(3) << std::setfill('0') << milliseconds.count()
			<< " mms] ";
	return builder.str();
}

std::string make_synthetic_tick(uint64_t sequence) {
	const double base_price = 75000.0 + static_cast<double>(sequence % 200) * 0.25;
	const double bid_price = base_price - 0.01;
	const double ask_price = base_price + 0.01;
	const double volume = 14000.0 + static_cast<double>((sequence * 17) % 1000) / 10.0;

	std::ostringstream builder;
	builder << std::fixed << std::setprecision(8)
			<< "{\"kind\":\"synthetic_ticker\","
			<< "\"seq\":" << sequence << ','
			<< "\"c\":\"" << base_price << "\"," 
			<< "\"b\":\"" << bid_price << "\"," 
			<< "\"a\":\"" << ask_price << "\"," 
			<< "\"v\":\"" << volume << "\"}";
	return builder.str();
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
	std::atomic<uint64_t> inbound_messages{0};
	std::atomic<uint64_t> published_messages{0};
	std::atomic<uint64_t> synthetic_sequence{0};
	std::atomic<bool> listen_failed{false};
	std::atomic<bool> server_ready{false};
	std::mutex latest_message_mutex;
	std::string latest_message;
	uWS::Loop *app_loop = nullptr;
	us_listen_socket_t *listen_socket = nullptr;
	uWS::App app;

	std::thread synthetic_publisher([&]() {
		while (!g_stop_requested.load()) {
			std::this_thread::sleep_for(std::chrono::milliseconds(1000));
			if (g_stop_requested.load()) {
				break;
			}

			if (!server_ready.load() || app_loop == nullptr) {
				continue;
			}

			const std::string payload = make_synthetic_tick(++synthetic_sequence);
			app_loop->defer([&app, &published_messages, &latest_message, &latest_message_mutex, payload]() {
				app.publish("synthetic", payload, uWS::OpCode::TEXT, false);
				++published_messages;
				{
					std::lock_guard<std::mutex> guard(latest_message_mutex);
					latest_message = payload;
				}
				std::cerr << make_log_prefix() << "latest=" << payload << '\n';
				std::cerr.flush();
			});
		}
	});

	app_loop = uWS::Loop::get();

	app
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
				ws->subscribe("synthetic");

				std::cerr << make_log_prefix() << "open connection_id=" << data->connection_id
					  << " active=" << open_connections.load()
					  << " auto_subscribed=synthetic" << '\n';
				std::cerr.flush();
			},
			.message = [&](auto *ws, std::string_view message, uWS::OpCode opCode) {
				++inbound_messages;

				std::cerr << make_log_prefix() << "message connection_id=" << ws->getUserData()->connection_id
						  << " size=" << message.size()
						  << " payload=" << message << '\n';
				std::cerr.flush();

				if (message == "ping") {
					ws->send("pong", uWS::OpCode::TEXT, false);
					return;
				}

				ws->send(message, opCode, false);
			},
			.drain = nullptr,
			.ping = nullptr,
			.pong = nullptr,
			.close = [&](auto *ws, int code, std::string_view message) {
				if (open_connections.load() > 0) {
					--open_connections;
				}

				std::cerr << make_log_prefix() << "close connection_id=" << ws->getUserData()->connection_id
						  << " code=" << code
						  << " reason=" << message
						  << " active=" << open_connections.load() << '\n';
				std::cerr.flush();
			}
		})
		.listen(9001, [&](auto *socket) {
			if (socket) {
				listen_socket = socket;
				server_ready.store(true);
				std::cerr << make_log_prefix() << "listening on ws://127.0.0.1:9001" << '\n';
				std::cerr << make_log_prefix() << "health check: http://127.0.0.1:9001/health" << '\n';
				std::cerr << make_log_prefix() << "auto-publishing synthetic ticks once per second on topic 'synthetic'" << '\n';
				std::cerr.flush();
			} else {
				listen_failed.store(true);
				g_stop_requested.store(true);
				std::cerr << make_log_prefix() << "failed to listen on port 9001" << '\n';
				std::cerr.flush();
			}
		})
		.run();

	g_stop_requested.store(true);
	if (synthetic_publisher.joinable()) {
		synthetic_publisher.join();
	}

#ifdef _WIN32
	SetConsoleCtrlHandler(handle_console_signal, FALSE);
#endif

	return (server_ready.load() && !listen_failed.load()) ? EXIT_SUCCESS : EXIT_FAILURE;
}

} // namespace ccp
