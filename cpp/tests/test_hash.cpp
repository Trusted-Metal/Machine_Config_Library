// Catch2 configuration-hash tests — CONFIGURATION_HASH_PLAN.md Step 5.
#include <catch2/catch_test_macros.hpp>

#include <regex>
#include <string>

#include "machine_config/hash.hpp"
#include "machine_config/reader.hpp"

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

using namespace machine_config;

static const std::string REF = std::string(FIXTURES_DIR) + "/reference_config.h5";

static bool isHexDigest64(const std::string& s) {
    static const std::regex re("^[0-9a-f]{64}$");
    return std::regex_match(s, re);
}

TEST_CASE("ComputeConfigurationHashStableAndWellFormed") {
    auto cfg = MachineConfigReader{REF}.parse();
    auto a = computeConfigurationHash(cfg);
    auto b = computeConfigurationHash(cfg);
    REQUIRE(a == b);
    REQUIRE(isHexDigest64(a));
}

TEST_CASE("ComputeConfigurationHashIgnoresItsOwnStoredValue") {
    auto cfg = MachineConfigReader{REF}.parse();
    auto a = computeConfigurationHash(cfg);
    auto tampered = cfg;
    tampered.meta.configuration_hash = std::string(64, 'f');
    auto b = computeConfigurationHash(tampered);
    REQUIRE(a == b);
}

TEST_CASE("ComputeConfigurationHashIgnoresExportDate") {
    auto cfg = MachineConfigReader{REF}.parse();
    auto a = computeConfigurationHash(cfg);
    auto changed = cfg;
    changed.meta.export_date = "2099-01-01T00:00:00.000Z";
    auto b = computeConfigurationHash(changed);
    REQUIRE(a == b);
}

TEST_CASE("ComputeConfigurationHashIgnoresIsValid") {
    auto cfg = MachineConfigReader{REF}.parse();
    auto a = computeConfigurationHash(cfg);
    auto changed = cfg;
    changed.meta.is_valid = true;
    auto b = computeConfigurationHash(changed);
    REQUIRE(a == b);
}

TEST_CASE("ComputeConfigurationHashChangesWhenAScalarFieldChanges") {
    auto cfg = MachineConfigReader{REF}.parse();
    auto a = computeConfigurationHash(cfg);
    auto changed = cfg;
    changed.machine.build_plate_x =
        changed.machine.build_plate_x.value_or(0.0) + 1.0;
    auto b = computeConfigurationHash(changed);
    REQUIRE(a != b);
}

TEST_CASE("ComputeConfigurationHashIgnoresBinaryGridData") {
    auto withBinary = MachineConfigReader{REF}.parseWithBinary();
    auto withoutBinary = MachineConfigReader{REF}.parse();
    REQUIRE(computeConfigurationHash(withBinary) == computeConfigurationHash(withoutBinary));
}

TEST_CASE("ReaderSetsIsValidTrueForSelfWrittenFileAndFalseForReference") {
    // Permanent regression guard, mirroring every other language's
    // equivalent test: reference_config.h5 is externally-authored and is
    // expected to read back invalid *forever* — that's the correct,
    // intended result (see hash.hpp's module docs), not a defect to fix.
    auto ref = MachineConfigReader{REF}.parse();
    REQUIRE(ref.meta.is_valid.has_value());
    REQUIRE(*ref.meta.is_valid == false);
}
