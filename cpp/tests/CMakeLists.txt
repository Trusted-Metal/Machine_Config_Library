add_executable(machine_config_tests
  test_models.cpp
  test_reader.cpp
  test_writer.cpp
  test_builder.cpp          # §4.20
  test_schema.cpp           # §4.21
  test_capabilities.cpp     # stable model facade
  test_adapter_migration.cpp # mock v1.1 adapter (AV-09-11)
  test_v1_1_adapter.cpp     # real v1.1 adapter (Phase 1 + Phase 2)
  test_power_characterization.cpp # shared shape-conversion functions (Phase 2)
  test_version_adapter_isolation.cpp # cross-version import guard
)

target_link_libraries(machine_config_tests PRIVATE
  machine_config
  Catch2::Catch2WithMain
  nlohmann_json_schema_validator)

target_include_directories(machine_config_tests PRIVATE ${picosha2_SOURCE_DIR})
# Absolute path to shared fixtures so tests can open HDF5 files regardless of cwd.
target_compile_definitions(machine_config_tests PRIVATE
  FIXTURES_DIR="${PROJECT_SOURCE_DIR}/../fixtures"
  SCHEMA_DIR="${PROJECT_SOURCE_DIR}/../schema"
  VALIDATION_FIXTURES_DIR="${PROJECT_SOURCE_DIR}/../docs/validation/fixtures"
  CPP_SOURCE_DIR="${PROJECT_SOURCE_DIR}")

include(Catch)
catch_discover_tests(machine_config_tests)
