#pragma once
// Mock v1.1 adapter — test artifact only, exercises the change-category
// architecture (Addition/Removal/Name/Path/Name+Path), not a planned schema
// change. See docs/migrations/mock_v1_0_to_v1_1.md for the full manifest.
//
// Design: the reader/writer *delegate* to the real, public v1.0 adapter for
// the whole file, then patch exactly the 10 documented differences, rather
// than reimplementing or reaching into v1.0's private parsing internals.
// "Unchanged" subcomponents (light_source, collimator, scanner_card,
// clearbox, sfcf, opcua) are never re-tested here — they're already covered
// by the existing suite exercising the real v1.0 adapter. Mirrors
// nodejs/tests/mockV1_1.ts's and rust/tests/mock_v1_1/mod.rs's design.
//
// Goes one step further than Node's/Rust's mocks: this library's individual
// attribute helpers (readStr/readFloat/readRequiredStr in hdf5.hpp; ws/wf in
// writer.hpp) are namespace-scope `inline` functions in `machine_config`,
// not private to a class — so the patch step reuses those exact primitives
// instead of re-deriving its own (e.g. re-implementing the VarLen-string
// SpacePadded/NullTerminated workaround readRaw() already has for HDF5
// 1.14 would just be a second copy that could drift from the real one).
//
// Lives in cpp/tests/ as a plain header (not test-only conditionally
// compiled) because this library is header-only with no visibility barrier
// at all — every header under include/machine_config/ is already reachable
// from any test file, unlike Rust's pub/private boundary.
//
// No dispatch-table injection, unlike Python's _ADAPTERS or Node's
// _READERS/_WRITERS: this library's public dispatcher
// (MachineConfigReader/MachineConfigWriter) is a hardcoded
// `if (ver != "1.0") throw ...`, not a registry — there is no seam to
// inject a "1.1-mock" entry into. AV-09-11 therefore call this mock
// directly rather than through the public facade (see
// VALIDATION_PLAN.md §9.5).

#include "machine_config/machine_config.hpp"
#include "machine_config/capabilities/v1_0/hdf5.hpp"
#include "machine_config/capabilities/v1_0/layout.hpp"
#include "machine_config/capabilities/v1_0/writer.hpp"

#include <highfive/H5File.hpp>

#include <optional>
#include <string>

namespace machine_config::mock_v1_1 {

inline constexpr const char* FILE_VERSION = "1.1-mock";
inline constexpr const char* ATTR_FACILITY_ID = "Facility_ID";
inline constexpr const char* ATTR_CONFIG_AUTHOR = "Config_Author";
inline constexpr const char* ATTR_MACHINE_LABEL = "Machine_Label";
inline constexpr const char* ATTR_FOCAL_DISTANCE = "Focal_Distance";
inline constexpr const char* ATTR_BP_WIDTH = "Width";
inline constexpr const char* ATTR_BP_HEIGHT = "Height";
inline constexpr const char* SUBGROUP_DIMENSIONS = "Dimensions";

// ---------------------------------------------------------------------------
// MockV1_1Reader — delegates to the real v1.0 parser, then patches the delta
// ---------------------------------------------------------------------------

class MockV1_1Reader {
public:
    explicit MockV1_1Reader(std::filesystem::path path) : path_(std::move(path)) {}

    MachineConfig parse() const {
        // Step 1: the real v1.0 parser correctly reads every subcomponent
        // this mock doesn't change. Fields it can't find (renamed, moved, or
        // removed) come back nullopt/empty — never a throw, since none of
        // the 10 changes remove or rename a *group*, only attributes within
        // one, or an attribute's presence.
        MachineConfig cfg = capabilities::v1_0::Hdf5AdapterV1_0(path_).parse();

        // Step 2: patch exactly the 10 documented differences by reading
        // their real, mock-v1.1 locations directly, reusing the same
        // readStr/readFloat/readRequiredStr the real reader uses.
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        HighFive::Group machineGrp = f.getGroup(capabilities::v1_0::ROOT_MACHINE);
        std::optional<HighFive::Group> dims;
        if (machineGrp.exist(SUBGROUP_DIMENSIONS)) {
            dims = machineGrp.getGroup(SUBGROUP_DIMENSIONS);
        }

        cfg.meta.facility_id = readStr(f, ATTR_FACILITY_ID);
        cfg.meta.config_author = readStr(f, ATTR_CONFIG_AUTHOR);
        cfg.machine.machine_name = readRequiredStr(machineGrp, ATTR_MACHINE_LABEL);
        cfg.machine.build_plate_x = dims ? readFloat(*dims, ATTR_BP_WIDTH) : std::nullopt;
        cfg.machine.build_plate_y = dims ? readFloat(*dims, ATTR_BP_HEIGHT) : std::nullopt;
        cfg.machine.build_plate_z = dims ? readFloat(*dims, "Build_Plate_Z_Dimension") : std::nullopt;
        cfg.machine.build_plate_radius = dims ? readFloat(*dims, "Build_Plate_Corner_Radius") : std::nullopt;
        // gas_flow_direction / recoat_direction: already nullopt from the
        // base parse (the attrs are genuinely absent) — no patch needed.

        for (std::size_t i = 0; i < cfg.optical_trains.size(); ++i) {
            std::string scannerPath = capabilities::v1_0::trainPath(i) + "/Scanner";
            if (f.exist(scannerPath)) {
                auto scannerGrp = f.getGroup(scannerPath);
                cfg.optical_trains[i].scanner.working_distance = readFloat(scannerGrp, ATTR_FOCAL_DISTANCE);
            }
        }

        // The base parser doesn't know Facility_ID/Config_Author are typed
        // fields, so it swept them into meta.extra as unknown attrs. Strip
        // them out there before returning, or they'd exist in both places —
        // and get written twice, colliding, on the next write. This is the
        // exact bug found and fixed in Node's and Rust's mocks this session.
        if (cfg.meta.extra.is_object()) {
            cfg.meta.extra.erase(ATTR_FACILITY_ID);
            cfg.meta.extra.erase(ATTR_CONFIG_AUTHOR);
        }

        return cfg;
    }

private:
    std::filesystem::path path_;
};

// ---------------------------------------------------------------------------
// MockV1_1Writer — writes a real v1.0-shaped file, then patches the delta
// ---------------------------------------------------------------------------

class MockV1_1Writer {
public:
    explicit MockV1_1Writer(const MachineConfig& cfg) : cfg_(cfg) {}

    void write(std::filesystem::path path) const {
        // Step 1: the real v1.0 writer correctly writes every subcomponent
        // this mock doesn't change, plus File_Version itself (already
        // "1.1-mock" on the input config — the writer just persists
        // whatever string is there).
        capabilities::v1_0::Hdf5WriterV1_0{cfg_}.write(path);

        // Step 2: patch exactly the 10 documented differences in place,
        // reusing the same ws/wf helpers the real writer uses.
        HighFive::File f(path.string(), HighFive::File::ReadWrite);

        // ADDITION (x2)
        ws(f, ATTR_FACILITY_ID, cfg_.meta.facility_id.value_or(""));
        ws(f, ATTR_CONFIG_AUTHOR, cfg_.meta.config_author.value_or(""));

        HighFive::Group machine = f.getGroup(capabilities::v1_0::ROOT_MACHINE);

        // REMOVAL (x2)
        deleteIfPresent(machine, "Gas_Flow_Direction");
        deleteIfPresent(machine, "Recoat_Direction");

        // NAME (Machine_Name -> Machine_Label)
        renameAttrStr(machine, "Machine_Name", ATTR_MACHINE_LABEL);

        // PATH / NAME+PATH: build-plate values move into Machine/Dimensions/.
        // Unit attrs are unaffected — they stay on Machine/ per the manifest.
        HighFive::Group dims = machine.exist(SUBGROUP_DIMENSIONS)
            ? machine.getGroup(SUBGROUP_DIMENSIONS)
            : machine.createGroup(SUBGROUP_DIMENSIONS);
        moveAttrF64(machine, dims, "Build_Plate_X_Dimension", ATTR_BP_WIDTH);
        moveAttrF64(machine, dims, "Build_Plate_Y_Dimension", ATTR_BP_HEIGHT);
        moveAttrF64(machine, dims, "Build_Plate_Z_Dimension", "Build_Plate_Z_Dimension");
        moveAttrF64(machine, dims, "Build_Plate_Corner_Radius", "Build_Plate_Corner_Radius");

        // NAME (Working_Distance -> Focal_Distance), once per optical train.
        for (std::size_t i = 0; i < cfg_.optical_trains.size(); ++i) {
            std::string scannerPath = capabilities::v1_0::trainPath(i) + "/Scanner";
            if (f.exist(scannerPath)) {
                auto scannerGrp = f.getGroup(scannerPath);
                moveAttrF64(scannerGrp, scannerGrp, "Working_Distance", ATTR_FOCAL_DISTANCE);
            }
        }
    }

private:
    const MachineConfig& cfg_;

    static void deleteIfPresent(HighFive::Group& loc, const std::string& key) {
        if (loc.hasAttribute(key)) loc.deleteAttribute(key);
    }

    static void renameAttrStr(HighFive::Group& loc, const std::string& oldKey, const std::string& newKey) {
        if (!loc.hasAttribute(oldKey)) return;
        std::string val = readRequiredStr(loc, oldKey);
        ws(loc, newKey, val);
        loc.deleteAttribute(oldKey);
    }

    // from/to may be the same group (a same-group rename) or different
    // groups (a move).
    static void moveAttrF64(HighFive::Group& from, HighFive::Group& to,
                             const std::string& oldKey, const std::string& newKey) {
        if (!from.hasAttribute(oldKey)) return;
        std::optional<double> val = readFloat(from, oldKey);
        wf(to, newKey, val);
        from.deleteAttribute(oldKey);
    }
};

// ---------------------------------------------------------------------------
// Factory helper — mirrors Python's _make_config() / Node's makeMockConfig
// ---------------------------------------------------------------------------

inline MachineConfig makeMockConfig(
    const std::string& machineName = "MigrationTestMachine",
    std::optional<std::string> facilityId = std::nullopt,
    std::optional<std::string> configAuthor = std::nullopt)
{
    MockConfigBuilder builder;
    builder.laser_count = 1;
    builder.build_plate_x = 250.0;
    builder.build_plate_y = 175.0; // distinct from x so name+path assertions are unambiguous
    builder.machine_name = machineName;

    MachineConfig cfg = builder.build();
    cfg.meta.file_version = FILE_VERSION;
    cfg.meta.facility_id = std::move(facilityId);
    cfg.meta.config_author = std::move(configAuthor);
    cfg.machine.gas_flow_direction = std::nullopt; // absent in v1.1-mock by design
    cfg.machine.recoat_direction = std::nullopt;
    // MockConfigBuilder never sets this — give it a real value so the PATH
    // category (Build_Plate_Corner_Radius) has something non-trivial to
    // verify preservation of, mirroring Rust's mock_v1_1 factory.
    cfg.machine.build_plate_radius = 10.0;
    return cfg;
}

} // namespace machine_config::mock_v1_1
