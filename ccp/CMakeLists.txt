cmake_minimum_required(VERSION 3.20)

project(ccp VERSION 0.1.0 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(LIBUV_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(LIBUV_BUILD_BENCH OFF CACHE BOOL "" FORCE)
set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)

add_subdirectory(deps/libuv EXCLUDE_FROM_ALL)

add_library(ccp_usockets STATIC
    deps/uWebSockets/uSockets/src/bsd.c
    deps/uWebSockets/uSockets/src/context.c
    deps/uWebSockets/uSockets/src/loop.c
    deps/uWebSockets/uSockets/src/socket.c
    deps/uWebSockets/uSockets/src/udp.c
    deps/uWebSockets/uSockets/src/eventing/libuv.c
)

target_include_directories(ccp_usockets
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/deps/uWebSockets/src
        ${CMAKE_CURRENT_SOURCE_DIR}/deps/uWebSockets/uSockets/src
)

target_compile_definitions(ccp_usockets
    PUBLIC
        LIBUS_USE_LIBUV
        LIBUS_NO_SSL
        UWS_NO_ZLIB
        WIN32_LEAN_AND_MEAN
        NOMINMAX
)

target_link_libraries(ccp_usockets
    PUBLIC
        uv_a
        ws2_32
        iphlpapi
        psapi
        userenv
    PRIVATE
        advapi32
)

add_library(ccp_core
    src/app.cpp
)

target_include_directories(ccp_core
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/include
        ${CMAKE_CURRENT_SOURCE_DIR}/deps/uWebSockets/src
        ${CMAKE_CURRENT_SOURCE_DIR}/deps/uWebSockets/uSockets/src
)

target_link_libraries(ccp_core
    PRIVATE
        ccp_usockets
)

add_executable(ccp_app
    src/main.cpp
)

target_link_libraries(ccp_app
    PRIVATE
        ccp_core
)

add_executable(ccp_uws_app
    src/main.cpp
)

target_link_libraries(ccp_uws_app
    PRIVATE
        ccp_core
)

if (WIN32)
    add_custom_command(TARGET ccp_app PRE_LINK
        COMMAND powershell -NoProfile -ExecutionPolicy Bypass -Command "$processes = Get-Process ccp_app -ErrorAction SilentlyContinue; if ($processes) { $processes | Stop-Process -Force }; exit 0"
        VERBATIM
    )

    add_custom_command(TARGET ccp_uws_app PRE_LINK
        COMMAND powershell -NoProfile -ExecutionPolicy Bypass -Command "$processes = Get-Process ccp_uws_app -ErrorAction SilentlyContinue; if ($processes) { $processes | Stop-Process -Force }; exit 0"
        VERBATIM
    )
endif()

add_custom_target(build_ccp_app
    DEPENDS ccp_app
)

add_custom_target(stop_ccp_app
    COMMAND powershell -NoProfile -ExecutionPolicy Bypass -Command "$processes = Get-Process ccp_app -ErrorAction SilentlyContinue; if ($processes) { $processes | Stop-Process -Force }; exit 0"
    USES_TERMINAL
)

add_custom_target(run_ccp_app
    COMMAND powershell -NoProfile -ExecutionPolicy Bypass -Command "$processes = Get-Process ccp_app -ErrorAction SilentlyContinue; if ($processes) { $processes | Stop-Process -Force }; Start-Process -FilePath '$<TARGET_FILE:ccp_app>' -WorkingDirectory '${CMAKE_CURRENT_SOURCE_DIR}'; Write-Host 'Started ccp_app in background.'; exit 0"
    DEPENDS ccp_app
    USES_TERMINAL
)

add_custom_target(build_ccp_uws_app
    DEPENDS ccp_uws_app
)

add_custom_target(stop_ccp_uws_app
    COMMAND powershell -NoProfile -ExecutionPolicy Bypass -Command "$processes = Get-Process ccp_uws_app -ErrorAction SilentlyContinue; if ($processes) { $processes | Stop-Process -Force }; exit 0"
    USES_TERMINAL
)

add_custom_target(run_ccp_uws_app
    COMMAND $<TARGET_FILE:ccp_uws_app>
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    DEPENDS ccp_uws_app
    USES_TERMINAL
)

enable_testing()

add_executable(ccp_smoke_test
    tests/smoke_test.cpp
)

target_link_libraries(ccp_smoke_test
    PRIVATE
        ccp_core
)

add_custom_target(build_ccp_smoke_test
    DEPENDS ccp_smoke_test
)

add_custom_target(run_ccp_smoke_test
    COMMAND $<TARGET_FILE:ccp_smoke_test>
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    USES_TERMINAL
)

add_test(
    NAME ccp_smoke_test
    COMMAND ccp_smoke_test
)
