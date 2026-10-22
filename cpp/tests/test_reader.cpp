// Catch2 integration tests for reader.hpp — §4.8 acceptance criteria.
// Fixture values come from fixtures/reference_output.json (the ground truth).
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>
#include <nlohmann/json.hpp>
#include <picosha2.h>
#include <highfive/H5File.hpp>

#include <cmath>
#include <filesystem>
#include <stdexcept>
#include <string>

#include "machine_config/capabilities.hpp"
#include "machine_config/reader.hpp"

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

using namespace machine_config;

static const std::string REF       = std::string(FIXTURES_DIR) + "/reference_config.h5";
static const std::string OPCUA_REF = std::string(FIXTURES_DIR) + "/reference_config_opcua.h5";
static const std::string SYNTHETIC = std::string(FIXTURES_DIR) + "/synthetic_2laser.h5";
static const std::string SENSORS_REF =
    std::string(FIXTURES_DIR) + "/reference_config_synchronous_sensors.h5";
static const std::string OPCUA_SENSORS_REF =
    std::string(FIXTURES_DIR) + "/reference_config_opcua_synchronous_sensors.h5";

// §4.8: ParsesMeta
TEST_CASE("ParsesMeta") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();

    REQUIRE(cfg.meta.schema_version    == "v1");
    REQUIRE(cfg.meta.machine_name      == "TM-LPBF-02: AconityMIDI+_OG");
    REQUIRE(cfg.meta.configuration_hash == "9bc38c92c582a15439fe74990c04bcf75705ca6ac499ef4332460b923400e431");
    REQUIRE_FALSE(cfg.meta.extra.empty()); // Description + Generator are extra attrs
}

// §4.8: ParsesMachineGroup
TEST_CASE("ParsesMachineGroup") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();

    REQUIRE(cfg.machine.manufacturer == "Aconity3D");
    REQUIRE(cfg.machine.model        == "AconityMIDI+");
    REQUIRE(cfg.machine.build_plate_x.has_value());
    REQUIRE_THAT(*cfg.machine.build_plate_x,     Catch::Matchers::WithinRel(250.0));
    REQUIRE_THAT(*cfg.machine.build_plate_radius, Catch::Matchers::WithinRel(125.0));
    REQUIRE(cfg.machine.build_plate_x_unit == std::optional<std::string>{"mm"});
}

// §4.9: OpticalTrainCount
TEST_CASE("OpticalTrainCount") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    REQUIRE(cfg.optical_trains.size() == 2);
    REQUIRE(cfg.optical_trains[0].train_id == "Optical_Train_01");
    REQUIRE(cfg.optical_trains[1].train_id == "Optical_Train_02");
}

// §4.9: TrainZeroWorkingDistance
TEST_CASE("TrainZeroWorkingDistance") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    const auto& s = cfg.optical_trains[0].scanner;
    REQUIRE(s.working_distance.has_value());
    REQUIRE_THAT(*s.working_distance, Catch::Matchers::WithinRel(670.0));
}

// §4.9: TrainZeroScannerOffsets
TEST_CASE("TrainZeroScannerOffsets") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    const auto& s = cfg.optical_trains[0].scanner;
    REQUIRE(s.scan_head_offset_x.has_value());
    REQUIRE(s.scan_head_offset_y.has_value());
    REQUIRE_THAT(*s.scan_head_offset_x, Catch::Matchers::WithinRel(-87.5));
    REQUIRE_THAT(*s.scan_head_offset_y, Catch::Matchers::WithinRel(23.5));
}

// §4.10: ClearBoxPresentForBothTrains — reference fixture has ClearBox on both trains
TEST_CASE("ClearBoxPresentForBothTrains") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    REQUIRE(cfg.optical_trains[0].optional_components.clearbox.has_value());
    REQUIRE(cfg.optical_trains[1].optional_components.clearbox.has_value());
}

// §4.10: ClearBoxIpAddress — each train has a distinct IP
TEST_CASE("ClearBoxIpAddress") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    REQUIRE(cfg.optical_trains[0].optional_components.clearbox->ip_address == "192.168.1.10");
    REQUIRE(cfg.optical_trains[1].optional_components.clearbox->ip_address == "192.168.1.11");
}

// §4.10: ClearBoxScalars — scalar fields parse correctly; correction grids remain null (§4.11)
TEST_CASE("ClearBoxScalars") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    const auto& cb = *cfg.optical_trains[0].optional_components.clearbox;
    REQUIRE(cb.serial_number     == std::optional<std::string>{"001"});
    REQUIRE(cb.data_port         == std::optional<int64_t>{5001});
    REQUIRE(cb.server_port       == std::optional<int64_t>{20101});
    REQUIRE(cb.actual_timing_offset    == std::optional<int64_t>{-8});
    REQUIRE(cb.commanded_timing_offset == std::optional<int64_t>{50});
    REQUIRE(cb.show_console      == std::optional<bool>{false});
    REQUIRE(cb.software_trigger_delay == std::optional<int64_t>{3000});
    REQUIRE(cb.power_characterization.has_value());
    REQUIRE(cb.power_characterization->algorithm_type == std::optional<std::string>{"LINEAR"});
    // correction_data and inverse_correction_data deferred to §4.11
    REQUIRE_FALSE(cb.correction_data.has_value());
    REQUIRE_FALSE(cb.inverse_correction_data.has_value());
}

// ---------------------------------------------------------------------------
// SynchronousSensor
// ---------------------------------------------------------------------------

TEST_CASE("SynchronousSensorsEmptyWhenFixtureHasNone") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    const auto& cb = *cfg.optical_trains[0].optional_components.clearbox;
    REQUIRE(cb.synchronous_sensors.empty());
}

// reference_config_synchronous_sensors.h5, not reference_config.h5, which
// deliberately has no sensors (see SYNCHRONOUS_SENSOR_PLAN.md Phase 0).
TEST_CASE("SynchronousSensorFixtureHasRealValues") {
    MachineConfigReader reader{SENSORS_REF};
    auto cfg = reader.parse();
    const auto& cb = *cfg.optical_trains[0].optional_components.clearbox;
    REQUIRE(cb.synchronous_sensors.size() == 1);
    REQUIRE(cb.synchronous_sensors.count("Oxygen Sensor") == 1);
    const auto& s = cb.synchronous_sensors.at("Oxygen Sensor");

    REQUIRE(s.enabled == std::optional<bool>{true});
    REQUIRE(s.sensor_name == std::optional<std::string>{"ZR800 Oxygen Analyzer"});
    REQUIRE(s.sensor_output_range_low == std::optional<double>{-1.0});
    REQUIRE(s.sensor_output_range_high == std::optional<double>{6.0});
    REQUIRE(s.sensor_output_space == std::optional<std::string>{"log10(ppm)"});
    REQUIRE(s.sensor_model == std::optional<std::string>{"ZR810"});
    REQUIRE(s.sensor_manufacturer == std::optional<std::string>{"Industrial Physics"});
    REQUIRE(s.sensor_scope == std::optional<std::string>{"Global"});
    REQUIRE(s.units_derived_quantity == std::optional<std::string>{"ppm"});
    REQUIRE(s.port_id == std::optional<int64_t>{5});
    REQUIRE(s.sensor_type == std::optional<std::string>{"Oxygen Sensor"});
    REQUIRE(s.input_type == std::optional<std::string>{"4-20 mA"});
    REQUIRE(s.algorithm_type == std::optional<std::string>{"Log-Linear"});
    REQUIRE(s.algorithm_equation == std::optional<std::string>{"log(ppm) = a*mA + b"});
    REQUIRE(s.calibration_source == std::optional<std::string>{"Datasheet"});
    REQUIRE(s.calibration_verified == std::optional<bool>{false});
    REQUIRE(s.sample_period == std::optional<double>{5.0});
    REQUIRE(s.metadata.has_value());
    REQUIRE(s.metadata->find("Synchronous Sensor because Clearbox is responsible") !=
            std::string::npos);

    REQUIRE(s.derivation_equation_constants.size() == 2);
    REQUIRE(s.derivation_equation_constants[0].name == "a");
    REQUIRE(s.derivation_equation_constants[0].value == 0.4375);
    REQUIRE(s.derivation_equation_constants[1].name == "b");
    REQUIRE(s.derivation_equation_constants[1].value == -2.75);

    REQUIRE(s.calibration_points.size() == 2);
    REQUIRE(s.calibration_points[0].input_value == 4.0);
    REQUIRE(s.calibration_points[0].output_value == -1.0);
    REQUIRE(s.calibration_points[1].input_value == 20.0);
    REQUIRE(s.calibration_points[1].output_value == 6.0);

    // log(ppm) = a*mA + b, per algorithm_equation — proves the points are
    // recorded in Sensor_Output_Space (log10(ppm)) units, not the linear
    // Units_Derived_Quantity (ppm) units of the same underlying quantity.
    double a = s.derivation_equation_constants[0].value;
    double b = s.derivation_equation_constants[1].value;
    for (const auto& p : s.calibration_points) {
        REQUIRE(std::abs(a * p.input_value + b - p.output_value) < 1e-10);
    }
}

TEST_CASE("CombinedOpcuaAndSynchronousSensorsFixtureHasBoth") {
    MachineConfigReader reader{OPCUA_SENSORS_REF};
    auto cfg = reader.parse();
    REQUIRE(cfg.extensions.count("TM_OPCUA") == 1);
    const auto& cb = *cfg.optical_trains[0].optional_components.clearbox;
    REQUIRE(cb.synchronous_sensors.count("Oxygen Sensor") == 1);
    const auto& s = cb.synchronous_sensors.at("Oxygen Sensor");
    REQUIRE(s.derivation_equation_constants.size() == 2);
    REQUIRE(s.derivation_equation_constants[0].name == "a");
    REQUIRE(s.derivation_equation_constants[0].value == 0.4375);
}

// §4.9: AxisConfig3DNoFocus — reference fixture is Axis_Configuration='3D':
// X_Axis + Y_Axis + Z_Axis present; Focus group absent.
TEST_CASE("AxisConfig3DNoFocus") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    const auto& s = cfg.optical_trains[0].scanner;
    REQUIRE(s.axis_configuration == std::optional<std::string>{"3D"});
    REQUIRE(s.x_axis.actual_bit_resolution == std::optional<int64_t>{20});
    REQUIRE(s.y_axis.actual_bit_resolution == std::optional<int64_t>{20});
    REQUIRE(s.z_axis.has_value());
    REQUIRE(s.z_axis->actual_bit_resolution == std::optional<int64_t>{20});
    REQUIRE_FALSE(s.focus.has_value());
}

TEST_CASE("ScannerInvertFlagsAbsentFromFixtureReadAsFalse") {
    // reference_config.h5 has none of the four Invert_* attributes at all.
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    const auto& s = cfg.optical_trains[0].scanner;
    REQUIRE_FALSE(s.invert_actual_x);
    REQUIRE_FALSE(s.invert_actual_y);
    REQUIRE_FALSE(s.invert_commanded_x);
    REQUIRE_FALSE(s.invert_commanded_y);
}

TEST_CASE("ScannerInvertFlagsOmittedFromJSONWhenFalse") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    nlohmann::json j = cfg.optical_trains[0].scanner;
    REQUIRE_FALSE(j.contains("invert_actual_x"));
    REQUIRE_FALSE(j.contains("invert_actual_y"));
    REQUIRE_FALSE(j.contains("invert_commanded_x"));
    REQUIRE_FALSE(j.contains("invert_commanded_y"));
}

// §4.11: CorrectionDataShape — 3-D grid for train 0 has the canonical dimensions.
TEST_CASE("CorrectionDataShape") {
    MachineConfigReader reader{REF};
    auto cd = reader.getCorrectionData(0);
    REQUIRE(cd.shape[0] == 257);
    REQUIRE(cd.shape[1] == 257);
    REQUIRE(cd.shape[2] == 2);
}

// §4.11: CorrectionDataSize — flat buffer has exactly d0*d1*d2 elements.
TEST_CASE("CorrectionDataSize") {
    MachineConfigReader reader{REF};
    auto cd = reader.getCorrectionData(0);
    REQUIRE(cd.data.size() == 257 * 257 * 2);
}

// §4.11: CorrectionHashMatchesReference — SHA-256 of flat LE float64 bytes.
TEST_CASE("CorrectionHashMatchesReference") {
    static const std::string EXPECTED =
        "b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6";
    MachineConfigReader reader{REF};
    auto cd = reader.getCorrectionData(0);
    const auto* begin = reinterpret_cast<const uint8_t*>(cd.data.data());
    const auto* end   = begin + cd.data.size() * sizeof(double);
    REQUIRE(picosha2::hash256_hex_string(begin, end) == EXPECTED);
}

// §4.11: ScanFieldCorrectionBytesSize — byte count matches file_size attribute.
TEST_CASE("ScanFieldCorrectionBytesSize") {
    MachineConfigReader reader{REF};
    auto bytes = reader.getScanFieldCorrectionBytes(0);
    REQUIRE(bytes.size() == 1138799);
}

// ---------------------------------------------------------------------------
// Per-train isolation — train 1 values distinct from train 0
// ---------------------------------------------------------------------------

// Train 1 has different offsets than train 0; verifies per-train reading.
TEST_CASE("Train1ScannerOffsets") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    const auto& s = cfg.optical_trains[1].scanner;
    REQUIRE_THAT(*s.scan_head_offset_x, Catch::Matchers::WithinAbs(86.074,  1e-3));
    REQUIRE_THAT(*s.scan_head_offset_y, Catch::Matchers::WithinAbs(-21.695, 1e-3));
}

// train 0 = 0°, train 1 = 180° — each read from its own HDF5 group.
TEST_CASE("ScanHeadRotation") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    REQUIRE_THAT(*cfg.optical_trains[0].scanner.scan_head_rotation,
                 Catch::Matchers::WithinRel(0.0, 1e-6));
    REQUIRE_THAT(*cfg.optical_trains[1].scanner.scan_head_rotation,
                 Catch::Matchers::WithinRel(180.0));
}

// ---------------------------------------------------------------------------
// Train-level fields not yet covered
// ---------------------------------------------------------------------------

// Tests readBoolFromInt for both outcomes: 0→false (train 0), 1→true (train 1).
TEST_CASE("ThermalLensingPassedBool") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    REQUIRE(cfg.optical_trains[0].thermal_lensing_passed == std::optional<bool>{false});
    REQUIRE(cfg.optical_trains[1].thermal_lensing_passed == std::optional<bool>{true});
}

// ---------------------------------------------------------------------------
// Sub-component values not previously tested
// ---------------------------------------------------------------------------

TEST_CASE("CollimatorFocalLength") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    for (const auto& t : cfg.optical_trains) {
        REQUIRE_THAT(*t.collimator.focal_length, Catch::Matchers::WithinRel(120.0));
        REQUIRE(t.collimator.focal_length_unit == std::optional<std::string>{"mm"});
    }
}

TEST_CASE("ScannerCardModel") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    for (const auto& t : cfg.optical_trains) {
        REQUIRE(t.scanner_card.model == "SP-ICE-3");
        REQUIRE(t.scanner_card.sample_period_unit == std::optional<std::string>{"\u03bcs"});
    }
}

TEST_CASE("LightSourceLockedUnits") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    const auto& ls = cfg.optical_trains[0].light_source;
    REQUIRE_THAT(*ls.wavelength, Catch::Matchers::WithinRel(1070.0));
    REQUIRE(ls.wavelength_unit      == std::optional<std::string>{"nm"});
    REQUIRE(ls.power_max_nominal_unit == std::optional<std::string>{"W"});
}

TEST_CASE("AxisSmoothingKernel") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    REQUIRE(cfg.optical_trains[0].scanner.x_axis.smoothing_kernel
            == std::optional<std::string>{"GAUSSIAN"});
}

// ---------------------------------------------------------------------------
// Machine group completeness
// ---------------------------------------------------------------------------

TEST_CASE("MachineBuildPlateYZ") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    REQUIRE_THAT(*cfg.machine.build_plate_y, Catch::Matchers::WithinRel(250.0));
    REQUIRE_THAT(*cfg.machine.build_plate_z, Catch::Matchers::WithinRel(20.0));
    REQUIRE(cfg.machine.build_plate_y_unit == std::optional<std::string>{"mm"});
    REQUIRE(cfg.machine.build_plate_z_unit == std::optional<std::string>{"mm"});
}

// ---------------------------------------------------------------------------
// ScanFieldCorrectionFile scalar fields (not just byte count)
// ---------------------------------------------------------------------------

TEST_CASE("ScanFieldCorrectionFileScalars") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    const auto& s0 = *cfg.optical_trains[0].scan_field_correction_file;
    REQUIRE(s0.file_size     == 1138799);
    REQUIRE(s0.document_name == "Story6.17.3.1_Laser_1_VMM.fc3");
    REQUIRE(s0.document_id   == "c1e808bc-75a3-4998-b009-72f37068b1e4");
    // Train 1 has a distinct file_size — proves per-train sfcf reading.
    const auto& s1 = *cfg.optical_trains[1].scan_field_correction_file;
    REQUIRE(s1.file_size == 1142763);
}

// ---------------------------------------------------------------------------
// Binary API completeness
// ---------------------------------------------------------------------------

// Verify the inverse grid is also readable and has the same dimensions.
TEST_CASE("InverseCorrectionDataShape") {
    MachineConfigReader reader{REF};
    auto inv = reader.getInverseCorrectionData(0);
    REQUIRE(inv.shape[0] == 257);
    REQUIRE(inv.shape[1] == 257);
    REQUIRE(inv.shape[2] == 2);
    REQUIRE(inv.data.size() == 257 * 257 * 2);
}

// ---------------------------------------------------------------------------
// JSON serialisation
// ---------------------------------------------------------------------------

// toJson() must produce well-formed JSON with the expected top-level structure.
TEST_CASE("ToJsonTopLevelStructure") {
    MachineConfigReader reader{REF};
    auto j = nlohmann::json::parse(reader.toJson());
    REQUIRE(j.contains("meta"));
    REQUIRE(j.contains("machine"));
    REQUIRE(j.contains("optical_trains"));
    REQUIRE(j["optical_trains"].is_array());
    REQUIRE(j["optical_trains"].size() == 2);
    // opcua key must be absent (not null) when no OPCUA group exists.
    REQUIRE_FALSE(j.contains("extensions"));
}

// ---------------------------------------------------------------------------
// Meta completeness — root-level HDF5 attrs not yet verified
// ---------------------------------------------------------------------------

// Distinct from machine.manufacturer; comes from a different HDF5 path.
TEST_CASE("MetaCompleteness") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    REQUIRE(cfg.meta.manufacturer == "Aconity3D");
    REQUIRE(cfg.meta.file_version == "1.0");
    REQUIRE(cfg.meta.configuration_hash.size() == 64);
}

// ---------------------------------------------------------------------------
// Train-level duplicate attributes
// ---------------------------------------------------------------------------

// collimator_focal_length is stored both on the train group AND in the Collimator
// sub-group; both fields must be populated independently.
TEST_CASE("TrainLevelCollimatorFocalLength") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    for (const auto& t : cfg.optical_trains) {
        REQUIRE_THAT(*t.collimator_focal_length, Catch::Matchers::WithinRel(120.0));
        REQUIRE(t.collimator_focal_length_unit == std::optional<std::string>{"mm"});
    }
}

// ---------------------------------------------------------------------------
// Locked units not previously exercised
// ---------------------------------------------------------------------------

TEST_CASE("LockedUnitsComplete") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    const auto& t = cfg.optical_trains[0];
    REQUIRE(t.scanner.scan_head_rotation_unit == std::optional<std::string>{"degrees"});
    REQUIRE(t.thermal_lensing_threshold_unit  == std::optional<std::string>{"mm"});
    REQUIRE(t.light_source.power_min_nominal_unit == std::optional<std::string>{"W"});
}

// ---------------------------------------------------------------------------
// Scanner serial_number (not previously asserted)
// ---------------------------------------------------------------------------

TEST_CASE("ScannerSerialNumberNonEmpty") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    for (const auto& t : cfg.optical_trains)
        REQUIRE_FALSE(t.scanner.serial_number.empty());
}

// ---------------------------------------------------------------------------
// Correction data semantic correctness
// ---------------------------------------------------------------------------

// Border cells outside the scanner field are stored as NaN; the reader must
// preserve them, not silently convert to 0 or throw.
TEST_CASE("CorrectionDataHasNaN") {
    MachineConfigReader reader{REF};
    auto cd = reader.getCorrectionData(0);
    auto it = std::find_if(cd.data.begin(), cd.data.end(),
                           [](double v) { return std::isnan(v); });
    REQUIRE(it != cd.data.end());
}

// Forward and inverse grids are distinct datasets; reading one must not alias
// the other.
TEST_CASE("ForwardAndInverseCorrectionDataDiffer") {
    MachineConfigReader reader{REF};
    auto fwd = reader.getCorrectionData(0);
    auto inv = reader.getInverseCorrectionData(0);
    REQUIRE(fwd.shape == inv.shape);
    bool any_differs = false;
    for (size_t i = 0; i < fwd.data.size() && !any_differs; ++i) {
        if (!std::isnan(fwd.data[i]) && !std::isnan(inv.data[i]))
            any_differs = (fwd.data[i] != inv.data[i]);
    }
    REQUIRE(any_differs);
}

// Per-train correction data must be distinct; getCorrectionData(0) and (1)
// must read different HDF5 datasets.
TEST_CASE("Train0AndTrain1CorrectionDataDiffer") {
    MachineConfigReader reader{REF};
    auto cd0 = reader.getCorrectionData(0);
    auto cd1 = reader.getCorrectionData(1);
    REQUIRE(cd0.shape == cd1.shape);
    bool any_differs = false;
    for (size_t i = 0; i < cd0.data.size() && !any_differs; ++i) {
        if (!std::isnan(cd0.data[i]) && !std::isnan(cd1.data[i]))
            any_differs = (cd0.data[i] != cd1.data[i]);
    }
    REQUIRE(any_differs);
}

// ---------------------------------------------------------------------------
// OPCUA completeness
// ---------------------------------------------------------------------------

#ifndef VALIDATION_FIXTURES_DIR
#  error "VALIDATION_FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

// ---------------------------------------------------------------------------
// Synthetic fixture — validates a completely different HDF5 file
// ---------------------------------------------------------------------------

TEST_CASE("SyntheticFixtureParsesCorrectly") {
    MachineConfigReader reader{SYNTHETIC};
    auto cfg = reader.parse();
    REQUIRE(cfg.meta.schema_version == "v1");
    REQUIRE(cfg.meta.machine_name   == "SyntheticMachine");
    // Synthetic fixture has a 64-char all-zero hash (builder default).
    REQUIRE(cfg.meta.configuration_hash.size() == 64);
    REQUIRE(cfg.optical_trains.size() == 2);
    REQUIRE(cfg.optical_trains[0].train_id == "Optical_Train_01");
    REQUIRE(cfg.optical_trains[1].train_id == "Optical_Train_02");
    // Synthetic fixture has no OPCUA group.
    REQUIRE(cfg.extensions.empty());
    // Basic scanner sanity check.
    REQUIRE_THAT(*cfg.optical_trains[0].scanner.working_distance,
                 Catch::Matchers::WithinRel(670.0));
}

// §4.19: GetRawGroupMissingPath — absent path returns empty object, no throw.
TEST_CASE("GetRawGroupMissingPath") {
    MachineConfigReader reader{REF};
    auto result = reader.getRawGroup("does/not/exist");
    REQUIRE(result.is_object());
    REQUIRE(result.empty());
}

// §4.19: GetRawGroupOpcuaClient — OPCUA fixture has Server_URL in Client group.
TEST_CASE("GetRawGroupOpcuaClient") {
    MachineConfigReader reader{OPCUA_REF};
    auto client = reader.getRawGroup("OPCUA/Client");
    REQUIRE(client.is_object());
    REQUIRE(client.contains("Server_URL"));
    REQUIRE_FALSE(client["Server_URL"].get<std::string>().empty());
}

static void writeFutureFileVersion(const std::filesystem::path& path) {
    HighFive::File f(path.string(), HighFive::File::Truncate);
    f.createAttribute<std::string>("File_Version", HighFive::DataSpace::Scalar())
        .write(std::string("2.0"));
}

TEST_CASE("UnknownFileVersionDoesNotUseV1Layout") {
    auto out = std::filesystem::temp_directory_path() / "mc_reader_future_2_0.h5";
    writeFutureFileVersion(out);

    MachineConfigReader reader{out};
    try {
        reader.parse();
        FAIL("expected parse() to throw for File_Version 2.0");
    } catch (const std::runtime_error& e) {
        REQUIRE(std::string(e.what()).find("2.0") != std::string::npos);
    }

    auto opened = machine_config::capabilities::openMachineConfig(out);
    REQUIRE_FALSE(opened.ok());
    REQUIRE(opened.errorCode() == "UnsupportedVersion");
    REQUIRE(opened.errorMessage().find("2.0") != std::string::npos);

    std::filesystem::remove(out);
}
