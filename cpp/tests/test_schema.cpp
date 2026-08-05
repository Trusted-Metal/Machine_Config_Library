// Catch2 schema validation tests — §4.21 acceptance criteria.
#include <catch2/catch_test_macros.hpp>
#include <nlohmann/json.hpp>

#include <filesystem>
#include <string>

#include "machine_config/builder.hpp"
#include "machine_config/reader.hpp"
#include "machine_config/schema.hpp"

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

using namespace machine_config;

static const std::string REF = std::string(FIXTURES_DIR) + "/reference_config.h5";

static std::filesystem::path tmpSchemaPath(const std::string& tag) {
    return std::filesystem::temp_directory_path() /
           ("mc_schema_test_" + tag + ".h5");
}

// ---------------------------------------------------------------------------

// §4.21: reader output for the reference fixture validates against the schema.
TEST_CASE("ValidateReferenceConfigOutput") {
    auto j = nlohmann::json::parse(MachineConfigReader{REF}.toJson());
    auto errors = validate(j);
    REQUIRE(errors.empty());
}

// §4.21: MockConfigBuilder output also satisfies the schema.
TEST_CASE("ValidateMockBuilderOutput") {
    auto out = tmpSchemaPath("mock");
    MockConfigBuilder{}.save(out);
    auto j = nlohmann::json::parse(MachineConfigReader{out}.toJson());
    auto errors = validate(j);
    REQUIRE(errors.empty());
    std::filesystem::remove(out);
}

// §4.21: an empty JSON object lacks all required fields and must fail validation.
TEST_CASE("ValidateEmptyConfigFails") {
    auto errors = validate(nlohmann::json::object());
    REQUIRE_FALSE(errors.empty());
}
