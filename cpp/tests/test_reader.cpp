// Catch2 integration tests for reader.hpp — §4.8 acceptance criteria.
// Fixture values come from fixtures/reference_output.json (the ground truth).
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>
#include <nlohmann/json.hpp>
#include <picosha2.h>

#include <cmath>

#include "machine_config/reader.hpp"

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

using namespace machine_config;

static const std::string REF       = std::string(FIXTURES_DIR) + "/reference_config.h5";
static const std::string OPCUA_REF = std::string(FIXTURES_DIR) + "/reference_config_opcua.h5";
static const std::string SYNTHETIC = std::string(FIXTURES_DIR) + "/synthetic_2laser.h5";

// §4.8: ParsesMeta
TEST_CASE("ParsesMeta") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();

    REQUIRE(cfg.meta.file_version == "1.0");
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
    REQUIRE(cb.volts_to_watts_algorithm == std::optional<std::string>{"LINEAR"});
    // correction_data and inverse_correction_data deferred to §4.11
    REQUIRE_FALSE(cb.correction_data.has_value());
    REQUIRE_FALSE(cb.inverse_correction_data.has_value());
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

// §4.12: OpcuaAbsentReturnsNullopt — reference fixture has no OPCUA group.
TEST_CASE("OpcuaAbsentReturnsNullopt") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    REQUIRE_FALSE(cfg.opcua.has_value());
}

// §4.12: OpcuaClientServerUrl — known URL from the opcua fixture.
TEST_CASE("OpcuaClientServerUrl") {
    MachineConfigReader reader{OPCUA_REF};
    auto cfg = reader.parse();
    REQUIRE(cfg.opcua.has_value());
    REQUIRE(cfg.opcua->client.server_url ==
        "opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer");
    REQUIRE(cfg.opcua->client.auth_mode         == "UsernamePassword");
    REQUIRE(cfg.opcua->client.security_mode     == "SignAndEncrypt");
    REQUIRE(cfg.opcua->client.publish_interval  == 250);
    REQUIRE(cfg.opcua->client.session_timeout   == 60000);
}

// §4.12: OpcuaTriggerCount — 2 triggers in the opcua fixture.
TEST_CASE("OpcuaTriggerCount") {
    MachineConfigReader reader{OPCUA_REF};
    auto cfg = reader.parse();
    REQUIRE(cfg.opcua->triggers.size() == 2);
    REQUIRE(cfg.opcua->triggers_enabled == std::optional<bool>{true});
}

// §4.12: OpcuaTriggerFieldsPopulated — spot-check "Chamber Oxygen Level".
TEST_CASE("OpcuaTriggerFieldsPopulated") {
    MachineConfigReader reader{OPCUA_REF};
    auto cfg = reader.parse();
    auto it = cfg.opcua->triggers.find("Chamber Oxygen Level");
    REQUIRE(it != cfg.opcua->triggers.end());
    const auto& t = it->second;
    REQUIRE(t.signal    == std::optional<std::string>{"oxygen_level"});
    REQUIRE(t.subsystem == std::optional<std::string>{"Gas"});
    REQUIRE(t.rule_enabled == std::optional<bool>{true});
    REQUIRE(t.start_value  == std::optional<std::string>{"700"});
    REQUIRE(t.stop_value   == std::optional<std::string>{"1000"});
}

// §4.12: OpcuaPipeConfig — pipe_enabled and buffer_size.
TEST_CASE("OpcuaPipeConfig") {
    MachineConfigReader reader{OPCUA_REF};
    auto cfg = reader.parse();
    REQUIRE(cfg.opcua->pipe.pipe_enabled == true);
    REQUIRE(cfg.opcua->pipe.buffer_size  == 65536);
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
    REQUIRE_FALSE(j.contains("opcua"));
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

// Integer fields and security_policy not yet covered by OpcuaClientServerUrl.
TEST_CASE("OpcuaClientFullFields") {
    MachineConfigReader reader{OPCUA_REF};
    auto cfg = reader.parse();
    const auto& c = cfg.opcua->client;
    REQUIRE(c.bfs_max_depth     == 16);
    REQUIRE(c.sampling_interval == 250);
    REQUIRE(c.security_policy   == "ECC_brainpoolP384r1");
}

// Verify the second trigger is also parsed correctly.
TEST_CASE("OpcuaLaserEmissionInterlockTrigger") {
    MachineConfigReader reader{OPCUA_REF};
    auto cfg = reader.parse();
    auto it = cfg.opcua->triggers.find("Laser Emission Interlock");
    REQUIRE(it != cfg.opcua->triggers.end());
    const auto& t = it->second;
    REQUIRE(t.id        == std::optional<std::string>{"trigger_1"});
    REQUIRE(t.signal    == std::optional<std::string>{"yellow_light"});
    REQUIRE(t.subsystem == std::optional<std::string>{"Chamber"});
    REQUIRE(t.rule_enabled == std::optional<bool>{true});
}

// toJson must include the "opcua" key when the fixture has an OPCUA group.
TEST_CASE("ToJsonOpcuaKeyPresent") {
    MachineConfigReader reader{OPCUA_REF};
    auto j = nlohmann::json::parse(reader.toJson());
    REQUIRE(j.contains("opcua"));
    REQUIRE(j["opcua"].contains("client"));
    REQUIRE(j["opcua"].contains("triggers"));
}

// ---------------------------------------------------------------------------
// Synthetic fixture — validates a completely different HDF5 file
// ---------------------------------------------------------------------------

TEST_CASE("SyntheticFixtureParsesCorrectly") {
    MachineConfigReader reader{SYNTHETIC};
    auto cfg = reader.parse();
    REQUIRE(cfg.meta.file_version == "1.0");
    REQUIRE(cfg.meta.machine_name   == "SyntheticMachine");
    // Synthetic fixture has a 64-char all-zero hash (builder default).
    REQUIRE(cfg.meta.configuration_hash.size() == 64);
    REQUIRE(cfg.optical_trains.size() == 2);
    REQUIRE(cfg.optical_trains[0].train_id == "Optical_Train_01");
    REQUIRE(cfg.optical_trains[1].train_id == "Optical_Train_02");
    // Synthetic fixture has no OPCUA group.
    REQUIRE_FALSE(cfg.opcua.has_value());
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

// ---------------------------------------------------------------------------
// Phase D.4 — adapter dispatch tests
// ---------------------------------------------------------------------------
static const std::string SYNTHETIC_V09 =
    std::string(FIXTURES_DIR) + "/../fixtures/adapters/test/reference_synthetic_v0_9.h5";

TEST_CASE("dispatch: v0.9 fixture is upgraded to file_version 1.0") {
    MachineConfigReader reader{SYNTHETIC_V09};
    auto cfg = reader.parse();
    REQUIRE(cfg.meta.file_version == "1.0");
}

TEST_CASE("dispatch: real fixture file_version unchanged") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    REQUIRE(cfg.meta.file_version == "1.0");
}

TEST_CASE("dispatch: v0.9 fixture returns 2 optical trains") {
    MachineConfigReader reader{SYNTHETIC_V09};
    auto cfg = reader.parse();
    REQUIRE(cfg.optical_trains.size() == 2);
}

TEST_CASE("dispatch: beam_waist_major survives upgrade") {
    MachineConfigReader reader{SYNTHETIC_V09};
    auto cfg = reader.parse();
    REQUIRE(cfg.optical_trains[0].beam_waist_major.has_value());
    REQUIRE(*cfg.optical_trains[0].beam_waist_major > 0.0);
}

TEST_CASE("dispatch: real fixture parses successfully") {
    MachineConfigReader reader{REF};
    auto cfg = reader.parse();
    REQUIRE_FALSE(cfg.meta.machine_name.empty());
    REQUIRE_FALSE(cfg.optical_trains.empty());
}
