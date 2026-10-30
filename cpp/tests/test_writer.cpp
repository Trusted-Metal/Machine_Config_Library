// Catch2 writer tests — §4.14 acceptance criteria.
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <nlohmann/json.hpp>
#include <picosha2.h>

#include <filesystem>
#include <stdexcept>
#include <string>

#include "machine_config/builder.hpp"
#include "machine_config/reader.hpp"
#include "machine_config/writer.hpp"
#include "clearbox_helpers.hpp"

#include <highfive/H5File.hpp>

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

using namespace machine_config;
namespace ext = machine_config::extension;

static const std::string REF      = std::string(FIXTURES_DIR) + "/reference_config.h5";
static const std::string OPCUA_REF= std::string(FIXTURES_DIR) + "/reference_config_opcua.h5";
static const std::string SENSORS_REF =
    std::string(FIXTURES_DIR) + "/reference_config_synchronous_sensors.h5";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

static std::filesystem::path tmpPath(const std::string& tag) {
    return std::filesystem::temp_directory_path() /
           ("mc_writer_test_" + tag + ".h5");
}

// Build a minimal valid MachineConfig with no ClearBox and no OPCUA.
static MachineConfig makeMinimalConfig() {
    MachineConfig cfg;
    cfg.meta.schema_version     = "v1";
    cfg.meta.machine_name       = "MinimalMachine";
    cfg.meta.manufacturer       = "Acme";
    cfg.meta.model              = "M1";
    cfg.meta.serial_number      = "SN-0001";
    cfg.meta.file_version       = "1.0";
    cfg.meta.export_date        = "2024-01-01T00:00:00Z";
    cfg.meta.configuration_hash = std::string(64, '0');
    cfg.meta.extra              = nlohmann::json::object();

    cfg.machine.machine_name  = "MinimalMachine";
    cfg.machine.manufacturer  = "Acme";
    cfg.machine.model         = "M1";
    cfg.machine.serial_number = "SN-0001";
    cfg.machine.build_plate_x = 200.0;
    cfg.machine.build_plate_x_unit = "mm";
    cfg.machine.build_plate_y = 200.0;
    cfg.machine.build_plate_y_unit = "mm";

    OpticalTrain t;
    t.train_id = "Optical_Train_01";
    t.scanner.manufacturer  = "ScanMfr"; t.scanner.model = "SM1"; t.scanner.serial_number = "S-1";
    t.scanner.axis_configuration = "2D";
    t.light_source.manufacturer = "LSMfr";  t.light_source.model = "LS1"; t.light_source.serial_number = "L-1";
    t.collimator.manufacturer   = "ColMfr"; t.collimator.model = "C1";   t.collimator.serial_number = "C-1";
    t.scanner_card.manufacturer = "SCMfr";  t.scanner_card.model = "K1"; t.scanner_card.serial_number = "K-1";
    cfg.optical_trains.push_back(t);
    return cfg;
}

// ---------------------------------------------------------------------------
// RoundtripAllScalarFields
// Read reference fixture → write to temp → read back → compare key fields.
// ---------------------------------------------------------------------------
TEST_CASE("RoundtripAllScalarFields") {
    auto out = tmpPath("roundtrip");
    MachineConfigReader src{REF};
    auto orig = src.parse();

    REQUIRE_NOTHROW(MachineConfigWriter{orig}.write(out));

    MachineConfigReader back{out};
    auto rb = back.parse();

    // meta
    REQUIRE(rb.meta.machine_name       == orig.meta.machine_name);
    // REF (reference_config.h5) is externally-authored, so its own stored
    // hash is expected to be invalid — but once *this* writer produces a
    // file, that file's stored hash must be real (64 hex chars, not a
    // passthrough of the input) and must read back valid.
    REQUIRE(rb.meta.configuration_hash.size() == 64);
    REQUIRE(rb.meta.configuration_hash != orig.meta.configuration_hash);
    REQUIRE(rb.meta.is_valid.has_value());
    REQUIRE(*rb.meta.is_valid == true);
    REQUIRE(rb.meta.file_version       == orig.meta.file_version);
    REQUIRE(rb.meta.manufacturer       == orig.meta.manufacturer);

    // machine
    REQUIRE(rb.machine.manufacturer == orig.machine.manufacturer);
    REQUIRE(rb.machine.model        == orig.machine.model);
    REQUIRE(rb.machine.build_plate_x.has_value());
    REQUIRE_THAT(*rb.machine.build_plate_x, Catch::Matchers::WithinRel(*orig.machine.build_plate_x));
    REQUIRE(rb.machine.build_plate_x_unit == orig.machine.build_plate_x_unit);
    REQUIRE(rb.machine.recoater_blade_type == orig.machine.recoater_blade_type);

    // trains
    REQUIRE(rb.optical_trains.size() == orig.optical_trains.size());

    const auto& t0  = orig.optical_trains[0];
    const auto& rb0 = rb.optical_trains[0];
    REQUIRE(rb0.train_id == t0.train_id);
    REQUIRE(rb0.thermal_lensing_passed == t0.thermal_lensing_passed);
    REQUIRE_THAT(*rb0.scanner.scan_head_offset_x,
                 Catch::Matchers::WithinRel(*t0.scanner.scan_head_offset_x));
    REQUIRE_THAT(*rb0.scanner.working_distance,
                 Catch::Matchers::WithinRel(*t0.scanner.working_distance));

    // ClearBox: the whole extension tree (attrs with stored dtypes, grids) is bit-stable.
    REQUIRE(ext::canonicalDigest(rb.extensions.at("ClearBox")) ==
            ext::canonicalDigest(orig.extensions.at("ClearBox")));
    REQUIRE(cbtest::text(cbtest::clearbox(rb, 0), "Ip_Address") ==
            cbtest::text(cbtest::clearbox(orig, 0), "Ip_Address"));

    // SFCF
    REQUIRE(rb0.scan_field_correction_file.has_value());
    REQUIRE(rb0.scan_field_correction_file->document_name ==
            t0.scan_field_correction_file->document_name);
    REQUIRE(rb0.scan_field_correction_file->file_size ==
            t0.scan_field_correction_file->file_size);

    // synchronous sensors: reference_config.h5 has none.
    REQUIRE(cbtest::clearbox(rb, 0).children.count("Synchronous_Sensors") == 0);

    std::filesystem::remove(out);
}

// ---------------------------------------------------------------------------
// RoundtripWithoutClearBox
// Minimal config with no ClearBox survives a write → read cycle.
// ---------------------------------------------------------------------------
TEST_CASE("RoundtripWithoutClearBox") {
    auto out = tmpPath("no_clearbox");
    auto cfg = makeMinimalConfig();
    REQUIRE_NOTHROW(MachineConfigWriter{cfg}.write(out));

    MachineConfigReader back{out};
    auto rb = back.parse();

    REQUIRE(rb.meta.machine_name == "MinimalMachine");
    REQUIRE(rb.optical_trains.size() == 1);
    REQUIRE(rb.extensions.empty());

    std::filesystem::remove(out);
}

// ---------------------------------------------------------------------------
// Scanner invert_* flags
// ---------------------------------------------------------------------------

TEST_CASE("RoundtripInvertFlagsDefaultFalseAndOmittedFromJSON") {
    auto out = tmpPath("invert_defaults");
    MachineConfigReader src{REF};
    auto orig = src.parse();
    REQUIRE_NOTHROW(MachineConfigWriter{orig}.write(out));

    MachineConfigReader back{out};
    auto rb = back.parse();
    const auto& s = rb.optical_trains[0].scanner;
    REQUIRE_FALSE(s.invert_actual_x);
    REQUIRE_FALSE(s.invert_actual_y);
    REQUIRE_FALSE(s.invert_commanded_x);
    REQUIRE_FALSE(s.invert_commanded_y);

    nlohmann::json j = s;
    REQUIRE_FALSE(j.contains("invert_actual_x"));
    REQUIRE_FALSE(j.contains("invert_actual_y"));
    REQUIRE_FALSE(j.contains("invert_commanded_x"));
    REQUIRE_FALSE(j.contains("invert_commanded_y"));

    std::filesystem::remove(out);
}

TEST_CASE("OnlyTrueInvertFlagsSurviveAsRealHDF5Attributes") {
    auto out = tmpPath("invert_mixed");
    MachineConfigReader src{REF};
    auto cfg = src.parse();
    cfg.optical_trains[0].scanner.invert_actual_x = true;
    cfg.optical_trains[0].scanner.invert_actual_y = false;
    cfg.optical_trains[0].scanner.invert_commanded_x = true;
    cfg.optical_trains[0].scanner.invert_commanded_y = false;

    REQUIRE_NOTHROW(MachineConfigWriter{cfg}.write(out));

    // Raw HDF5 inspection: only the two true-valued attributes exist at all.
    {
        HighFive::File f(out.string(), HighFive::File::ReadOnly);
        auto scanner_grp = f.getGroup("Machine/Optical_Trains/Optical_Train_01/Scanner");
        REQUIRE(scanner_grp.hasAttribute("Invert_Actual_X"));
        REQUIRE(scanner_grp.hasAttribute("Invert_Commanded_X"));
        REQUIRE_FALSE(scanner_grp.hasAttribute("Invert_Actual_Y"));
        REQUIRE_FALSE(scanner_grp.hasAttribute("Invert_Commanded_Y"));
    }

    // Read path still returns the correct value either way.
    MachineConfigReader back{out};
    auto rb = back.parse();
    const auto& s = rb.optical_trains[0].scanner;
    REQUIRE(s.invert_actual_x);
    REQUIRE_FALSE(s.invert_actual_y);
    REQUIRE(s.invert_commanded_x);
    REQUIRE_FALSE(s.invert_commanded_y);

    std::filesystem::remove(out);
}

// ---------------------------------------------------------------------------
// Synchronous sensors
// ---------------------------------------------------------------------------

TEST_CASE("RoundtripSynchronousSensorTablesExactValuesInOrder") {
    auto out = tmpPath("sensor_roundtrip");
    MachineConfigReader src{SENSORS_REF};
    auto orig = src.parse();
    REQUIRE_NOTHROW(MachineConfigWriter{orig}.write(out));

    MachineConfigReader back{out};
    auto rb = back.parse();
    const auto& s = cbtest::clearbox(rb, 0).children.at("Synchronous_Sensors").children.at("Oxygen Sensor");
    REQUIRE(cbtest::text(s, "Sensor_Name") == "ZR800 Oxygen Analyzer");
    const auto constants = cbtest::constants(s);
    REQUIRE(constants.size() == 2);
    REQUIRE(constants[0].first == "a");
    REQUIRE(constants[0].second == 0.4375);
    REQUIRE(constants[1].first == "b");
    REQUIRE(constants[1].second == -2.75);
    const auto& points = s.tables.at("Calibration_Points");
    REQUIRE(std::get<std::vector<double>>(points.columns.at("input_value")) == std::vector<double>{4.0, 20.0});
    REQUIRE(std::get<std::vector<double>>(points.columns.at("output_value")) == std::vector<double>{-1.0, 6.0});

    std::filesystem::remove(out);
}

// The sensor itself exists (its group is created), but its compound dataset
// has zero rows — proving 0-length compound dataset creation/read genuinely
// works, not assumed.
TEST_CASE("RoundtripSynchronousSensorWithZeroRowTable") {
    auto out = tmpPath("sensor_zero_row");
    MachineConfigReader src{REF};
    auto cfg = src.parse();
    auto sensor = cbtest::sensorTree({});
    sensor.attrs["Sensor_Name"] = {"utf8", std::string("Placeholder")};
    cbtest::addSensor(cfg, "Untested Sensor", sensor);

    REQUIRE_NOTHROW(MachineConfigWriter{cfg}.write(out));
    MachineConfigReader back{out};
    auto rb = back.parse();
    const auto& s = cbtest::clearbox(rb, 0).children.at("Synchronous_Sensors").children.at("Untested Sensor");
    REQUIRE(s.tables.at("Derivation_Equation_Constants").rows == 0);
    REQUIRE(cbtest::text(s, "Sensor_Name") == "Placeholder");

    std::filesystem::remove(out);
}

// The map key is a free-form label with no schema meaning — proves a key
// unlike the fixture's own "Oxygen Sensor" (different style: underscore-
// joined, all-caps) survives a write->read cycle verbatim.
TEST_CASE("RoundtripSynchronousSensorArbitraryDifferentlyStyledKey") {
    auto out = tmpPath("sensor_arbitrary_key");
    MachineConfigReader src{REF};
    auto cfg = src.parse();
    ext::Tree sensor;
    sensor.attrs["Port_ID"] = {"i32", std::int64_t{9}};
    cbtest::addSensor(cfg, "HUMIDITY_SENSOR_2", sensor);

    REQUIRE_NOTHROW(MachineConfigWriter{cfg}.write(out));
    MachineConfigReader back{out};
    auto rb = back.parse();
    const auto& sensors = cbtest::clearbox(rb, 0).children.at("Synchronous_Sensors").children;
    REQUIRE(sensors.count("HUMIDITY_SENSOR_2") == 1);
    std::int64_t port = 0;
    REQUIRE(cbtest::integer(sensors.at("HUMIDITY_SENSOR_2"), "Port_ID", port));
    REQUIRE(port == 9);

    std::filesystem::remove(out);
}

// Proves the Writer side of the cross-feature guarantee: parses a real
// OPCUA-only fixture, adds a sensor purely in memory, writes, and confirms
// both survive re-reading (mirrors Rust's/Python's/Node's/Go's equivalent
// tests).
TEST_CASE("RoundtripOpcuaAndSynchronousSensorCoexist") {
    auto out = tmpPath("opcua_and_sensor");
    MachineConfigReader src{OPCUA_REF};
    auto cfg = src.parse();
    REQUIRE(cfg.extensions.count("TM_OPCUA") == 1);

    cbtest::addSensor(cfg, "Oxygen Sensor", cbtest::sensorTree({{"a", 0.4375}, {"b", -2.75}}));

    REQUIRE_NOTHROW(MachineConfigWriter{cfg}.write(out));
    MachineConfigReader back{out};
    auto rb = back.parse();
    REQUIRE(rb.extensions.count("TM_OPCUA") == 1);
    const auto& s = cbtest::clearbox(rb, 0).children.at("Synchronous_Sensors").children.at("Oxygen Sensor");
    const auto constants = cbtest::constants(s);
    REQUIRE(constants.size() == 2);
    REQUIRE(constants[0].first == "a");
    REQUIRE(constants[0].second == 0.4375);

    std::filesystem::remove(out);
}

// Derivation_Equation_Constants.name is a 64-byte fixed-length field (see
// SYNCHRONOUS_SENSOR_PLAN.md's "Compound dataset string convention"). A name
// whose UTF-8 encoding exceeds 64 bytes must be rejected with a clear error
// at write time, not silently truncated.
TEST_CASE("WriterRejectsEquationConstantNameOver64Bytes") {
    auto out = tmpPath("oversized_name");
    MachineConfigReader src{REF};
    auto cfg = src.parse();
    cbtest::addSensor(cfg, "Oversized Name Sensor", cbtest::sensorTree({{std::string(65, 'a'), 1.0}}));

    REQUIRE_THROWS_WITH(MachineConfigWriter{cfg}.write(out), Catch::Matchers::ContainsSubstring("fit"));
}

// ---------------------------------------------------------------------------
// WrittenHdf5ValidatesSchema
// JSON from reading back a written file has the required structure.
// ---------------------------------------------------------------------------
TEST_CASE("WrittenHdf5ValidatesSchema") {
    auto out = tmpPath("schema");
    MachineConfigReader src{REF};
    REQUIRE_NOTHROW(MachineConfigWriter{src.parse()}.write(out));

    auto j = nlohmann::json::parse(MachineConfigReader{out}.toJson());

    // Required top-level keys present with correct types
    REQUIRE(j.contains("meta"));
    REQUIRE(j.contains("machine"));
    REQUIRE(j.contains("optical_trains"));
    REQUIRE(j["meta"].is_object());
    REQUIRE(j["machine"].is_object());
    REQUIRE(j["optical_trains"].is_array());
    REQUIRE_FALSE(j["optical_trains"].empty());

    // Required meta fields
    REQUIRE(j["meta"]["schema_version"].get<std::string>() == "v1");
    REQUIRE_FALSE(j["meta"]["machine_name"].get<std::string>().empty());
    REQUIRE(j["meta"]["configuration_hash"].get<std::string>().size() == 64);

    // Required machine fields
    REQUIRE_FALSE(j["machine"]["manufacturer"].get<std::string>().empty());
    REQUIRE_FALSE(j["machine"]["model"].get<std::string>().empty());

    // Each train has required sub-objects
    for (const auto& train : j["optical_trains"]) {
        REQUIRE(train.contains("scanner"));
        REQUIRE(train.contains("light_source"));
        REQUIRE(train.contains("collimator"));
        REQUIRE(train.contains("scanner_card"));
        REQUIRE(train.contains("train_id"));
    }

    std::filesystem::remove(out);
}

// ---------------------------------------------------------------------------
// BinaryRoundtripCorrectionGridHash  §4.15
// parseWithBinary + write; SHA-256 of copy's correction grid must match original.
// ---------------------------------------------------------------------------
TEST_CASE("BinaryRoundtripCorrectionGridHash") {
    auto out = tmpPath("bin_hash");
    MachineConfigWriter{MachineConfigReader{REF}.parseWithBinary()}.write(out);

    auto orig = MachineConfigReader{REF}.parse();
    auto copy = MachineConfigReader{out}.parse();
    for (const char* name : {"Correction_Data", "Inverse_Correction_Data"}) {
        const auto& a = cbtest::grid(orig, 0, name);
        const auto& b = cbtest::grid(copy, 0, name);
        REQUIRE(a.size() == b.size());
        auto hashOf = [](const std::vector<double>& data) {
            const auto* p = reinterpret_cast<const unsigned char*>(data.data());
            std::vector<unsigned char> h;
            picosha2::hash256(p, p + data.size() * sizeof(double), h);
            return h;
        };
        REQUIRE(hashOf(a) == hashOf(b));
    }

    std::filesystem::remove(out);
}

// ---------------------------------------------------------------------------
// BinaryRoundtripFc3Size  §4.15
// Raw fc3 byte count of copy must match original.
// ---------------------------------------------------------------------------
TEST_CASE("BinaryRoundtripFc3Size") {
    auto out = tmpPath("fc3_size");
    MachineConfigWriter{MachineConfigReader{REF}.parseWithBinary()}.write(out);

    auto orig_bytes = MachineConfigReader{REF}.getScanFieldCorrectionBytes(0);
    auto copy_bytes = MachineConfigReader{out}.getScanFieldCorrectionBytes(0);
    REQUIRE(copy_bytes.size() == orig_bytes.size());

    std::filesystem::remove(out);
}

TEST_CASE("WriterRejectsUnknownFileVersion") {
    auto cfg = makeMinimalConfig();
    cfg.meta.file_version = "2.0";
    auto out = tmpPath("unknown_version");
    try {
        MachineConfigWriter{cfg}.write(out);
        FAIL("expected write() to throw for File_Version 2.0");
    } catch (const std::runtime_error& e) {
        REQUIRE(std::string(e.what()).find("2.0") != std::string::npos);
    }
    std::filesystem::remove(out);
}

// ---------------------------------------------------------------------------
// Prerequisite fix (CONFIGURATION_HASH_PLAN.md): writers must no longer
// materialize a hardcoded default (e.g. "mm") for an unset *_unit field —
// that default was never real data, just the reader's own Rule-8 locked
// constant written early for no benefit, and it broke pre-write/post-write
// hash agreement. One representative field per affected struct, mirroring
// the other languages' equivalent test's shape exactly.
// ---------------------------------------------------------------------------
TEST_CASE("UnsetUnitFieldsRoundTripAsNulloptNotADefault") {
    MockConfigBuilder b;
    b.laser_count = 1;
    auto cfg = b.build();

    cfg.machine.build_plate_radius_unit = std::nullopt;
    for (auto& t : cfg.optical_trains) {
        t.major_axis_angle_unit = std::nullopt;
        t.scanner.working_distance_unit = std::nullopt;
        t.light_source.wavelength_unit = std::nullopt;
        t.collimator.focal_length_unit = std::nullopt;
        t.scanner_card.sample_period_unit = std::nullopt;
    }

    auto out = tmpPath("unset_units");
    MachineConfigWriter{cfg}.write(out);
    auto rt = MachineConfigReader{out}.parse();

    REQUIRE_FALSE(rt.machine.build_plate_radius_unit.has_value());
    REQUIRE_FALSE(rt.optical_trains[0].major_axis_angle_unit.has_value());
    REQUIRE_FALSE(rt.optical_trains[0].scanner.working_distance_unit.has_value());
    REQUIRE_FALSE(rt.optical_trains[0].light_source.wavelength_unit.has_value());
    REQUIRE_FALSE(rt.optical_trains[0].collimator.focal_length_unit.has_value());
    REQUIRE_FALSE(rt.optical_trains[0].scanner_card.sample_period_unit.has_value());

    std::filesystem::remove(out);
}
