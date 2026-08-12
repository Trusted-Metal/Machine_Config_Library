// Catch2 tests for the stable model facade (File_Version 1.0).
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include <filesystem>
#include <string>

#include "machine_config/capabilities.hpp"
#include "machine_config/reader.hpp"

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

using machine_config::MachineConfigReader;
using machine_config::capabilities::MachineConfigFileV10;
using machine_config::capabilities::SetMode;
using machine_config::capabilities::createMachineConfig;
using machine_config::capabilities::openMachineConfig;

static const std::string REF       = std::string(FIXTURES_DIR) + "/reference_config.h5";
static const std::string OPCUA_REF = std::string(FIXTURES_DIR) + "/reference_config_opcua.h5";

static std::filesystem::path tmpPath(const std::string& tag) {
    return std::filesystem::temp_directory_path() /
           ("mc_cap_test_" + tag + ".h5");
}

TEST_CASE("CapabilityOpenGetScannerMatchesReader") {
    auto opened = MachineConfigFileV10::open(REF);
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
    auto opened = MachineConfigFileV10::open(REF);
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

    auto again = MachineConfigFileV10::open(out);
    REQUIRE(again.ok());
    auto after = again.value()->getScanner(0);
    REQUIRE(after.ok());
    REQUIRE_THAT(*after.value().working_distance, Catch::Matchers::WithinRel(123.5));
    REQUIRE(after.value().manufacturer == manufacturer);
    again.value()->close();
    std::filesystem::remove(out);
}

TEST_CASE("CapabilityReplaceSetScanner") {
    auto opened = MachineConfigFileV10::open(REF);
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
    auto opened = MachineConfigFileV10::open(REF);
    REQUIRE(opened.ok());
    auto bad = opened.value()->getTrain(999);
    REQUIRE_FALSE(bad.ok());
    REQUIRE(bad.errorCode() == "InvalidIndex");
    REQUIRE(opened.value()->opticalTrainCount() >= 1);
    opened.value()->close();
}

TEST_CASE("CapabilityOpcuaNotPresentVsPresent") {
    auto no_opc = MachineConfigFileV10::open(REF);
    REQUIRE(no_opc.ok());
    auto missing = no_opc.value()->getOpcua();
    REQUIRE_FALSE(missing.ok());
    REQUIRE(missing.errorCode() == "NotPresent");
    no_opc.value()->close();

    auto with_opc = MachineConfigFileV10::open(OPCUA_REF);
    REQUIRE(with_opc.ok());
    REQUIRE(with_opc.value()->getOpcua().ok());
    with_opc.value()->close();
}

TEST_CASE("CapabilityClearboxOptional") {
    auto opened = MachineConfigFileV10::open(REF);
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
    auto no_cb = MachineConfigFileV10::open(out);
    REQUIRE(no_cb.ok());
    REQUIRE_FALSE(no_cb.value()->hasOptionalComponents(0));
    auto cb = no_cb.value()->getClearbox(0);
    REQUIRE_FALSE(cb.ok());
    REQUIRE(cb.errorCode() == "NotPresent");
    no_cb.value()->close();
    std::filesystem::remove(out);
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

    auto again = MachineConfigFileV10::open(out);
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
