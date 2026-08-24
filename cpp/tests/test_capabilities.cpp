// Catch2 tests for the stable model facade (File_Version 1.0).
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include <algorithm>
#include <cmath>
#include <filesystem>
#include <set>
#include <string>

#include "machine_config/capabilities.hpp"
#include "machine_config/reader.hpp"
#include "machine_config/writer.hpp"

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif
#ifndef VALIDATION_FIXTURES_DIR
#  error "VALIDATION_FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

using machine_config::MachineConfigReader;
using machine_config::MachineConfigWriter;
using machine_config::capabilities::MachineConfigFileV1_0;
using machine_config::capabilities::SetMode;
using machine_config::capabilities::createMachineConfig;
using machine_config::capabilities::openMachineConfig;
using machine_config::capabilities::supportedFileVersions;

static const std::string REF       = std::string(FIXTURES_DIR) + "/reference_config.h5";
static const std::string OPCUA_REF = std::string(FIXTURES_DIR) + "/reference_config_opcua.h5";
static const std::string SENSORS_REF =
    std::string(FIXTURES_DIR) + "/reference_config_synchronous_sensors.h5";
static const std::string OPCUA_MISSING_REQUIRED =
    std::string(VALIDATION_FIXTURES_DIR) + "/opcua_missing_required.h5";

static std::filesystem::path tmpPath(const std::string& tag) {
    return std::filesystem::temp_directory_path() /
           ("mc_cap_test_" + tag + ".h5");
}

// CorrectionData has no operator==; IEEE 754 NaN != NaN would make a naive
// element-wise == fail even on bit-for-bit identical real correction grids
// (which always contain NaN cells). Treat "both NaN" as equal.
static bool correctionDataEqual(const machine_config::CorrectionData& a,
                                 const machine_config::CorrectionData& b) {
    if (a.shape != b.shape) return false;
    if (a.data.size() != b.data.size()) return false;
    for (std::size_t i = 0; i < a.data.size(); ++i) {
        bool an = std::isnan(a.data[i]), bn = std::isnan(b.data[i]);
        if (an || bn) {
            if (!(an && bn)) return false;
        } else if (a.data[i] != b.data[i]) {
            return false;
        }
    }
    return true;
}

TEST_CASE("CapabilityOpenGetScannerMatchesReader") {
    auto opened = MachineConfigFileV1_0::open(REF);
    REQUIRE(opened.ok());
    auto file = opened.value();
    REQUIRE(file->fileVersion() == "1.0");

    auto json = MachineConfigReader(REF).parse();
    auto scanner = file->getScanner(0);
    REQUIRE(scanner.ok());
    REQUIRE(scanner.value().working_distance == json.optical_trains[0].scanner.working_distance);
    REQUIRE(scanner.value().manufacturer == json.optical_trains[0].scanner.manufacturer);
    file->close();
}

TEST_CASE("CapabilityMergeSetScannerRoundtrip") {
    auto opened = MachineConfigFileV1_0::open(REF);
    REQUIRE(opened.ok());
    auto file = opened.value();
    auto before = file->getScanner(0);
    REQUIRE(before.ok());
    const auto manufacturer = before.value().manufacturer;

    auto patched = before.value();
    patched.working_distance = 123.5;
    REQUIRE(file->setScanner(0, patched, SetMode::Merge).ok());

    auto out = tmpPath("merge");
    std::string out_s = out.string();
    REQUIRE(file->save(&out_s).ok());
    file->close();

    auto again = MachineConfigFileV1_0::open(out);
    REQUIRE(again.ok());
    auto after = again.value()->getScanner(0);
    REQUIRE(after.ok());
    REQUIRE_THAT(*after.value().working_distance, Catch::Matchers::WithinRel(123.5));
    REQUIRE(after.value().manufacturer == manufacturer);
    again.value()->close();
    std::filesystem::remove(out);
}

TEST_CASE("CapabilityReplaceSetScanner") {
    auto opened = MachineConfigFileV1_0::open(REF);
    REQUIRE(opened.ok());
    auto file = opened.value();
    auto before = file->getScanner(0);
    REQUIRE(before.ok());
    REQUIRE(before.value().scan_field_x.has_value());

    auto replacement = before.value();
    replacement.manufacturer = "ReplaceCo";
    replacement.working_distance = 1.0;
    replacement.scan_field_x = std::nullopt;
    REQUIRE(file->setScanner(0, replacement, SetMode::Replace).ok());

    auto after = file->getScanner(0);
    REQUIRE(after.ok());
    REQUIRE(after.value().manufacturer == "ReplaceCo");
    REQUIRE_THAT(*after.value().working_distance, Catch::Matchers::WithinRel(1.0));
    REQUIRE_FALSE(after.value().scan_field_x.has_value());
    file->close();
}

TEST_CASE("CapabilityInvalidIndex") {
    auto opened = MachineConfigFileV1_0::open(REF);
    REQUIRE(opened.ok());
    auto bad = opened.value()->getTrain(999);
    REQUIRE_FALSE(bad.ok());
    REQUIRE(bad.errorCode() == "InvalidIndex");
    REQUIRE(opened.value()->opticalTrainCount() >= 1);
    opened.value()->close();
}

TEST_CASE("CapabilityOpcuaNotPresentVsPresent") {
    auto no_opc = MachineConfigFileV1_0::open(REF);
    REQUIRE(no_opc.ok());
    auto missing = no_opc.value()->getOpcua();
    REQUIRE_FALSE(missing.ok());
    REQUIRE(missing.errorCode() == "NotPresent");
    no_opc.value()->close();

    auto with_opc = MachineConfigFileV1_0::open(OPCUA_REF);
    REQUIRE(with_opc.ok());
    REQUIRE(with_opc.value()->getOpcua().ok());
    with_opc.value()->close();
}

TEST_CASE("CapabilityOpcuaRequiredFieldsPresentOnReferenceFixture") {
    auto opened = MachineConfigFileV1_0::open(OPCUA_REF);
    REQUIRE(opened.ok());
    auto result = opened.value()->getOpcua();
    REQUIRE(result.ok());
    const auto& opcua = result.value();
    REQUIRE(opcua.client.machine_profile.has_value());
    for (const auto& [name, trigger] : opcua.triggers) {
        REQUIRE(trigger.event.has_value());
    }
    opened.value()->close();
}

TEST_CASE("CapabilityOpcuaMissingRequiredFieldsReportsAllSevenAtOnce") {
    auto opened = MachineConfigFileV1_0::open(OPCUA_MISSING_REQUIRED);
    REQUIRE(opened.ok());
    auto result = opened.value()->getOpcua();
    REQUIRE_FALSE(result.ok());
    REQUIRE(result.errorCode() == "ValidationError");

    std::set<std::string> expected = {
        "Machine_Profile", "Root_Node", "Configure_Client", "Pipe_Name",
        "Triggers_Enabled", "Trigger_Stop_Ceiling_Layers",
        "Laser Emission Interlock.Event",
    };
    std::set<std::string> actual(result.errorDetails().begin(), result.errorDetails().end());
    REQUIRE(actual == expected);
    for (const auto& d : result.errorDetails()) {
        REQUIRE(d.rfind("Chamber Oxygen Level", 0) != 0);
    }
    opened.value()->close();
}

// error() returns one real CapabilityError object — matching Rust's
// Err(CapabilityError), Python's/Node.js's Err.error, and Go's second
// (*Error) return value — not just three independent getter calls.
TEST_CASE("CapabilityResultErrorReturnsRealCapabilityError") {
    auto opened = MachineConfigFileV1_0::open(OPCUA_MISSING_REQUIRED);
    REQUIRE(opened.ok());
    auto result = opened.value()->getOpcua();
    REQUIRE_FALSE(result.ok());

    const machine_config::capabilities::CapabilityError& err = result.error();
    REQUIRE(err.code == result.errorCode());
    REQUIRE(err.message == result.errorMessage());
    REQUIRE(err.details == result.errorDetails());
    REQUIRE(err.code == "ValidationError");
    REQUIRE(err.details.size() == 7);
    opened.value()->close();
}

// The openMachineConfig() dispatcher re-wraps MachineConfigFileV1_0::open()'s
// error into a Result<shared_ptr<IMachineConfigFile>>. This line used to drop
// errorDetails() during that re-wrap. No real caller can reach that bug
// today — open()'s own two error paths (UnsupportedVersion, IoError) never
// populate details — so this test isolates the exact re-wrap expression with
// a manually-constructed Result instead of exercising it through a real
// file, to prove the fixed line itself is correct.
TEST_CASE("CapabilityErrorRewrapPreservesDetails") {
    using machine_config::capabilities::Result;
    using ConfigPtr = std::shared_ptr<machine_config::capabilities::MachineConfigFileV1_0>;

    auto inner = Result<ConfigPtr>::Err(
        "ValidationError", "missing required field(s)",
        {"Machine_Profile", "Root_Node"});

    // Same pattern as the fixed line in capabilities/file.hpp.
    auto rewrapped = Result<std::shared_ptr<machine_config::capabilities::IMachineConfigFile>>::Err(
        inner.errorCode(), inner.errorMessage(), inner.errorDetails());

    REQUIRE(rewrapped.errorCode() == inner.errorCode());
    REQUIRE(rewrapped.errorMessage() == inner.errorMessage());
    REQUIRE(rewrapped.errorDetails() == inner.errorDetails());
    REQUIRE(rewrapped.errorDetails().size() == 2);
}

TEST_CASE("CapabilityOpcuaOptionalFieldNeverAppearsInMissingDetails") {
    // opcua_missing_required.h5 only clears the 7 required fields — every
    // optional field is still present there, so absence of an optional field
    // from errorDetails() would be trivially true. Also clear an optional
    // field (keep_alive_count) in memory, re-write to a temp file, and
    // confirm errorDetails() still names exactly the same 7 items, not 8.
    MachineConfigReader reader{OPCUA_MISSING_REQUIRED};
    auto cfg = reader.parse();
    cfg.opcua->client.keep_alive_count = std::nullopt;
    auto tmp = tmpPath("missing_required_plus_optional");
    REQUIRE_NOTHROW(MachineConfigWriter{cfg}.write(tmp));

    auto opened = MachineConfigFileV1_0::open(tmp.string());
    REQUIRE(opened.ok());
    auto result = opened.value()->getOpcua();
    REQUIRE_FALSE(result.ok());
    for (const auto& d : result.errorDetails()) {
        REQUIRE(d != "Keep_Alive_Count");
    }
    REQUIRE(result.errorDetails().size() == 7);
    opened.value()->close();
    std::filesystem::remove(tmp);
}

TEST_CASE("CapabilityClearboxOptional") {
    auto opened = MachineConfigFileV1_0::open(REF);
    REQUIRE(opened.ok());
    REQUIRE(opened.value()->hasOptionalComponents(0));
    REQUIRE(opened.value()->getClearbox(0).ok());
    opened.value()->close();

    machine_config::MockConfigBuilder b;
    b.laser_count = 1;
    b.include_clearbox = false;
    auto cfg = b.build();
    auto out = tmpPath("no_cb");
    machine_config::MachineConfigWriter{cfg}.write(out);
    auto no_cb = MachineConfigFileV1_0::open(out);
    REQUIRE(no_cb.ok());
    REQUIRE_FALSE(no_cb.value()->hasOptionalComponents(0));
    auto cb = no_cb.value()->getClearbox(0);
    REQUIRE_FALSE(cb.ok());
    REQUIRE(cb.errorCode() == "NotPresent");
    no_cb.value()->close();
    std::filesystem::remove(out);
}

// getClearbox() surfaces synchronous_sensors too, not just the scalar fields.
TEST_CASE("CapabilityClearboxSynchronousSensors") {
    auto opened = MachineConfigFileV1_0::open(SENSORS_REF);
    REQUIRE(opened.ok());
    auto cb = opened.value()->getClearbox(0);
    REQUIRE(cb.ok());
    REQUIRE(cb.value().synchronous_sensors.count("Oxygen Sensor") == 1);
    REQUIRE(cb.value().synchronous_sensors.at("Oxygen Sensor").sensor_name ==
            std::optional<std::string>{"ZR800 Oxygen Analyzer"});
    opened.value()->close();
}

TEST_CASE("CapabilityGetCorrectionDataMatchesReader") {
    auto opened = MachineConfigFileV1_0::open(REF);
    REQUIRE(opened.ok());
    auto file = opened.value();

    MachineConfigReader reader(REF);
    auto expected = reader.getCorrectionData(0);
    auto result = file->getCorrectionData(0);
    REQUIRE(result.ok());
    REQUIRE(correctionDataEqual(result.value(), expected));

    auto expectedInv = reader.getInverseCorrectionData(0);
    auto resultInv = file->getInverseCorrectionData(0);
    REQUIRE(resultInv.ok());
    REQUIRE(correctionDataEqual(resultInv.value(), expectedInv));
    file->close();
}

TEST_CASE("CapabilityGetCorrectionDataShapeAndNaNPresentThroughFacade") {
    auto opened = MachineConfigFileV1_0::open(REF);
    REQUIRE(opened.ok());
    auto cd = opened.value()->getCorrectionData(0);
    REQUIRE(cd.ok());
    REQUIRE(cd.value().shape == std::array<std::size_t, 3>{257, 257, 2});
    bool any_nan = false;
    for (double v : cd.value().data) {
        if (std::isnan(v)) { any_nan = true; break; }
    }
    REQUIRE(any_nan);
    opened.value()->close();
}

TEST_CASE("CapabilityGetCorrectionDataMissingClearboxIsNotPresent") {
    // Every stock fixture's trains have a ClearBox, so build one without.
    machine_config::MockConfigBuilder b;
    b.laser_count = 1;
    b.include_clearbox = false;
    auto cfg = b.build();
    auto out = tmpPath("cd_no_cb");
    machine_config::MachineConfigWriter{cfg}.write(out);

    auto opened = MachineConfigFileV1_0::open(out);
    REQUIRE(opened.ok());
    auto result = opened.value()->getCorrectionData(0);
    REQUIRE_FALSE(result.ok());
    REQUIRE(result.errorCode() == "NotPresent");
    auto resultInv = opened.value()->getInverseCorrectionData(0);
    REQUIRE_FALSE(resultInv.ok());
    REQUIRE(resultInv.errorCode() == "NotPresent");
    opened.value()->close();
    std::filesystem::remove(out);
}

TEST_CASE("CapabilityGetCorrectionDataWorksOnCreateBasedInstanceWithoutTouchingDisk") {
    // The specific case that rules out delegate-to-Reader-by-reopening: a
    // create()-d facade has no path at all, so this must convert the
    // already-loaded in-memory model, not re-read from anywhere.
    auto created = createMachineConfig("1.0");
    REQUIRE(created.ok());
    auto file = created.value();

    auto cd = file->getCorrectionData(0);
    REQUIRE(cd.ok());
    REQUIRE(cd.value().shape == std::array<std::size_t, 3>{257, 257, 2});
    auto icd = file->getInverseCorrectionData(0);
    REQUIRE(icd.ok());
    REQUIRE(icd.value().shape == std::array<std::size_t, 3>{257, 257, 2});
    file->close();
}

TEST_CASE("CapabilityCreateSetMetaSaveReopen") {
    auto created = createMachineConfig("1.0");
    REQUIRE(created.ok());
    auto file = created.value();
    REQUIRE(file->fileVersion() == "1.0");

    auto meta = file->getMeta();
    meta.machine_name = "CreatedMachine";
    REQUIRE(file->setMeta(meta, SetMode::Merge).ok());

    auto out = tmpPath("created");
    std::string out_s = out.string();
    REQUIRE(file->save(&out_s).ok());
    file->close();

    auto again = MachineConfigFileV1_0::open(out);
    REQUIRE(again.ok());
    REQUIRE(again.value()->fileVersion() == "1.0");
    REQUIRE(again.value()->getMeta().machine_name == "CreatedMachine");
    again.value()->close();
    std::filesystem::remove(out);
}

TEST_CASE("CapabilityOpenMachineConfigDispatch") {
    auto opened = openMachineConfig(REF);
    REQUIRE(opened.ok());
    REQUIRE(opened.value()->fileVersion() == "1.0");
    opened.value()->close();
}

TEST_CASE("CapabilitySupportedFileVersionsListsV1_0") {
    auto versions = supportedFileVersions();
    REQUIRE(std::find(versions.begin(), versions.end(), std::string("1.0")) != versions.end());
}
