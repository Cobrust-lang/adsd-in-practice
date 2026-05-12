cmake_minimum_required(VERSION 3.20)
project(lfq
    VERSION 0.1.0
    LANGUAGES CXX
    DESCRIPTION "ADSD CS-06: lock-free SPSC + MPMC queues + benchmark"
)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

# Strict warnings
if(CMAKE_CXX_COMPILER_ID STREQUAL "GNU" OR CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
    add_compile_options(-Wall -Wextra -Wpedantic -Werror -Wshadow -Wcast-align)
endif()

# Header-only library
add_library(lfq INTERFACE)
target_include_directories(lfq INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_compile_features(lfq INTERFACE cxx_std_20)

# GoogleTest for unit + stress tests
include(FetchContent)
FetchContent_Declare(
    googletest
    GIT_REPOSITORY https://github.com/google/googletest.git
    GIT_TAG v1.15.2
)
FetchContent_MakeAvailable(googletest)

enable_testing()
add_executable(lfq_tests
    tests/test_spsc.cpp
)
target_link_libraries(lfq_tests PRIVATE lfq gtest gtest_main)
include(GoogleTest)
gtest_discover_tests(lfq_tests)

# google-benchmark (FetchContent; opt-in via -DLFQ_BENCH=ON)
option(LFQ_BENCH "Build google-benchmark targets" OFF)
if(LFQ_BENCH)
    set(BENCHMARK_ENABLE_TESTING OFF CACHE BOOL "" FORCE)
    FetchContent_Declare(
        benchmark
        GIT_REPOSITORY https://github.com/google/benchmark.git
        GIT_TAG v1.9.0
    )
    FetchContent_MakeAvailable(benchmark)
    add_executable(queue_bench bench/queue_bench.cpp)
    target_link_libraries(queue_bench PRIVATE lfq benchmark::benchmark)
endif()
