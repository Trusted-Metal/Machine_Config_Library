cmake_minimum_required(VERSION 3.20)
project(machine_config VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

include(FetchContent)

# HighFive — header-only HDF5 C++ wrapper (Boost / Eigen / xtensor disabled)
set(HIGHFIVE_USE_BOOST      OFF CACHE BOOL "" FORCE)
set(HIGHFIVE_USE_EIGEN      OFF CACHE BOOL "" FORCE)
set(HIGHFIVE_USE_XTENSOR    OFF CACHE BOOL "" FORCE)
set(HIGHFIVE_USE_OPENCV     OFF CACHE BOOL "" FORCE)
set(HIGHFIVE_EXAMPLES       OFF CACHE BOOL "" FORCE)
set(HIGHFIVE_UNIT_TESTS     OFF CACHE BOOL "" FORCE)
FetchContent_Declare(HighFive
  GIT_REPOSITORY https://github.com/BlueBrain/HighFive.git
  GIT_TAG        v2.10.0)
FetchContent_MakeAvailable(HighFive)

# nlohmann/json — header-only JSON library
FetchContent_Declare(nlohmann_json
  URL https://github.com/nlohmann/json/releases/download/v3.11.3/json.tar.xz)
FetchContent_MakeAvailable(nlohmann_json)

# CLI11 — header-only argument parser (used by the CLI executable in §4.8)
FetchContent_Declare(CLI11
  GIT_REPOSITORY https://github.com/CLIUtils/CLI11.git
  GIT_TAG        v2.4.1)
FetchContent_MakeAvailable(CLI11)

# Catch2 — unit test framework
FetchContent_Declare(Catch2
  GIT_REPOSITORY https://github.com/catchorg/Catch2.git
  GIT_TAG        v3.7.1)
FetchContent_MakeAvailable(Catch2)

# picosha2 — header-only SHA-256 (used in §4.11 tests and §4.16 correction-hash CLI)
FetchContent_Declare(picosha2
  GIT_REPOSITORY https://github.com/okdshin/PicoSHA2.git
  GIT_TAG        master)
FetchContent_MakeAvailable(picosha2)

# json-schema-validator — JSON Schema draft 4/6/7 validation (§4.21)
set(JSON_VALIDATOR_INSTALL OFF CACHE BOOL "" FORCE)
FetchContent_Declare(json_schema_validator
  GIT_REPOSITORY https://github.com/pboettch/json-schema-validator.git
  GIT_TAG        2.3.0)
FetchContent_MakeAvailable(json_schema_validator)

# HDF5 is found transitively via HighFive; the version gate above catches < 1.12.
# Ubuntu CI: cmake -DCMAKE_PREFIX_PATH=<hdf5-install> (apt ships 1.10.10, incompatible)
# Windows:   cmake -DCMAKE_TOOLCHAIN_FILE=<vcpkg>/scripts/buildsystems/vcpkg.cmake

# Header-only library — consumers link machine_config and get HighFive + nlohmann transitively
add_library(machine_config INTERFACE)
target_include_directories(machine_config INTERFACE include)
target_link_libraries(machine_config INTERFACE
  HighFive
  nlohmann_json::nlohmann_json)
if(MSVC)
  target_compile_options(machine_config INTERFACE /W4 /utf-8)
else()
  target_compile_options(machine_config INTERFACE -Wall -Wextra -Wpedantic)
endif()

# Tests
enable_testing()
add_subdirectory(tests)

# CLI executable (§4.8)
add_executable(machine_config_cli src/main.cpp)
target_link_libraries(machine_config_cli PRIVATE machine_config CLI11::CLI11)
target_include_directories(machine_config_cli PRIVATE ${picosha2_SOURCE_DIR})

# §4.17: full_workflow example
add_executable(full_workflow ${PROJECT_SOURCE_DIR}/../examples/full_workflow/cpp/main.cpp)
target_link_libraries(full_workflow PRIVATE machine_config)
target_compile_definitions(full_workflow PRIVATE
  EXAMPLES_DIR="${PROJECT_SOURCE_DIR}/../examples")

# §4.18: quickstart example
add_executable(quickstart ${PROJECT_SOURCE_DIR}/../examples/quickstart/cpp/main.cpp)
target_link_libraries(quickstart PRIVATE machine_config)
target_compile_definitions(quickstart PRIVATE
  EXAMPLES_DIR="${PROJECT_SOURCE_DIR}/../examples")

# Examples — uncomment in §4.18 (quickstart)
# add_subdirectory(examples)
