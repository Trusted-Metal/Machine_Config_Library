// Catch2 tests for models.hpp — §4.7 acceptance criteria.
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "machine_config/models.hpp"

using namespace machine_config;

// ---------------------------------------------------------------------------
// NullOptionalSerializesAsNull
// ---------------------------------------------------------------------------
TEST_CASE("NullOptionalSerializesAsNull") {
    Collimator c;
    c.manufacturer   = "ACME";
    c.model          = "CL-100";
    c.serial_number  = "12345";
    c.focal_length   = std::nullopt;
    c.focal_length_unit = std::nullopt;

    nlohmann::json j = c;
    REQUIRE(j.contains("focal_length"));
    REQUIRE(j["focal_length"].is_null());
    REQUIRE(j.contains("focal_length_unit"));
    REQUIRE(j["focal_length_unit"].is_null());
}

// ---------------------------------------------------------------------------
// PresentOptionalSerializesValue
// ---------------------------------------------------------------------------
TEST_CASE("PresentOptionalSerializesValue") {
    Collimator c;
    c.manufacturer   = "ACME";
    c.model          = "CL-100";
    c.serial_number  = "12345";
    c.focal_length   = 100.0;
    c.focal_length_unit = "mm";

    nlohmann::json j = c;
    REQUIRE(j["focal_length"].get<double>() == 100.0);
    REQUIRE(j["focal_length_unit"].get<std::string>() == "mm");
    REQUIRE(j["manufacturer"].get<std::string>() == "ACME");
}

// ---------------------------------------------------------------------------
// MachineConfigRoundtrip
// Serialise a minimal MachineConfig to JSON and deserialise it back.
// Verifies that to_json / from_json are inverse operations for required fields.
// ---------------------------------------------------------------------------
TEST_CASE("MachineConfigRoundtrip") {
    MachineConfig cfg;
    cfg.meta.schema_version    = "1.0";
    cfg.meta.machine_name      = "TestMachine";
    cfg.meta.manufacturer      = "ACME";
    cfg.meta.model             = "TM-01";
    cfg.meta.serial_number     = "SN001";
    cfg.meta.file_version      = "1.0";
    cfg.meta.export_date       = "2026-08-04";
    cfg.meta.configuration_hash= "abc123";
    cfg.meta.extra             = nlohmann::json::object();

    cfg.machine.machine_name  = "TestMachine";
    cfg.machine.manufacturer  = "ACME";
    cfg.machine.model         = "TM-01";
    cfg.machine.serial_number = "SN001";
    cfg.machine.build_plate_x = 150.0;
    cfg.machine.build_plate_radius = std::nullopt;

    cfg.extensions.clear();

    nlohmann::json j = cfg;

    // extensions must be absent (not null) when empty.
    REQUIRE_FALSE(j.contains("extensions"));
    REQUIRE(j["meta"]["machine_name"].get<std::string>() == "TestMachine");
    REQUIRE(j["machine"]["build_plate_x"].get<double>() == 150.0);
    REQUIRE(j["machine"]["build_plate_radius"].is_null());

    MachineConfig cfg2 = j.get<MachineConfig>();
    REQUIRE(cfg2.meta.machine_name == "TestMachine");
    REQUIRE(cfg2.machine.build_plate_x == std::optional<double>{150.0});
    REQUIRE(cfg2.machine.build_plate_radius == std::nullopt);
    REQUIRE(cfg2.extensions.empty());
}
