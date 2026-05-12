cmake_minimum_required(VERSION 3.20)
project(phys2d
    VERSION 0.1.0
    LANGUAGES CXX
    DESCRIPTION "ADSD CS-05: 2D rigid body physics core"
)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

# Strict warnings (5-gate Gate 2 will fail on warnings)
if(CMAKE_CXX_COMPILER_ID STREQUAL "GNU" OR CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
    add_compile_options(-Wall -Wextra -Wpedantic -Werror -Wshadow -Wnon-virtual-dtor)
endif()

# Public library
add_library(phys2d
    src/vec2.cpp
    src/aabb.cpp
    src/body.cpp
    src/world.cpp
)
target_include_directories(phys2d PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)

# Tests via GoogleTest
include(FetchContent)
FetchContent_Declare(
    googletest
    GIT_REPOSITORY https://github.com/google/googletest.git
    GIT_TAG v1.15.2
)
FetchContent_MakeAvailable(googletest)

enable_testing()
add_executable(phys2d_tests
    tests/test_vec2.cpp
    tests/test_aabb.cpp
)
target_link_libraries(phys2d_tests PRIVATE phys2d gtest gtest_main)
include(GoogleTest)
gtest_discover_tests(phys2d_tests)

# Emscripten wasm target (only when building with emcmake)
if(EMSCRIPTEN)
    add_executable(phys2d_wasm src/wasm_glue.cpp)
    target_link_libraries(phys2d_wasm PRIVATE phys2d)
    set_target_properties(phys2d_wasm PROPERTIES
        OUTPUT_NAME "phys2d"
        SUFFIX ".js"
        LINK_FLAGS "-sMODULARIZE=1 -sEXPORT_NAME='Phys2D' -sEXPORTED_FUNCTIONS=['_World_step','_World_add_body','_World_get_bodies','_malloc','_free'] -sEXPORTED_RUNTIME_METHODS=['ccall','cwrap','HEAP32','HEAPF32'] -sALLOW_MEMORY_GROWTH=1 --no-entry"
    )
endif()
