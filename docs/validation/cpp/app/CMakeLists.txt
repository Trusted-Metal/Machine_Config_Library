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
  src/scenarios/s01_read_scalars.cpp
  src/scenarios/s02_read_binary.cpp
  src/scenarios/s03_read_real.cpp
  src/scenarios/s04_write_modify.cpp
  src/scenarios/s05_binary_roundtrip.cpp
  src/scenarios/s06_builder.cpp
  src/scenarios/s07_opcua.cpp
  src/scenarios/s08_drastic_change.cpp
  src/scenarios/s09_type_exports.cpp
  src/scenarios/av01_unknown_version.cpp
  src/scenarios/av02_missing_version.cpp
  src/scenarios/av03_future_version.cpp
  src/scenarios/av04_missing_group.cpp
  src/scenarios/av05_corrupt_scalar.cpp
  src/scenarios/av06_whitespace_version.cpp
  src/scenarios/av07_empty_version.cpp
  src/scenarios/av08_version_fidelity.cpp)
target_link_libraries(validation_app PRIVATE machine_config)
target_include_directories(validation_app PRIVATE ${picosha2_SOURCE_DIR})
