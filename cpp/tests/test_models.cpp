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
// NanCellSerializesAsNull
// Grid cells are std::optional<double>; nullopt (representing HDF5 NaN) → JSON null.
// ---------------------------------------------------------------------------
TEST_CASE("NanCellSerializesAsNull") {
    // 1×1×2 grid: first cell nullopt (NaN), second cell has a value.
    Grid3D grid = {{{std::nullopt, GridCell{1.5}}}};

    ClearBox cb;
    cb.ip_address       = "192.168.1.1";
    cb.correction_data  = grid;

    nlohmann::json j = cb;
    REQUIRE(j.contains("correction_data"));
    REQUIRE(j["correction_data"][0][0][0].is_null());
    REQUIRE_THAT(j["correction_data"][0][0][1].get<double>(),
                 Catch::Matchers::WithinRel(1.5));
}

// ---------------------------------------------------------------------------
// OmitCorrectionDataWhenNullopt
// correction_data must be absent from JSON (not null) when not populated.
// ---------------------------------------------------------------------------
TEST_CASE("OmitCorrectionDataWhenNullopt") {
    ClearBox cb;
    cb.ip_address      = "192.168.1.1";
    cb.correction_data = std::nullopt;

    nlohmann::json j = cb;
    REQUIRE_FALSE(j.contains("correction_data"));
    REQUIRE_FALSE(j.contains("inverse_correction_data"));
}

// ---------------------------------------------------------------------------
// CorrectionDataFlatIndexing
// Element at (i, j, k) lives at data[i*d1*d2 + j*d2 + k].
// ---------------------------------------------------------------------------
TEST_CASE("CorrectionDataFlatIndexing") {
    CorrectionData cd;
    cd.shape = {3, 4, 2};
    const auto [d0, d1, d2] = cd.shape;
    cd.data.resize(d0 * d1 * d2, 0.0);

    // Write known value at (1, 2, 1).
    const std::size_t i = 1, j = 2, k = 1;
    cd.data[i * d1 * d2 + j * d2 + k] = 42.0;

    REQUIRE(cd.data[1 * 4 * 2 + 2 * 2 + 1] == 42.0);
    REQUIRE(cd.data.size() == d0 * d1 * d2);
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

    cfg.opcua = std::nullopt;

    nlohmann::json j = cfg;

    // opcua must be absent (not null) when nullopt.
    REQUIRE_FALSE(j.contains("opcua"));
    REQUIRE(j["meta"]["machine_name"].get<std::string>() == "TestMachine");
    REQUIRE(j["machine"]["build_plate_x"].get<double>() == 150.0);
    REQUIRE(j["machine"]["build_plate_radius"].is_null());

    MachineConfig cfg2 = j.get<MachineConfig>();
    REQUIRE(cfg2.meta.machine_name == "TestMachine");
    REQUIRE(cfg2.machine.build_plate_x == std::optional<double>{150.0});
    REQUIRE(cfg2.machine.build_plate_radius == std::nullopt);
    REQUIRE(cfg2.opcua == std::nullopt);
}
