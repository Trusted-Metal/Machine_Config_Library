cmake_minimum_required(VERSION 3.20)
project(mcl_cpp_validation CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(FetchContent)

# HDF5 is found transitively via HighFive, pulled in by the machine_config
# subdirectory below — matches how cpp/CMakeLists.txt itself does it (see its
# comment: "HDF5 is found transitively via HighFive").
add_subdirectory(
  "${CMAKE_CURRENT_SOURCE_DIR}/../../../../cpp"
  machine_config_build)

# picosha2 is used by S-02/S-05 to cross-check correction-grid hashes against
# the values already recorded in Python's/Node's/Rust's results.md. It is
# NOT part of the public `machine_config` INTERFACE target (only the CLI and
# test binary get its include path added, individually, inside
# cpp/CMakeLists.txt) — a real external consumer needing SHA-256 has to bring
# its own, same as this app does here.
FetchContent_Declare(picosha2
  GIT_REPOSITORY https://github.com/okdshin/PicoSHA2.git
  GIT_TAG        master)
FetchContent_MakeAvailable(picosha2)

add_executable(validation_app
  src/main.cpp
  src/scenarios/scenarios.cpp)
target_link_libraries(validation_app PRIVATE machine_config)
target_include_directories(validation_app PRIVATE ${picosha2_SOURCE_DIR})
