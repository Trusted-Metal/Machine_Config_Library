add_executable(machine_config_tests
  test_models.cpp
  test_reader.cpp
)

target_link_libraries(machine_config_tests PRIVATE
  machine_config
  Catch2::Catch2WithMain)

target_include_directories(machine_config_tests PRIVATE ${picosha2_SOURCE_DIR})
# Absolute path to shared fixtures so tests can open HDF5 files regardless of cwd.
target_compile_definitions(machine_config_tests PRIVATE
  FIXTURES_DIR="${CMAKE_SOURCE_DIR}/../fixtures")

include(Catch)
catch_discover_tests(machine_config_tests)
