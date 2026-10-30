// AV-09-AV-11: mock v1.1 adapter migration tests.
//
// Mirrors python/tests/test_adapter_migration.py's adapter-level tests
// (test_v1_1_read, test_v1_1_roundtrip, test_v1_to_v1_1, test_v1_1_to_v1,
// test_v1_unaffected), rust/tests/adapter_migration_test.rs, and
// nodejs/tests/adapterMigration.test.ts.
//
// Deliberately missing, by design (see VALIDATION_PLAN.md §9.5 and
// mock_v1_1.hpp's file doc): the *dispatcher-level* tests other languages
// have, which prove the public MachineConfigReader/Writer facade itself
// routes to the mock via a temporarily-injected dispatch-table entry. This
// library's public dispatcher is a hardcoded `if (ver != "1.0") throw ...`
// in reader.hpp/writer.hpp, not a registry — there's no entry to inject.
// AV-09's actual rationale ("adding v1.1 doesn't require modifying the v1.0
// adapter") is satisfied here by the mock living in its own header with
// zero edits to capabilities/v1_0/, plus the full pre-existing suite
// (v1_unaffected below, and every other test in this binary) staying green.
#include <catch2/catch_test_macros.hpp>

#include <filesystem>
#include <string>

#include "machine_config/machine_config.hpp"
#include "mock_v1_1.hpp"

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

using namespace machine_config;
using namespace machine_config::mock_v1_1;

static const std::string REF = std::string(FIXTURES_DIR) + "/reference_config.h5";

static std::filesystem::path tmpPath(const std::string& tag) {
    return std::filesystem::temp_directory_path() /
           ("mc_adapter_migration_test_" + tag + ".h5");
}

// Mock v1.1 file -> StableModel -- all five change categories asserted.
TEST_CASE("v1_1_read_all_categories") {
    MachineConfig cfg = makeMockConfig("MigrationTestMachine", "Lab-001", "TestEngineer");
    auto p = tmpPath("read");
    MockV1_1Writer{cfg}.write(p);
    MachineConfig result = MockV1_1Reader{p}.parse();

    // ADDITION (x2)
    REQUIRE(result.meta.facility_id == std::optional<std::string>{"Lab-001"});
    REQUIRE(result.meta.config_author == std::optional<std::string>{"TestEngineer"});

    // REMOVAL (x2)
    REQUIRE_FALSE(result.machine.gas_flow_direction.has_value());
    REQUIRE_FALSE(result.machine.recoat_direction.has_value());

    // NAME (x2)
    REQUIRE(result.machine.machine_name == "MigrationTestMachine");
    REQUIRE(result.optical_trains[0].scanner.working_distance == cfg.optical_trains[0].scanner.working_distance);

    // PATH (x2)
    REQUIRE(result.machine.build_plate_z == cfg.machine.build_plate_z);
    REQUIRE(result.machine.build_plate_radius == cfg.machine.build_plate_radius);

    // NAME+PATH (x2)
    REQUIRE(result.machine.build_plate_x == cfg.machine.build_plate_x);
    REQUIRE(result.machine.build_plate_y == cfg.machine.build_plate_y);
}

// Mock v1.1 -> StableModel -> mock v1.1 -> StableModel -- all categories
// survive both passes.
TEST_CASE("v1_1_roundtrip") {
    MachineConfig cfg = makeMockConfig("MigrationTestMachine", "RoundtripLab", "RoundtripEngineer");
    auto p1 = tmpPath("roundtrip_a");
    MockV1_1Writer{cfg}.write(p1);
    MachineConfig mid = MockV1_1Reader{p1}.parse();

    auto p2 = tmpPath("roundtrip_b");
    MockV1_1Writer{mid}.write(p2);
    MachineConfig result = MockV1_1Reader{p2}.parse();

    REQUIRE(result.meta.facility_id == std::optional<std::string>{"RoundtripLab"});
    REQUIRE(result.meta.config_author == std::optional<std::string>{"RoundtripEngineer"});
    REQUIRE_FALSE(result.machine.gas_flow_direction.has_value());
    REQUIRE_FALSE(result.machine.recoat_direction.has_value());
    REQUIRE(result.machine.machine_name == cfg.machine.machine_name);
    REQUIRE(result.optical_trains[0].scanner.working_distance == cfg.optical_trains[0].scanner.working_distance);
    REQUIRE(result.machine.build_plate_z == cfg.machine.build_plate_z);
    REQUIRE(result.machine.build_plate_radius == cfg.machine.build_plate_radius);
    REQUIRE(result.machine.build_plate_x == cfg.machine.build_plate_x);
    REQUIRE(result.machine.build_plate_y == cfg.machine.build_plate_y);
}

// Real v1.0 fixture -> StableModel -> mock v1.1 layout -- surviving fields
// preserved; ADDITION fields nullopt (no v1.0 source).
TEST_CASE("v1_to_v1_1_forward_migration") {
    MachineConfig source = MachineConfigReader{REF}.parse();
    MachineConfig migratedInput = source;
    migratedInput.meta.file_version = "1.1-mock";

    auto out = tmpPath("forward");
    MockV1_1Writer{migratedInput}.write(out);
    MachineConfig result = MockV1_1Reader{out}.parse();

    REQUIRE(result.machine.machine_name == source.machine.machine_name);
    REQUIRE(result.machine.build_plate_x == source.machine.build_plate_x);
    REQUIRE(result.machine.build_plate_y == source.machine.build_plate_y);
    REQUIRE(result.machine.build_plate_z == source.machine.build_plate_z);
    REQUIRE(result.machine.build_plate_radius == source.machine.build_plate_radius);
    REQUIRE(result.optical_trains[0].scanner.working_distance == source.optical_trains[0].scanner.working_distance);

    REQUIRE_FALSE(result.machine.gas_flow_direction.has_value());
    REQUIRE_FALSE(result.machine.recoat_direction.has_value());
    REQUIRE_FALSE(result.meta.facility_id.has_value());
    REQUIRE_FALSE(result.meta.config_author.has_value());
}

// Mock v1.1 file -> StableModel -> v1.0 layout -- surviving fields
// preserved; ADDITION fields lost (v1.0 writer doesn't write them).
TEST_CASE("v1_1_to_v1_backward_migration") {
    MachineConfig cfg = makeMockConfig("MigrationTestMachine", "Lab-V11", "MigrationBot");

    auto v11File = tmpPath("backward_v11");
    MockV1_1Writer{cfg}.write(v11File);
    MachineConfig v11Config = MockV1_1Reader{v11File}.parse();

    MachineConfig downgradeInput = v11Config;
    downgradeInput.meta.file_version = "1.0";

    auto v1Out = tmpPath("backward_v1");
    MachineConfigWriter{downgradeInput}.write(v1Out);
    MachineConfig result = MachineConfigReader{v1Out}.parse();

    REQUIRE(result.machine.machine_name == cfg.machine.machine_name);
    REQUIRE(result.machine.build_plate_x == cfg.machine.build_plate_x);
    REQUIRE(result.machine.build_plate_y == cfg.machine.build_plate_y);
    REQUIRE(result.machine.build_plate_z == cfg.machine.build_plate_z);
    REQUIRE(result.machine.build_plate_radius == cfg.machine.build_plate_radius);
    REQUIRE(result.optical_trains[0].scanner.working_distance == cfg.optical_trains[0].scanner.working_distance);

    // REMOVAL: absent in v1.1 -> remain nullopt after roundtrip through v1.0
    REQUIRE_FALSE(result.machine.gas_flow_direction.has_value());
    REQUIRE_FALSE(result.machine.recoat_direction.has_value());

    // ADDITION: typed v1.1 fields are lost during backward migration
    REQUIRE_FALSE(result.meta.facility_id.has_value());
    REQUIRE_FALSE(result.meta.config_author.has_value());
}

// Existing v1.0 read path is undisturbed -- no mock adapter involved. This,
// combined with every other test in this binary staying green after the
// mock was added, is AV-09's actual proof: adding v1.1 required zero edits
// to the v1.0 adapter.
TEST_CASE("v1_unaffected") {
    MachineConfig config = MachineConfigReader{REF}.parse();
    REQUIRE(config.meta.file_version == "1.0");
    REQUIRE_FALSE(config.optical_trains.empty());
    REQUIRE(config.machine.build_plate_x.has_value());
}
