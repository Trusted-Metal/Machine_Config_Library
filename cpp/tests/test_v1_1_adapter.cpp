// File_Version 1.1 adapter tests — the real v1.1 adapter (not the mock in
// test_adapter_migration.cpp). Mirrors python/tests/test_v1_1_adapter.py and
// rust/tests/v1_1_adapter_test.rs, adapted to this project's C++ idiom (no
// dataclasses.replace/struct update syntax — copies are mutated directly
// field-by-field).
//
// Phase 2 (V1_1_IMPLEMENTATION_PLAN.md) removed migrateV1ToV1_1/
// migrateV1_1ToV1 — upgrade/downgrade is now just parse()/write(), using
// MachineConfigWriter's two-argument (targetVersion) constructor. Every test
// below that used to call a migrate function directly now goes through a
// real HDF5 write+read instead — a genuine strengthening, since it now
// exercises the same code path a real caller uses.
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <algorithm>
#include <filesystem>
#include <memory>
#include <string>

#include "machine_config/machine_config.hpp"
#include "clearbox_helpers.hpp"

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

using namespace machine_config;
using namespace machine_config::capabilities::v1_1;
namespace extension = machine_config::extension;

static const std::string REFERENCE_V1_0 =
    std::string(FIXTURES_DIR) + "/reference_config_opcua_synchronous_sensors.h5";
static const std::string REFERENCE_V1_1 =
    std::string(FIXTURES_DIR) + "/reference_config_v1_1.h5";

static std::filesystem::path tmpPath(const std::string& tag) {
    return std::filesystem::temp_directory_path() /
           ("mc_v1_1_adapter_test_" + tag + ".h5");
}

static MachineConfig mockV1Config(std::size_t laserCount = 2) {
    MockConfigBuilder b;
    b.laser_count = laserCount;
    return b.build();
}

// In-memory v1.1-shaped MachineConfig for tests that need one without
// touching disk — MockConfigBuilder already builds power_characterization
// natively, so this is just the v1.0 mock with file_version overridden.
static MachineConfig mockV1_1Config(std::size_t laserCount = 2) {
    MachineConfig cfg = mockV1Config(laserCount);
    cfg.meta.file_version = "1.1";
    return cfg;
}

// Flattened constant/point values, in on-disk order — for comparing a
// PowerCharacterization's derived data without caring about its name/label.
static std::vector<double> constantValues(const PowerCharacterization& pc) {
    std::vector<double> v;
    for (const auto& c : pc.derivation_equation_constants) v.push_back(c.value);
    return v;
}

static std::vector<double> pointValues(const PowerCharacterization& pc) {
    std::vector<double> v;
    for (const auto& p : pc.characterization_points) {
        v.push_back(p.input_value);
        v.push_back(p.output_value);
    }
    return v;
}

// ---------------------------------------------------------------------------
// test_v1_1_read
// ---------------------------------------------------------------------------

static std::vector<std::string> cbNames(const extension::Tree& pc) {
    std::vector<std::string> out;
    for (const auto& c : cbtest::constants(pc)) out.push_back(c.first);
    return out;
}

static std::vector<double> cbValues(const extension::Tree& pc) {
    std::vector<double> out;
    for (const auto& c : cbtest::constants(pc)) out.push_back(c.second);
    return out;
}

TEST_CASE("v1_1 read: natively-authored fixture, all 5 changes asserted") {
    MachineConfig cfg = MachineConfigReader{REFERENCE_V1_1}.parse();
    REQUIRE(cfg.meta.file_version == "1.1");

    // Change 1: Consolidate — one shared value, on the extension root.
    const auto& root = cbtest::clearboxRoot(cfg);
    std::int64_t v = 0;
    REQUIRE(cbtest::text(root, "Output_Path") == "/recordings/");
    REQUIRE(cbtest::integer(root, "Software_Trigger_Delay", v));
    REQUIRE(v == 3000);

    for (std::size_t i = 0; i < cfg.optical_trains.size(); ++i) {
        const auto& cb = cbtest::clearbox(cfg, i);
        // Change 1: Addition.
        REQUIRE(cbtest::text(cb, "Firmware_Version") == "2.4.1");
        // Change 1: Removal.
        for (const char* removed : {"Selected_Camera", "Custom_Video_Format", "Video_Output", "Show_Console",
                                    "Output_Path", "Software_Trigger_Delay"})
            REQUIRE_FALSE(cbtest::hasAttr(cb, removed));

        // Change 5: no on-disk source in this File_Version, always nullopt.
        const auto& t = cfg.optical_trains[i];
        REQUIRE_FALSE(t.scanner.x_axis.tuning_parameters.has_value());
        REQUIRE_FALSE(t.scanner.x_axis.tuning_type.has_value());
        REQUIRE_FALSE(t.scanner.y_axis.tuning_parameters.has_value());
        REQUIRE_FALSE(t.scanner.y_axis.tuning_type.has_value());
    }

    // Change 2: the legacy OPCUA group is carried as the opaque "TM_OPCUA" extension, contents unaffected.
    REQUIRE(cfg.extensions.count("TM_OPCUA") == 1);
    REQUIRE(cfg.extensions.at("TM_OPCUA").children.at("Client").attrs.count("Machine_Profile") == 1);

    // Change 3: ClearBox Power_Characterization — train 1 LINEAR, train 2 POLYNOMIAL.
    const auto& pc0 = cbtest::powerCharacterization(cfg, 0);
    REQUIRE(cbtest::text(pc0, "Algorithm_Type") == "LINEAR");
    REQUIRE(cbtest::text(pc0, "Algorithm_Equation") == "W = a*V + b");
    REQUIRE(cbtest::text(pc0, "Input_Type") == "0-10 V");
    REQUIRE(cbtest::text(pc0, "Units_Derived_Quantity") == "Watts");
    REQUIRE(cbNames(pc0) == std::vector<std::string>{"b", "a"});
    REQUIRE(cbtest::pointCount(pc0) == 3);

    const auto& pc1 = cbtest::powerCharacterization(cfg, 1);
    REQUIRE(cbtest::text(pc1, "Algorithm_Type") == "POLYNOMIAL");
    REQUIRE(cbtest::text(pc1, "Algorithm_Equation") == "W = c0 + c1*V + c2*V^2");
    REQUIRE(cbNames(pc1) == std::vector<std::string>{"c0", "c1", "c2"});

    // Change 4: LightSource Power_Characterization — inverse data availability.
    const PowerCharacterization& lspc0 = *cfg.optical_trains[0].light_source.power_characterization;
    REQUIRE(lspc0.algorithm_type == std::optional<std::string>{"LINEAR"});
    REQUIRE(lspc0.input_type == std::optional<std::string>{"Volts"});
    REQUIRE(lspc0.characterization_points.size() == 5);
    REQUIRE_FALSE(lspc0.derivation_equation_constants.empty());
}

// ---------------------------------------------------------------------------
// test_v1_1_roundtrip
// ---------------------------------------------------------------------------

TEST_CASE("v1_1 roundtrip: write -> read -> write -> read, cross-wiring guard") {
    MachineConfig cfg = mockV1_1Config(1);
    cbtest::clearbox(cfg, 0).children["Power_Characterization"] =
        cbtest::makePowerCharacterization("LINEAR", {{"b", 1.0}, {"a", 2.0}});

    LightSource ls = cfg.optical_trains[0].light_source;
    ls.power_characterization = PowerCharacterization{
        "POLYNOMIAL", std::nullopt, std::nullopt, std::nullopt,
        {}, {CalibrationPoint{9.0, 99.0}}};
    cfg.optical_trains[0].light_source = ls;

    auto p1 = tmpPath("roundtrip_a");
    MachineConfigWriter{cfg}.write(p1);
    MachineConfig mid = MachineConfigReader{p1}.parse();

    auto p2 = tmpPath("roundtrip_b");
    MachineConfigWriter{mid}.write(p2);
    MachineConfig result = MachineConfigReader{p2}.parse();

    const auto& cbPc = cbtest::powerCharacterization(result, 0);
    const LightSource& lsR = result.optical_trains[0].light_source;
    REQUIRE(cbtest::text(cbPc, "Algorithm_Type") == "LINEAR");
    REQUIRE(cbNames(cbPc) == std::vector<std::string>{"b", "a"});
    REQUIRE(cbtest::pointCount(cbPc) == 0);
    REQUIRE(lsR.power_characterization->algorithm_type == std::optional<std::string>{"POLYNOMIAL"});
    REQUIRE(lsR.power_characterization->derivation_equation_constants.empty());
    REQUIRE(lsR.power_characterization->characterization_points.size() == 1);
    REQUIRE(lsR.power_characterization->characterization_points[0].input_value == 9.0);
    // Cross-wiring guard: the two instances must not have swapped.
    REQUIRE(cbtest::text(cbPc, "Algorithm_Type") != *lsR.power_characterization->algorithm_type);
    // The whole ClearBox tree, grids included, is bit-stable across the double round trip.
    REQUIRE(extension::canonicalDigest(result.extensions.at("ClearBox")) ==
            extension::canonicalDigest(cfg.extensions.at("ClearBox")));
}

// ---------------------------------------------------------------------------
// test_v1_to_v1_1 (v1.0 fixture written as v1.1)
// ---------------------------------------------------------------------------

TEST_CASE("v1_1 forward: v1.0 fixture written as v1.1, manifest rules") {
    MachineConfig source = MachineConfigReader{REFERENCE_V1_0}.parse();
    auto out = tmpPath("forward_from_v1_0");
    MachineConfigWriter{source, "1.1"}.write(out);
    MachineConfig migrated = MachineConfigReader{out}.parse();
    REQUIRE(migrated.meta.file_version == "1.1");

    // The shimmed tree is already in the v1.1 layout, so it survives unchanged.
    REQUIRE(extension::canonicalDigest(migrated.extensions.at("ClearBox")) ==
            extension::canonicalDigest(source.extensions.at("ClearBox")));
    const auto& root = cbtest::clearboxRoot(migrated);
    REQUIRE(cbtest::hasAttr(root, "Output_Path"));
    REQUIRE(cbtest::hasAttr(root, "Software_Trigger_Delay"));

    for (std::size_t i = 0; i < migrated.optical_trains.size(); ++i) {
        const auto& cb = cbtest::clearbox(migrated, i);
        // Addition: no v1.0 source.
        REQUIRE_FALSE(cbtest::hasAttr(cb, "Firmware_Version"));
        // v1.0-only attributes ride along as extra per-train attributes.
        REQUIRE(cbtest::hasAttr(cb, "Selected_Camera"));
        // Change 3: derived ClearBox data has real constants, zero points.
        const auto& pc = cbtest::powerCharacterization(migrated, i);
        REQUIRE(cbtest::pointCount(pc) == 0);
        REQUIRE_FALSE(cbtest::constants(pc).empty());
        REQUIRE(cbtest::text(pc, "Algorithm_Type") ==
                cbtest::text(cbtest::powerCharacterization(source, i), "Algorithm_Type"));

        const LightSource& ls = migrated.optical_trains[i].light_source;
        // Change 4: derived Light_Source data is the inverse — zero
        // constants, real points.
        REQUIRE(ls.power_characterization->derivation_equation_constants.empty());
        REQUIRE_FALSE(ls.power_characterization->characterization_points.empty());
    }
}

TEST_CASE("v1_0: the first train wins for the shared Output_Path / Software_Trigger_Delay") {
    // File_Version 1.0 stores them per train; the extension tree holds one value each.
    MachineConfig cfg = mockV1Config(2);
    auto& second = cbtest::clearbox(cfg, 1);
    second.attrs["Output_Path"] = {"utf8", std::string("/other/")};
    second.attrs["Software_Trigger_Delay"] = {"i64", std::int64_t{9999}};

    auto out = tmpPath("first_train_wins");
    MachineConfigWriter(cfg, "1.0").write(out);
    MachineConfig back = MachineConfigReader{out}.parse();
    const auto& root = cbtest::clearboxRoot(back);
    std::int64_t v = 0;
    REQUIRE(cbtest::text(root, "Output_Path") == "/recordings/");
    REQUIRE(cbtest::integer(root, "Software_Trigger_Delay", v));
    REQUIRE(v == 3000);
    REQUIRE_FALSE(cbtest::hasAttr(cbtest::clearbox(back, 1), "Output_Path"));
}

TEST_CASE("v1_1 forward: unrecognized ClearBox Algorithm_Type is best-effort, never raises") {
    MachineConfig cfg = mockV1_1Config(1);
    cbtest::clearbox(cfg, 0).children["Power_Characterization"] =
        cbtest::makePowerCharacterization("EXPONENTIAL", {{"0", 1.5}, {"1", 2.5}, {"2", 3.5}});

    auto v1_0Out = tmpPath("unrecognized_clearbox_v1_0");
    MachineConfigWriter(cfg, "1.0").write(v1_0Out);
    MachineConfig v1_0 = MachineConfigReader{v1_0Out}.parse();
    const auto& pc = cbtest::powerCharacterization(v1_0, 0);
    REQUIRE(cbtest::text(pc, "Algorithm_Type") == "EXPONENTIAL");
    REQUIRE(cbtest::text(pc, "Algorithm_Equation").empty());
    REQUIRE(cbNames(pc) == std::vector<std::string>{"0", "1", "2"});
    REQUIRE(cbValues(pc) == std::vector<double>{1.5, 2.5, 3.5});

    auto v1_1Out = tmpPath("unrecognized_clearbox_v1_1");
    MachineConfigWriter(v1_0, "1.1").write(v1_1Out);
    MachineConfig back = MachineConfigReader{v1_1Out}.parse();
    const auto& backPc = cbtest::powerCharacterization(back, 0);
    REQUIRE(cbtest::text(backPc, "Algorithm_Type") == "EXPONENTIAL");
    REQUIRE(cbValues(backPc) == std::vector<double>{1.5, 2.5, 3.5});
}

TEST_CASE("v1_1 forward: unrecognized LightSource Algorithm_Type is best-effort, never raises") {
    MachineConfig cfg = mockV1Config(1);
    cfg.optical_trains[0].light_source.power_characterization =
        forwardPowerCharacterizationPoints("QUADRATIC", "1,2,3,4");
    auto out = tmpPath("unrecognized_lightsource");
    MachineConfigWriter(cfg, "1.1").write(out);
    MachineConfig result = MachineConfigReader{out}.parse();
    const PowerCharacterization& pc = *result.optical_trains[0].light_source.power_characterization;
    REQUIRE(pc.algorithm_type == std::optional<std::string>{"QUADRATIC"});
    REQUIRE_FALSE(pc.algorithm_equation.has_value());
    REQUIRE(pc.characterization_points.size() == 2);
}

TEST_CASE("a ClearBox train with no matching optical train is rejected for v1.0") {
    MachineConfig cfg = mockV1_1Config(1);
    cfg.extensions.at("ClearBox").children["Optical_Train_07"] = cbtest::clearbox(cfg, 0);
    REQUIRE_THROWS_WITH(MachineConfigWriter(cfg, "1.0").write(tmpPath("no_matching_train")),
                        Catch::Matchers::ContainsSubstring("Optical_Train_07"));
}

// ---------------------------------------------------------------------------
// test_v1_1_to_v1 (v1.1 fixture written as v1.0)
// ---------------------------------------------------------------------------

TEST_CASE("v1_1 backward: v1.1 fixture written as v1.0, surviving fields preserved") {
    MachineConfig source = MachineConfigReader{REFERENCE_V1_1}.parse();
    auto out = tmpPath("backward_from_v1_1");
    MachineConfigWriter{source, "1.0"}.write(out);
    MachineConfig back = MachineConfigReader{out}.parse();
    REQUIRE(back.meta.file_version == "1.0");

    const auto& srcPc0 = cbtest::powerCharacterization(source, 0);
    const auto& pc0 = cbtest::powerCharacterization(back, 0);
    REQUIRE(cbtest::text(pc0, "Algorithm_Type") == cbtest::text(srcPc0, "Algorithm_Type"));
    auto expected = cbValues(srcPc0);
    std::sort(expected.begin(), expected.end());
    auto actual = cbValues(pc0);
    std::sort(actual.begin(), actual.end());
    REQUIRE(actual == expected);

    const auto& srcRoot = cbtest::clearboxRoot(source);
    const auto& root = cbtest::clearboxRoot(back);
    REQUIRE(cbtest::text(root, "Output_Path") == cbtest::text(srcRoot, "Output_Path"));
    std::int64_t a = 0, b = 0;
    REQUIRE(cbtest::integer(root, "Software_Trigger_Delay", a));
    REQUIRE(cbtest::integer(srcRoot, "Software_Trigger_Delay", b));
    REQUIRE(a == b);

    const PowerCharacterization& srcLsPc0 = *source.optical_trains[0].light_source.power_characterization;
    const PowerCharacterization& lsPc0 = *back.optical_trains[0].light_source.power_characterization;
    REQUIRE(lsPc0.algorithm_type == srcLsPc0.algorithm_type);
    REQUIRE(pointValues(lsPc0) == pointValues(srcLsPc0));

    // v1.1-only fields: lost on the way down, not restored.
    REQUIRE_FALSE(cbtest::hasAttr(cbtest::clearbox(back, 0), "Firmware_Version"));
    REQUIRE_FALSE(cbtest::hasAttr(cbtest::clearbox(back, 0), "Selected_Camera"));
}

TEST_CASE("v1_1 backward reorders Derivation_Equation_Constants by name") {
    MachineConfig cfg = mockV1_1Config(1);
    cbtest::clearbox(cfg, 0).children["Power_Characterization"] =
        cbtest::makePowerCharacterization("LINEAR", {{"a", 2.0}, {"b", 1.0}});

    auto out = tmpPath("backward_reorders");
    MachineConfigWriter(cfg, "1.0").write(out);
    MachineConfig back = MachineConfigReader{out}.parse();
    // Written CSV is "1.0,2.0" (b before a); the v1.0 reader forward-derives
    // that back into named constants positionally (b, then a) for LINEAR.
    const auto got = cbtest::constants(cbtest::powerCharacterization(back, 0));
    REQUIRE(got.size() == 2);
    REQUIRE(got[0].first == "b");
    REQUIRE(got[0].second == 1.0);
    REQUIRE(got[1].first == "a");
    REQUIRE(got[1].second == 2.0);
}

TEST_CASE("v1_1 backward: missing Derivation_Equation_Constants writes blank") {
    MachineConfig cfg = mockV1_1Config(1);
    cbtest::clearbox(cfg, 0).children["Power_Characterization"] =
        cbtest::makePowerCharacterization("LINEAR", {});

    auto out = tmpPath("backward_blank_constants");
    MachineConfigWriter(cfg, "1.0").write(out);
    MachineConfig back = MachineConfigReader{out}.parse();
    // Backward derivation produces "" for Volts_To_Watts_Params (blank, not an error); forward
    // re-derivation on read then produces zero constants.
    REQUIRE(cbtest::constants(cbtest::powerCharacterization(back, 0)).empty());
}

TEST_CASE("v1_1 backward: missing Characterization_Points writes blank") {
    MachineConfig cfg = mockV1_1Config(1);
    LightSource ls0 = cfg.optical_trains[0].light_source;
    ls0.power_characterization->characterization_points = {};
    cfg.optical_trains[0].light_source = ls0;

    auto out = tmpPath("backward_blank_points");
    MachineConfigWriter(cfg, "1.0").write(out);
    MachineConfig back = MachineConfigReader{out}.parse();
    REQUIRE(back.optical_trains[0].light_source.power_characterization->characterization_points.empty());
}

// ---------------------------------------------------------------------------
// test_v1_unaffected
// ---------------------------------------------------------------------------

TEST_CASE("v1.0 read path surfaces the flat Volts_To_Watts fields as Power_Characterization") {
    MachineConfig config = MachineConfigReader{REFERENCE_V1_0}.parse();
    REQUIRE(config.meta.file_version == "1.0");
    REQUIRE_FALSE(config.optical_trains.empty());
    REQUIRE(cbtest::text(cbtest::powerCharacterization(config, 0), "Algorithm_Type") == "LINEAR");
    REQUIRE_FALSE(cbtest::hasAttr(cbtest::clearbox(config, 0), "Volts_To_Watts_Algorithm"));
}

// ---------------------------------------------------------------------------
// Round-trip tests — the acceptance criterion stated 2026-09-01, made concrete
// ---------------------------------------------------------------------------

TEST_CASE("roundtrip v1.0 -> v1.1 -> v1.0 reproduces the original exactly") {
    MachineConfig source = MachineConfigReader{REFERENCE_V1_0}.parse();
    auto v1_1Path = tmpPath("roundtrip_up");
    MachineConfigWriter{source, "1.1"}.write(v1_1Path);
    MachineConfig mid = MachineConfigReader{v1_1Path}.parse();
    auto v1_0Path = tmpPath("roundtrip_down");
    MachineConfigWriter{mid, "1.0"}.write(v1_0Path);
    MachineConfig back = MachineConfigReader{v1_0Path}.parse();

    REQUIRE(back.meta.file_version == "1.0");
    REQUIRE(extension::canonicalDigest(back.extensions.at("ClearBox")) ==
            extension::canonicalDigest(source.extensions.at("ClearBox")));
    for (std::size_t i = 0; i < back.optical_trains.size(); ++i) {
        const LightSource& srcLs = source.optical_trains[i].light_source;
        const LightSource& ls = back.optical_trains[i].light_source;
        REQUIRE(ls.power_characterization->algorithm_type == srcLs.power_characterization->algorithm_type);
        REQUIRE(pointValues(*ls.power_characterization) == pointValues(*srcLs.power_characterization));
    }
}

TEST_CASE("roundtrip v1.1 -> v1.0 -> v1.1: fields v1.0 can hold survive, the rest comes back blank") {
    MachineConfig source = MachineConfigReader{REFERENCE_V1_1}.parse();
    auto v1_0Path = tmpPath("roundtrip_mirror_down");
    MachineConfigWriter{source, "1.0"}.write(v1_0Path);
    MachineConfig mid = MachineConfigReader{v1_0Path}.parse();
    auto v1_1Path = tmpPath("roundtrip_mirror_up");
    MachineConfigWriter{mid, "1.1"}.write(v1_1Path);
    MachineConfig back = MachineConfigReader{v1_1Path}.parse();

    REQUIRE(back.meta.file_version == "1.1");
    for (std::size_t i = 0; i < back.optical_trains.size(); ++i) {
        const auto& srcPc = cbtest::powerCharacterization(source, i);
        const auto& pc = cbtest::powerCharacterization(back, i);
        REQUIRE(cbtest::text(pc, "Algorithm_Type") == cbtest::text(srcPc, "Algorithm_Type"));
        auto sorted = [](std::vector<double> v) { std::sort(v.begin(), v.end()); return v; };
        REQUIRE(sorted(cbValues(pc)) == sorted(cbValues(srcPc)));
        // Expected loss: v1.1-only fields have no v1.0 round-trip path.
        REQUIRE_FALSE(cbtest::hasAttr(cbtest::clearbox(back, i), "Firmware_Version"));
        REQUIRE(cbtest::text(pc, "Input_Type").empty());
        REQUIRE(cbtest::text(pc, "Units_Derived_Quantity").empty());
        REQUIRE(cbtest::pointCount(pc) == 0);
    }
}

// ---------------------------------------------------------------------------
// Writer targetVersion parameter
// ---------------------------------------------------------------------------

TEST_CASE("writer targetVersion overrides meta, without mutating the input") {
    MachineConfig source = MachineConfigReader{REFERENCE_V1_0}.parse();
    REQUIRE(source.meta.file_version == "1.0");
    auto out = tmpPath("target_version_up");
    MachineConfigWriter{source, "1.1"}.write(out);
    MachineConfig result = MachineConfigReader{out}.parse();
    REQUIRE(result.meta.file_version == "1.1");
    REQUIRE(source.meta.file_version == "1.0");

    MachineConfig v1_1Source = MachineConfigReader{REFERENCE_V1_1}.parse();
    REQUIRE(v1_1Source.meta.file_version == "1.1");
    auto out2 = tmpPath("target_version_down");
    MachineConfigWriter{v1_1Source, "1.0"}.write(out2);
    MachineConfig result2 = MachineConfigReader{out2}.parse();
    REQUIRE(result2.meta.file_version == "1.0");
    REQUIRE(v1_1Source.meta.file_version == "1.1");
}

TEST_CASE("writer defaults to meta.file_version when no targetVersion is given") {
    MachineConfig source = MachineConfigReader{REFERENCE_V1_0}.parse();
    auto out = tmpPath("target_version_default");
    MachineConfigWriter{source}.write(out);  // no targetVersion
    MachineConfig result = MachineConfigReader{out}.parse();
    REQUIRE(result.meta.file_version == "1.0");
}

// ---------------------------------------------------------------------------
// Dispatcher-level tests — full public API via the real "1.1" registry entry.
// ---------------------------------------------------------------------------

TEST_CASE("v1_1 dispatcher: v1.0 fixture written as v1.1 through the public API") {
    MachineConfig source = MachineConfigReader{REFERENCE_V1_0}.parse();
    auto out = tmpPath("dispatcher_forward");
    MachineConfigWriter{source, "1.1"}.write(out);
    MachineConfig result = MachineConfigReader{out}.parse();
    REQUIRE(result.meta.file_version == "1.1");
    REQUIRE(cbtest::text(cbtest::clearboxRoot(result), "Output_Path") == "/recordings/");
    REQUIRE(cbtest::powerCharacterization(result, 0).tables.count("Derivation_Equation_Constants") == 1);
}

TEST_CASE("v1_1 dispatcher: v1.1 fixture written as v1.0 through the public API") {
    MachineConfig v1_1Cfg = MachineConfigReader{REFERENCE_V1_1}.parse();
    auto out = tmpPath("dispatcher_backward");
    MachineConfigWriter{v1_1Cfg, "1.0"}.write(out);
    MachineConfig result = MachineConfigReader{out}.parse();
    REQUIRE(result.meta.file_version == "1.0");
    REQUIRE(cbtest::text(cbtest::powerCharacterization(result, 0), "Algorithm_Type") == "LINEAR");
}

// ---------------------------------------------------------------------------
// Adapters satisfy the shared ReaderAdapter/WriterAdapter interfaces.
// ---------------------------------------------------------------------------

TEST_CASE("v1_1 adapters satisfy the ReaderAdapter/WriterAdapter interfaces") {
    MachineConfig cfg = mockV1_1Config(1);
    auto p = tmpPath("adapters_satisfy_interface");
    Hdf5WriterV1_1{cfg}.write(p);

    std::unique_ptr<ReaderAdapter> reader = std::make_unique<Hdf5AdapterV1_1>(p);
    std::unique_ptr<WriterAdapter> writer = std::make_unique<Hdf5WriterV1_1>(cfg);
    REQUIRE(reader != nullptr);
    REQUIRE(writer != nullptr);
    REQUIRE(reader->parse().meta.file_version == "1.1");
}
