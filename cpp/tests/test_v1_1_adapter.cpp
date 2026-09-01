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

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

using namespace machine_config;
using namespace machine_config::capabilities::v1_1;

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
// touching disk — mirrors exactly what a real v1.1 file read produces under
// Phase 2 (power_characterization populated, flat fields nullopt), so tests
// exercising a writer's fallback behave the same as they would against real
// v1.1-sourced data.
static MachineConfig mockV1_1Config(std::size_t laserCount = 2) {
    MachineConfig cfg = mockV1Config(laserCount);
    cfg.meta.file_version = "1.1";
    for (auto& train : cfg.optical_trains) {
        if (train.optional_components.clearbox) {
            auto& cb = *train.optional_components.clearbox;
            cb.power_characterization =
                forwardPowerCharacterizationCoefficients(cb.volts_to_watts_algorithm, cb.volts_to_watts_params);
            cb.volts_to_watts_algorithm = std::nullopt;
            cb.volts_to_watts_params = std::nullopt;
        }
        auto& ls = train.light_source;
        ls.power_characterization =
            forwardPowerCharacterizationPoints(ls.watts_to_volts_algorithm, ls.watts_to_volts_params);
        ls.watts_to_volts_algorithm = std::nullopt;
        ls.watts_to_volts_params = std::nullopt;
    }
    return cfg;
}

static std::vector<double> csvFloats(const std::optional<std::string>& s) {
    return pcParseCsvFloats(s);
}

// ---------------------------------------------------------------------------
// test_v1_1_read
// ---------------------------------------------------------------------------

TEST_CASE("v1_1 read: natively-authored fixture, all 5 changes asserted") {
    MachineConfig cfg = MachineConfigReader{REFERENCE_V1_1}.parse();
    REQUIRE(cfg.meta.file_version == "1.1");

    for (const auto& t : cfg.optical_trains) {
        REQUIRE(t.optional_components.clearbox.has_value());
        const ClearBox& cb = *t.optional_components.clearbox;
        // Change 1: Consolidate — same shared value on every train.
        REQUIRE(cb.output_path == std::optional<std::string>{"/recordings/"});
        REQUIRE(cb.software_trigger_delay == std::optional<int64_t>{3000});
        // Change 1: Addition.
        REQUIRE(cb.firmware_version == std::optional<std::string>{"2.4.1"});
        // Change 1: Removal.
        REQUIRE_FALSE(cb.selected_camera.has_value());
        REQUIRE_FALSE(cb.custom_video_format.has_value());
        REQUIRE_FALSE(cb.video_output.has_value());
        REQUIRE_FALSE(cb.show_console.has_value());
        REQUIRE_FALSE(cb.correction_grid_domain_shape.has_value());
        REQUIRE_FALSE(cb.inverse_grid_domain_shape.has_value());
        // Change 3: superseded — v1.1's reader reads only its own native
        // shape (Phase 2 does not change this); volts_to_watts_* stay
        // nullopt unless/until this model is written as v1.0.
        REQUIRE_FALSE(cb.volts_to_watts_algorithm.has_value());
        REQUIRE_FALSE(cb.volts_to_watts_params.has_value());

        // Change 5: no on-disk source in this File_Version, always nullopt.
        REQUIRE_FALSE(t.scanner.x_axis.tuning_parameters.has_value());
        REQUIRE_FALSE(t.scanner.x_axis.tuning_type.has_value());
        REQUIRE_FALSE(t.scanner.y_axis.tuning_parameters.has_value());
        REQUIRE_FALSE(t.scanner.y_axis.tuning_type.has_value());

        // Change 4: superseded, same reasoning as Change 3.
        REQUIRE_FALSE(t.light_source.watts_to_volts_algorithm.has_value());
        REQUIRE_FALSE(t.light_source.watts_to_volts_params.has_value());
    }

    // Change 2: OPCUA relocated, contents unaffected.
    REQUIRE(cfg.opcua.has_value());
    REQUIRE(cfg.opcua->client.machine_profile.has_value());

    // Change 3: ClearBox Power_Characterization — train 1 LINEAR, train 2 POLYNOMIAL.
    const ClearBox& cb0 = *cfg.optical_trains[0].optional_components.clearbox;
    REQUIRE(cb0.power_characterization.has_value());
    const PowerCharacterization& pc0 = *cb0.power_characterization;
    REQUIRE(pc0.algorithm_type == std::optional<std::string>{"LINEAR"});
    REQUIRE(pc0.algorithm_equation == std::optional<std::string>{"W = a*V + b"});
    REQUIRE(pc0.input_type == std::optional<std::string>{"0-10 V"});
    REQUIRE(pc0.units_derived_quantity == std::optional<std::string>{"Watts"});
    REQUIRE(pc0.derivation_equation_constants.size() == 2);
    REQUIRE(pc0.derivation_equation_constants[0].name == "b");
    REQUIRE(pc0.derivation_equation_constants[1].name == "a");
    REQUIRE(pc0.characterization_points.size() == 3);

    const ClearBox& cb1 = *cfg.optical_trains[1].optional_components.clearbox;
    const PowerCharacterization& pc1 = *cb1.power_characterization;
    REQUIRE(pc1.algorithm_type == std::optional<std::string>{"POLYNOMIAL"});
    REQUIRE(pc1.algorithm_equation == std::optional<std::string>{"W = c0 + c1*V + c2*V^2"});
    REQUIRE(pc1.derivation_equation_constants.size() == 3);
    REQUIRE(pc1.derivation_equation_constants[0].name == "c0");
    REQUIRE(pc1.derivation_equation_constants[1].name == "c1");
    REQUIRE(pc1.derivation_equation_constants[2].name == "c2");

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
    OpticalTrain train = cfg.optical_trains[0];

    ClearBox cb = *train.optional_components.clearbox;
    cb.power_characterization = PowerCharacterization{
        "LINEAR", std::nullopt, std::nullopt, std::nullopt,
        {EquationConstant{"b", 1.0}, EquationConstant{"a", 2.0}}, {}};
    train.optional_components.clearbox = cb;

    LightSource ls = train.light_source;
    ls.power_characterization = PowerCharacterization{
        "POLYNOMIAL", std::nullopt, std::nullopt, std::nullopt,
        {}, {CalibrationPoint{9.0, 99.0}}};
    train.light_source = ls;

    cfg.optical_trains = {train};

    auto p1 = tmpPath("roundtrip_a");
    MachineConfigWriter{cfg}.write(p1);
    MachineConfig mid = MachineConfigReader{p1}.parse();

    auto p2 = tmpPath("roundtrip_b");
    MachineConfigWriter{mid}.write(p2);
    MachineConfig result = MachineConfigReader{p2}.parse();

    const ClearBox& cbR = *result.optical_trains[0].optional_components.clearbox;
    const LightSource& lsR = result.optical_trains[0].light_source;
    REQUIRE(cbR.power_characterization->algorithm_type == std::optional<std::string>{"LINEAR"});
    REQUIRE(cbR.power_characterization->derivation_equation_constants.size() == 2);
    REQUIRE(cbR.power_characterization->derivation_equation_constants[0].name == "b");
    REQUIRE(cbR.power_characterization->derivation_equation_constants[1].name == "a");
    REQUIRE(cbR.power_characterization->characterization_points.empty());
    REQUIRE(lsR.power_characterization->algorithm_type == std::optional<std::string>{"POLYNOMIAL"});
    REQUIRE(lsR.power_characterization->derivation_equation_constants.empty());
    REQUIRE(lsR.power_characterization->characterization_points.size() == 1);
    REQUIRE(lsR.power_characterization->characterization_points[0].input_value == 9.0);
    // Cross-wiring guard: the two instances must not have swapped.
    REQUIRE(cbR.power_characterization->algorithm_type != lsR.power_characterization->algorithm_type);
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

    for (std::size_t i = 0; i < migrated.optical_trains.size(); ++i) {
        const ClearBox& srcCb = *source.optical_trains[i].optional_components.clearbox;
        const ClearBox& cb = *migrated.optical_trains[i].optional_components.clearbox;
        // Consolidate: same per-train value carried straight through.
        REQUIRE(cb.output_path == srcCb.output_path);
        REQUIRE(cb.software_trigger_delay == srcCb.software_trigger_delay);
        // Addition: no v1.0 source.
        REQUIRE_FALSE(cb.firmware_version.has_value());
        // Removal.
        REQUIRE_FALSE(cb.selected_camera.has_value());
        REQUIRE_FALSE(cb.custom_video_format.has_value());
        // Change 3: derived ClearBox data has real constants, zero points.
        REQUIRE(cb.power_characterization->characterization_points.empty());
        REQUIRE_FALSE(cb.power_characterization->derivation_equation_constants.empty());
        REQUIRE(cb.power_characterization->algorithm_type == srcCb.volts_to_watts_algorithm);

        const LightSource& ls = migrated.optical_trains[i].light_source;
        // Change 4: derived Light_Source data is the inverse — zero
        // constants, real points.
        REQUIRE(ls.power_characterization->derivation_equation_constants.empty());
        REQUIRE_FALSE(ls.power_characterization->characterization_points.empty());
    }
}

TEST_CASE("v1_1 forward: Output_Path disagreement raises") {
    MachineConfig cfg = mockV1Config(2);
    cfg.optical_trains[1].optional_components.clearbox->output_path = "/other/";
    REQUIRE_THROWS_WITH(
        MachineConfigWriter(cfg, "1.1").write(tmpPath("output_path_conflict")),
        Catch::Matchers::ContainsSubstring("Consolidate conflict on 'Output_Path'"));
}

TEST_CASE("v1_1 forward: Software_Trigger_Delay disagreement raises") {
    MachineConfig cfg = mockV1Config(2);
    cfg.optical_trains[1].optional_components.clearbox->software_trigger_delay = 9999;
    REQUIRE_THROWS_WITH(
        MachineConfigWriter(cfg, "1.1").write(tmpPath("software_trigger_delay_conflict")),
        Catch::Matchers::ContainsSubstring("Consolidate conflict on 'Software_Trigger_Delay'"));
}

TEST_CASE("v1_1 forward: unrecognized ClearBox Algorithm_Type is best-effort, never raises") {
    MachineConfig cfg = mockV1Config(1);
    cfg.optical_trains[0].optional_components.clearbox->volts_to_watts_algorithm = "EXPONENTIAL";
    cfg.optical_trains[0].optional_components.clearbox->volts_to_watts_params = "1.5,2.5,3.5";
    auto out = tmpPath("unrecognized_clearbox");
    MachineConfigWriter(cfg, "1.1").write(out);
    MachineConfig result = MachineConfigReader{out}.parse();
    const PowerCharacterization& pc = *result.optical_trains[0].optional_components.clearbox->power_characterization;
    REQUIRE(pc.algorithm_type == std::optional<std::string>{"EXPONENTIAL"});
    REQUIRE_FALSE(pc.algorithm_equation.has_value());
    REQUIRE(pc.derivation_equation_constants.size() == 3);
    REQUIRE(pc.derivation_equation_constants[0].name == "0");
    REQUIRE(pc.derivation_equation_constants[1].name == "1");
    REQUIRE(pc.derivation_equation_constants[2].name == "2");
    REQUIRE(pc.derivation_equation_constants[0].value == 1.5);
    REQUIRE(pc.derivation_equation_constants[1].value == 2.5);
    REQUIRE(pc.derivation_equation_constants[2].value == 3.5);

    // Round trip: writing back to v1.0 reproduces the original CSV exactly.
    auto v1_0Out = tmpPath("unrecognized_clearbox_roundtrip");
    MachineConfigWriter(result, "1.0").write(v1_0Out);
    MachineConfig back = MachineConfigReader{v1_0Out}.parse();
    REQUIRE(back.optical_trains[0].optional_components.clearbox->volts_to_watts_params ==
            std::optional<std::string>{"1.5,2.5,3.5"});
}

TEST_CASE("v1_1 forward: unrecognized LightSource Algorithm_Type is best-effort, never raises") {
    MachineConfig cfg = mockV1Config(1);
    cfg.optical_trains[0].light_source.watts_to_volts_algorithm = "QUADRATIC";
    cfg.optical_trains[0].light_source.watts_to_volts_params = "1,2,3,4";
    auto out = tmpPath("unrecognized_lightsource");
    MachineConfigWriter(cfg, "1.1").write(out);
    MachineConfig result = MachineConfigReader{out}.parse();
    const PowerCharacterization& pc = *result.optical_trains[0].light_source.power_characterization;
    REQUIRE(pc.algorithm_type == std::optional<std::string>{"QUADRATIC"});
    REQUIRE_FALSE(pc.algorithm_equation.has_value());
    REQUIRE(pc.characterization_points.size() == 2);
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

    const ClearBox& srcCb0 = *source.optical_trains[0].optional_components.clearbox;
    const ClearBox& cb0 = *back.optical_trains[0].optional_components.clearbox;
    const PowerCharacterization& srcPc0 = *srcCb0.power_characterization;
    REQUIRE(cb0.volts_to_watts_algorithm == srcPc0.algorithm_type);
    auto expected = [&] {
        std::vector<double> v;
        for (const auto& c : srcPc0.derivation_equation_constants) v.push_back(c.value);
        std::sort(v.begin(), v.end());
        return v;
    }();
    auto actual = csvFloats(cb0.volts_to_watts_params);
    std::sort(actual.begin(), actual.end());
    REQUIRE(actual == expected);
    REQUIRE(cb0.output_path == srcCb0.output_path);
    REQUIRE(cb0.software_trigger_delay == srcCb0.software_trigger_delay);

    const LightSource& srcLs0 = source.optical_trains[0].light_source;
    const LightSource& ls0 = back.optical_trains[0].light_source;
    const PowerCharacterization& srcLsPc0 = *srcLs0.power_characterization;
    REQUIRE(ls0.watts_to_volts_algorithm == srcLsPc0.algorithm_type);
    std::vector<double> expectedPoints;
    for (const auto& p : srcLsPc0.characterization_points) {
        expectedPoints.push_back(p.input_value);
        expectedPoints.push_back(p.output_value);
    }
    REQUIRE(csvFloats(ls0.watts_to_volts_params) == expectedPoints);

    // Change 1 Removal fields: lost forever, not restored.
    REQUIRE_FALSE(cb0.selected_camera.has_value());
}

TEST_CASE("v1_1 backward reorders Derivation_Equation_Constants by name") {
    MachineConfig cfg = mockV1_1Config(1);
    ClearBox cb0 = *cfg.optical_trains[0].optional_components.clearbox;
    cb0.power_characterization->algorithm_type = "LINEAR";
    cb0.power_characterization->derivation_equation_constants = {
        EquationConstant{"a", 2.0}, EquationConstant{"b", 1.0}};
    cfg.optical_trains[0].optional_components.clearbox = cb0;

    auto out = tmpPath("backward_reorders");
    MachineConfigWriter(cfg, "1.0").write(out);
    MachineConfig back = MachineConfigReader{out}.parse();
    REQUIRE(back.optical_trains[0].optional_components.clearbox->volts_to_watts_params ==
            std::optional<std::string>{"1.0,2.0"});
}

TEST_CASE("v1_1 backward: missing Derivation_Equation_Constants writes blank") {
    MachineConfig cfg = mockV1_1Config(1);
    ClearBox cb0 = *cfg.optical_trains[0].optional_components.clearbox;
    cb0.power_characterization->derivation_equation_constants = {};
    cfg.optical_trains[0].optional_components.clearbox = cb0;

    auto out = tmpPath("backward_blank_constants");
    MachineConfigWriter(cfg, "1.0").write(out);
    MachineConfig back = MachineConfigReader{out}.parse();
    // The writer's fallback produces "" (blank, not an error) — but a real
    // disk round-trip normalizes an empty string attribute back to nullopt
    // on read, this codebase's standard convention for absent optional
    // strings (confirmed against Python's/Rust's identical finding).
    REQUIRE_FALSE(back.optical_trains[0].optional_components.clearbox->volts_to_watts_params.has_value());
}

TEST_CASE("v1_1 backward: missing Characterization_Points writes blank") {
    MachineConfig cfg = mockV1_1Config(1);
    LightSource ls0 = cfg.optical_trains[0].light_source;
    ls0.power_characterization->characterization_points = {};
    cfg.optical_trains[0].light_source = ls0;

    auto out = tmpPath("backward_blank_points");
    MachineConfigWriter(cfg, "1.0").write(out);
    MachineConfig back = MachineConfigReader{out}.parse();
    REQUIRE_FALSE(back.optical_trains[0].light_source.watts_to_volts_params.has_value());
}

// ---------------------------------------------------------------------------
// test_v1_unaffected
// ---------------------------------------------------------------------------

TEST_CASE("v1.0 read path is undisturbed by v1.1's existence") {
    // Phase 2 does not touch either reader at all (Option C: derivation
    // lives only in the writers' fallbacks).
    MachineConfig config = MachineConfigReader{REFERENCE_V1_0}.parse();
    REQUIRE(config.meta.file_version == "1.0");
    REQUIRE_FALSE(config.optical_trains.empty());
    REQUIRE(config.optical_trains[0].optional_components.clearbox->volts_to_watts_algorithm ==
            std::optional<std::string>{"LINEAR"});
    REQUIRE_FALSE(config.optical_trains[0].optional_components.clearbox->power_characterization.has_value());
}

TEST_CASE("v1.0 write never invokes the fallback when native data is present") {
    // The writer's fallback must never fire when the native field is
    // already present — a deliberately odd-but-valid string must pass
    // through byte-for-byte, proving Hdf5WriterV1_0 never reformats via
    // power_characterization when there's nothing for the fallback to do.
    MachineConfig cfg = mockV1Config(1);
    cfg.optical_trains[0].optional_components.clearbox->volts_to_watts_params = "50.50,107.500";
    auto out = tmpPath("v1_0_verbatim");
    MachineConfigWriter{cfg}.write(out);
    MachineConfig result = MachineConfigReader{out}.parse();
    REQUIRE(result.optical_trains[0].optional_components.clearbox->volts_to_watts_params ==
            std::optional<std::string>{"50.50,107.500"});
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
    for (std::size_t i = 0; i < back.optical_trains.size(); ++i) {
        const ClearBox& srcCb = *source.optical_trains[i].optional_components.clearbox;
        const ClearBox& cb = *back.optical_trains[i].optional_components.clearbox;
        REQUIRE(cb.output_path == srcCb.output_path);
        REQUIRE(cb.software_trigger_delay == srcCb.software_trigger_delay);
        REQUIRE(cb.volts_to_watts_algorithm == srcCb.volts_to_watts_algorithm);
        REQUIRE(csvFloats(cb.volts_to_watts_params) == csvFloats(srcCb.volts_to_watts_params));

        const LightSource& srcLs = source.optical_trains[i].light_source;
        const LightSource& ls = back.optical_trains[i].light_source;
        REQUIRE(ls.watts_to_volts_algorithm == srcLs.watts_to_volts_algorithm);
        REQUIRE(csvFloats(ls.watts_to_volts_params) == csvFloats(srcLs.watts_to_volts_params));
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
        const ClearBox& srcCb = *source.optical_trains[i].optional_components.clearbox;
        const ClearBox& cb = *back.optical_trains[i].optional_components.clearbox;
        const PowerCharacterization& srcPc = *srcCb.power_characterization;
        const PowerCharacterization& pc = *cb.power_characterization;
        REQUIRE(pc.algorithm_type == srcPc.algorithm_type);
        auto sorted = [](std::vector<EquationConstant> v) {
            std::vector<double> out;
            for (const auto& c : v) out.push_back(c.value);
            std::sort(out.begin(), out.end());
            return out;
        };
        REQUIRE(sorted(pc.derivation_equation_constants) == sorted(srcPc.derivation_equation_constants));
        // Expected loss: v1.1-only fields have no v1.0 round-trip path.
        REQUIRE_FALSE(cb.firmware_version.has_value());
        REQUIRE_FALSE(pc.input_type.has_value());
        REQUIRE_FALSE(pc.units_derived_quantity.has_value());
        REQUIRE(pc.characterization_points.empty());
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
    REQUIRE(result.optical_trains[0].optional_components.clearbox->output_path ==
            std::optional<std::string>{"/recordings/"});
    REQUIRE(result.optical_trains[0].optional_components.clearbox->power_characterization.has_value());
}

TEST_CASE("v1_1 dispatcher: v1.1 fixture written as v1.0 through the public API") {
    MachineConfig v1_1Cfg = MachineConfigReader{REFERENCE_V1_1}.parse();
    auto out = tmpPath("dispatcher_backward");
    MachineConfigWriter{v1_1Cfg, "1.0"}.write(out);
    MachineConfig result = MachineConfigReader{out}.parse();
    REQUIRE(result.meta.file_version == "1.0");
    REQUIRE(result.optical_trains[0].optional_components.clearbox->volts_to_watts_algorithm ==
            std::optional<std::string>{"LINEAR"});
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
